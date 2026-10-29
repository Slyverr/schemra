import type { BufferTarget, Output, StreamTarget, StreamTargetChunk, VideoCodec } from 'mediabunny';
import { studioMessage } from '../i18n/messages.js';

export interface ExportSettings {
	format: 'mp4' | 'webm';
	width: number;
	height: number;
	fps: number;
	videoBitsPerSecond: number;
	codec: VideoCodec;
	background: string;
	maxOutputBytes: number;
	directSave: boolean;
}

export type ExportWorkerRequest =
	| { id: number; kind: 'start'; settings: ExportSettings }
	| { id: number; kind: 'frame'; bitmap: ImageBitmap; timestamp: number; duration: number }
	| { id: number; kind: 'finish' }
	| { id: number; kind: 'cancel' }
	| { kind: 'write-ack'; writeId: number; error?: string };

export type ExportWorkerPayload = ExportWorkerRequest extends infer Request
	? Request extends { id: number }
		? Omit<Request, 'id'>
		: never
	: never;

export type ExportWorkerReply =
	| { id: number; kind: 'ready' }
	| { id: number; kind: 'frame-done' }
	| { id: number; kind: 'complete'; buffer?: ArrayBuffer }
	| { kind: 'write'; writeId: number; chunk: StreamTargetChunk }
	| { kind: 'stream-close'; writeId: number }
	| { kind: 'stream-abort'; writeId: number }
	| { id: number; kind: 'error'; message: string; name: string };

interface ActiveExport {
	canvas: OffscreenCanvas;
	context: OffscreenCanvasRenderingContext2D;
	source: import('mediabunny').CanvasSource;
	output: Output;
	target: BufferTarget | StreamTarget;
	settings: ExportSettings;
	exceededOutputLimit: boolean;
}

let active: ActiveExport | undefined;
let nextWriteId = 0;
const pendingWrites = new Map<number, { resolve: () => void; reject: (error: Error) => void }>();

const workerScope = self as unknown as {
	onmessage: ((event: MessageEvent<ExportWorkerRequest>) => void) | null;
	postMessage(message: ExportWorkerReply, transfer?: Transferable[]): void;
};

workerScope.onmessage = (event) => {
	if (event.data.kind === 'write-ack') {
		const pending = pendingWrites.get(event.data.writeId);
		if (!pending) return;
		pendingWrites.delete(event.data.writeId);
		if (event.data.error) pending.reject(new Error(event.data.error));
		else pending.resolve();
		return;
	}
	void handleRequest(event.data);
};

async function handleRequest(request: Exclude<ExportWorkerRequest, { kind: 'write-ack' }>) {
	try {
		switch (request.kind) {
			case 'start':
				await startExport(request.settings);
				workerScope.postMessage({ id: request.id, kind: 'ready' });
				return;
			case 'frame':
				await addFrame(request.bitmap, request.timestamp, request.duration);
				workerScope.postMessage({ id: request.id, kind: 'frame-done' });
				return;
			case 'finish': {
				const buffer = await finishExport();
				workerScope.postMessage(
					buffer ? { id: request.id, kind: 'complete', buffer } : { id: request.id, kind: 'complete' },
					buffer ? [buffer] : []
				);
				return;
			}
			case 'cancel':
				await cancelExport();
				workerScope.postMessage({ id: request.id, kind: 'ready' });
				return;
		}
	} catch (error) {
		await cancelExport();
		workerScope.postMessage({
			id: request.id,
			kind: 'error',
			message: error instanceof Error ? error.message : String(error),
			name: error instanceof Error ? error.name : 'Error'
		});
	}
}

