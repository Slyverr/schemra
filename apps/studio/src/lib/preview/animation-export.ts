import type { StreamTargetChunk, VideoCodec } from 'mediabunny';
import type {
	ExportSettings as AnimationWorkerSettings,
	ExportWorkerPayload,
	ExportWorkerReply,
	ExportWorkerRequest
} from './animation-export-worker.js';
import {
	getAnimationExportFrameCount,
	getAnimationExportFrameTime,
	getAnimationExportRenderTimeMs
} from './animation-frame-timing.js';
import { studioMessage } from '../i18n/messages.js';

export type AnimationFrameRenderer = (timeMs: number, background: string) => string;

export const animationFormatOptions = [
	{ value: 'mp4', labelKey: 'export.format.mp4' },
	{ value: 'webm', labelKey: 'export.format.webm' },
	{ value: 'mov', labelKey: 'export.format.mov' }
] as const;

export type AnimationFormat = (typeof animationFormatOptions)[number]['value'];

export type AnimationExportCapabilitySettings = Pick<
	AnimationExportOptions,
	'format' | 'width' | 'height' | 'fps' | 'videoBitsPerSecond'
>;

export interface AnimationExportOptions {
	format: AnimationFormat;
	durationMs: number;
	width: number;
	height: number;
	fps: 24 | 30 | 60;
	videoBitsPerSecond: number;
	background: string;
	fileName: string;
	renderFrame: AnimationFrameRenderer;
	onProgress?: (progress: number) => void;
	signal?: AbortSignal;
}

export const MAX_ANIMATION_EXPORT_DURATION_MS = 120_000;
export const MAX_ANIMATION_EXPORT_SIDE = 3840;
export const MAX_ANIMATION_EXPORT_PIXELS = 3840 * 2160;

export function isValidAnimationExportDuration(durationMs: number): boolean {
	return Number.isFinite(durationMs) && durationMs > 0 && durationMs <= MAX_ANIMATION_EXPORT_DURATION_MS;
}

export function isValidAnimationExportDimensions(width: number, height: number): boolean {
	return Number.isInteger(width) && Number.isInteger(height) &&
		width >= 320 && height >= 180 &&
		width <= MAX_ANIMATION_EXPORT_SIDE && height <= MAX_ANIMATION_EXPORT_SIDE &&
		width * height <= MAX_ANIMATION_EXPORT_PIXELS;
}

export function isValidAnimationExportBitrate(bitrate: number): boolean {
	return Number.isInteger(bitrate) && bitrate >= 100_000 && bitrate <= 50_000_000;
}

export function isValidAnimationExportFps(fps: number): fps is 24 | 30 | 60 {
	return fps === 24 || fps === 30 || fps === 60;
}

const FRAME_LOAD_TIMEOUT_MS = 10_000;
export const MAX_BUFFERED_ANIMATION_OUTPUT_BYTES = 32 * 1024 * 1024;
const BUFFER_ESTIMATE_HEADROOM = 1.5;
const activeExportControllers = new Set<AbortController>();
const codecCache = new Map<string, Promise<VideoCodec | null>>();
let mediaBunnyPromise: Promise<typeof import('mediabunny')> | null = null;

type FileWritableLike = {
	write: (chunk: Blob | { type: 'write'; position: number; data: Uint8Array }) => Promise<void>;
	close: () => Promise<void>;
	abort: (reason?: unknown) => Promise<void>;
};

type SaveFileHandleLike = {
	createWritable: () => Promise<FileWritableLike>;
};

type SavePickerOptionsLike = {
	suggestedName: string;
	types: Array<{
		description: string;
		accept: Record<string, string[]>;
	}>;
};

type SavePickerWindow = Window & {
	showSaveFilePicker?: (options: SavePickerOptionsLike) => Promise<SaveFileHandleLike>;
};

function loadMediabunny() {
	mediaBunnyPromise ??= import('mediabunny');
	return mediaBunnyPromise;
}

function capabilityKey(settings: AnimationExportCapabilitySettings) {
	return [settings.format, settings.width, settings.height, settings.fps, settings.videoBitsPerSecond].join(':');
}

function codecCandidates(format: AnimationFormat): VideoCodec[] {
	return format === 'webm' ? ['vp9', 'vp8', 'av1'] : ['avc'];
}

