import { afterEach, beforeEach, describe, expect, test } from 'bun:test';
import {
	DEFAULT_THEME_PREFERENCES,
	THEME_PREFERENCES_STORAGE_KEY,
	editorThemeOptions,
	editorThemeStyles,
	isDarkStudioTheme,
	normalizeThemePreferences,
	readThemePreferences,
	resolveEditorTheme,
	resolveStudioTheme,
	studioThemeOptions,
	writeThemePreferences
} from '../../src/lib/settings/appearance';
import { installMemoryStorage } from './storage-test-helper';

const previousStorageKey = 'schemra.studio.theme-preferences.v2';
const legacyStorageKey = 'schemra.studio.theme-preferences.v1';
let storage: ReturnType<typeof installMemoryStorage>;

beforeEach(() => {
	storage = installMemoryStorage();
});

afterEach(() => storage.restore());

describe('appearance preferences', () => {
	test('defaults to system appearance with Studio-matched editor colors', () => {
		expect(readThemePreferences()).toEqual(DEFAULT_THEME_PREFERENCES);
		expect(studioThemeOptions.map(({ value }) => value)).toEqual(['system', 'schemra-light', 'schemra-dark']);
	});

	test('resolves system appearance to Schemra light or dark', () => {
		expect(resolveStudioTheme('system', false)).toBe('schemra-light');
		expect(resolveStudioTheme('system', true)).toBe('schemra-dark');
		expect(isDarkStudioTheme('schemra-dark')).toBe(true);
		expect(isDarkStudioTheme('schemra-light')).toBe(false);
	});

	test('round-trips supported theme and motion choices', () => {
		const preferences = { studio: 'schemra-dark', editor: 'github-dark', reduceMotion: true } as const;
		expect(writeThemePreferences(preferences)).toBe(true);
		expect(readThemePreferences()).toEqual(preferences);
	});

	test('normalizes unsupported settings into supported Schemra defaults', () => {
		expect(normalizeThemePreferences({ studio: 'catppuccin-mocha', editor: 'unknown', reduceMotion: 1 }))
			.toEqual({ studio: 'schemra-dark', editor: 'app', reduceMotion: false });
		expect(normalizeThemePreferences({ studio: 'not-a-theme' }).studio).toBe('system');
	});

	test('migrates saved palette names to the available Schemra themes', () => {
		storage.records.set(legacyStorageKey, JSON.stringify({ studio: 'dark', editor: 'one-dark' }));
		expect(readThemePreferences()).toEqual({ studio: 'schemra-dark', editor: 'one-dark', reduceMotion: false });
		expect(JSON.parse(storage.records.get(THEME_PREFERENCES_STORAGE_KEY) ?? '{}')).toEqual({
			studio: 'schemra-dark', editor: 'one-dark', reduceMotion: false
		});
	});

	test('keeps an explicitly saved Schemra theme and prioritizes the newest stored version', () => {
		storage.records.set(previousStorageKey, JSON.stringify({ studio: 'schemra-light', editor: 'app' }));
		expect(readThemePreferences().studio).toBe('schemra-light');
		storage.records.set(THEME_PREFERENCES_STORAGE_KEY, JSON.stringify({ studio: 'schemra-dark' }));
		expect(readThemePreferences().studio).toBe('schemra-dark');
	});

	test('exposes valid standalone syntax themes and resolves app colors from Studio theme', () => {
		expect(editorThemeOptions.map(({ value }) => value)).toEqual([
			'app', 'github-light', 'github-dark', 'one-dark', 'dracula'
		]);
		expect(resolveEditorTheme('app', 'schemra-light')).toBe('studio-schemra-light');
		expect(resolveEditorTheme('app', 'schemra-dark')).toBe('studio-schemra-dark');
		expect(resolveEditorTheme('dracula', 'schemra-light')).toBe('dracula');
		expect(editorThemeStyles('studio-schemra-light')).toContain('--editor-syntax-property:#0758C8');
	});
});
