import { readStoredJson, writeStoredJson } from './storage.js';

export const EDITOR_ZOOM_LEVELS = [80, 90, 100, 110, 120, 130, 140, 150, 160] as const;

export type EditorZoom = (typeof EDITOR_ZOOM_LEVELS)[number];
export type EditorIndentSize = 2 | 4;
export type DiagnosticPresentation = 'hover' | 'inline';

export interface EditorPreferences {
	zoom: EditorZoom;
	indentSize: EditorIndentSize;
	lineNumbers: boolean;
	indentGuides: boolean;
	diagnosticPresentation: DiagnosticPresentation;
	activeLineHighlight: boolean;
	matchingBracketHighlight: boolean;
	autoIndent: boolean;
	autoClosePairs: boolean;
	formatOnSave: boolean;
}

export const DEFAULT_EDITOR_PREFERENCES: EditorPreferences = {
	zoom: 100,
	indentSize: 2,
	lineNumbers: true,
	indentGuides: true,
	diagnosticPresentation: 'hover',
	activeLineHighlight: true,
	matchingBracketHighlight: true,
	autoIndent: true,
	autoClosePairs: true,
	formatOnSave: true
};

export const EDITOR_PREFERENCES_STORAGE_KEY = 'schemra.studio.editor-preferences.v1';

export function normalizeEditorPreferences(value: unknown): EditorPreferences {
	if (!isRecord(value)) return { ...DEFAULT_EDITOR_PREFERENCES };
	const requestedZoom = typeof value.zoom === 'number' && Number.isFinite(value.zoom)
		? value.zoom
		: DEFAULT_EDITOR_PREFERENCES.zoom;
	const zoom = EDITOR_ZOOM_LEVELS.reduce((closest, candidate) =>
		Math.abs(candidate - requestedZoom) < Math.abs(closest - requestedZoom) ? candidate : closest
	);

	return {
		zoom,
		indentSize: value.indentSize === 4 ? 4 : 2,
		lineNumbers: readBoolean(value.lineNumbers, DEFAULT_EDITOR_PREFERENCES.lineNumbers),
		indentGuides: readBoolean(value.indentGuides, DEFAULT_EDITOR_PREFERENCES.indentGuides),
		diagnosticPresentation: value.diagnosticPresentation === 'inline' ? 'inline' : 'hover',
		activeLineHighlight: readBoolean(value.activeLineHighlight, DEFAULT_EDITOR_PREFERENCES.activeLineHighlight),
		matchingBracketHighlight: readBoolean(
			value.matchingBracketHighlight,
			DEFAULT_EDITOR_PREFERENCES.matchingBracketHighlight
		),
		autoIndent: readBoolean(value.autoIndent, DEFAULT_EDITOR_PREFERENCES.autoIndent),
		autoClosePairs: readBoolean(value.autoClosePairs, DEFAULT_EDITOR_PREFERENCES.autoClosePairs),
		formatOnSave: readBoolean(value.formatOnSave, DEFAULT_EDITOR_PREFERENCES.formatOnSave)
	};
}

export function stepEditorZoom(zoom: EditorZoom, direction: -1 | 1): EditorZoom {
	const currentIndex = EDITOR_ZOOM_LEVELS.indexOf(zoom);
	const nextIndex = Math.min(EDITOR_ZOOM_LEVELS.length - 1, Math.max(0, currentIndex + direction));
	return EDITOR_ZOOM_LEVELS[nextIndex] ?? DEFAULT_EDITOR_PREFERENCES.zoom;
}

export function readEditorPreferences(): EditorPreferences {
	const stored = readStoredJson(EDITOR_PREFERENCES_STORAGE_KEY);
	return stored.found ? normalizeEditorPreferences(stored.value) : { ...DEFAULT_EDITOR_PREFERENCES };
}

export function writeEditorPreferences(preferences: EditorPreferences): boolean {
	return writeStoredJson(EDITOR_PREFERENCES_STORAGE_KEY, normalizeEditorPreferences(preferences));
}

function readBoolean(value: unknown, fallback: boolean): boolean {
	return typeof value === 'boolean' ? value : fallback;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
