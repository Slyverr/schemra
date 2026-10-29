import { describe, expect, test } from 'bun:test';
import {
	calculateSceneFocusTransform,
	clampPreviewZoom,
	clampScenePan,
	getPreviewConnectionBounds,
	MAX_PREVIEW_ZOOM,
	zoomPanAtPoint
} from '../../src/lib/preview/viewport.js';

describe('preview viewport behavior', () => {
	test('builds connection focus bounds from its endpoints and cubic route', () => {
		expect(getPreviewConnectionBounds({
			nodes: [
				{ id: 'a', bounds: { x: 0, y: 20, width: 20, height: 20 } },
				{ id: 'b', bounds: { x: 180, y: 40, width: 20, height: 20 } }
			],
			edges: [{
				from: 'a',
				to: 'b',
				path: [{
					start: { x: 20, y: 30 },
					control1: { x: 70, y: 0 },
					control2: { x: 130, y: 100 },
					end: { x: 180, y: 50 }
				}]
			}]
		}, 0)).toEqual({ x: 0, y: 0, width: 200, height: 100 });
	});

	test('rejects missing connection indexes and keeps a degenerate route focusable', () => {
		const scene = {
			nodes: [],
			edges: [{ from: 'missing', to: 'also-missing', path: [{
				start: { x: 12, y: 12 }, control1: { x: 12, y: 12 }, control2: { x: 12, y: 12 }, end: { x: 12, y: 12 }
			}] }]
		};
		expect(getPreviewConnectionBounds(scene, -1)).toBeNull();
		expect(getPreviewConnectionBounds(scene, 1)).toBeNull();
		expect(getPreviewConnectionBounds(scene, 0)).toEqual({ x: 12, y: 12, width: 1, height: 1 });
	});

	test('fits and centers a requested scene target', () => {
		const transform = calculateSceneFocusTransform(
			{ width: 600, height: 400 },
			{ width: 1000, height: 500 },
			{ x: 300, y: 100, width: 300, height: 200 }
		);
		expect(transform?.zoomPercent).toBe(253);
		expect(transform?.x).toBeCloseTo(75.9);
		expect(transform?.y).toBeCloseTo(75.9);
	});

	test('keeps focus zoom bounded and rejects invalid geometry', () => {
		expect(calculateSceneFocusTransform(
			{ width: 400, height: 500 },
			{ width: 1200, height: 300 },
			{ x: 1100, y: 30, width: 1, height: 1 }
		)?.zoomPercent).toBe(300);
		expect(calculateSceneFocusTransform(
			{ width: 600, height: 400 },
			{ width: 0, height: 500 },
			{ x: 0, y: 0, width: 20, height: 20 }
		)).toBeNull();
	});

	test('clamps wheel zoom to the supported range', () => {
		expect(clampPreviewZoom(-10)).toBe(50);
		expect(clampPreviewZoom(MAX_PREVIEW_ZOOM + 1)).toBe(MAX_PREVIEW_ZOOM);
	});

	test('zooms around the pointer while preserving the point under it', () => {
		expect(zoomPanAtPoint(
			{ x: 0, y: 0 },
			100,
			200,
			{ x: 450, y: 100 },
			{ width: 600, height: 400 }
		)).toEqual({ x: -150, y: 100 });
	});

	test('clamps drag offsets so part of the enlarged diagram remains reachable', () => {
		const pan = clampScenePan(
			{ x: 100_000, y: -100_000 },
			{ width: 600, height: 400 },
			{ width: 1000, height: 500 },
			200
		);
		expect(pan.x).toBe(836);
		expect(pan.y).toBe(-436);
	});
});