function createOutputFormat(mediabunny: typeof import('mediabunny'), format: AnimationFormat) {
	switch (format) {
		case 'mp4': return new mediabunny.Mp4OutputFormat({ fastStart: false });
		case 'webm': return new mediabunny.WebMOutputFormat();
		case 'mov': return new mediabunny.MovOutputFormat({ fastStart: false });
	}
}

function formatMimeType(format: AnimationFormat): string {
	switch (format) {
		case 'mp4': return 'video/mp4';
		case 'webm': return 'video/webm';
		case 'mov': return 'video/quicktime';
	}
}

function formatLabelKey(format: AnimationFormat) {
	return animationFormatOptions.find((option) => option.value === format)?.labelKey ?? 'export.format.mp4';
}

function formatExtension(format: AnimationFormat) {
	return `.${format}`;
}

async function findVideoCodec(
	settings: AnimationExportCapabilitySettings
): Promise<VideoCodec | null> {
	const key = capabilityKey(settings);
	let result = codecCache.get(key);
	if (!result) {
		result = (async () => {
			if (typeof VideoEncoder === 'undefined' ||
				!isValidAnimationExportDimensions(settings.width, settings.height) ||
				!isValidAnimationExportBitrate(settings.videoBitsPerSecond) ||
				!isValidAnimationExportFps(settings.fps)) return null;

			try {
				const mediabunny = await loadMediabunny();
				const outputFormat = createOutputFormat(mediabunny, settings.format);
				const containerCodecs = outputFormat.getSupportedVideoCodecs();

				for (const codec of codecCandidates(settings.format)) {
					if (!containerCodecs.includes(codec)) continue;
					try {
						const supported = await mediabunny.canEncodeVideo(codec, {
							width: settings.width,
							height: settings.height,
							quality: new mediabunny.Quality({ bitrate: settings.videoBitsPerSecond }),
							frameRate: settings.fps,
							latencyMode: 'quality'
						});
						if (supported) return codec;
					} catch {
						// A browser may reject the specific codec, size, or bitrate combination.
					}
				}
			} catch {
				// Some browsers cannot encode the selected format at these settings.
			}
			return null;
		})();
		codecCache.set(key, result);
		if (codecCache.size > 32) {
			const oldestKey = codecCache.keys().next().value;
			if (oldestKey) codecCache.delete(oldestKey);
		}
	}
	return result;
}

/** Returns whether the selected container can be encoded with the exact settings. */
export async function supportsAnimationFormat(
	settings: AnimationExportCapabilitySettings
): Promise<boolean> {
	if (!isValidAnimationExportDimensions(settings.width, settings.height) ||
		!isValidAnimationExportFps(settings.fps) ||
		!isValidAnimationExportBitrate(settings.videoBitsPerSecond)) {
		return false;
	}
	return (await findVideoCodec(settings)) !== null;
}

/** Returns whether MP4 can be encoded with the exact requested settings. */
export async function supportsAnimationMp4(
	settings: Omit<AnimationExportCapabilitySettings, 'format'>
): Promise<boolean> {
	return supportsAnimationFormat({ ...settings, format: 'mp4' });
}

export interface AnimationExportStoragePlan {
	estimatedBytes: number;
	requiresDirectSave: boolean;
	canExport: boolean;
}

/** Applies a hard in-memory cap to buffered video exports. */
export function isBufferedAnimationOutputWithinLimit(byteLength: number): boolean {
	return Number.isSafeInteger(byteLength) &&
		byteLength >= 0 &&
		byteLength <= MAX_BUFFERED_ANIMATION_OUTPUT_BYTES;
}

/** Mirrors the exporter's conservative memory estimate so the dialog can warn before rendering. */
export function getAnimationExportStoragePlan(
	durationMs: number,
	videoBitsPerSecond: number,
	canSaveDirectly: boolean
): AnimationExportStoragePlan {
	const estimatedBytes = estimatedOutputBytes(durationMs, videoBitsPerSecond);
	const requiresDirectSave = estimatedBytes > MAX_BUFFERED_ANIMATION_OUTPUT_BYTES;
	return {
		estimatedBytes,
		requiresDirectSave,
		canExport: !requiresDirectSave || canSaveDirectly
	};
}

