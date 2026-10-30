<script lang="ts">
	import { onMount } from 'svelte';
	import { DropdownMenu } from 'bits-ui';
	import {
		AlertCircle,
		Check,
		ChevronDown,
		ChevronRight,
		FileCode2,
		GripVertical,
		PanelLeft,
		Play,
		Plus,
		Puzzle,
		Settings2,
		Workflow
	} from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import type { SidebarResource, SidebarVisual } from './project-sidebar.js';
	import ResourceContextMenu from './ResourceContextMenu.svelte';
	import StudioDropdownContent from './StudioDropdownContent.svelte';
	import type { ResourceTabKind, ResourceTabMenuPolicy } from './resource-tab-menu.js';

	type SidebarResourceMenuPolicy = Pick<ResourceTabMenuPolicy, 'showRename' | 'renameDisabled' | 'removeDisabled'>;

	interface Props {
		wordmark: string;
		mark: string;
		reduceMotion: boolean;
		primaryVisuals: SidebarVisual[];
		exampleVisuals: SidebarVisual[];
		sources: SidebarResource[];
		resourcesDisabled: boolean;
		creationDisabled: boolean;
		getResourceMenuPolicy: (fileId: string, kind: ResourceTabKind, resourceId: string | null) => SidebarResourceMenuPolicy;
		activeFileId: string | null;
		activeVisualId: string | null;
		selectedScenarioId: string | null;
		onSelectFile: (fileId: string) => void;
		onSelectScenario: (fileId: string, visualId: string, scenarioId: string) => void;
		onCreateVisual: () => void;
		onRenameResource: (fileId: string, kind: ResourceTabKind, resourceId: string | null) => void;
		onRemoveResource: (fileId: string, kind: ResourceTabKind, resourceId: string | null) => void;
		onOpenSettings: () => void;
	}

	let {
		wordmark,
		mark,
		reduceMotion,
		primaryVisuals,
		exampleVisuals,
		sources,
		resourcesDisabled,
		creationDisabled,
		getResourceMenuPolicy,
		activeFileId,
		activeVisualId,
		selectedScenarioId,
		onSelectFile,
		onSelectScenario,
		onCreateVisual,
		onRenameResource,
		onRemoveResource,
		onOpenSettings
	}: Props = $props();

	const DEFAULT_WIDTH = 240;
	const MIN_WIDTH = 184;
	const MAX_WIDTH = 360;
	const COLLAPSED_WIDTH = 56;
	const COLLAPSE_THRESHOLD = 168;
	const STORAGE_KEY = 'schemra.studio.sidebar-width.v1';

	let width = $state(DEFAULT_WIDTH);
	let collapsed = $state(false);
	let examplesExpanded = $state(false);
	let expandedVisualIds = $state<string[]>([]);
	let resizeStart = $state<{ x: number; width: number; pointerId: number } | null>(null);

	onMount(() => {
		try {
			const savedWidth = Number(window.localStorage.getItem(STORAGE_KEY));
			if (Number.isFinite(savedWidth) && savedWidth >= COLLAPSED_WIDTH && savedWidth <= MAX_WIDTH) {
				if (savedWidth === COLLAPSED_WIDTH) collapsed = true;
				else width = Math.max(MIN_WIDTH, savedWidth);
			}
		} catch {
			// Keep the sidebar usable when browser storage is unavailable.
		}
	});

	function persistWidth() {
		try {
			window.localStorage.setItem(STORAGE_KEY, String(collapsed ? COLLAPSED_WIDTH : width));
		} catch {
			// Keep resizing available for this session when storage is unavailable.
		}
	}

	function toggleExpanded(visualId: string) {
		expandedVisualIds = expandedVisualIds.includes(visualId)
			? expandedVisualIds.filter((id) => id !== visualId)
			: [...expandedVisualIds, visualId];
	}

	function toggleCollapsed() {
		collapsed = !collapsed;
		persistWidth();
	}

	function startResize(event: PointerEvent) {
		if (event.button !== 0) return;
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
		resizeStart = { x: event.clientX, width: collapsed ? COLLAPSED_WIDTH : width, pointerId: event.pointerId };
	}

	function moveResize(event: PointerEvent) {
		if (!resizeStart || event.pointerId !== resizeStart.pointerId) return;
		const nextWidth = resizeStart.width + event.clientX - resizeStart.x;
		if (nextWidth <= COLLAPSE_THRESHOLD) {
			collapsed = true;
			return;
		}
		collapsed = false;
		width = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, nextWidth));
	}

	function endResize(event: PointerEvent) {
		if (!resizeStart || event.pointerId !== resizeStart.pointerId) return;
		resizeStart = null;
		persistWidth();
	}

	function handleResizeKeydown(event: KeyboardEvent) {
		const step = event.shiftKey ? 32 : 12;
		if (event.key === 'Home') collapsed = true;
		else if (event.key === 'End') {
			collapsed = false;
			width = MAX_WIDTH;
		} else if (event.key === 'ArrowLeft' && !collapsed) width = Math.max(MIN_WIDTH, width - step);
		else if (event.key === 'ArrowRight') {
			if (collapsed) collapsed = false;
			else width = Math.min(MAX_WIDTH, width + step);
		} else return;
		event.preventDefault();
		persistWidth();
	}


