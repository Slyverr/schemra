import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import {
	DEFAULT_EDITOR_PREFERENCES,
	EDITOR_PREFERENCES_STORAGE_KEY,
	normalizeEditorPreferences,
	readEditorPreferences,
	stepEditorZoom,
	writeEditorPreferences
} from '../../src/lib/settings/editor';
import { installMemoryStorage } from './storage-test-helper';

let storage: ReturnType<typeof installMemoryStorage>;

beforeEach(() => {
	storage = installMemoryStorage();
});

afterEach(() => storage.restore());

describe('editor preferences', () => {
	test('provides a stable default set for a new browser profile', () => {
		expect(readEditorPreferences()).toEqual(DEFAULT_EDITOR_PREFERENCES);
	});

	test('round-trips supported behavior and appearance settings', () => {
		const preferences = {
			zoom: 130,
			indentSize: 4,
			lineNumbers: false,
			indentGuides: true,
			diagnosticPresentation: 'inline',
			activeLineHighlight: false,
			matchingBracketHighlight: true,
			autoIndent: false,
			autoClosePairs: false,
			formatOnSave: false
		} as const;

		expect(writeEditorPreferences(preferences)).toBe(true);
		expect(storage.records.has(EDITOR_PREFERENCES_STORAGE_KEY)).toBe(true);
		expect(readEditorPreferences()).toEqual(preferences);
	});

	test('clamps zoom to the nearest allowed level and defaults invalid fields', () => {
		expect(normalizeEditorPreferences({ zoom: 128, indentSize: 3, indentGuides: false })).toEqual({
			...DEFAULT_EDITOR_PREFERENCES,
			zoom: 130,
			indentGuides: false
		});
		expect(normalizeEditorPreferences({ zoom: Number.NaN }).zoom).toBe(100);
		expect(normalizeEditorPreferences({ diagnosticPresentation: 'invalid' }).diagnosticPresentation).toBe('hover');
	});

	test('steps zoom within its declared bounds', () => {
		expect(stepEditorZoom(100, 1)).toBe(110);
		expect(stepEditorZoom(80, -1)).toBe(80);
		expect(stepEditorZoom(160, 1)).toBe(160);
	});

	test('tolerates malformed saved JSON and blocked browser storage', () => {
		storage.records.set(EDITOR_PREFERENCES_STORAGE_KEY, '{');
		expect(readEditorPreferences()).toEqual(DEFAULT_EDITOR_PREFERENCES);
		Object.defineProperty(globalThis, 'localStorage', {
			configurable: true,
			get() { throw new Error('storage blocked'); }
		});
		expect(readEditorPreferences()).toEqual(DEFAULT_EDITOR_PREFERENCES);
		expect(writeEditorPreferences(DEFAULT_EDITOR_PREFERENCES)).toBe(false);
	});
});