export function supportsDirectAnimationFileSave(): boolean {
	const pickerWindow = typeof window === 'undefined' ? null : window as SavePickerWindow;
	return typeof pickerWindow?.showSaveFilePicker === 'function';
}

/** Cancels any active animation export started by this module. */
export function cancelActiveAnimationExport() {
	for (const controller of activeExportControllers) controller.abort();
}

function abortError(): Error {
	const error = new Error(studioMessage('export.cancelled'));
	error.name = 'AbortError';
	return error;
}

function throwIfAborted(signal: AbortSignal) {
	if (signal.aborted) throw abortError();
}

function loadSvgImageElement(
	blob: Blob,
	signal: AbortSignal
): Promise<{ image: HTMLImageElement; release: () => void }> {
	const url = URL.createObjectURL(blob);
	let image: HTMLImageElement;
	try {
		image = new Image();
	} catch {
		URL.revokeObjectURL(url);
		return Promise.reject(new Error(studioMessage('export.frameRenderFailed')));
	}
	return new Promise((resolve, reject) => {
		let released = false;
		let loadTimer = 0;
		const release = () => {
			if (released) return;
			released = true;
			window.clearTimeout(loadTimer);
			signal.removeEventListener('abort', cancel);
			image.onload = null;
			image.onerror = null;
			URL.revokeObjectURL(url);
		};
		const fail = (error: Error) => {
			release();
			reject(error);
		};
		const cancel = () => fail(abortError());
		loadTimer = window.setTimeout(() => fail(new Error(studioMessage('export.frameTimeout'))), FRAME_LOAD_TIMEOUT_MS);
		image.onload = () => {
			if (signal.aborted) {
				fail(abortError());
				return;
			}
			signal.removeEventListener('abort', cancel);
			resolve({ image, release });
		};
		image.onerror = () => fail(new Error(studioMessage('export.frameRenderFailed')));
		signal.addEventListener('abort', cancel, { once: true });
		if (signal.aborted) cancel();
		else {
			try {
				image.src = url;
			} catch {
				fail(new Error(studioMessage('export.frameRenderFailed')));
			}
		}
	});
}

async function createSvgImage(
	markup: string,
	signal: AbortSignal
): Promise<{ image: HTMLImageElement | ImageBitmap; release: () => void }> {
	const blob = new Blob([markup], { type: 'image/svg+xml;charset=utf-8' });
	if (typeof createImageBitmap === 'function') {
		let removeAbortListener = () => {};
		try {
			const abort = new Promise<never>((_resolve, reject) => {
				const cancel = () => reject(abortError());
				if (signal.aborted) cancel();
				else {
					signal.addEventListener('abort', cancel, { once: true });
					removeAbortListener = () => signal.removeEventListener('abort', cancel);
				}
			});
			const bitmapPromise = createImageBitmap(blob);
			void bitmapPromise.then(
				(bitmap) => {
					if (signal.aborted) bitmap.close();
				},
				() => undefined
			);
			const image = await Promise.race([bitmapPromise, abort]);
			return { image, release: () => image.close() };
		} catch {
			if (signal.aborted) throw abortError();
			// Keep the SVG-image path for browsers that expose createImageBitmap but cannot decode SVG.
		} finally {
			removeAbortListener();
		}
	}
	return loadSvgImageElement(blob, signal);
}

async function createTransferableSvgImage(
	markup: string,
	signal: AbortSignal
): Promise<{ bitmap: ImageBitmap; release: () => void }> {
	const { image, release } = await createSvgImage(markup, signal);
	if (typeof ImageBitmap !== 'undefined' && image instanceof ImageBitmap) return { bitmap: image, release };
	const canvas = document.createElement('canvas');
	try {
		canvas.width = image instanceof HTMLImageElement ? image.naturalWidth : image.width;
		canvas.height = image instanceof HTMLImageElement ? image.naturalHeight : image.height;
		const context = canvas.getContext('2d');
		if (!context || canvas.width <= 0 || canvas.height <= 0) {
			throw new Error(studioMessage('export.frameTransferFailed'));
		}
		context.drawImage(image, 0, 0);
		const bitmap = await createImageBitmap(canvas);
		return {
			bitmap,
			release: () => {
				release();
				canvas.width = 0;
				canvas.height = 0;
			}
		};
	} catch (error) {
		release();
		canvas.width = 0;
		canvas.height = 0;
		throw error;
	}
}

