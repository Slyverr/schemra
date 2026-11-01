import { studioMessage } from '../i18n/messages.js';
import {
	isBufferedAnimationOutputWithinLimit,
	type AnimationFormat
} from './animation-export-settings.js';

const STOP_TIMEOUT_BASE_MS = 8_000;
const STOP_TIMEOUT_PER_16_MB_MS = 2_000;
const MAX_STOP_TIMEOUT_MS = 45_000;

const recorderMimeTypes: Record<AnimationFormat, string[]> = {
	webm: ['video/webm;codecs=vp9', 'video/webm;codecs=vp8', 'video/webm'],
	mp4: ['video/mp4;codecs=avc1.42E01E', 'video/mp4;codecs=avc1', 'video/mp4'],
	mov: ['video/quicktime;codecs=avc1', 'video/quicktime']
};

export function findRecorderMimeType(
	format: AnimationFormat,
	isTypeSupported: (mimeType: string) => boolean
): string | null {
	return recorderMimeTypes[format].find((mimeType) => {
		try {
			return isTypeSupported(mimeType);
		} catch {
			return false;
		}
	}) ?? null;
}

export function supportsRecorderFormat(
	format: AnimationFormat,
	canvasCaptureAvailable: boolean,
	isTypeSupported: (mimeType: string) => boolean
): boolean {
	return canvasCaptureAvailable && findRecorderMimeType(format, isTypeSupported) !== null;
}

export function recorderMimeType(format: AnimationFormat): string | null {
	if (typeof MediaRecorder === 'undefined' || typeof MediaRecorder.isTypeSupported !== 'function') return null;
	return findRecorderMimeType(format, (mimeType) => MediaRecorder.isTypeSupported(mimeType));
}

export interface CanvasRecordingOptions {
	canvas: HTMLCanvasElement;
	format: AnimationFormat;
	mimeType: string;
	durationMs: number;
	frameCount: number;
	fps: number;
	videoBitsPerSecond: number;
	signal: AbortSignal;
	drawFrame: (timeMs: number) => Promise<void>;
	onProgress?: (progress: number) => void;
	writeChunk?: (chunk: Blob) => Promise<void>;
	abort: () => void;
	outputTooLargeError: () => Error;
}

function abortError(): Error {
	const error = new Error(studioMessage('export.cancelled'));
	error.name = 'AbortError';
	return error;
}

function runtimeError(format: AnimationFormat): Error {
	const error = new Error(studioMessage('export.recorderFailed', { format: format.toUpperCase() }));
	error.name = 'AnimationRecorderError';
	return error;
}

function startError(error: unknown, format: AnimationFormat): unknown {
	if (!(error instanceof Error) || error.name !== 'NotSupportedError') return error;
	const unavailable = new Error(studioMessage('export.recorderStartUnsupported', { format: format.toUpperCase() }));
	unavailable.name = 'AnimationFormatUnavailableError';
	return unavailable;
}

function delay(milliseconds: number, signal: AbortSignal): Promise<void> {
	return new Promise((resolve, reject) => {
		const finish = () => {
			window.clearTimeout(timer);
			signal.removeEventListener('abort', cancel);
			resolve();
		};
		const cancel = () => {
			window.clearTimeout(timer);
			signal.removeEventListener('abort', cancel);
			reject(abortError());
		};
		const timer = window.setTimeout(finish, milliseconds);
		if (signal.aborted) cancel();
		else signal.addEventListener('abort', cancel, { once: true });
	});
}

function stopRecorder(recorder: MediaRecorder | null) {
	if (!recorder || recorder.state === 'inactive') return;
	try {
		recorder.stop();
	} catch {
		// The tracks are stopped by the outer cleanup if the recorder is already stopping.
	}
}

