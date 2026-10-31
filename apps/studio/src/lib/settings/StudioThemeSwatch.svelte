<script lang="ts">
	import { cn } from '../utils.js';
	import type { StudioTheme } from './appearance.js';
	import { studioThemePreviewPalettes } from './theme-preview.js';

	let { theme, class: className = '' }: { theme: StudioTheme; class?: string } = $props();

	const palette = $derived(studioThemePreviewPalettes[theme]);
</script>

<svg class={cn('size-12 shrink-0', className)} viewBox="0 0 48 40" aria-hidden="true">
	<rect x=".5" y=".5" width="47" height="39" rx="6" fill={palette.background} stroke="var(--border)" stroke-width=".6" />
	{#if palette.backgroundDark}
		<path d="M25 1H47.5V39H19Z" fill={palette.backgroundDark} />
	{/if}
	<rect x="4.5" y="4.5" width="39" height="31" rx="4" fill={palette.card} />
	{#if palette.cardDark}
		<path d="M26 5H43.5V35H21Z" fill={palette.cardDark} />
	{/if}
	<circle cx="13" cy="20" r="4" fill={palette.primary} />
	<rect x="22" y="15.5" width="11.2" height="3" rx="1.5" fill={palette.foreground} opacity=".85" />
	<rect x="22" y="20" width="16" height="2" rx="1" fill={palette.muted} opacity=".65" />
	<rect x="22" y="24" width="8.8" height="2" rx="1" fill={palette.muted} opacity=".65" />
</svg>