export function safeAnimationFileName(fileName: string, format: AnimationFormat = 'mp4') {
	const extension = formatExtension(format);
	const stem = fileName
		.trim()
		.replace(/\.(?:mp4|webm|mov)$/i, '')
		.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '-')
		.replace(/\s+/g, '-')
		.slice(0, 100);
	return `${stem || 'schemra-animation'}${extension}`;
}

function estimatedOutputBytes(durationMs: number, videoBitsPerSecond: number): number {
	return (durationMs / 1_000) * (videoBitsPerSecond / 8) * BUFFER_ESTIMATE_HEADROOM;
}

function outputTooLargeError(): Error {
	const error = new Error(studioMessage('export.tooLarge'));
	error.name = 'AnimationOutputTooLargeError';
	return error;
}

function savePickerOptions(options: AnimationExportOptions): SavePickerOptionsLike {
	const extension = formatExtension(options.format);
	const mimeType = formatMimeType(options.format);
	return {
		suggestedName: safeAnimationFileName(options.fileName, options.format),
		types: [{ description: studioMessage(formatLabelKey(options.format)), accept: { [mimeType]: [extension] } }]
	};
}

function downloadBlob(blob: Blob, fileName: string): void {
	const url = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = url;
	link.download = fileName;
	link.click();
	window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
}

function supportsWorkerAnimationExport(): boolean {
	return typeof Worker !== 'undefined' &&
		typeof OffscreenCanvas !== 'undefined' &&
		typeof createImageBitmap === 'function';
}

async function encodeAnimationInWorker(
	options: AnimationExportOptions,
	settings: AnimationWorkerSettings,
	frameCount: number,
	signal: AbortSignal,
	stream: {
		write: (chunk: StreamTargetChunk) => Promise<void>;
		close: () => Promise<void>;
		abort: () => Promise<void>;
	} | null
): Promise<ArrayBuffer | undefined> {
	const worker = new Worker(new URL('./animation-export-worker.ts', import.meta.url), {
		type: 'module',
		name: 'schemra-animation-export'
	});
	let requestId = 0;
	let pending: {
		id: number;
		resolve: (reply: ExportWorkerReply) => void;
		reject: (error: Error) => void;
	} | undefined;

	const fail = (error: Error) => {
		const current = pending;
		pending = undefined;
		current?.reject(error);
	};
	worker.onmessage = (event: MessageEvent<ExportWorkerReply>) => {
		if (event.data.kind === 'write') {
			const { writeId, chunk } = event.data;
			acknowledgeStreamWrite(writeId, stream ? () => stream.write(chunk) : undefined);
			return;
		}
		if (event.data.kind === 'stream-close' || event.data.kind === 'stream-abort') {
			const { writeId, kind } = event.data;
			acknowledgeStreamWrite(
				writeId,
				stream ? () => kind === 'stream-close' ? stream.close() : stream.abort() : undefined
			);
			return;
		}
		if (!pending || pending.id !== event.data.id) return;
		const current = pending;
		pending = undefined;
		if (event.data.kind === 'error') {
			const error = new Error(event.data.message);
			error.name = event.data.name;
			current.reject(error);
		} else {
			current.resolve(event.data);
		}
	};
	worker.onerror = (event) => {
		event.preventDefault();
		fail(new Error(event.message || studioMessage('export.workerStopped')));
	};
	worker.onmessageerror = () => fail(new Error(studioMessage('export.workerUnreadable')));

	const onAbort = () => {
		fail(abortError());
		worker.terminate();
	};
	const acknowledgeStreamWrite = (writeId: number, operation?: () => Promise<void>) => {
		if (!operation) {
			worker.postMessage({
				kind: 'write-ack',
				writeId,
				error: studioMessage('export.workerOutputUnavailable')
			});
			return;
		}
		void Promise.resolve().then(operation).then(
			() => worker.postMessage({ kind: 'write-ack', writeId }),
			(error: unknown) => worker.postMessage({
				kind: 'write-ack',
				writeId,
				error: error instanceof Error ? error.message : String(error)
			})
		);
	};
	signal.addEventListener('abort', onAbort, { once: true });

	const send = (
		request: ExportWorkerPayload,
		transfer: Transferable[] = []
	): Promise<ExportWorkerReply> => {
		throwIfAborted(signal);
		const id = ++requestId;
		return new Promise((resolve, reject) => {
			pending = { id, resolve, reject };
			try {
				worker.postMessage({ ...request, id } as ExportWorkerRequest, transfer);
			} catch (error) {
				pending = undefined;
				reject(error instanceof Error ? error : new Error(String(error)));
			}
		});
	};

	try {
		options.onProgress?.(0);
		const ready = await send({ kind: 'start', settings });
		if (ready.kind !== 'ready') throw new Error(studioMessage('export.workerInitializeFailed'));
		for (let index = 0; index < frameCount; index++) {
			throwIfAborted(signal);
			const frameTime = getAnimationExportFrameTime(options.durationMs, options.fps, index);
			const rendered = await createTransferableSvgImage(
				options.renderFrame(getAnimationExportRenderTimeMs(options.durationMs, options.fps, index), options.background),
				signal
			);
			let framePromise: Promise<ExportWorkerReply>;
			try {
				framePromise = send({
					kind: 'frame',
					bitmap: rendered.bitmap,
					timestamp: frameTime.timestampNumerator / frameTime.fps,
					duration: frameTime.durationNumeratorMicroseconds / (frameTime.fps * 1_000_000)
				}, [rendered.bitmap]);
			} finally {
				rendered.release();
			}
			const frame = await framePromise;
			if (frame.kind !== 'frame-done') throw new Error(studioMessage('export.workerFrameFailed'));
			options.onProgress?.(Math.min(0.97, (index + 1) / frameCount * 0.97));
		}
		options.onProgress?.(0.98);
		const result = await send({ kind: 'finish' });
		if (result.kind !== 'complete') throw new Error(studioMessage('export.workerVideoFailed'));
		return result.buffer;
	} finally {
		signal.removeEventListener('abort', onAbort);
		worker.terminate();
	}
}

