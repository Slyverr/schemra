const MICROSECONDS_PER_SECOND = 1_000_000;
const MICROSECONDS_PER_MILLISECOND = 1_000;

export type AnimationExportFps = 24 | 30 | 60;

export interface AnimationExportFrameTime {
	frameIndex: number;
	fps: AnimationExportFps;
	timestampNumerator: number;
	durationNumeratorMicroseconds: number;
}

function durationMicroseconds(durationMs: number): number {
	if (!Number.isFinite(durationMs) || durationMs <= 0) {
		throw new RangeError('Animation export duration must be finite and positive.');
	}
	const duration = Math.max(1, Math.round(durationMs * MICROSECONDS_PER_MILLISECOND));
	if (!Number.isSafeInteger(duration)) {
		throw new RangeError('Animation export duration exceeds the supported precision.');
	}
	return duration;
}

function frameRate(fps: number): AnimationExportFps {
	if (fps !== 24 && fps !== 30 && fps !== 60) {
		throw new RangeError('Animation export frame rate must be 24, 30, or 60 FPS.');
	}
	return fps;
}

function frameCount(durationUs: number, fps: AnimationExportFps): number {
	return frameCountFromTicks(durationUs * fps);
}

function frameCountFromTicks(ticks: number): number {
	if (!Number.isSafeInteger(ticks)) {
		throw new RangeError('Animation export frame count exceeds the supported precision.');
	}
	const completeFrames = Math.floor(ticks / MICROSECONDS_PER_SECOND);
	const partialFrame = ticks % MICROSECONDS_PER_SECOND === 0 ? 0 : 1;
	return Math.max(1, completeFrames + partialFrame);
}

/** Calculate frame timestamps on an exact rational grid without cumulative rounding. */
export function getAnimationExportFrameCount(durationMs: number, fps: number): number {
	return frameCount(durationMicroseconds(durationMs), frameRate(fps));
}

/** Return one frame's rational timestamp and clipped duration. */
export function getAnimationExportFrameTime(
	durationMs: number,
	fps: number,
	frameIndex: number
): AnimationExportFrameTime {
	const durationUs = durationMicroseconds(durationMs);
	const rate = frameRate(fps);
	const ticks = durationUs * rate;
	const count = frameCountFromTicks(ticks);
	if (!Number.isSafeInteger(frameIndex) || frameIndex < 0 || frameIndex >= count) {
		throw new RangeError('Animation export frame index is outside the output.');
	}

	return {
		frameIndex,
		fps: rate,
		timestampNumerator: frameIndex,
		durationNumeratorMicroseconds: Math.min(
			MICROSECONDS_PER_SECOND,
			ticks - frameIndex * MICROSECONDS_PER_SECOND
		)
	};
}

/** Render the final sample at the exact scenario end, including clipped frames. */
export function getAnimationExportRenderTimeMs(
	durationMs: number,
	fps: number,
	frameIndex: number
): number {
	const durationUs = durationMicroseconds(durationMs);
	const frame = getAnimationExportFrameTime(durationMs, fps, frameIndex);
	const count = frameCount(durationUs, frame.fps);
	if (frameIndex === count - 1) return durationUs / MICROSECONDS_PER_MILLISECOND;
	return (frame.timestampNumerator * MICROSECONDS_PER_MILLISECOND) / frame.fps;
}
