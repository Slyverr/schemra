export type AnimationFrameRenderer = (timeMs: number, background: string) => string;

export const animationFormatOptions = [
	{ value: 'mp4', labelKey: 'export.format.mp4' },
	{ value: 'webm', labelKey: 'export.format.webm' },
	{ value: 'mov', labelKey: 'export.format.mov' }
] as const;

export type AnimationFormat = (typeof animationFormatOptions)[number]['value'];

export function formatExtension(format: AnimationFormat): string {
	return `.${format}`;
}

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

export type AnimationExportCapabilitySettings = Pick<
	AnimationExportOptions,
	'format' | 'width' | 'height' | 'fps' | 'videoBitsPerSecond'
>;

export const MAX_ANIMATION_EXPORT_DURATION_MS = 120_000;
export const MAX_ANIMATION_EXPORT_SIDE = 3840;
export const MAX_ANIMATION_EXPORT_PIXELS = 3840 * 2160;
export const MAX_BUFFERED_ANIMATION_OUTPUT_BYTES = 32 * 1024 * 1024;

const BUFFER_ESTIMATE_HEADROOM = 1.5;

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

export interface AnimationExportStoragePlan {
	estimatedBytes: number;
	requiresDirectSave: boolean;
	canExport: boolean;
}

/** Applies the exporter's hard in-memory cap to buffered video output. */
export function isBufferedAnimationOutputWithinLimit(byteLength: number): boolean {
	return Number.isSafeInteger(byteLength) &&
		byteLength >= 0 &&
		byteLength <= MAX_BUFFERED_ANIMATION_OUTPUT_BYTES;
}

/** Estimates output size before rendering so the dialog can require direct saving. */
export function getAnimationExportStoragePlan(
	durationMs: number,
	videoBitsPerSecond: number,
	canSaveDirectly: boolean
): AnimationExportStoragePlan {
	const estimatedBytes = (durationMs / 1_000) * (videoBitsPerSecond / 8) * BUFFER_ESTIMATE_HEADROOM;
	const requiresDirectSave = estimatedBytes > MAX_BUFFERED_ANIMATION_OUTPUT_BYTES;
	return { estimatedBytes, requiresDirectSave, canExport: !requiresDirectSave || canSaveDirectly };
}

export function safeAnimationFileName(fileName: string, format: AnimationFormat = 'mp4'): string {
	const extension = formatExtension(format);
	const stem = fileName
		.trim()
		.replace(/\.(?:mp4|webm|mov)$/i, '')
		.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '-')
		.replace(/\s+/g, '-')
		.slice(0, 100);
	return `${stem || 'schemra-animation'}${extension}`;
}