/** Encodes deterministic frame samples into the selected video format through WebCodecs. */
export async function exportAnimation(options: AnimationExportOptions): Promise<void> {
	if (typeof HTMLCanvasElement === 'undefined') {
		throw new Error(studioMessage('export.unsupported'));
	}
	if (!isValidAnimationExportDuration(options.durationMs) ||
		!Number.isFinite(options.width) || options.width <= 0 ||
		!Number.isFinite(options.height) || options.height <= 0) {
		throw new Error(studioMessage('export.durationInvalid'));
	}
	if (!isValidAnimationExportFps(options.fps)) {
		throw new Error(studioMessage('export.frameRateInvalid'));
	}
	if (!isValidAnimationExportBitrate(options.videoBitsPerSecond)) {
		throw new Error(studioMessage('export.bitrateInvalid'));
	}
	const frameCount = getAnimationExportFrameCount(options.durationMs, options.fps);
	if (!Number.isSafeInteger(frameCount)) throw new Error(studioMessage('export.scenarioTooLong'));
	if (activeExportControllers.size > 0) throw new Error(studioMessage('export.alreadyRunning'));

	const pixelWidth = Math.round(options.width);
	const pixelHeight = Math.round(options.height);
	if (!Number.isSafeInteger(pixelWidth) || !Number.isSafeInteger(pixelHeight) ||
		!isValidAnimationExportDimensions(pixelWidth, pixelHeight)) {
		throw new Error(studioMessage('export.dimensionsInvalid'));
	}

	const controller = new AbortController();
	const externalSignal = options.signal;
	const forwardAbort = () => controller.abort();
	if (externalSignal?.aborted) forwardAbort();
	else externalSignal?.addEventListener('abort', forwardAbort, { once: true });
	activeExportControllers.add(controller);
	const { signal } = controller;

	let canvas: HTMLCanvasElement | null = null;
	let output: import('mediabunny').Output | null = null;
	let fileWritable: FileWritableLike | null = null;
	let fileWritableOpen = false;

	// Launch a required save-as prompt before awaiting encoder support, while user activation is available.
	let fileHandlePromise: Promise<SaveFileHandleLike> | null = null;
	try {
		const storagePlan = getAnimationExportStoragePlan(
			options.durationMs,
			options.videoBitsPerSecond,
			supportsDirectAnimationFileSave()
		);
		if (storagePlan.requiresDirectSave) {
			const pickerWindow = typeof window === 'undefined' ? null : window as SavePickerWindow;
			if (!storagePlan.canExport || !pickerWindow?.showSaveFilePicker) throw outputTooLargeError();
			fileHandlePromise = pickerWindow.showSaveFilePicker(savePickerOptions(options));
			void fileHandlePromise.catch(() => undefined);
		}

		throwIfAborted(signal);
		const settings = {
			width: pixelWidth,
			height: pixelHeight,
			fps: options.fps,
			videoBitsPerSecond: options.videoBitsPerSecond
		};
		const codec = await findVideoCodec({ ...settings, format: options.format });
		if (!codec) {
			const unavailable = new Error(studioMessage('export.encoderUnavailable', { format: studioMessage(formatLabelKey(options.format)) }));
			unavailable.name = 'AnimationFormatUnavailableError';
			throw unavailable;
		}

		if (fileHandlePromise) {
			const fileHandle = await fileHandlePromise;
			fileWritable = await fileHandle.createWritable();
			fileWritableOpen = true;
		}
		throwIfAborted(signal);

		if (supportsWorkerAnimationExport()) {
			const buffer = await encodeAnimationInWorker(
				options,
				{
					format: options.format,
					width: pixelWidth,
					height: pixelHeight,
					fps: options.fps,
					videoBitsPerSecond: options.videoBitsPerSecond,
					codec,
					background: options.background,
					maxOutputBytes: MAX_BUFFERED_ANIMATION_OUTPUT_BYTES,
					directSave: fileWritable !== null
				},
				frameCount,
				signal,
				fileWritable ? {
					write: (chunk) => {
						if (!fileWritable || !fileWritableOpen) throw new Error(studioMessage('export.outputFileUnavailable'));
						return fileWritable.write(chunk);
					},
					close: async () => {
						if (!fileWritable || !fileWritableOpen) return;
						fileWritableOpen = false;
						await fileWritable.close();
					},
					abort: async () => {
						if (!fileWritable || !fileWritableOpen) return;
						fileWritableOpen = false;
						await fileWritable.abort();
					}
				} : null
			);
			throwIfAborted(signal);
			if (fileWritable) fileWritableOpen = false;
			else {
				if (!buffer?.byteLength) throw new Error(studioMessage('export.emptyAnimation'));
				downloadBlob(new Blob([buffer], { type: formatMimeType(options.format) }), safeAnimationFileName(options.fileName, options.format));
			}
			options.onProgress?.(1);
			return;
		}

		const exportCanvas = document.createElement('canvas');
		canvas = exportCanvas;
		exportCanvas.width = pixelWidth;
		exportCanvas.height = pixelHeight;
		const context = exportCanvas.getContext('2d', { alpha: false });
		if (!context) throw new Error(studioMessage('export.rendererInitializeFailed'));
		context.imageSmoothingEnabled = true;
		context.imageSmoothingQuality = 'high';

		const drawFrame = async (timeMs: number) => {
			throwIfAborted(signal);
			const markup = options.renderFrame(timeMs, options.background);
			const { image, release } = await createSvgImage(markup, signal);
			try {
				throwIfAborted(signal);
				context.fillStyle = options.background;
				context.fillRect(0, 0, exportCanvas.width, exportCanvas.height);
				const fit = Math.min(exportCanvas.width / image.width, exportCanvas.height / image.height);
				const drawWidth = image.width * fit;
				const drawHeight = image.height * fit;
				context.drawImage(image, (exportCanvas.width - drawWidth) / 2, (exportCanvas.height - drawHeight) / 2, drawWidth, drawHeight);
			} finally {
				release();
			}
		};
		const mediabunny = await loadMediabunny();
		const format = createOutputFormat(mediabunny, options.format);
		const target = fileWritable
			? new mediabunny.StreamTarget(new WritableStream<import('mediabunny').StreamTargetChunk>({
				write: async (chunk) => {
					if (!fileWritable) throw new Error(studioMessage('export.outputFileUnavailable'));
					await fileWritable.write(chunk);
				},
				close: async () => {
					if (!fileWritable || !fileWritableOpen) return;
					fileWritableOpen = false;
					await fileWritable.close();
				},
				abort: async (reason) => {
					if (!fileWritable || !fileWritableOpen) return;
					fileWritableOpen = false;
					await fileWritable.abort(reason);
				}
			}), { chunked: false })
			: new mediabunny.BufferTarget();
		const videoOutput = new mediabunny.Output({ format, target });
		output = videoOutput;
		const source = new mediabunny.CanvasSource(exportCanvas, {
			codec,
			quality: new mediabunny.Quality({ bitrate: options.videoBitsPerSecond }),
			keyFrameInterval: 2,
			sizeChangeBehavior: 'deny',
			latencyMode: 'quality'
		});
		videoOutput.addVideoTrack(source, { frameRate: options.fps });

		let outputBytes = 0;
		let exceededBufferLimit = false;
		if (!fileWritable && target instanceof mediabunny.BufferTarget) {
			target.on('write', ({ end }) => {
				outputBytes = Math.max(outputBytes, end);
				if (!isBufferedAnimationOutputWithinLimit(outputBytes) && !exceededBufferLimit) {
					exceededBufferLimit = true;
					void videoOutput.cancel().catch(() => undefined);
				}
			});
		}
		const cancelOutput = () => {
			void videoOutput.cancel().catch(() => undefined);
		};
		signal.addEventListener('abort', cancelOutput, { once: true });
		try {
			await videoOutput.start();
			options.onProgress?.(0);
			for (let frame = 0; frame < frameCount; frame++) {
				throwIfAborted(signal);
				if (exceededBufferLimit) throw outputTooLargeError();
				const frameTime = getAnimationExportFrameTime(options.durationMs, options.fps, frame);
				const timestampSeconds = frameTime.timestampNumerator / frameTime.fps;
				const durationSeconds = frameTime.durationNumeratorMicroseconds /
					(frameTime.fps * 1_000_000);
				await drawFrame(getAnimationExportRenderTimeMs(
					options.durationMs,
					options.fps,
					frame
				));
				await source.add(timestampSeconds, durationSeconds);
				if (exceededBufferLimit) throw outputTooLargeError();
				options.onProgress?.(Math.min(0.97, (frame + 1) / frameCount * 0.97));
			}
			source.close();
			throwIfAborted(signal);
			options.onProgress?.(0.98);
			try {
				await videoOutput.finalize();
			} catch (error) {
				if (exceededBufferLimit) throw outputTooLargeError();
				throw error;
			}
			if (fileWritable) {
				// StreamTarget closes the file as it finalizes.
				fileWritableOpen = false;
			} else if (target instanceof mediabunny.BufferTarget) {
				const buffer = target.buffer;
				if (exceededBufferLimit || !isBufferedAnimationOutputWithinLimit(buffer?.byteLength ?? 0)) {
					throw outputTooLargeError();
				}
				if (!buffer?.byteLength) throw new Error(studioMessage('export.emptyAnimation'));
				downloadBlob(new Blob([buffer], { type: format.mimeType }), safeAnimationFileName(options.fileName, options.format));
			}
			options.onProgress?.(1);
		} finally {
			signal.removeEventListener('abort', cancelOutput);
			source.close();
			if (videoOutput.state !== 'finalized' && videoOutput.state !== 'canceled') {
				await videoOutput.cancel().catch(() => undefined);
			}
		}
	} catch (error) {
		controller.abort();
		if (output && output.state !== 'finalized' && output.state !== 'canceled') {
			await output.cancel().catch(() => undefined);
		}
		throw error;
	} finally {
		try {
			if (fileWritable && fileWritableOpen) {
				fileWritableOpen = false;
				await fileWritable.abort();
			}
		} finally {
			if (canvas) {
				canvas.width = 0;
				canvas.height = 0;
			}
			externalSignal?.removeEventListener('abort', forwardAbort);
			activeExportControllers.delete(controller);
		}
	}
}
