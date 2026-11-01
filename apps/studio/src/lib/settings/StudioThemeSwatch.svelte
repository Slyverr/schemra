<script lang="ts">
	import { cn } from '../utils.js';
	import type { StudioTheme } from './appearance.js';
	import { studioThemePreviewPalettes } from './theme-preview.js';

	let { theme, class: className = '' }: { theme: StudioTheme; class?: string } = $props();

	const palette = $derived(studioThemePreviewPalettes[theme]);
</script>

<svg data-slot="theme-preview" class={cn('w-16 aspect-video shrink-0', className)} viewBox="0 0 64 36" aria-hidden="true">
	<rect width="64" height="36" rx="4" fill={palette.background} />
	{#if palette.backgroundDark}
		<path d="M32 0H60A4 4 0 0 1 64 4V32A4 4 0 0 1 60 36H32Z" fill={palette.backgroundDark} />
		<circle cx="8" cy="18" r="2.5" fill={palette.primary} />
		<rect x="14" y="13.5" width="14" height="2.5" rx="1.25" fill={palette.foreground} opacity=".85" />
		<rect x="14" y="18" width="15" height="1.75" rx=".875" fill={palette.muted} opacity=".65" />
		<rect x="14" y="22" width="10" height="1.75" rx=".875" fill={palette.muted} opacity=".65" />
		<circle cx="40" cy="18" r="2.5" fill={palette.primaryDark ?? palette.primary} />
		<rect x="46" y="13.5" width="13" height="2.5" rx="1.25" fill={palette.foregroundDark ?? palette.foreground} opacity=".85" />
		<rect x="46" y="18" width="13" height="1.75" rx=".875" fill={palette.mutedDark ?? palette.muted} opacity=".65" />
		<rect x="46" y="22" width="9" height="1.75" rx=".875" fill={palette.mutedDark ?? palette.muted} opacity=".65" />
	{:else}
		<circle cx="8" cy="18" r="3" fill={palette.primary} />
		<rect x="15" y="13.5" width="35" height="3" rx="1.5" fill={palette.foreground} opacity=".85" />
		<rect x="15" y="18" width="42" height="2" rx="1" fill={palette.muted} opacity=".65" />
		<rect x="15" y="22" width="27" height="2" rx="1" fill={palette.muted} opacity=".65" />
	{/if}
	<rect x=".5" y=".5" width="63" height="35" rx="4" fill="none" stroke="var(--border)" />
</svg>