function waitForStop(stopped: Promise<void>, timeoutMs: number, signal?: AbortSignal): Promise<void> {
	return new Promise((resolve, reject) => {
		let settled = false;
		let timer = 0;
		const finish = (settle: () => void) => {
			if (settled) return;
			settled = true;
			window.clearTimeout(timer);
			signal?.removeEventListener('abort', cancel);
			settle();
		};
		const cancel = () => finish(() => reject(abortError()));
		timer = window.setTimeout(
			() => finish(() => reject(new Error(studioMessage('export.recorderStopTimeout')))),
			timeoutMs
		);
		if (signal?.aborted) cancel();
		else signal?.addEventListener('abort', cancel, { once: true });
		stopped.then(() => finish(resolve), (error) => finish(() => reject(error)));
	});
}

function stopTimeoutMs(options: Pick<CanvasRecordingOptions, 'durationMs' | 'videoBitsPerSecond'>): number {
	const estimatedBytes = Math.max(0, options.durationMs) * Math.max(0, options.videoBitsPerSecond) / 8_000;
	const sizeGrace = Math.ceil(estimatedBytes / (16 * 1024 * 1024)) * STOP_TIMEOUT_PER_16_MB_MS;
	return Math.min(MAX_STOP_TIMEOUT_MS, STOP_TIMEOUT_BASE_MS + sizeGrace);
}