async function startExport(settings: ExportSettings) {
	if (active) throw new Error(studioMessage('export.alreadyRunning'));
	const canvas = new OffscreenCanvas(settings.width, settings.height);
	const context = canvas.getContext('2d', { alpha: false });
	if (!context) throw new Error(studioMessage('export.rendererInitializeFailed'));
	context.imageSmoothingEnabled = true;
	context.imageSmoothingQuality = 'high';

	const mediabunny = await import('mediabunny');
	const target = settings.directSave
		? new mediabunny.StreamTarget(new WritableStream<StreamTargetChunk>({
				write: sendChunk,
				close: () => sendControl('close'),
				abort: () => sendControl('abort')
			}), { chunked: false })
		: new mediabunny.BufferTarget();
	const output = new mediabunny.Output({
		format: settings.format === 'mp4'
			? new mediabunny.Mp4OutputFormat({ fastStart: false })
			: new mediabunny.WebMOutputFormat(),
		target
	});
	const source = new mediabunny.CanvasSource(canvas, {
		codec: settings.codec,
		quality: new mediabunny.Quality({ bitrate: settings.videoBitsPerSecond }),
		keyFrameInterval: 2,
		sizeChangeBehavior: 'deny',
		latencyMode: 'quality'
	});
	output.addVideoTrack(source, { frameRate: settings.fps });

	const exportState: ActiveExport = {
		canvas,
		context,
		source,
		output,
		target,
		settings,
		exceededOutputLimit: false
	};
	if (target instanceof mediabunny.BufferTarget) {
		target.on('write', ({ end }) => {
			if (end <= settings.maxOutputBytes || exportState.exceededOutputLimit) return;
			exportState.exceededOutputLimit = true;
			void output.cancel().catch(() => undefined);
		});
	}

	active = exportState;
	try {
		await output.start();
	} catch (error) {
		await cancelExport();
		throw error;
	}
}

async function addFrame(bitmap: ImageBitmap, timestamp: number, duration: number) {
	const current = requireActive();
	try {
		const { canvas, context, settings, source } = current;
		context.fillStyle = settings.background;
		context.fillRect(0, 0, canvas.width, canvas.height);
		const scale = Math.min(canvas.width / bitmap.width, canvas.height / bitmap.height);
		const width = bitmap.width * scale;
		const height = bitmap.height * scale;
		context.drawImage(bitmap, (canvas.width - width) / 2, (canvas.height - height) / 2, width, height);
	} finally {
		bitmap.close();
	}

	await current.source.add(timestamp, duration);
	if (current.exceededOutputLimit) throw outputTooLargeError();
}

async function finishExport(): Promise<ArrayBuffer | undefined> {
	const current = requireActive();
	current.source.close();
	try {
		await current.output.finalize();
	} catch (error) {
		if (current.exceededOutputLimit) throw outputTooLargeError();
		throw error;
	}
	if (current.exceededOutputLimit) throw outputTooLargeError();
	const buffer = current.settings.directSave
		? undefined
		: ((current.target as BufferTarget).buffer ?? undefined);
	if (!current.settings.directSave) {
		if (!buffer?.byteLength) throw new Error(studioMessage('export.emptyAnimation'));
		if (buffer.byteLength > current.settings.maxOutputBytes) throw outputTooLargeError();
	}
	active = undefined;
	current.canvas.width = 0;
	current.canvas.height = 0;
	return buffer;
}

function sendChunk(chunk: StreamTargetChunk): Promise<void> {
	const writeId = ++nextWriteId;
	return new Promise<void>((resolve, reject) => {
		pendingWrites.set(writeId, { resolve, reject });
		try {
			workerScope.postMessage({ kind: 'write', writeId, chunk }, [chunk.data.buffer]);
		} catch (error) {
			pendingWrites.delete(writeId);
			reject(error instanceof Error ? error : new Error(String(error)));
		}
	});
}

function sendControl(kind: 'close' | 'abort'): Promise<void> {
	const writeId = ++nextWriteId;
	return new Promise<void>((resolve, reject) => {
		pendingWrites.set(writeId, { resolve, reject });
		try {
			workerScope.postMessage({
				kind: kind === 'close' ? 'stream-close' : 'stream-abort',
				writeId
			});
		} catch (error) {
			pendingWrites.delete(writeId);
			reject(error instanceof Error ? error : new Error(String(error)));
		}
	});
}

async function cancelExport() {
	const current = active;
	active = undefined;
	if (!current) return;
	current.source.close();
	if (current.output.state !== 'finalized' && current.output.state !== 'canceled') {
		await current.output.cancel().catch(() => undefined);
	}
	current.canvas.width = 0;
	current.canvas.height = 0;
}

function requireActive(): ActiveExport {
	if (!active) throw new Error(studioMessage('export.workerNotReady'));
	return active;
}

function outputTooLargeError() {
	const error = new Error(studioMessage('export.workerTooLarge'));
	error.name = 'AnimationOutputTooLargeError';
	return error;
}
