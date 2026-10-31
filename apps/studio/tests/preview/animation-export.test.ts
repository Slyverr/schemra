import { describe, expect, test } from 'bun:test';
import {
	animationFormatOptions,
	getAnimationExportStoragePlan,
	isBufferedAnimationOutputWithinLimit,
	isValidAnimationExportBitrate,
	isValidAnimationExportDimensions,
	isValidAnimationExportDuration,
	isValidAnimationExportFps,
	MAX_ANIMATION_EXPORT_DURATION_MS,
	MAX_ANIMATION_EXPORT_PIXELS,
	MAX_BUFFERED_ANIMATION_OUTPUT_BYTES,
	safeAnimationFileName,
	supportsAnimationFormat,
	supportsAnimationMp4
} from '../../src/lib/preview/animation-export.js';
import { studioMessage } from '../../src/lib/i18n/messages.js';

describe('animation export choices', () => {
	test('offers each implemented video container with a readable localized label', () => {
		expect(animationFormatOptions.map(({ value }) => value)).toEqual(['mp4', 'webm', 'mov']);
		expect(animationFormatOptions.map(({ labelKey }) => studioMessage(labelKey))).toEqual([
			'MP4',
			'WebM',
			'QuickTime MOV'
		]);
	});

	test('shows bitrate choices in Mbps instead of raw bits-per-second values', () => {
		const labels = [
			studioMessage('export.bitrate.smaller'),
			studioMessage('export.bitrate.balanced'),
			studioMessage('export.bitrate.higherQuality')
		];
		expect(labels).toEqual(['Smaller · 8 Mbps', 'Balanced · 12 Mbps', 'Higher quality · 20 Mbps']);
		expect(labels).not.toContain('8000000');
	});
});

describe('animation export settings', () => {
	test('accepts supported duration, frame-rate, and dimension boundaries', () => {
		expect(isValidAnimationExportDuration(1)).toBe(true);
		expect(isValidAnimationExportDuration(MAX_ANIMATION_EXPORT_DURATION_MS)).toBe(true);
		expect(isValidAnimationExportDimensions(320, 180)).toBe(true);
		expect(isValidAnimationExportDimensions(3840, 2160)).toBe(true);
		expect(isValidAnimationExportFps(24)).toBe(true);
		expect(isValidAnimationExportFps(30)).toBe(true);
		expect(isValidAnimationExportFps(60)).toBe(true);
	});

	test('rejects invalid or over-limit settings', () => {
		for (const duration of [0, -1, Number.NaN, Number.POSITIVE_INFINITY, MAX_ANIMATION_EXPORT_DURATION_MS + 1]) {
			expect(isValidAnimationExportDuration(duration)).toBe(false);
		}
		for (const [width, height] of [[319, 180], [320, 179], [3841, 180], [3840, 2161], [3840, 3840], [320.5, 180]]) {
			expect(isValidAnimationExportDimensions(width, height)).toBe(false);
		}
		expect(3840 * 2160).toBe(MAX_ANIMATION_EXPORT_PIXELS);
		for (const bitrate of [99_999, 50_000_001, Number.NaN, 8_000_000.5]) {
			expect(isValidAnimationExportBitrate(bitrate)).toBe(false);
		}
		expect(isValidAnimationExportFps(25)).toBe(false);
	});
});

describe('animation export output limits', () => {
	test('enforces the buffered output cap inclusively', () => {
		expect(isBufferedAnimationOutputWithinLimit(0)).toBe(true);
		expect(isBufferedAnimationOutputWithinLimit(MAX_BUFFERED_ANIMATION_OUTPUT_BYTES)).toBe(true);
		expect(isBufferedAnimationOutputWithinLimit(MAX_BUFFERED_ANIMATION_OUTPUT_BYTES + 1)).toBe(false);
		expect(isBufferedAnimationOutputWithinLimit(-1)).toBe(false);
		expect(isBufferedAnimationOutputWithinLimit(Number.NaN)).toBe(false);
		expect(isBufferedAnimationOutputWithinLimit(Number.POSITIVE_INFINITY)).toBe(false);
	});

	test('requires direct saving when the conservative estimate exceeds the memory cap', () => {
		expect(getAnimationExportStoragePlan(10_000, 8_000_000, false)).toEqual({
			estimatedBytes: 15_000_000,
			requiresDirectSave: false,
			canExport: true
		});
		expect(getAnimationExportStoragePlan(20_000, 12_000_000, false)).toEqual({
			estimatedBytes: 45_000_000,
			requiresDirectSave: true,
			canExport: false
		});
		expect(getAnimationExportStoragePlan(20_000, 12_000_000, true)).toEqual({
			estimatedBytes: 45_000_000,
			requiresDirectSave: true,
			canExport: true
		});
	});

	test('normalizes unsafe scenario titles for each supported video format', () => {
		expect(safeAnimationFileName(' Service / request: v1?.mp4 ')).toBe('Service---request--v1-.mp4');
		expect(safeAnimationFileName('   ')).toBe('schemra-animation.mp4');
		expect(safeAnimationFileName('service-flow.mp4', 'webm')).toBe('service-flow.webm');
		expect(safeAnimationFileName('service-flow', 'webm')).toBe('service-flow.webm');
		expect(safeAnimationFileName('service-flow.mp4', 'mov')).toBe('service-flow.mov');
		expect(safeAnimationFileName('service-flow.mov', 'mov')).toBe('service-flow.mov');
	});

	test('returns unsupported for invalid encoder requests without probing codecs', async () => {
		expect(await supportsAnimationMp4({ width: 100, height: 100, fps: 30, videoBitsPerSecond: 8_000_000 })).toBe(false);
		expect(await supportsAnimationMp4({ width: 1280, height: 720, fps: 25 as unknown as 24, videoBitsPerSecond: 8_000_000 })).toBe(false);
		expect(await supportsAnimationFormat({ format: 'webm', width: 100, height: 100, fps: 30, videoBitsPerSecond: 8_000_000 })).toBe(false);
		expect(await supportsAnimationFormat({ format: 'webm', width: 1280, height: 720, fps: 25 as unknown as 24, videoBitsPerSecond: 8_000_000 })).toBe(false);
		expect(await supportsAnimationFormat({ format: 'mov', width: 100, height: 100, fps: 30, videoBitsPerSecond: 8_000_000 })).toBe(false);
		expect(await supportsAnimationFormat({ format: 'mov', width: 1280, height: 720, fps: 25 as unknown as 24, videoBitsPerSecond: 8_000_000 })).toBe(false);
	});
});
