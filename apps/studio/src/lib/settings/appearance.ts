import { readStoredJson, writeStoredJson } from './storage.js';
import type { StudioMessageId } from '../i18n/messages.js';

export type StudioTheme =
	| 'system'
	| 'schemra-light'
	| 'schemra-dark'
	| 'monochrome-light'
	| 'monochrome-dark'
	| 'catppuccin-latte'
	| 'catppuccin-mocha'
	| 'nord'
	| 'dracula';
export type ResolvedTheme = Exclude<StudioTheme, 'system'>;

export const studioThemeOptions: Array<{
	value: StudioTheme;
	labelKey: StudioMessageId;
}> = [
	{ value: 'system', labelKey: 'theme.system' },
	{ value: 'schemra-light', labelKey: 'theme.schemraLight' },
	{ value: 'schemra-dark', labelKey: 'theme.schemraDark' },
	{ value: 'monochrome-light', labelKey: 'theme.monochromeLight' },
	{ value: 'monochrome-dark', labelKey: 'theme.monochromeDark' },
	{ value: 'catppuccin-latte', labelKey: 'theme.catppuccinLatte' },
	{ value: 'catppuccin-mocha', labelKey: 'theme.catppuccinMocha' },
	{ value: 'nord', labelKey: 'theme.nord' },
	{ value: 'dracula', labelKey: 'theme.dracula' }
];

export const editorThemeOptions = [
	{ value: 'app', labelKey: 'settings.editorTheme.followStudio' },
	{ value: 'github-light', labelKey: 'settings.editorTheme.githubLight' },
	{ value: 'github-dark', labelKey: 'settings.editorTheme.githubDark' },
	{ value: 'one-dark', labelKey: 'settings.editorTheme.oneDark' },
	{ value: 'dracula', labelKey: 'settings.editorTheme.dracula' }
] as const;

export type EditorTheme = (typeof editorThemeOptions)[number]['value'];
export type ResolvedEditorTheme = `studio-${ResolvedTheme}` | Exclude<EditorTheme, 'app'>;

type EditorPalette = Record<
	| '--editor-color-scheme'
	| '--editor-background'
	| '--editor-foreground'
	| '--editor-gutter-background'
	| '--editor-gutter-foreground'
	| '--editor-border'
	| '--editor-selection'
	| '--editor-caret'
	| '--editor-syntax-keyword'
	| '--editor-syntax-property'
	| '--editor-syntax-value'
	| '--editor-syntax-number'
	| '--editor-syntax-string'
	| '--editor-syntax-comment'
	| '--editor-syntax-punctuation'
	| '--editor-error',
	string
>;

