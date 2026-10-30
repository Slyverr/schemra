<script lang="ts">
	import { AlertCircle, ChevronLeft, ChevronRight, FileCode2, Play, Puzzle, Workflow } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import ResourceContextMenu from './ResourceContextMenu.svelte';
	import { nextWorkspaceTabIndex, type WorkspaceTabNavigationKey, type WorkspaceTabView } from './workspace-tabs.js';

	let {
		tabs,
		activeTabId,
		initialized,
		onSelect
	}: {
		tabs: WorkspaceTabView[];
		activeTabId: string | null;
		initialized: boolean;
		onSelect: (fileId: string) => void;
	} = $props();

	let scroller = $state<HTMLDivElement>();
	let tabScrollState = $state({ canScrollLeft: false, canScrollRight: false });
	let scrollFrame = 0;

	$effect(() => {
		const element = scroller;
		const track = element?.querySelector<HTMLElement>('[data-tabs-track]');
		if (!element || !track || typeof ResizeObserver === 'undefined') return;

		const update = () => {
			if (scrollFrame) window.cancelAnimationFrame(scrollFrame);
			scrollFrame = window.requestAnimationFrame(() => {
				scrollFrame = 0;
				const maxScroll = Math.max(0, element.scrollWidth - element.clientWidth);
				tabScrollState = {
					canScrollLeft: element.scrollLeft > 1,
					canScrollRight: element.scrollLeft < maxScroll - 1
				};
			});
		};

		const observer = new ResizeObserver(update);
		observer.observe(element);
		observer.observe(track);
		element.addEventListener('scroll', update, { passive: true });
		update();

		return () => {
			if (scrollFrame) window.cancelAnimationFrame(scrollFrame);
			observer.disconnect();
			element.removeEventListener('scroll', update);
		};
	});

	$effect(() => {
		const element = scroller;
		const selectedId = activeTabId;
		if (!element || !selectedId) return;
		const frame = window.requestAnimationFrame(() => {
			if (scroller !== element || activeTabId !== selectedId) return;
			const selected = element.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
			if (!selected) return;
			const viewport = element.getBoundingClientRect();
			const bounds = selected.getBoundingClientRect();
			if (bounds.left < viewport.left) element.scrollLeft -= viewport.left - bounds.left;
			else if (bounds.right > viewport.right) element.scrollLeft += bounds.right - viewport.right;
		});
		return () => window.cancelAnimationFrame(frame);
	});

	function scroll(direction: 'left' | 'right') {
		if (!scroller) return;
		scroller.scrollBy({ left: direction === 'left' ? -scroller.clientWidth : scroller.clientWidth, behavior: 'smooth' });
	}

	function handleTabKeydown(event: KeyboardEvent, currentIndex: number) {
		if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
		const nextIndex = nextWorkspaceTabIndex(currentIndex, event.key as WorkspaceTabNavigationKey, tabs.length);
		const nextTab = nextIndex === null ? undefined : tabs[nextIndex];
		if (nextIndex === null || !nextTab) return;
		event.preventDefault();
		onSelect(nextTab.id);
		const tabButtons = event.currentTarget instanceof HTMLElement
			? event.currentTarget.closest('[role="tablist"]')?.querySelectorAll<HTMLButtonElement>('[role="tab"]')
			: undefined;
		tabButtons?.[nextIndex]?.focus();
	}
</script>

