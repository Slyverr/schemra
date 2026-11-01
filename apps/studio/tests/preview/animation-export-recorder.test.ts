import { describe, expect, test } from 'bun:test';
import { recordCanvasAnimation } from '../../src/lib/preview/animation-export-recorder.js';

class FakeMediaRecorder {
	static isTypeSupported(mimeType: string) {
		return mimeType === 'video/webm';
	}

	state: RecordingState = 'inactive';
	mimeType: string;
	ondataavailable: ((event: BlobEvent) => void) | null = null;
	onerror: ((event: Event) => void) | null = null;
	onstop: ((event: Event) => void) | null = null;

	constructor(_stream: MediaStream, options: MediaRecorderOptions) {
		this.mimeType = options.mimeType ?? '';
	}

	start() {
		this.state = 'recording';
	}

	stop() {
		if (this.state === 'inactive') return;
		this.state = 'inactive';
		this.ondataavailable?.({ data: new Blob(['recorded frame']) } as BlobEvent);
		this.onstop?.(new Event('stop'));
	}
}

describe('canvas recorder fallback', () => {
	test('records canvas frames and streams chunks to a direct-save writer', async () => {
		const savedDescriptors = new Map<string, PropertyDescriptor | undefined>();
		for (const key of ['window', 'MediaRecorder', 'CanvasCaptureMediaStreamTrack']) {
			savedDescriptors.set(key, Object.getOwnPropertyDescriptor(globalThis, key));
		}

		Object.defineProperty(globalThis, 'window', {
			configurable: true,
			value: { setTimeout, clearTimeout }
		});
		Object.defineProperty(globalThis, 'MediaRecorder', { configurable: true, value: FakeMediaRecorder });
		Object.defineProperty(globalThis, 'CanvasCaptureMediaStreamTrack', { configurable: true, value: undefined });

		let stoppedTracks = 0;
		const canvas = {
			captureStream: () => ({
				getTracks: () => [{ stop: () => stoppedTracks++ }],
				getVideoTracks: () => []
			})
		} as unknown as HTMLCanvasElement;
		const chunks: Blob[] = [];
		const frames: number[] = [];

		try {
			const result = await recordCanvasAnimation({
				canvas,
				format: 'webm',
				mimeType: 'video/webm',
				durationMs: 10,
				frameCount: 1,
				fps: 24,
				videoBitsPerSecond: 100_000,
				signal: new AbortController().signal,
				drawFrame: async (timeMs) => { frames.push(timeMs); },
				writeChunk: async (chunk) => { chunks.push(chunk); },
				abort: () => undefined,
				outputTooLargeError: () => new Error('too large')
			});

			expect(result).toBeNull();
			expect(frames).toEqual([0, 10]);
			expect(chunks).toHaveLength(1);
			expect(chunks[0]?.size).toBeGreaterThan(0);
			expect(stoppedTracks).toBe(1);
		} finally {
			for (const [key, descriptor] of savedDescriptors) {
				if (descriptor) Object.defineProperty(globalThis, key, descriptor);
				else Reflect.deleteProperty(globalThis, key);
			}
		}
	});
});