const editorPalettes: Record<ResolvedEditorTheme, EditorPalette> = {
	'studio-schemra-light': {
		'--editor-color-scheme': 'light',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 18%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#7B3FA1',
		'--editor-syntax-property': '#0758C8',
		'--editor-syntax-value': '#287548',
		'--editor-syntax-number': '#875B09',
		'--editor-syntax-string': '#287548',
		'--editor-syntax-comment': '#5F7086',
		'--editor-syntax-punctuation': '#5F7086',
		'--editor-error': 'var(--destructive)'
	},
	'studio-schemra-dark': {
		'--editor-color-scheme': 'dark',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 24%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#cf9cff',
		'--editor-syntax-property': '#66b8ff',
		'--editor-syntax-value': '#7bdbb0',
		'--editor-syntax-number': '#f1bd72',
		'--editor-syntax-string': '#7bdbb0',
		'--editor-syntax-comment': '#969EAA',
		'--editor-syntax-punctuation': '#969EAA',
		'--editor-error': 'var(--destructive)'
	},
	'github-light': {
		'--editor-color-scheme': 'light',
		'--editor-background': '#ffffff',
		'--editor-foreground': '#1f2328',
		'--editor-gutter-background': '#f6f8fa',
		'--editor-gutter-foreground': '#6e7781',
		'--editor-border': '#d1d9e0',
		'--editor-selection': 'rgb(9 105 218 / 18%)',
		'--editor-caret': '#0969da',
		'--editor-syntax-keyword': '#cf222e',
		'--editor-syntax-property': '#0550ae',
		'--editor-syntax-value': '#953800',
		'--editor-syntax-number': '#0550ae',
		'--editor-syntax-string': '#0a3069',
		'--editor-syntax-comment': '#6e7781',
		'--editor-syntax-punctuation': '#6e7781',
		'--editor-error': '#cf222e'
	},
	'github-dark': {
		'--editor-color-scheme': 'dark',
		'--editor-background': '#0d1117',
		'--editor-foreground': '#e6edf3',
		'--editor-gutter-background': '#161b22',
		'--editor-gutter-foreground': '#8b949e',
		'--editor-border': '#30363d',
		'--editor-selection': 'rgb(56 139 253 / 32%)',
		'--editor-caret': '#58a6ff',
		'--editor-syntax-keyword': '#ff7b72',
		'--editor-syntax-property': '#79c0ff',
		'--editor-syntax-value': '#7ee787',
		'--editor-syntax-number': '#79c0ff',
		'--editor-syntax-string': '#a5d6ff',
		'--editor-syntax-comment': '#8b949e',
		'--editor-syntax-punctuation': '#8b949e',
		'--editor-error': '#ff7b72'
	},
	'one-dark': {
		'--editor-color-scheme': 'dark',
		'--editor-background': '#282c34',
		'--editor-foreground': '#abb2bf',
		'--editor-gutter-background': '#21252b',
		'--editor-gutter-foreground': '#8b95a5',
		'--editor-border': '#3a3f49',
		'--editor-selection': 'rgb(62 68 81 / 72%)',
		'--editor-caret': '#528bff',
		'--editor-syntax-keyword': '#c678dd',
		'--editor-syntax-property': '#e5c07b',
		'--editor-syntax-value': '#98c379',
		'--editor-syntax-number': '#d19a66',
		'--editor-syntax-string': '#98c379',
		'--editor-syntax-comment': '#858c98',
		'--editor-syntax-punctuation': '#abb2bf',
		'--editor-error': '#e06c75'
	},
	'studio-monochrome-light': {
		'--editor-color-scheme': 'light',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 18%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#4a4a4a',
		'--editor-syntax-property': '#111111',
		'--editor-syntax-value': '#666666',
		'--editor-syntax-number': '#333333',
		'--editor-syntax-string': '#666666',
		'--editor-syntax-comment': '#777777',
		'--editor-syntax-punctuation': '#777777',
		'--editor-error': 'var(--destructive)'
	},
	'studio-monochrome-dark': {
		'--editor-color-scheme': 'dark',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 24%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#eeeeee',
		'--editor-syntax-property': '#ffffff',
		'--editor-syntax-value': '#bdbdbd',
		'--editor-syntax-number': '#d8d8d8',
		'--editor-syntax-string': '#bdbdbd',
		'--editor-syntax-comment': '#888888',
		'--editor-syntax-punctuation': '#888888',
		'--editor-error': 'var(--destructive)'
	},
	'studio-catppuccin-latte': {
		'--editor-color-scheme': 'light',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 18%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#8839ef',
		'--editor-syntax-property': '#1e66f5',
		'--editor-syntax-value': '#40a02b',
		'--editor-syntax-number': '#fe640b',
		'--editor-syntax-string': '#179299',
		'--editor-syntax-comment': '#6c6f85',
		'--editor-syntax-punctuation': '#6c6f85',
		'--editor-error': 'var(--destructive)'
	},
	'studio-catppuccin-mocha': {
		'--editor-color-scheme': 'dark',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 24%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#cba6f7',
		'--editor-syntax-property': '#89b4fa',
		'--editor-syntax-value': '#a6e3a1',
		'--editor-syntax-number': '#fab387',
		'--editor-syntax-string': '#94e2d5',
		'--editor-syntax-comment': '#a6adc8',
		'--editor-syntax-punctuation': '#a6adc8',
		'--editor-error': 'var(--destructive)'
	},
	'studio-nord': {
		'--editor-color-scheme': 'dark',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 24%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#b48ead',
		'--editor-syntax-property': '#88c0d0',
		'--editor-syntax-value': '#a3be8c',
		'--editor-syntax-number': '#ebcb8b',
		'--editor-syntax-string': '#a3be8c',
		'--editor-syntax-comment': '#d8dee9',
		'--editor-syntax-punctuation': '#d8dee9',
		'--editor-error': 'var(--destructive)'
	},
	'studio-dracula': {
		'--editor-color-scheme': 'dark',
		'--editor-background': 'var(--background)',
		'--editor-foreground': 'var(--foreground)',
		'--editor-gutter-background': 'var(--muted)',
		'--editor-gutter-foreground': 'var(--muted-foreground)',
		'--editor-border': 'var(--border)',
		'--editor-selection': 'color-mix(in srgb, var(--primary) 24%, transparent)',
		'--editor-caret': 'var(--primary)',
		'--editor-syntax-keyword': '#ff79c6',
		'--editor-syntax-property': '#8be9fd',
		'--editor-syntax-value': '#50fa7b',
		'--editor-syntax-number': '#bd93f9',
		'--editor-syntax-string': '#f1fa8c',
		'--editor-syntax-comment': '#919bb8',
		'--editor-syntax-punctuation': '#a6a9c4',
		'--editor-error': 'var(--destructive)'
	},
	dracula: {
		'--editor-color-scheme': 'dark',
		'--editor-background': '#282a36',
		'--editor-foreground': '#f8f8f2',
		'--editor-gutter-background': '#21222c',
		'--editor-gutter-foreground': '#9aa7d7',
		'--editor-border': '#44475a',
		'--editor-selection': 'rgb(68 71 90 / 76%)',
		'--editor-caret': '#f8f8f2',
		'--editor-syntax-keyword': '#ff79c6',
		'--editor-syntax-property': '#8be9fd',
		'--editor-syntax-value': '#bd93f9',
		'--editor-syntax-number': '#bd93f9',
		'--editor-syntax-string': '#f1fa8c',
		'--editor-syntax-comment': '#919bb8',
		'--editor-syntax-punctuation': '#a6a9c4',
		'--editor-error': '#ff5555'
	}
};

