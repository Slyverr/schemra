<script lang="ts">
	import { Pause, Play, RotateCcw } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';

	interface Props {
		durationMs: number;
		timeMs: number;
		playing: boolean;
		ready: boolean;
		onToggle: () => void;
		onReset: () => void;
		onSeek: (event: Event) => void;
	}

	let props: Props = $props();
	const formatTime = (value: number) => `${(Math.max(0, value) / 1000).toFixed(2)}s`;
	const timeDescription = $derived(studioMessage('preview.playbackTime', {
		current: formatTime(props.timeMs),
		duration: formatTime(props.durationMs)
	}));
</script>

<div class="timeline">
	<button
		class="play-button"
		type="button"
		aria-label={studioMessage(props.playing ? 'preview.pause' : 'preview.play')}
		title={studioMessage(props.playing ? 'preview.pause' : 'preview.play')}
		disabled={!props.ready || props.durationMs <= 0}
		onclick={props.onToggle}
	>
		{#if props.playing}<Pause size={16} aria-hidden="true" />{:else}<Play size={16} aria-hidden="true" />{/if}
	</button>
	<button class="reset-button" type="button" aria-label={studioMessage('preview.restartScenario')} title={studioMessage('preview.restartScenario')} disabled={!props.ready} onclick={props.onReset}>
		<RotateCcw size={16} aria-hidden="true" />
	</button>
	<input
		class="scenario-range"
		type="range"
		min="0"
		max={Math.max(1, props.durationMs)}
		step="1"
		value={props.timeMs}
		style={`--scenario-progress: ${props.durationMs > 0 ? (props.timeMs / props.durationMs) * 100 : 0}%`}
		aria-label={studioMessage('preview.scenarioTimeline')}
		disabled={!props.ready}
		oninput={props.onSeek}
	/>
	<output class="playback-time" aria-label={timeDescription} aria-live="off">
		<strong>{formatTime(props.timeMs)}</strong><span aria-hidden="true">/</span><span>{formatTime(props.durationMs)}</span>
	</output>
</div>

<style>
	.timeline { position: absolute; z-index: 10; inset: auto 1rem 1rem; display: flex; min-height: 3rem; align-items: center; gap: .5rem; padding: .5rem .625rem; border: 1px solid var(--border); border-radius: .5rem; background: var(--background); box-shadow: 0 2px 8px rgb(16 36 66 / 8%); }
	.timeline button { display: grid; width: 2rem; height: 2rem; flex: 0 0 auto; place-items: center; border: 0; border-radius: .35rem; color: var(--foreground); background: transparent; cursor: pointer; }
	.timeline button:hover:not(:disabled) { background: var(--accent); }
	.timeline button:disabled { cursor: not-allowed; opacity: .5; }
	.timeline button:focus-visible, .scenario-range:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
	.timeline .play-button { color: var(--primary-foreground); background: var(--primary); }
	.timeline .play-button:hover:not(:disabled) { background: color-mix(in srgb, var(--primary) 88%, var(--foreground)); }
	.scenario-range { appearance: none; min-width: 2rem; min-height: 1.5rem; flex: 1 1 auto; background: transparent; cursor: pointer; }
	.scenario-range:disabled { cursor: not-allowed; }
	.scenario-range::-webkit-slider-runnable-track { height: .25rem; border-radius: 9999px; background: linear-gradient(to right, var(--primary) 0 var(--scenario-progress), var(--border) var(--scenario-progress) 100%); }
	.scenario-range::-moz-range-track { height: .25rem; border-radius: 9999px; background: var(--border); }
	.scenario-range::-moz-range-progress { height: .25rem; border-radius: 9999px; background: var(--primary); }
	.scenario-range::-webkit-slider-thumb { appearance: none; width: .875rem; height: .875rem; margin-top: -.3125rem; border: 2px solid var(--background); border-radius: 50%; background: var(--primary); box-shadow: 0 0 0 1px var(--border); }
	.scenario-range::-moz-range-thumb { width: .75rem; height: .75rem; border: 2px solid var(--background); border-radius: 50%; background: var(--primary); box-shadow: 0 0 0 1px var(--border); }
	.playback-time { display: inline-flex; min-width: max-content; align-items: baseline; gap: .25rem; color: var(--muted-foreground); font-family: ui-monospace, monospace; font-size: .75rem; line-height: 1; font-variant-numeric: tabular-nums; white-space: nowrap; }
	.playback-time strong { color: var(--foreground); }
	@media (max-width: 480px) { .timeline { inset-inline: .5rem; flex-wrap: wrap; } .scenario-range { order: 3; flex-basis: 100%; } }
</style>