<div class="flex h-full min-w-0 flex-1 items-center gap-[.15rem]">
	{#if tabScrollState.canScrollLeft}
		<Button variant="ghost" size="icon-sm" class="size-[1.8rem] shrink-0 rounded-[.4rem] border border-transparent p-0 text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1" type="button" aria-label={studioMessage('tabs.scrollLeft')} title={studioMessage('tabs.scrollLeft')} aria-controls="project-file-tabs" onclick={() => scroll('left')}>
			<ChevronLeft size={15} class="size-[15px]" aria-hidden="true" />
		</Button>
	{/if}
	<div id="project-file-tabs" class="h-full min-w-0 flex-1 overflow-x-hidden overflow-y-hidden" bind:this={scroller} role="tablist" aria-label={studioMessage('tabs.projectSourceFiles')}>
	<div data-tabs-track class="box-border flex h-full w-max min-w-full items-end gap-0 rounded-t-[.45rem] bg-muted/35 px-0 pt-1">
			{#each tabs as tab, index (tab.id)}
				{@const selected = tab.id === activeTabId}
				{@const kindLabel = tab.displayKind === 'visual' ? 'sidebar.resourceKindVisual' : tab.displayKind === 'scenario' ? 'sidebar.resourceKindScenario' : tab.displayKind === 'component' ? 'sidebar.resourceKindComponent' : 'sidebar.resourceKindSource'}
				{@const tabColorClass = tab.hasErrors ? 'text-destructive' : tab.unsaved ? 'text-[var(--warning)]' : selected ? 'text-primary' : 'text-muted-foreground'}
				<ResourceContextMenu onRename={tab.rename} onRemove={tab.remove} renameDisabled={tab.renameDisabled} removeDisabled={tab.removeDisabled}>
					<Button
						id={`workspace-tab-${tab.id}`}
						variant="ghost"
						class={`relative mb-[-1px] h-9 max-w-[19rem] min-w-max shrink-0 items-center gap-2 rounded-t-[.4rem] rounded-b-none border border-transparent px-[.78rem] text-[.77rem] font-normal shadow-none hover:border-border/60 hover:bg-background/70 hover:text-current disabled:cursor-wait disabled:opacity-100 max-[700px]:px-[.55rem] max-[700px]:text-[.7rem] max-[700px]:[&>svg]:size-[.85rem] ${selected ? 'z-[1] border-border border-b-background bg-background shadow-[0_-0.125rem_0.25rem_rgb(15_35_65_/_4%)] hover:bg-background' : ''} ${tabColorClass}`}
						type="button"
						tabindex={selected ? 0 : -1}
						disabled={!initialized}
						role="tab"
						aria-selected={selected}
						aria-controls="project-file-panel"
						aria-label={tab.hasErrors ? studioMessage('tabs.hasErrors', { name: tab.title }) : `${studioMessage(kindLabel)}: ${tab.title}${tab.unsaved ? `, ${studioMessage('tabs.unsavedState')}` : ''}`}
						aria-haspopup="menu"
						aria-keyshortcuts="Shift+F10"
						title={tab.hasErrors ? `${tab.path} — ${studioMessage('tabs.hasErrors', { name: tab.title })}` : tab.path}
						onclick={() => onSelect(tab.id)}
						onkeydown={(event) => handleTabKeydown(event, index)}
					>
						{#if tab.hasErrors}<AlertCircle size={15} class={`size-[15px] ${tabColorClass}`} aria-hidden="true" />
						{:else if tab.displayKind === 'visual'}<Workflow size={15} class={`size-[15px] ${tabColorClass}`} aria-hidden="true" />
						{:else if tab.displayKind === 'scenario'}<Play size={15} class={`size-[15px] ${tabColorClass}`} aria-hidden="true" />
						{:else if tab.displayKind === 'component'}<Puzzle size={15} class={`size-[15px] ${tabColorClass}`} aria-hidden="true" />
						{:else}<FileCode2 size={15} class={`size-[15px] ${tabColorClass}`} aria-hidden="true" />{/if}
						<span class={`min-w-0 max-w-40 overflow-hidden text-ellipsis whitespace-nowrap ${tabColorClass}`}>{tab.title}</span>
					</Button>
				</ResourceContextMenu>
			{/each}
		</div>
	</div>
	{#if tabScrollState.canScrollRight}
		<Button variant="ghost" size="icon-sm" class="size-[1.8rem] shrink-0 rounded-[.4rem] border border-transparent p-0 text-muted-foreground hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1" type="button" aria-label={studioMessage('tabs.scrollRight')} title={studioMessage('tabs.scrollRight')} aria-controls="project-file-tabs" onclick={() => scroll('right')}>
			<ChevronRight size={15} class="size-[15px]" aria-hidden="true" />
		</Button>
	{/if}
</div>