export interface ThemePreferences {
	studio: StudioTheme;
	editor: EditorTheme;
	reduceMotion: boolean;
}

export const DEFAULT_THEME_PREFERENCES: ThemePreferences = {
	studio: 'system',
	editor: 'app',
	reduceMotion: false
};

export const THEME_PREFERENCES_STORAGE_KEY = 'schemra.studio.theme-preferences.v3';
const PREVIOUS_THEME_PREFERENCES_STORAGE_KEY = 'schemra.studio.theme-preferences.v2';
const LEGACY_THEME_PREFERENCES_STORAGE_KEY = 'schemra.studio.theme-preferences.v1';

export function normalizeThemePreferences(value: unknown): ThemePreferences {
	if (!isRecord(value)) return { ...DEFAULT_THEME_PREFERENCES };
	return {
		studio: normalizeStudioTheme(value.studio),
		editor: isEditorTheme(value.editor) ? value.editor : DEFAULT_THEME_PREFERENCES.editor,
		reduceMotion: value.reduceMotion === true
	};
}

export function readThemePreferences(): ThemePreferences {
	const current = readStoredJson(THEME_PREFERENCES_STORAGE_KEY);
	const previous = current.found ? { found: false } : readStoredJson(PREVIOUS_THEME_PREFERENCES_STORAGE_KEY);
	const legacy = current.found || previous.found
		? { found: false }
		: readStoredJson(LEGACY_THEME_PREFERENCES_STORAGE_KEY);
	const stored = current.found ? current : previous.found ? previous : legacy;
	if (!stored.found) return { ...DEFAULT_THEME_PREFERENCES };
	const preferences = normalizeThemePreferences(stored.value);
	if (!current.found) writeStoredJson(THEME_PREFERENCES_STORAGE_KEY, preferences);
	return preferences;
}

export function writeThemePreferences(preferences: ThemePreferences): boolean {
	return writeStoredJson(THEME_PREFERENCES_STORAGE_KEY, normalizeThemePreferences(preferences));
}

export function resolveStudioTheme(theme: StudioTheme, systemPrefersDark: boolean): ResolvedTheme {
	if (theme === 'system') return systemPrefersDark ? 'schemra-dark' : 'schemra-light';
	return theme;
}

export function isDarkStudioTheme(theme: ResolvedTheme): boolean {
	switch (theme) {
		case 'schemra-dark':
		case 'monochrome-dark':
		case 'catppuccin-mocha':
		case 'nord':
		case 'dracula':
			return true;
		default:
			return false;
	}
}

export function resolveEditorTheme(theme: EditorTheme, studioTheme: ResolvedTheme): ResolvedEditorTheme {
	if (theme !== 'app') return theme;
	return `studio-${studioTheme}`;
}

export function editorThemeStyles(theme: ResolvedEditorTheme): string {
	return Object.entries(editorPalettes[theme])
		.map(([property, value]) => `${property}:${value}`)
		.join(';');
}

function normalizeStudioTheme(value: unknown): StudioTheme {
	if (studioThemeOptions.some((option) => option.value === value)) return value as StudioTheme;
	if (value === 'light') return 'schemra-light';
	if (value === 'dark') return 'schemra-dark';
	return DEFAULT_THEME_PREFERENCES.studio;
}

function isEditorTheme(value: unknown): value is EditorTheme {
	return editorThemeOptions.some((option) => option.value === value);
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
