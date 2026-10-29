import { describe, expect, test } from 'bun:test';
import { calculateNodeFocusZoom, calculateNodeScrollTarget, getPreviewConnectionBounds } from '../../src/lib/preview/viewport.js';

describe('preview node focus geometry', () => {
	test('builds a connection focus rectangle from its endpoints and cubic route', () => {
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

	test('rejects missing connection indexes and keeps degenerate paths focusable', () => {
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

	test('zooms a node to fill available space while leaving a margin', () => {
		expect(calculateNodeFocusZoom(
			{ width: 1000, height: 500 },
			{ x: 300, y: 100, width: 300, height: 200 },
			{ width: 600, height: 400 }
		)).toBe(293);
	});

	test('caps zoom for tiny nodes and works for tall or wide layouts', () => {
		expect(calculateNodeFocusZoom(
			{ width: 1200, height: 300 },
			{ x: 1100, y: 30, width: 30, height: 40 },
			{ width: 400, height: 500 }
		)).toBe(500);
		expect(calculateNodeFocusZoom(
			{ width: 500, height: 1200 },
			{ x: 30, y: 900, width: 120, height: 260 },
			{ width: 500, height: 400 }
		)).toBeGreaterThan(100);
	});

	test('rejects invalid dimensions instead of emitting a non-finite zoom', () => {
		expect(calculateNodeFocusZoom(
			{ width: 0, height: 500 },
			{ x: 0, y: 0, width: 20, height: 20 },
			{ width: 600, height: 400 }
		)).toBeNull();
	});

	test('centers the selected node in the scroll viewport using scene and sheet scale', () => {
		expect(calculateNodeScrollTarget(
			{ width: 1000, height: 500 },
			{ x: 300, y: 100, width: 200, height: 100 },
			{ width: 800, height: 400 },
			{ x: 16, y: 16 },
			{ width: 400, height: 200 },
			{ x: 600, y: 300 }
		)).toEqual({ left: 136, top: 36 });
	});

	test('clamps reveal targets to available scroll bounds and rejects invalid geometry', () => {
		expect(calculateNodeScrollTarget(
			{ width: 100, height: 100 },
			{ x: 0, y: 0, width: 10, height: 10 },
			{ width: 100, height: 100 },
			{ x: 0, y: 0 },
			{ width: 200, height: 200 },
			{ x: 20, y: 30 }
		)).toEqual({ left: 0, top: 0 });
		expect(calculateNodeScrollTarget(
			{ width: 100, height: 100 },
			{ x: 90, y: 90, width: 10, height: 10 },
			{ width: 300, height: 300 },
			{ x: 0, y: 0 },
			{ width: 100, height: 100 },
			{ x: 50, y: 50 }
		)).toEqual({ left: 50, top: 50 });
		expect(calculateNodeScrollTarget(
			{ width: 100, height: 100 },
			{ x: 10, y: 10, width: 0, height: 20 },
			{ width: 300, height: 300 },
			{ x: 0, y: 0 },
			{ width: 100, height: 100 },
			{ x: 50, y: 50 }
		)).toBeNull();
	});
});
