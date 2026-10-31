<script lang="ts">
	import { Pause, Play, RotateCcw } from '@lucide/svelte';
	import { Slider } from 'bits-ui';
	import { Button } from '../components/ui/button/index.js';
	import { studioMessage } from '../i18n/messages.js';

	interface Props {
		durationMs: number;
		timeMs: number;
		playing: boolean;
		ready: boolean;
		onToggle: () => void;
		onReset: () => void;
		onSeek: (timeMs: number) => void;
		onTimelinePointerDown: () => void;
		onTimelineKeydown: (event: KeyboardEvent) => void;
	}

	let props: Props = $props();
	const formatTime = (value: number) => `${(Math.max(0, value) / 1000).toFixed(2)}s`;
	const timeDescription = $derived(studioMessage('preview.playbackTime', {
		current: formatTime(props.timeMs),
		duration: formatTime(props.durationMs)
	}));
</script>

<div class="timeline absolute inset-x-4 bottom-4 z-10 flex min-h-12 items-center gap-2 rounded-lg border border-border bg-background px-2.5 py-2 shadow-[0_2px_8px_rgb(16_36_66_/_8%)] max-[480px]:inset-x-2 max-[480px]:flex-wrap">
	<Button
		variant="default"
		size="icon-sm"
		class="size-8"
		type="button"
		aria-label={studioMessage(props.playing ? 'preview.pause' : 'preview.play')}
		title={studioMessage(props.playing ? 'preview.pause' : 'preview.play')}
		disabled={!props.ready || props.durationMs <= 0}
		onclick={props.onToggle}
	>
		{#if props.playing}<Pause size={16} aria-hidden="true" />{:else}<Play size={16} aria-hidden="true" />{/if}
	</Button>
	<Button variant="ghost" size="icon-sm" class="size-8" type="button" aria-label={studioMessage('preview.restartScenario')} title={studioMessage('preview.restartScenario')} disabled={!props.ready} onclick={props.onReset}>
		<RotateCcw size={16} aria-hidden="true" />
	</Button>
	<Slider.Root
		type="single"
		value={props.timeMs}
		min={0}
		max={Math.max(1, props.durationMs)}
		step={1}
		disabled={!props.ready}
		onValueChange={props.onSeek}
		class="relative flex min-h-6 min-w-8 flex-1 touch-none select-none items-center max-[480px]:order-3 max-[480px]:max-w-full max-[480px]:basis-full"
		onpointerdown={props.onTimelinePointerDown}
		onkeydown={props.onTimelineKeydown}
	>
		<span class="pointer-events-none absolute inset-x-0 top-1/2 h-1 -translate-y-1/2 rounded-full bg-border" aria-hidden="true"></span>
		<Slider.Range class="top-1/2 h-1 -translate-y-1/2 rounded-full bg-primary" />
		<Slider.Thumb
			index={0}
			class="top-1/2 size-3.5 rounded-full border-2 border-background bg-primary [transform:translateY(-50%)] shadow-[0_0_0_1px_var(--border)] outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background"
			aria-label={studioMessage('preview.scenarioTimeline')}
		/>
	</Slider.Root>
	<output class="inline-flex min-w-max items-baseline gap-1 whitespace-nowrap font-mono text-xs leading-none tabular-nums text-muted-foreground" aria-label={timeDescription} aria-live="off">
		<strong class="text-foreground">{formatTime(props.timeMs)}</strong><span aria-hidden="true">/</span><span>{formatTime(props.durationMs)}</span>
	</output>
</div>
