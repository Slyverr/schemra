<script lang="ts">
	import type { ComponentProps } from 'svelte';
import { Tooltip } from 'bits-ui';
import { AlertCircle, FileCode2, GripHorizontal, GripVertical, LoaderCircle, Maximize2, Minimize2, Minus, Plus, Workflow } from '@lucide/svelte';
import { Button } from '../components/ui/button/index.js';
	import DiagramPreview from '../preview/DiagramPreview.svelte';
	import DiagnosticsMenu from './DiagnosticsMenu.svelte';
	import SourceEditor from '../editor/SourceEditor.svelte';
	import { studioMessage } from '../i18n/messages.js';
import { DEFAULT_EDITOR_PREFERENCES, EDITOR_ZOOM_LEVELS, type EditorZoom } from '../settings/editor.js';

	type EditorProps = ComponentProps<typeof SourceEditor>;
	type DiagnosticsProps = ComponentProps<typeof DiagnosticsMenu>;
	type PreviewProps = ComponentProps<typeof DiagramPreview>;
	type FocusedPane = 'editor' | 'preview' | null;

	interface Props {
		editor: EditorProps | null;
		diagnostics: DiagnosticsProps;
		preview: PreviewProps | null;
		editorFileId: string | null;
		focusedPane: FocusedPane;
		editorPaneRatio: number;
		paneLayoutWide: boolean;
		stats: { nodes: number; links: number; groups: number } | null;
		editorZoom: EditorZoom;
		compilePending: boolean;
		showLastValidPreview: boolean;
		editorPalette: string;
		actions: {
			togglePaneFocus: (pane: Exclude<FocusedPane, null>) => void;
			startPaneResize: (event: PointerEvent) => void;
			movePaneResize: (event: PointerEvent) => void;
			endPaneResize: (event: PointerEvent) => void;
			paneResizeKeydown: (event: KeyboardEvent) => void;
			resetPaneResize: () => void;
			adjustEditorZoom: (direction: -1 | 1) => void;
			resetEditorZoom: () => void;
		};
	}

	let {
		editor,
		diagnostics,
		preview,
		editorFileId,
		focusedPane,
		editorPaneRatio,
		paneLayoutWide,
		stats,
		editorZoom,
		compilePending,
		showLastValidPreview,
		editorPalette,
		actions
	}: Props = $props();
	let previewHandle = $state<{ exportDiagram: (format: 'svg' | 'png') => Promise<void> } | null>(null);
	let sourceEditorHandle = $state<{ formatDocument: () => Promise<'formatted' | 'unchanged' | 'stale'> } | null>(null);
	let stalePreviewInfoOpen = $state(false);

	export async function exportDiagram(format: 'svg' | 'png') {
		await previewHandle?.exportDiagram(format);
	}

	export async function formatSource() {
		return sourceEditorHandle?.formatDocument() ?? 'stale';
	}
</script>

<div
	id="project-file-panel"
		class="relative flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden lg:flex-row"
	role="tabpanel"
	aria-label={studioMessage('pane.workspace')}
	aria-labelledby={editorFileId ? `workspace-tab-${editorFileId}` : undefined}
