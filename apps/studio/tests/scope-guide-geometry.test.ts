import { describe, expect, test } from 'bun:test';
import { getScopeGuideGeometry } from '../src/lib/editor/scope-guide-geometry.js';

describe('scope guide geometry', () => {
	test('starts below the opening line and ends at the last body line', () => {
		expect(
			getScopeGuideGeometry(
				{ left: 120, top: 200 },
				{ left: 136, bottom: 224 },
				{ bottom: 272 },
				8.5
			)
		).toEqual({ left: 24.5, top: 24, height: 48 });
	});

	test('keeps an empty or invalid measured body from producing a negative guide', () => {
		expect(
			getScopeGuideGeometry(
				{ left: 120, top: 200 },
				{ left: 136, bottom: 224 },
				{ bottom: 220 },
				8.5
			)
		).toEqual({ left: 24.5, top: 24, height: 0 });
	});
});
