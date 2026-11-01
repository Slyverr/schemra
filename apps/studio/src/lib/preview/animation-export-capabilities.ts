import type { VideoCodec } from 'mediabunny';
import {
	animationFormatOptions,
	isValidAnimationExportBitrate,
	isValidAnimationExportDimensions,
	isValidAnimationExportFps,
	type AnimationExportCapabilitySettings,
	type AnimationFormat
} from './animation-export-settings.js';
import { recorderMimeType } from './animation-export-recorder.js';

const codecCache = new Map<string, Promise<VideoCodec | null>>();
let mediaBunnyPromise: Promise<typeof import('mediabunny')> | null = null;

export function loadMediabunny() {
	mediaBunnyPromise ??= import('mediabunny');
	return mediaBunnyPromise;
}

export function createOutputFormat(mediabunny: typeof import('mediabunny'), format: AnimationFormat) {
	switch (format) {
		case 'mp4': return new mediabunny.Mp4OutputFormat({ fastStart: false });
		case 'webm': return new mediabunny.WebMOutputFormat();
		case 'mov': return new mediabunny.MovOutputFormat({ fastStart: false });
	}
}

function capabilityKey(settings: AnimationExportCapabilitySettings) {
	return [settings.format, settings.width, settings.height, settings.fps, settings.videoBitsPerSecond].join(':');
}

function codecCandidates(format: AnimationFormat): VideoCodec[] {
	return format === 'webm' ? ['vp9', 'vp8', 'av1'] : ['avc'];
}

function supportsCanvasRecording(format: AnimationFormat): boolean {
	return typeof HTMLCanvasElement !== 'undefined' &&
		typeof HTMLCanvasElement.prototype.captureStream === 'function' &&
		recorderMimeType(format) !== null;
}

export async function findVideoCodec(
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
				const containerCodecs = createOutputFormat(mediabunny, settings.format).getSupportedVideoCodecs();

				for (const codec of codecCandidates(settings.format)) {
					if (!containerCodecs.includes(codec)) continue;
					try {
						if (await mediabunny.canEncodeVideo(codec, {
							width: settings.width,
							height: settings.height,
							quality: new mediabunny.Quality({ bitrate: settings.videoBitsPerSecond }),
							frameRate: settings.fps,
							latencyMode: 'quality'
						})) return codec;
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
		!isValidAnimationExportBitrate(settings.videoBitsPerSecond)) return false;
	return (await findVideoCodec(settings)) !== null || supportsCanvasRecording(settings.format);
}

/** Returns each video container supported by either the exact offline encoder or real-time recorder. */
export async function getSupportedAnimationFormats(
	settings: Omit<AnimationExportCapabilitySettings, 'format'>
): Promise<AnimationFormat[]> {
	if (!isValidAnimationExportDimensions(settings.width, settings.height) ||
		!isValidAnimationExportFps(settings.fps) ||
		!isValidAnimationExportBitrate(settings.videoBitsPerSecond)) return [];

	const supported = await Promise.all(animationFormatOptions.map(async ({ value }) =>
		(await findVideoCodec({ ...settings, format: value })) !== null || supportsCanvasRecording(value)
			? value
			: null
	));
	return supported.filter((format): format is AnimationFormat => format !== null);
}

/** Returns whether MP4 can be encoded with the exact requested settings. */
export function supportsAnimationMp4(settings: Omit<AnimationExportCapabilitySettings, 'format'>): Promise<boolean> {
	return supportsAnimationFormat({ ...settings, format: 'mp4' });
}
