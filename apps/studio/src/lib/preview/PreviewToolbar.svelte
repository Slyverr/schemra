<script lang="ts">
	import { ChevronDown, Download, Expand, Focus, Minimize2, Play, ZoomIn, ZoomOut } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import { Separator } from '../components/ui/separator/index.js';
	import SceneNavigator from './SceneNavigator.svelte';
	import type { PreviewScene } from './model';
	import type { PreviewNodeBounds } from './viewport';

	interface Props {
		scene: PreviewScene | null;
		scenario: { id: string; title: string } | null;
		scenarios: readonly { id: string; title: string }[];
		zoomPercent: number;
		presentationMode: boolean;
		canExportAnimation: boolean;
		animationExportBusy: boolean;
		onScenarioSelect: (scenarioId: string | null) => void;
		onFocusTarget: (bounds: PreviewNodeBounds) => void;
		onFocusNode: (nodeId: string) => void;
		onFocusConnection: (edgeIndex: number) => void;
		onZoom: (value: number) => void;
		onFit: () => void;
		onTogglePresentation: () => void;
		onExportAnimation: () => void;
	}

	let props: Props = $props();
	const hasSceneContent = $derived(Boolean(props.scene && (props.scene.groups.length > 0 || props.scene.nodes.length > 0 || props.scene.edges.length > 0)));
</script>

<div class="preview-toolbar absolute inset-x-3 top-3 z-10 flex items-start justify-between gap-2">
	<div class="flex h-9 min-w-0 max-w-[min(32rem,calc(100%_-_3rem))] items-center gap-0.5 rounded-md border border-border bg-background p-0.5 shadow-xs" role="group" aria-label={studioMessage('preview.controls')}>
		{#if props.scenarios.length > 0}
			<label class="flex h-8 min-w-0 max-w-[min(16rem,55vw)] items-center gap-1.5 rounded-md px-2 text-primary">
				<Play size={14} aria-hidden="true" class="shrink-0" />
				<span class="sr-only">{studioMessage('preview.scenarioLabel')}</span>
				<select
					class="min-w-0 flex-1 appearance-none truncate bg-transparent text-xs text-foreground outline-none"
					value={props.scenario?.id ?? ''}
					aria-label={studioMessage('preview.scenarioLabel')}
					title={props.scenario?.title ?? studioMessage('preview.staticVisual')}
					onchange={(event) => props.onScenarioSelect((event.currentTarget as HTMLSelectElement).value || null)}
				>
					<option value="">{studioMessage('preview.staticVisual')}</option>
					{#each props.scenarios as scenario (scenario.id)}
						<option value={scenario.id}>{scenario.title}</option>
					{/each}
				</select>
				<ChevronDown size={14} aria-hidden="true" class="shrink-0 text-muted-foreground" />
			</label>
		{/if}
		{#if props.scenarios.length > 0 && hasSceneContent}
			<Separator orientation="vertical" decorative class="mx-0.5 h-5 !self-center" />
		{/if}
		{#if hasSceneContent}
			<SceneNavigator
				scene={props.scene!}
				onFocusTarget={props.onFocusTarget}
				onFocusNode={props.onFocusNode}
				onFocusConnection={props.onFocusConnection}
			/>
		{/if}
		{#if props.scenario && (props.scenarios.length > 0 || hasSceneContent)}
			<Separator orientation="vertical" decorative class="mx-0.5 h-5 !self-center" />
		{/if}
		{#if props.scenario}
			<Button
				variant="ghost"
				size="icon-sm"
				aria-label={studioMessage('preview.exportScenarioMp4')}
				title={props.animationExportBusy ? studioMessage('export.rendering') : studioMessage('preview.exportScenarioMp4')}
				disabled={!props.canExportAnimation}
				onclick={props.onExportAnimation}
			>
				<Download size={15} aria-hidden="true" />
			</Button>
		{/if}
	</div>

	<div class="flex h-9 shrink-0 items-center gap-0.5 rounded-md border border-border bg-background p-0.5 shadow-xs" role="group" aria-label={studioMessage('preview.zoomControls')}>
		<Button variant="ghost" size="icon-sm" aria-label={studioMessage('preview.zoomOut')} title={studioMessage('preview.zoomOut')} disabled={props.zoomPercent <= 50} onclick={() => props.onZoom(props.zoomPercent - 10)}>
			<ZoomOut size={15} aria-hidden="true" />
		</Button>
		<output class="min-w-12 px-1 text-center font-mono text-[11px] tabular-nums text-muted-foreground" aria-label={studioMessage('preview.zoomValue', { zoom: props.zoomPercent })}>{props.zoomPercent}%</output>
		<Button variant="ghost" size="icon-sm" aria-label={studioMessage('preview.zoomIn')} title={studioMessage('preview.zoomIn')} disabled={props.zoomPercent >= 2000} onclick={() => props.onZoom(props.zoomPercent + 10)}>
			<ZoomIn size={15} aria-hidden="true" />
		</Button>
		<Separator orientation="vertical" decorative class="mx-0.5 h-5 !self-center" />
		<Button variant="ghost" size="icon-sm" aria-label={studioMessage('preview.fit')} title={studioMessage('preview.fit')} onclick={props.onFit}>
			<Focus size={15} aria-hidden="true" />
		</Button>
		<Separator orientation="vertical" decorative class="mx-0.5 h-5 !self-center" />
		<Button variant="ghost" size="icon-sm" aria-label={studioMessage(props.presentationMode ? 'preview.exitPresentation' : 'preview.present')} title={studioMessage(props.presentationMode ? 'preview.exitPresentation' : 'preview.present')} onclick={props.onTogglePresentation}>
			{#if props.presentationMode}<Minimize2 size={15} aria-hidden="true" />{:else}<Expand size={15} aria-hidden="true" />{/if}
		</Button>
	</div>
</div>
