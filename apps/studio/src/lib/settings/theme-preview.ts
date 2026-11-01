import type { StudioTheme } from './appearance.js';

export interface ThemeSwatchPalette {
	background: string;
	foreground: string;
	muted: string;
	primary: string;
	backgroundDark?: string;
	foregroundDark?: string;
	mutedDark?: string;
	primaryDark?: string;
}

export const studioThemePreviewPalettes: Record<StudioTheme, ThemeSwatchPalette> = {
	system: {
		background: '#f7faff', backgroundDark: '#09172d', foreground: '#102442',
		muted: '#71839b', primary: '#0758c8',
		foregroundDark: '#eaf3ff', mutedDark: '#a9c1dd', primaryDark: '#28aaff'
	},
	'schemra-light': {
		background: '#f7faff', foreground: '#102442',
		muted: '#5c6f87', primary: '#0758c8'
	},
	'schemra-dark': {
		background: '#09172d', foreground: '#eaf3ff',
		muted: '#a9c1dd', primary: '#28aaff'
	},
	'monochrome-light': {
		background: '#ffffff', foreground: '#171717',
		muted: '#737373', primary: '#262626'
	},
	'monochrome-dark': {
		background: '#171717', foreground: '#f5f5f5',
		muted: '#b3b3b3', primary: '#e5e5e5'
	},
	'catppuccin-latte': {
		background: '#eff1f5', foreground: '#4c4f69',
		muted: '#6c6f85', primary: '#8839ef'
	},
	'catppuccin-mocha': {
		background: '#1e1e2e', foreground: '#cdd6f4',
		muted: '#a6adc8', primary: '#cba6f7'
	},
	nord: {
		background: '#2e3440', foreground: '#eceff4',
		muted: '#d8dee9', primary: '#88c0d0'
	},
	dracula: {
		background: '#282a36', foreground: '#f8f8f2',
		muted: '#bdc1d5', primary: '#bd93f9'
	}
};