>
	<div
		class="flex min-h-0 min-w-0 flex-col overflow-hidden border-b border-border max-[1023px]:min-h-52 lg:border-r lg:border-b-0"
		class:hidden={focusedPane === 'preview'}
		style={`flex: ${focusedPane === 'editor' ? 1 : focusedPane ? 0 : editorPaneRatio} 1 0%;`}
	>
		<header class="flex h-10 min-h-10 flex-[0_0_auto] items-center justify-between gap-2 border-b border-border bg-background px-3 sm:px-4">
			<div class="flex min-w-0 items-center gap-2">
				<h2 class="m-0 overflow-hidden text-ellipsis whitespace-nowrap text-[.8125rem] font-medium text-foreground">{studioMessage('pane.editor')}</h2>
				<DiagnosticsMenu {...diagnostics} />
			</div>
			<div class="flex min-w-0 items-center justify-end gap-3">
				<div class="flex h-8 flex-[0_0_auto] items-center gap-0.5 rounded-md border border-border/70 bg-background/70 p-0.5" role="group" aria-label={studioMessage('editor.zoomGroup')}>
					<Button
						variant="ghost"
						size="icon-xs"
						class="size-7 rounded-[.3rem] p-0 text-muted-foreground hover:bg-accent hover:text-foreground disabled:cursor-default disabled:opacity-40"
						type="button"
						aria-label={studioMessage('editor.zoomDecrease')}
						title={studioMessage('editor.zoomDecrease')}
						disabled={editorZoom === EDITOR_ZOOM_LEVELS[0]}
						onclick={() => actions.adjustEditorZoom(-1)}
					>
						<Minus size={14} class="size-3.5" aria-hidden="true" />
					</Button>
					<Button
						variant="ghost"
						size="xs"
						class="h-7 min-w-10 rounded-[.3rem] px-1 text-[.6875rem] font-normal tabular-nums text-muted-foreground hover:bg-accent hover:text-foreground"
						type="button"
						aria-label={studioMessage('editor.zoomReset', { zoom: DEFAULT_EDITOR_PREFERENCES.zoom })}
						title={studioMessage('editor.zoomReset', { zoom: DEFAULT_EDITOR_PREFERENCES.zoom })}
						onclick={actions.resetEditorZoom}
					>
						{editorZoom}%
					</Button>
					<Button
						variant="ghost"
						size="icon-xs"
						class="size-7 rounded-[.3rem] p-0 text-muted-foreground hover:bg-accent hover:text-foreground disabled:cursor-default disabled:opacity-40"
						type="button"
						aria-label={studioMessage('editor.zoomIncrease')}
						title={studioMessage('editor.zoomIncrease')}
						disabled={editorZoom === EDITOR_ZOOM_LEVELS[EDITOR_ZOOM_LEVELS.length - 1]}
						onclick={() => actions.adjustEditorZoom(1)}
					>
						<Plus size={14} class="size-3.5" aria-hidden="true" />
					</Button>
				</div>
				<Button
					variant="outline"
					size="icon-sm"
					class="size-8 shrink-0"
					type="button"
					aria-label={studioMessage(focusedPane === 'editor' ? 'pane.restoreSplit' : 'pane.maximizeEditor')}
					title={studioMessage(focusedPane === 'editor' ? 'pane.restoreSplit' : 'pane.maximizeEditor')}
					aria-pressed={focusedPane === 'editor'}
					onclick={() => actions.togglePaneFocus('editor')}
				>
					{#if focusedPane === 'editor'}<Minimize2 size={15} aria-hidden="true" />{:else}<Maximize2 size={15} aria-hidden="true" />{/if}
				</Button>
			</div>
		</header>
		<div class="flex min-h-0 flex-1 overflow-hidden bg-[var(--editor-background)] text-[var(--editor-foreground)] [color-scheme:var(--editor-color-scheme,light)]" style={editorPalette}>
			{#if editor}
				<SourceEditor bind:this={sourceEditorHandle} {...editor} />
			{:else}
				<div class="grid min-h-0 flex-1 content-center justify-items-center gap-3 text-[.84rem] text-muted-foreground"><FileCode2 size={22} aria-hidden="true" /><p class="m-0">{studioMessage('resource.emptyEditor')}</p></div>
			{/if}
		</div>
	</div>

	<!-- The separator resizes both panes by pointer or keyboard. -->
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class={[
			'pane-resizer group relative z-10 flex touch-none items-center justify-center bg-transparent text-muted-foreground outline-none hover:bg-transparent focus-visible:ring-2 focus-visible:ring-ring/50',
			focusedPane !== null ? 'hidden' : '',
			paneLayoutWide
				? 'h-auto w-3 min-w-0 flex-[0_0_.75rem] -mx-1.5 cursor-col-resize'
				: 'h-3 w-full min-h-0 flex-[0_0_.75rem] -my-1.5 cursor-row-resize'
		].filter(Boolean).join(' ')}
		role="separator"
		aria-orientation={paneLayoutWide ? 'vertical' : 'horizontal'}
		aria-label={studioMessage('pane.resize')}
		aria-valuemin={25}
		aria-valuemax={75}
		aria-valuenow={Math.round(editorPaneRatio)}
		aria-valuetext={studioMessage('pane.resizeValue', { editor: Math.round(editorPaneRatio), preview: 100 - Math.round(editorPaneRatio) })}
		tabindex={focusedPane ? -1 : 0}
		title={studioMessage('pane.resizeHelp', {
			decrease: studioMessage(paneLayoutWide ? 'pane.resizeDecreaseWide' : 'pane.resizeDecreaseStacked'),
			increase: studioMessage(paneLayoutWide ? 'pane.resizeIncreaseWide' : 'pane.resizeIncreaseStacked')
		})}
		onpointerdown={actions.startPaneResize}
		onpointermove={actions.movePaneResize}
		onpointerup={actions.endPaneResize}
		onpointercancel={actions.endPaneResize}
		onlostpointercapture={actions.endPaneResize}
		onkeydown={actions.paneResizeKeydown}
		ondblclick={actions.resetPaneResize}
	>
		<span
			class={paneLayoutWide
				? 'h-full w-px bg-border transition-colors group-hover:bg-primary/50 group-focus-visible:bg-primary'
				: 'h-px w-full bg-border transition-colors group-hover:bg-primary/50 group-focus-visible:bg-primary'}
			aria-hidden="true"
		></span>
		{#if paneLayoutWide}
			<GripVertical class="pointer-events-none absolute size-4 rounded-sm bg-background text-muted-foreground opacity-40 shadow-sm transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true" />
		{:else}
			<GripHorizontal class="pointer-events-none absolute size-4 rounded-sm bg-background text-muted-foreground opacity-40 shadow-sm transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true" />
		{/if}
	</div>

	<section
		class="flex min-h-0 min-w-0 flex-col overflow-hidden"
		class:hidden={focusedPane === 'editor'}
		style={`flex: ${focusedPane === 'preview' ? 1 : focusedPane ? 0 : 100 - editorPaneRatio} 1 0%;`}
		aria-label={studioMessage('pane.previewPanel')}
	>
		<header class="flex h-10 min-h-10 flex-[0_0_auto] items-center justify-between gap-2 border-b border-border bg-background px-3 sm:px-4">
			<div class="flex min-w-0 items-center gap-2">
				<h2 class="m-0 overflow-hidden text-ellipsis whitespace-nowrap text-[.8125rem] font-medium text-foreground">{studioMessage('pane.preview')}</h2>
				{#if showLastValidPreview}
					<Tooltip.Provider>
						<Tooltip.Root bind:open={stalePreviewInfoOpen}>
							<Tooltip.Trigger>
								{#snippet child({ props })}
									<Button {...props} variant="ghost" size="icon-xs" class="size-6 rounded-full bg-primary/15 p-0 text-primary hover:bg-primary/20 hover:text-primary" type="button" aria-label={studioMessage('resource.lastValidPreview')} aria-expanded={stalePreviewInfoOpen} title={studioMessage('resource.lastValidPreview')} onclick={() => (stalePreviewInfoOpen = !stalePreviewInfoOpen)}>
										<AlertCircle size={15} class="size-[15px]" aria-hidden="true" />
									</Button>
								{/snippet}
							</Tooltip.Trigger>
							<Tooltip.Portal>
								<Tooltip.Content side="bottom" align="start" sideOffset={8} collisionPadding={8} class="!z-[1000] max-w-[min(20rem,70vw)] rounded-md border border-primary/35 bg-primary/10 px-[.65rem] py-2 text-[.72rem] font-normal text-warning shadow-md" role="status">
									{studioMessage('resource.lastValidPreview')}
								</Tooltip.Content>
							</Tooltip.Portal>
						</Tooltip.Root>
					</Tooltip.Provider>
				{/if}
			</div>
			<div class="flex min-w-0 items-center justify-end gap-3 max-sm:[&_[data-slot=diagram-stats]]:hidden">
				{#if stats}<span data-slot="diagram-stats" class="text-[.65rem] whitespace-nowrap text-muted-foreground">{studioMessage('preview.stats', stats)}</span>{/if}
				<Button
					variant="outline"
					size="icon-sm"
					class="size-8 shrink-0"
					type="button"
					aria-label={studioMessage(focusedPane === 'preview' ? 'pane.restoreSplit' : 'pane.maximizePreview')}
					title={studioMessage(focusedPane === 'preview' ? 'pane.restoreSplit' : 'pane.maximizePreview')}
					aria-pressed={focusedPane === 'preview'}
					onclick={() => actions.togglePaneFocus('preview')}
				>
					{#if focusedPane === 'preview'}<Minimize2 size={15} aria-hidden="true" />{:else}<Maximize2 size={15} aria-hidden="true" />{/if}
				</Button>
			</div>
		</header>
		<div class="relative flex min-h-0 min-w-0 flex-1 overflow-hidden bg-muted">
			{#if preview}
				<DiagramPreview bind:this={previewHandle} {...preview} />
			{:else if compilePending}
				<div class="m-auto flex items-center gap-2 p-8 text-sm text-muted-foreground" role="status" aria-live="polite">
					<LoaderCircle size={16} class="animate-spin" aria-hidden="true" />
					<span>{studioMessage('preview.compiling')}</span>
				</div>
			{:else}
				<div class="m-auto flex max-w-[22rem] flex-col items-center gap-2 p-8 text-center text-muted-foreground">
					<div class="mb-1 grid size-11 place-items-center rounded-[.9rem] border border-border bg-background text-primary"><Workflow size={20} aria-hidden="true" /></div>
					<strong class="text-[.9rem] text-foreground">{studioMessage('preview.emptyTitle')}</strong>
					<p class="m-0 text-xs leading-[1.5]">{studioMessage('preview.emptyDescription')}</p>
				</div>
			{/if}
		</div>
	</section>
</div>
