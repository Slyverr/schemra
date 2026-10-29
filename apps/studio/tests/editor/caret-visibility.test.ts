import { describe, expect, test } from 'bun:test';
import { getCaretScrollAdjustment } from '../../src/lib/editor/caret-visibility.js';

const viewport = {
	viewportLeft: 100,
	viewportTop: 100,
	viewportRight: 500,
	viewportBottom: 300
};

describe('caret visibility geometry', () => {
	test('scrolls the caret into the horizontal safe area on either side', () => {
		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: 499,
			lineTop: 160,
			lineBottom: 184
		}, 24)).toEqual({ left: 7, top: 0 });

		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: 105,
			lineTop: 160,
			lineBottom: 184
		}, 24)).toEqual({ left: -7, top: 0 });
	});

	test('scrolls the caret line into view with a one-line vertical margin', () => {
		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: 250,
			lineTop: 280,
			lineBottom: 304
		}, 24)).toEqual({ left: 0, top: 28 });

		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: 250,
			lineTop: 105,
			lineBottom: 129
		}, 24)).toEqual({ left: 0, top: -19 });
	});

	test('leaves a visible caret in place and ignores invalid geometry', () => {
		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: 250,
			lineTop: 160,
			lineBottom: 184
		}, 24)).toEqual({ left: 0, top: 0 });

		expect(getCaretScrollAdjustment({
			...viewport,
			viewportBottom: 130,
			caretLeft: 250,
			lineTop: 105,
			lineBottom: 129
		}, 24)).toEqual({ left: 0, top: 2 });

		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: Number.NaN,
			lineTop: 160,
			lineBottom: 184
		}, 24)).toEqual({ left: 0, top: 0 });

		expect(getCaretScrollAdjustment({
			...viewport,
			caretLeft: 250,
			lineTop: 184,
			lineBottom: 160
		}, 24)).toEqual({ left: 0, top: 0 });
	});
});
