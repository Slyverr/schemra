import { describe, expect, test } from 'bun:test';
import {
	DEFAULT_EDITOR_PREFERENCES,
	normalizeEditorPreferences
} from '../src/lib/settings/editor.js';

describe('editor preferences passed to SourceEditor', () => {
	test('keeps the settings defaults when no preference object is supplied', () => {
		expect(normalizeEditorPreferences(undefined)).toEqual(DEFAULT_EDITOR_PREFERENCES);
	});

	test('retains editor options and snaps unsupported zoom values to a supported step', () => {
		expect(normalizeEditorPreferences({
			zoom: 125,
			indentSize: 4,
			lineNumbers: false,
			indentGuides: false,
			diagnosticPresentation: 'inline',
			activeLineHighlight: false,
			matchingBracketHighlight: false,
			autoIndent: false,
			autoClosePairs: false,
			formatOnSave: false
		})).toEqual({
			zoom: 120,
			indentSize: 4,
			lineNumbers: false,
			indentGuides: false,
			diagnosticPresentation: 'inline',
			activeLineHighlight: false,
			matchingBracketHighlight: false,
			autoIndent: false,
			autoClosePairs: false,
			formatOnSave: false
		});
	});
});
