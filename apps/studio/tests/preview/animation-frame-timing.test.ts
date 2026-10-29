import { describe, expect, it } from 'bun:test';
import {
	getAnimationExportFrameCount,
	getAnimationExportFrameTime,
	getAnimationExportRenderTimeMs
} from '../../src/lib/preview/animation-frame-timing.ts';

describe('animation export frame timing', () => {
	it('keeps exact frame-aligned durations without an extra frame', () => {
		expect(getAnimationExportFrameCount(1_000, 24)).toBe(24);
		expect(getAnimationExportFrameTime(1_000, 24, 23)).toEqual({
			frameIndex: 23,
			fps: 24,
			timestampNumerator: 23,
			durationNumeratorMicroseconds: 1_000_000
		});
	});

	it('clips a partial final frame to the authored end time', () => {
		expect(getAnimationExportFrameCount(1_001, 24)).toBe(25);
		const durations = Array.from({ length: 25 }, (_, index) =>
			getAnimationExportFrameTime(1_001, 24, index).durationNumeratorMicroseconds
		);
		expect(durations.reduce((sum, duration) => sum + duration, 0) / 24).toBe(1_001_000);
		expect(getAnimationExportFrameTime(1_001, 24, 24)).toEqual({
			frameIndex: 24,
			fps: 24,
			timestampNumerator: 24,
			durationNumeratorMicroseconds: 24_000
		});
	});

	it('quantizes milliseconds once to microsecond precision and keeps a positive sub-microsecond duration', () => {
		expect(getAnimationExportFrameTime(1_001.0004, 24, 24).durationNumeratorMicroseconds).toBe(24_000);
		expect(getAnimationExportFrameTime(1_001.0006, 24, 24).durationNumeratorMicroseconds).toBe(24_024);
		expect(getAnimationExportFrameCount(0.0001, 24)).toBe(1);
		expect(getAnimationExportRenderTimeMs(0.0001, 24, 0)).toBe(0.001);
	});

	it('renders the final aligned and clipped frames at the exact end state', () => {
		const clippedLastFrame = getAnimationExportFrameCount(4_800, 24) - 1;
		expect(getAnimationExportFrameTime(4_800, 24, clippedLastFrame).timestampNumerator).toBe(115);
		expect(getAnimationExportRenderTimeMs(4_800, 24, clippedLastFrame)).toBe(4_800);
		const alignedLastFrame = getAnimationExportFrameCount(1_000, 24) - 1;
		expect(getAnimationExportFrameTime(1_000, 24, alignedLastFrame).timestampNumerator).toBe(23);
		expect(getAnimationExportRenderTimeMs(1_000, 24, alignedLastFrame)).toBe(1_000);
	});

	it('rejects invalid durations, unsupported rates, and out-of-range frame indices', () => {
		for (const duration of [0, -1, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_VALUE]) {
			expect(() => getAnimationExportFrameCount(duration, 24)).toThrow(RangeError);
		}
		expect(() => getAnimationExportFrameCount(1_000, 25)).toThrow(RangeError);
		expect(() => getAnimationExportFrameTime(1_000, 24, -1)).toThrow(RangeError);
		expect(() => getAnimationExportFrameTime(1_000, 24, 24)).toThrow(RangeError);
		expect(() => getAnimationExportFrameTime(1_000, 24, 0.5)).toThrow(RangeError);
	});
});