</script>

	{#snippet visualRows(items: SidebarVisual[])}
		<ul class="mt-[.16rem] grid list-none gap-[.12rem] p-0">
			{#each items as visual (visual.id)}
				{@const menuPolicy = getResourceMenuPolicy(visual.fileId, 'visual', visual.id)}
				<li>
					<ResourceContextMenu wrapperClass="!flex !w-full !min-w-0" onRename={menuPolicy.showRename ? () => onRenameResource(visual.fileId, 'visual', visual.id) : undefined} renameDisabled={menuPolicy.renameDisabled} onRemove={() => onRemoveResource(visual.fileId, 'visual', visual.id)} removeDisabled={menuPolicy.removeDisabled}>
					<Button
						variant="ghost"
						class={[
							'flex min-h-[1.85rem] w-full min-w-0 justify-start gap-2 rounded-[.38rem] border border-transparent px-2 text-left text-[.78rem] font-normal text-muted-foreground hover:bg-accent/55 hover:text-foreground',
							activeVisualId === visual.id ? 'bg-accent font-semibold text-foreground' : ''
						].filter(Boolean).join(' ')}
							aria-current={activeVisualId === visual.id ? 'page' : undefined}
							aria-expanded={visual.scenarios.length > 0 ? expandedVisualIds.includes(visual.id) : undefined}
							aria-controls={visual.scenarios.length > 0 && expandedVisualIds.includes(visual.id) ? `sidebar-scenarios-${visual.id}` : undefined}
							aria-label={visual.dirty ? studioMessage('sidebar.unsavedChanges', { name: visual.title }) : visual.title}
							title={visual.pathHint ? `${visual.title} — ${visual.pathHint}` : visual.title}
							disabled={resourcesDisabled}
							onclick={() => {
								onSelectFile(visual.fileId);
								if (visual.scenarios.length > 0) toggleExpanded(visual.id);
							}}
						>
							{#if visual.scenarios.length > 0}{#if expandedVisualIds.includes(visual.id)}<ChevronDown aria-hidden="true" />{:else}<ChevronRight aria-hidden="true" />{/if}{/if}
						<Workflow class="size-4 shrink-0" aria-hidden="true" />
						<span class="min-w-0 flex-1 truncate">{visual.title}</span>
						{#if visual.pathHint}<span class="max-w-20 flex-[0_1_auto] truncate text-[.62rem] text-muted-foreground" aria-hidden="true">{visual.pathHint}</span>{/if}
						{#if visual.scenarios.length > 0}<span class="shrink-0 text-[.62rem] tabular-nums text-muted-foreground" title={studioMessage('sidebar.scenarioCount', { count: visual.scenarios.length })}>{visual.scenarios.length}</span>{/if}
						{#if visual.dirty}<span class="size-[.38rem] shrink-0 rounded-full bg-primary" aria-label={studioMessage('sidebar.unsavedChanges', { name: visual.title })}></span>{/if}
					</Button>
					</ResourceContextMenu>
					{#if visual.scenarios.length > 0 && expandedVisualIds.includes(visual.id)}
						<ul id={`sidebar-scenarios-${visual.id}`} class="ml-3 mt-[.15rem] mb-1 grid list-none gap-[.12rem] border-l border-border pl-[.35rem]" aria-label={studioMessage('sidebar.scenariosFor', { visual: visual.title })}>
							{#each visual.scenarios as scenario (`${scenario.fileId}:${scenario.id}`)}
								{@const scenarioPolicy = getResourceMenuPolicy(scenario.fileId, 'scenario', scenario.id)}
								<li>
									<ResourceContextMenu wrapperClass="!flex !w-full !min-w-0" onRename={scenarioPolicy.showRename ? () => onRenameResource(scenario.fileId, 'scenario', scenario.id) : undefined} renameDisabled={scenarioPolicy.renameDisabled} onRemove={() => onRemoveResource(scenario.fileId, 'scenario', scenario.id)} removeDisabled={scenarioPolicy.removeDisabled}>
									<Button
									variant="ghost"
									class={[
										'flex min-h-7 w-full min-w-0 justify-start gap-2 rounded-[.38rem] border border-transparent px-[.4rem] text-left text-[.73rem] font-normal text-muted-foreground hover:bg-accent/55 hover:text-foreground',
										selectedScenarioId === scenario.id && activeVisualId === visual.id ? 'bg-accent font-semibold text-foreground' : ''
									].filter(Boolean).join(' ')}
										aria-current={selectedScenarioId === scenario.id && activeVisualId === visual.id ? 'page' : undefined}
										aria-label={scenario.hasErrors ? studioMessage('sidebar.hasErrors', { name: scenario.title }) : scenario.title}
										title={scenario.title}
										disabled={resourcesDisabled}
										onclick={() => onSelectScenario(scenario.fileId, visual.id, scenario.id)}
									>
										{#if scenario.hasErrors}<AlertCircle class="size-[.82rem] shrink-0 text-destructive" aria-hidden="true" />{:else}<Play class="size-[.82rem] shrink-0" aria-hidden="true" />{/if}
										<span class="min-w-0 flex-1 truncate">{scenario.title}</span>
										{#if scenario.dirty}<span class="size-[.38rem] shrink-0 rounded-full bg-primary" aria-label={studioMessage('sidebar.unsavedChanges', { name: scenario.title })}></span>{/if}
									</Button>
									</ResourceContextMenu>
								</li>
							{/each}
						</ul>
					{/if}
				</li>
			{:else}
				<li class="px-[.55rem] py-[.4rem] text-[.72rem] text-muted-foreground">{studioMessage('sidebar.noVisuals')}</li>
			{/each}
		</ul>
	{/snippet}

<aside
	id="project-resource-sidebar"
	aria-label={studioMessage('sidebar.resources')}
	class={[
		'relative flex h-full min-h-0 shrink-0 flex-col overflow-visible border-r border-sidebar-border bg-sidebar text-sidebar-foreground transition-[width] duration-[240ms] ease-[cubic-bezier(0.22,1,0.36,1)] max-[767px]:hidden',
		resizeStart !== null || reduceMotion ? 'transition-none' : ''
	].filter(Boolean).join(' ')}
	style:width="{collapsed ? COLLAPSED_WIDTH : width}px"
>
	<header class={[
		'flex shrink-0 items-center justify-center',
		collapsed ? 'min-h-[3.5rem] px-[.375rem] py-2' : 'min-h-[3.25rem] px-3 py-2.5'
	].join(' ')}>
		{#if collapsed}
			<Button variant="ghost" size="icon-sm" class="group relative size-10 shrink-0 rounded-md text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring" type="button" aria-label={studioMessage('sidebar.expand')} title={studioMessage('sidebar.expand')} onclick={toggleCollapsed}>
				<img class="size-8 object-contain transition-opacity group-hover:opacity-0 group-focus-visible:opacity-0" src={mark} alt="" aria-hidden="true" />
				<PanelLeft class="pointer-events-none absolute size-4 opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100" aria-hidden="true" />
			</Button>
		{:else}
			<img class="block h-8 w-[min(100%,9rem)] object-contain" src={wordmark} alt="Schemra" />
		{/if}
	</header>

	{#if collapsed}
		<nav class="flex shrink-0 justify-center py-3" aria-label={studioMessage('sidebar.resources')}>
			<DropdownMenu.Root>
				<DropdownMenu.Trigger>
					{#snippet child({ props })}
						<Button {...props} variant="ghost" size="icon" class="size-9 shrink-0 rounded-[.4rem] text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground" type="button" aria-label={studioMessage('sidebar.visuals')} title={studioMessage('sidebar.visuals')}>
							<Workflow class="size-4" aria-hidden="true" />
						</Button>
					{/snippet}
				</DropdownMenu.Trigger>
				<DropdownMenu.Portal>
				<StudioDropdownContent side="right" align="start" sideOffset={6} class="grid max-h-[min(70vh,30rem)] w-[min(14rem,calc(100vw-4rem))] gap-[.15rem] overflow-auto rounded-[.55rem] border border-border bg-popover p-[.45rem] text-popover-foreground shadow-[0_.75rem_2rem_rgb(7_21_41_/_18%)] [&>[role=group]]:grid [&>[role=group]]:gap-[.1rem]">
					<DropdownMenu.Group>
						<DropdownMenu.GroupHeading class="px-[.45rem] py-[.3rem] text-[.63rem] font-[650] tracking-[.06em] text-muted-foreground uppercase">{studioMessage('sidebar.visuals')}</DropdownMenu.GroupHeading>
						{#each [...primaryVisuals, ...exampleVisuals] as visual (visual.id)}
							<DropdownMenu.Item disabled={resourcesDisabled} onSelect={() => onSelectFile(visual.fileId)} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg:first-child]:size-[.95rem] [&>svg:first-child]:shrink-0 [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate">
								<Workflow class="size-[.95rem] shrink-0" aria-hidden="true" />
								<span>{visual.title}</span>
								{#if activeVisualId === visual.id}<Check aria-hidden="true" class="size-[.85rem] shrink-0" />{/if}
							</DropdownMenu.Item>
						{:else}
							<div class="px-[.55rem] py-[.4rem] text-[.72rem] text-muted-foreground">{studioMessage('sidebar.noVisuals')}</div>
						{/each}
					</DropdownMenu.Group>
					<DropdownMenu.Separator class="my-1 h-px bg-border" />
					<DropdownMenu.Item disabled={resourcesDisabled || creationDisabled} onSelect={onCreateVisual} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg:first-child]:size-[.95rem] [&>svg:first-child]:shrink-0 [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate">
						<Plus class="size-[.95rem] shrink-0" aria-hidden="true" /><span>{studioMessage('sidebar.newVisual')}</span>
					</DropdownMenu.Item>
				</StudioDropdownContent>
				</DropdownMenu.Portal>
			</DropdownMenu.Root>
		</nav>
	{/if}

	<div class="min-h-0 flex-1 overflow-y-auto overscroll-contain px-3 py-1" class:hidden={collapsed} aria-hidden={collapsed} inert={collapsed}>
		<section aria-labelledby="sidebar-visuals-heading">
			<div class="flex min-h-[1.6rem] items-center gap-[.4rem] rounded-md bg-muted/40 px-1">
				<h2 id="sidebar-visuals-heading" class="min-w-0 flex-1 truncate text-[.62rem] font-[650] tracking-[.09em] text-muted-foreground uppercase">{studioMessage('sidebar.visuals')}</h2>
				<span class="text-[.62rem] tabular-nums text-muted-foreground" aria-label={studioMessage('sidebar.visualsCount', { count: primaryVisuals.length })}>{primaryVisuals.length}</span>
				<Button variant="ghost" size="icon-sm" class="size-[1.65rem] rounded-[.35rem] px-0 text-muted-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground" type="button" aria-label={studioMessage('sidebar.newVisual')} title={studioMessage('sidebar.newVisual')} disabled={resourcesDisabled || creationDisabled} onclick={onCreateVisual}>
					<Plus class="size-[.9rem]" strokeWidth={1.5} aria-hidden="true" />
				</Button>
			</div>
			{@render visualRows(primaryVisuals)}
		</section>

		{#if exampleVisuals.length > 0}
			<section class="mt-3 border-t border-border pt-[.65rem]" aria-label={studioMessage('sidebar.examples')}>
				<Button variant="ghost" class="h-7 w-full justify-start gap-[.4rem] rounded-[.35rem] px-2 text-left text-[.76rem] font-normal text-muted-foreground hover:bg-accent/55 hover:text-foreground" type="button" aria-expanded={examplesExpanded} aria-controls="sidebar-example-visuals" onclick={() => (examplesExpanded = !examplesExpanded)}>
					{#if examplesExpanded}<ChevronDown class="size-[.85rem]" aria-hidden="true" />{:else}<ChevronRight class="size-[.85rem]" aria-hidden="true" />{/if}
					<span class="min-w-0 flex-1">{studioMessage('sidebar.examples')}</span>
					<span class="shrink-0 text-[.62rem] tabular-nums text-muted-foreground">{exampleVisuals.length}</span>
				</Button>
				<div id="sidebar-example-visuals" class="grid gap-1 py-1 pl-[.55rem]" hidden={!examplesExpanded}>
					<h3 class="min-w-0 flex-1 truncate px-2 text-[.62rem] font-[650] tracking-[.09em] text-muted-foreground uppercase">{studioMessage('sidebar.focusedVisuals')}</h3>
					{@render visualRows(exampleVisuals)}
				</div>
			</section>
		{/if}

		{#if sources.length > 0}
			<section class="mt-3 border-t border-border pt-[.65rem]" aria-labelledby="sidebar-sources-heading">
			<div class="flex min-h-[1.6rem] items-center gap-[.4rem] px-2">
					<h2 id="sidebar-sources-heading" class="min-w-0 flex-1 truncate text-[.62rem] font-[650] tracking-[.09em] text-muted-foreground uppercase">{studioMessage('sidebar.unlinkedSources')}</h2>
					<span class="text-[.62rem] tabular-nums text-muted-foreground" aria-label={studioMessage('sidebar.sourcesCount', { count: sources.length })}>{sources.length}</span>
				</div>
				<ul class="mt-[.16rem] grid list-none gap-[.12rem] p-0">
					{#each sources as source (source.fileId)}
						{@const sourceKind: ResourceTabKind = source.kind === 'scenario' ? 'scenario' : 'source'}
						{@const sourcePolicy = getResourceMenuPolicy(source.fileId, sourceKind, source.resourceId)}
						<li>
							<ResourceContextMenu wrapperClass="!flex !w-full !min-w-0" onRename={sourcePolicy.showRename ? () => onRenameResource(source.fileId, sourceKind, source.resourceId) : undefined} renameDisabled={sourcePolicy.renameDisabled} onRemove={() => onRemoveResource(source.fileId, sourceKind, source.resourceId)} removeDisabled={sourcePolicy.removeDisabled}>
							<Button variant="ghost" class={[
								'flex min-h-[1.85rem] w-full min-w-0 justify-start gap-2 rounded-[.38rem] border border-transparent px-2 text-left text-[.78rem] font-normal text-muted-foreground hover:bg-accent/55 hover:text-foreground disabled:cursor-wait disabled:opacity-100',
								activeFileId === source.fileId ? 'bg-accent font-semibold text-foreground' : ''
							].filter(Boolean).join(' ')} aria-current={activeFileId === source.fileId ? 'page' : undefined} aria-label={source.hasErrors ? studioMessage('sidebar.hasErrors', { name: source.title }) : source.title} title={source.path} disabled={resourcesDisabled} onclick={() => onSelectFile(source.fileId)}>
								{#if source.kind === 'scenario'}<AlertCircle class="size-4 shrink-0 text-destructive" aria-hidden="true" />{:else if source.kind === 'component'}<Puzzle class="size-4 shrink-0" aria-hidden="true" />{:else}<FileCode2 class="size-4 shrink-0" aria-hidden="true" />{/if}
								<span class="min-w-0 flex-1 truncate">{source.title}</span>
								{#if source.dirty}<span class="size-[.38rem] shrink-0 rounded-full bg-primary" aria-label={studioMessage('sidebar.unsavedChanges', { name: source.title })}></span>{/if}
							</Button>
							</ResourceContextMenu>
						</li>
					{/each}
				</ul>
			</section>
		{/if}
	</div>

	<footer class="shrink-0 p-2">
		<Button variant="ghost" class={[
			'w-full min-h-9 justify-start gap-[.55rem] rounded-[.42rem] px-[.55rem] text-left text-[.78rem] text-sidebar-foreground hover:bg-sidebar-accent hover:text-sidebar-accent-foreground [&>svg]:size-4 [&>svg]:shrink-0',
			collapsed ? 'size-9 justify-center px-0' : ''
		].filter(Boolean).join(' ')} type="button" aria-label={studioMessage('sidebar.settings')} title={studioMessage('sidebar.settings')} onclick={onOpenSettings}>
			<Settings2 aria-hidden="true" />{#if !collapsed}<span>{studioMessage('sidebar.settings')}</span>{/if}
		</Button>
	</footer>

	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div
		class="group absolute top-0 right-0 z-[2] grid h-full w-2 place-items-center touch-none cursor-col-resize after:absolute after:inset-y-0 after:right-0 after:w-px after:bg-sidebar-border after:content-[''] hover:after:bg-sidebar-ring/70 focus-visible:after:bg-sidebar-ring"
		role="separator"
		tabindex="0"
		aria-label={studioMessage('sidebar.resize')}
		aria-orientation="vertical"
		aria-valuemin={COLLAPSED_WIDTH}
		aria-valuemax={MAX_WIDTH}
		aria-valuenow={collapsed ? COLLAPSED_WIDTH : width}
		aria-valuetext={collapsed ? studioMessage('sidebar.collapsedWidth') : studioMessage('sidebar.widthPixels', { width })}
		title={studioMessage('sidebar.resizeHint')}
		onpointerdown={startResize}
		onpointermove={moveResize}
		onpointerup={endResize}
		onpointercancel={endResize}
		onkeydown={handleResizeKeydown}
		ondblclick={toggleCollapsed}
	>
		<GripVertical class="pointer-events-none z-[1] size-[.875rem] rounded-[.15rem] bg-background text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100 group-focus-visible:opacity-100 group-active:opacity-100 motion-reduce:transition-none" aria-hidden="true" />
	</div>
</aside>
