<script lang="ts">
	import { Download, Expand, Eye, Focus, Minimize2, Play, ZoomIn, ZoomOut } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { Separator } from '../components/ui/separator/index.js';
	import * as Select from '../components/ui/select/index.js';
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
	let scenarioPickerOpen = $state(false);
	let scenarioSearch = $state('');
	let scenarioSearchInput = $state<HTMLInputElement | null>(null);
	const STATIC_VISUAL_VALUE = '__static_visual__';
	const scenarioOptions = $derived([
		{ value: STATIC_VISUAL_VALUE, label: studioMessage('preview.staticVisual') },
		...props.scenarios.map(({ id, title }) => ({ value: id, label: title }))
	]);
	const matchingScenarios = $derived.by(() => {
		const query = scenarioSearch.trim().toLocaleLowerCase();
		return query ? scenarioOptions.filter(({ label }) => label.toLocaleLowerCase().includes(query)) : scenarioOptions;
	});
	const selectedScenarioValue = $derived(props.scenario?.id ?? STATIC_VISUAL_VALUE);
	const selectedScenarioLabel = $derived(scenarioOptions.find(({ value }) => value === selectedScenarioValue)?.label ?? studioMessage('preview.staticVisual'));
	const hasSceneContent = $derived(Boolean(props.scene && (props.scene.groups.length > 0 || props.scene.nodes.length > 0 || props.scene.edges.length > 0)));

	function selectScenario(value: string) {
		props.onScenarioSelect(value === STATIC_VISUAL_VALUE ? null : value);
		scenarioSearch = '';
	}

	function changeScenarioPickerOpen(open: boolean) {
		scenarioPickerOpen = open;
		scenarioSearch = '';
		if (open) requestAnimationFrame(() => scenarioSearchInput?.focus({ preventScroll: true }));
	}
</script>

<div class="preview-toolbar absolute inset-x-3 top-3 z-10 grid min-w-0 grid-cols-1 gap-1.5 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-start">
	<div class="flex h-9 w-fit min-w-0 max-w-full flex-wrap items-center gap-0.5 rounded-md border border-border bg-background px-0.5 py-0 shadow-xs sm:max-w-[min(32rem,calc(100%_-_3rem))]" role="group" aria-label={studioMessage('preview.controls')}>
		{#if props.scenarios.length > 0}
			<Select.Root
				type="single"
				value={selectedScenarioValue}
				open={scenarioPickerOpen}
				onOpenChange={changeScenarioPickerOpen}
				onValueChange={selectScenario}
			>
				<Select.Trigger size="sm" class="w-fit max-w-[min(16rem,55vw)] min-w-0 justify-start gap-1.5 border-0 px-3 text-xs shadow-none focus-visible:ring-2 focus-visible:ring-ring" aria-label={studioMessage('preview.scenarioLabel')}>
					{#if props.scenario}
						<Play size={14} aria-hidden="true" class="shrink-0 text-primary" />
					{:else}
						<Eye size={14} aria-hidden="true" class="shrink-0 text-muted-foreground" />
					{/if}
					<span class="min-w-0 max-w-[min(12rem,40vw)] truncate text-left" title={selectedScenarioLabel}>{selectedScenarioLabel}</span>
				</Select.Trigger>
				<Select.Content side="bottom" align="start" class="!z-[1000] min-w-48 p-1">
					<div class="p-1">
						<Input
							bind:ref={scenarioSearchInput}
							bind:value={scenarioSearch}
							class="h-8 text-sm"
							placeholder={studioMessage('preview.searchScenarios')}
							aria-label={studioMessage('preview.searchScenarios')}
							onkeydown={(event) => {
								if (event.key.length === 1 || event.key === 'Backspace' || event.key === 'Delete') {
									event.stopPropagation();
								}
							}}
						/>
					</div>
					{#each matchingScenarios as scenario (scenario.value)}
						<Select.Item value={scenario.value} label={scenario.label}>{scenario.label}</Select.Item>
					{/each}
					{#if matchingScenarios.length === 0}
						<p class="px-2 py-1.5 text-sm text-muted-foreground">{studioMessage('preview.noMatchingScenarios')}</p>
					{/if}
				</Select.Content>
			</Select.Root>
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
				aria-label={studioMessage('preview.exportScenarioAnimation')}
				title={props.animationExportBusy ? studioMessage('export.renderingVideo') : studioMessage('preview.exportScenarioAnimation')}
				disabled={!props.canExportAnimation}
				onclick={props.onExportAnimation}
			>
				<Download size={15} aria-hidden="true" />
			</Button>
		{/if}
	</div>

	<div class="flex h-9 w-fit shrink-0 items-center gap-0.5 justify-self-end rounded-md border border-border bg-background px-0.5 py-0 shadow-xs" role="group" aria-label={studioMessage('preview.zoomControls')}>
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
