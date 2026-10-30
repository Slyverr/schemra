import type { StudioTheme } from './appearance.js';

export interface ThemeSwatchPalette {
	background: string;
	border: string;
	card: string;
	foreground: string;
	muted: string;
	primary: string;
	backgroundDark?: string;
	cardDark?: string;
}

export const studioThemePreviewPalettes: Record<StudioTheme, ThemeSwatchPalette> = {
	system: {
		background: '#f7faff', backgroundDark: '#09172d', border: '#9eb4ce', card: '#ffffff',
		cardDark: '#0d2342', foreground: '#102442', muted: '#71839b', primary: '#0758c8'
	},
	'schemra-light': {
		background: '#f7faff', border: '#d6e2f1', card: '#ffffff', foreground: '#102442',
		muted: '#5c6f87', primary: '#0758c8'
	},
	'schemra-dark': {
		background: '#09172d', border: '#23476e', card: '#0d2342', foreground: '#eaf3ff',
		muted: '#a9c1dd', primary: '#28aaff'
	},
	'monochrome-light': {
		background: '#ffffff', border: '#d4d4d4', card: '#f5f5f5', foreground: '#171717',
		muted: '#737373', primary: '#262626'
	},
	'monochrome-dark': {
		background: '#171717', border: '#414141', card: '#202020', foreground: '#f5f5f5',
		muted: '#b3b3b3', primary: '#e5e5e5'
	},
	'catppuccin-latte': {
		background: '#eff1f5', border: '#ccd0da', card: '#e6e9ef', foreground: '#4c4f69',
		muted: '#6c6f85', primary: '#8839ef'
	},
	'catppuccin-mocha': {
		background: '#1e1e2e', border: '#45475a', card: '#181825', foreground: '#cdd6f4',
		muted: '#a6adc8', primary: '#cba6f7'
	},
	nord: {
		background: '#2e3440', border: '#4c566a', card: '#3b4252', foreground: '#eceff4',
		muted: '#d8dee9', primary: '#88c0d0'
	},
	dracula: {
		background: '#282a36', border: '#44475a', card: '#21222c', foreground: '#f8f8f2',
		muted: '#bdc1d5', primary: '#bd93f9'
	}
};