/** Records canvas frames in real time when this browser has no compatible WebCodecs encoder. */
export async function recordCanvasAnimation(options: CanvasRecordingOptions): Promise<Blob | null> {
	if (typeof MediaRecorder === 'undefined' || typeof options.canvas.captureStream !== 'function') {
		throw new Error(studioMessage('export.encoderUnavailable', { format: options.format.toUpperCase() }));
	}
	if (options.signal.aborted) throw abortError();

	let stream: MediaStream | null = null;
	let canvasTrack: CanvasCaptureMediaStreamTrack | null = null;
	let recorder: MediaRecorder | null = null;
	let recordingStarted = false;
	let stopRequested = false;
	let stopped: Promise<void> | null = null;
	let recordingError: Error | null = null;
	let rejectRecordingFailure: ((error: Error) => void) | null = null;
	let recorderWrites = Promise.resolve();
	const chunks: Blob[] = [];
	let bufferedBytes = 0;
	let pendingBytes = 0;
	const recordingFailure = new Promise<never>((_resolve, reject) => {
		rejectRecordingFailure = reject;
	});
	void recordingFailure.catch(() => undefined);
	const failRecording = (error: Error) => {
		if (recordingError) return;
		recordingError = error;
		rejectRecordingFailure?.(error);
		options.abort();
	};

	try {
		const canRequestFrames = typeof CanvasCaptureMediaStreamTrack !== 'undefined' &&
			typeof CanvasCaptureMediaStreamTrack.prototype.requestFrame === 'function';
		stream = options.canvas.captureStream(canRequestFrames ? 0 : options.fps);
		if (canRequestFrames) {
			const track = stream.getVideoTracks()[0] as CanvasCaptureMediaStreamTrack | undefined;
			if (track && typeof track.requestFrame === 'function') canvasTrack = track;
			else {
				for (const captureTrack of stream.getTracks()) captureTrack.stop();
				stream = options.canvas.captureStream(options.fps);
			}
		}

		try {
			recorder = new MediaRecorder(stream, {
				mimeType: options.mimeType,
				videoBitsPerSecond: options.videoBitsPerSecond
			});
		} catch (error) {
			throw startError(error, options.format);
		}

		stopped = new Promise<void>((resolve, reject) => {
			recorder!.ondataavailable = (event) => {
				if (event.data.size <= 0) return;
				if (options.writeChunk) {
					pendingBytes += event.data.size;
					if (!isBufferedAnimationOutputWithinLimit(pendingBytes)) {
						failRecording(options.outputTooLargeError());
						return;
					}
					const chunk = event.data;
					recorderWrites = recorderWrites.then(async () => {
						await options.writeChunk!(chunk);
						pendingBytes -= chunk.size;
					}).catch((error: unknown) => {
						failRecording(error instanceof Error ? error : runtimeError(options.format));
					});
					return;
				}

				bufferedBytes += event.data.size;
				if (!isBufferedAnimationOutputWithinLimit(bufferedBytes)) {
					failRecording(options.outputTooLargeError());
					return;
				}
				chunks.push(event.data);
			};
			recorder!.onerror = () => {
				const error = runtimeError(options.format);
				reject(error);
				failRecording(error);
			};
			recorder!.onstop = () => {
				resolve();
				if (!stopRequested) failRecording(new Error(studioMessage('export.recorderStoppedEarly')));
			};
		});
		void stopped.catch(() => undefined);

		await Promise.race([options.drawFrame(0), recordingFailure]);
		if (options.signal.aborted) throw recordingError ?? abortError();
		try {
			recorder.start(1_000);
		} catch (error) {
			throw startError(error, options.format);
		}
		recordingStarted = true;
		canvasTrack?.requestFrame();
		const recordingStartedAt = performance.now();
		options.onProgress?.(0);
		const frameIntervalMs = 1_000 / options.fps;
		let nextFrame = 1;
		let lastProgressStep = 0;
		const renderScheduledFrame = async (timeMs: number) => {
			await Promise.race([options.drawFrame(timeMs), recordingFailure]);
			if (canvasTrack) canvasTrack.requestFrame();
			const progressStep = Math.floor(timeMs / options.durationMs * 100);
			if (progressStep > lastProgressStep) {
				lastProgressStep = progressStep;
				options.onProgress?.(Math.min(0.99, progressStep / 100));
			}
		};

		while (nextFrame < options.frameCount) {
			const elapsedMs = performance.now() - recordingStartedAt;
			if (elapsedMs >= options.durationMs) break;
			const latestDueFrame = Math.floor(elapsedMs / frameIntervalMs);
			const frame = Math.max(nextFrame, Math.min(options.frameCount - 1, latestDueFrame));
			const waitMs = recordingStartedAt + frame * frameIntervalMs - performance.now();
			if (waitMs > 1) await Promise.race([delay(waitMs, options.signal), recordingFailure]);
			await renderScheduledFrame(Math.min(options.durationMs, frame * frameIntervalMs));
			nextFrame = frame + 1;
		}

		const finishWaitMs = recordingStartedAt + options.durationMs - performance.now();
		if (finishWaitMs > 1) await Promise.race([delay(finishWaitMs, options.signal), recordingFailure]);
		if (options.signal.aborted) throw recordingError ?? abortError();
		await renderScheduledFrame(options.durationMs);
		await Promise.race([delay(frameIntervalMs, options.signal), recordingFailure]);
		if (options.signal.aborted) throw recordingError ?? abortError();
		stopRequested = true;
		stopRecorder(recorder);
		await Promise.race([waitForStop(stopped, stopTimeoutMs(options), options.signal), recordingFailure]);
		await recorderWrites;
		if (recordingError) throw recordingError;
		if (options.writeChunk) return null;
		if (!chunks.length || bufferedBytes <= 0) throw new Error(studioMessage('export.emptyAnimation'));
		return new Blob(chunks, { type: recorder.mimeType || options.mimeType });
	} catch (error) {
		options.abort();
		stopRequested = true;
		stopRecorder(recorder);
		if (recordingStarted && stopped) {
			try {
				await waitForStop(stopped, stopTimeoutMs(options));
				await recorderWrites;
			} catch {
				// Cleanup must finish even when the browser does not emit a final stop event.
			}
		}
		throw recordingError ?? error;
	} finally {
		if (recorder) {
			recorder.ondataavailable = null;
			recorder.onerror = null;
			recorder.onstop = null;
		}
		for (const track of stream?.getTracks() ?? []) {
			try {
				track.stop();
			} catch {
				// Continue releasing the remaining tracks.
			}
		}
	}
}
