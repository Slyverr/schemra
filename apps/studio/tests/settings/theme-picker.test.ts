import { describe, expect, it } from 'bun:test';
import { nextThemeOptionIndex } from '../../src/lib/settings/theme-picker.ts';

describe('searchable theme picker navigation', () => {
	it('wraps arrow navigation across the filtered options', () => {
		expect(nextThemeOptionIndex(0, 3, 'ArrowUp')).toBe(2);
		expect(nextThemeOptionIndex(2, 3, 'ArrowDown')).toBe(0);
		expect(nextThemeOptionIndex(1, 3, 'ArrowDown')).toBe(2);
	});

	it('moves to the first and last filtered options', () => {
		expect(nextThemeOptionIndex(1, 3, 'Home')).toBe(0);
		expect(nextThemeOptionIndex(1, 3, 'End')).toBe(2);
	});

	it('keeps the active option inside the filtered result range', () => {
		expect(nextThemeOptionIndex(8, 3, 'ArrowDown')).toBe(0);
		expect(nextThemeOptionIndex(-2, 3, 'ArrowUp')).toBe(2);
		expect(nextThemeOptionIndex(0, 0, 'ArrowDown')).toBe(-1);
	});
});
