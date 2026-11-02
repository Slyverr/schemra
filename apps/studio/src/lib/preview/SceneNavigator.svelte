<script lang="ts">
	import { tick } from 'svelte';
	import { ArrowRight, CircleDot, ListTree, Search } from '@lucide/svelte';
	import { computeCommandScore, Popover } from 'bits-ui';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { sceneNavigatorItemClass } from '../studio/ui-classes.js';
	import { searchPreviewScene, type PreviewScene } from './model.js';
	import { cn } from '#lib/utils.js';

	interface Props {
		scene: PreviewScene;
		onFocusTarget: (bounds: PreviewScene['nodes'][number]['bounds']) => void;
		onFocusNode: (nodeId: string) => void;
		onFocusConnection: (edgeIndex: number) => void;
	}

	const props: Props = $props();
	const instanceId = $props.id();
	const searchId = `${instanceId}-search`;
	const resultsId = `${instanceId}-results`;
	let searchInput = $state<HTMLInputElement | null>(null);
	let open = $state(false);
	let searchQuery = $state('');
	let activeIndex = $state(0);
	const results = $derived(searchPreviewScene(props.scene, searchQuery, 30, computeCommandScore));
	const resultCount = $derived(results.visibleMatches);
	const activeOptionId = $derived(resultCount > 0 && activeIndex < resultCount ? `${instanceId}-result-${activeIndex}` : undefined);
	const groupResults = $derived(results.groups.map((match, index) => ({ ...match, activeIndex: index })));
	const nodeResults = $derived(results.nodes.map((match, index) => ({ ...match, activeIndex: results.groups.length + index })));
	const connectionResults = $derived(results.connections.map((match, index) => ({ ...match, activeIndex: results.groups.length + results.nodes.length + index })));

	$effect(() => {
		if (open) void tick().then(() => searchInput?.focus());
	});

	function changeOpen(nextOpen: boolean) {
		open = nextOpen;
		if (nextOpen) {
			searchQuery = '';
			activeIndex = 0;
		}
	}

	function close() {
		open = false;
	}

	function handleSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close();
			return;
		}
		if ((event.key === 'ArrowDown' || event.key === 'ArrowUp') && resultCount > 0) {
			event.preventDefault();
			const direction = event.key === 'ArrowDown' ? 1 : -1;
			activeIndex = (Math.max(0, Math.min(activeIndex, resultCount - 1)) + direction + resultCount) % resultCount;
			void tick().then(() => document.getElementById(activeOptionId ?? '')?.scrollIntoView({ block: 'nearest' }));
			return;
		}
		if (event.key === 'Enter' && resultCount > 0) {
			event.preventDefault();
			selectResult(Math.max(0, Math.min(activeIndex, resultCount - 1)));
		}
	}

	function selectResult(index: number) {
		if (index < results.groups.length) {
			focusTarget(results.groups[index]!.item.bounds);
			return;
		}
		index -= results.groups.length;
		if (index < results.nodes.length) {
			focusNode(results.nodes[index]!.item.id);
			return;
		}
		index -= results.nodes.length;
		const connection = results.connections[index];
		if (connection) focusConnection(connection.index);
	}

	function focusTarget(bounds: PreviewScene['nodes'][number]['bounds']) {
		close();
		props.onFocusTarget(bounds);
	}

	function focusNode(nodeId: string) {
		close();
		props.onFocusNode(nodeId);
	}

	function focusConnection(edgeIndex: number) {
		close();
		props.onFocusConnection(edgeIndex);
	}
</script>

<Popover.Root open={open} onOpenChange={changeOpen}>
	<Popover.Trigger>
		{#snippet child({ props: triggerProps })}
			<Button
				{...triggerProps}
				variant="ghost"
				size="icon-sm"
				class="size-8"
				aria-label={studioMessage('preview.findDiagram')}
				aria-controls={`${instanceId}-panel`}
				title={studioMessage('preview.findInDiagram')}
			>
				<Search size={16} aria-hidden="true" />
			</Button>
		{/snippet}
	</Popover.Trigger>

	<Popover.Portal>
		<Popover.Content
			id={`${instanceId}-panel`}
			side="bottom"
			align="start"
			sideOffset={8}
			collisionPadding={8}
			class="!z-[1000] flex max-h-[min(24rem,60vh)] w-[min(24rem,calc(100vw-2rem))] flex-col overflow-hidden rounded-[.6rem] border border-border bg-background text-foreground shadow-[0_12px_32px_rgb(15_23_42_/_16%)]"
			role="dialog"
			aria-label={studioMessage('preview.findInDiagram')}
		>
			<label class="sr-only" for={searchId}>{studioMessage('preview.searchScene')}</label>
			<Input
				id={searchId}
				bind:ref={searchInput}
				bind:value={searchQuery}
				type="search"
				role="combobox"
				aria-autocomplete="list"
				aria-expanded="true"
				aria-controls={resultsId}
				aria-activedescendant={activeOptionId}
				placeholder={studioMessage('preview.searchScene')}
				autocomplete="off"
				class="h-10 min-w-0 rounded-none border-0 border-b border-border bg-transparent px-3 text-[.8125rem] shadow-none focus-visible:border-ring focus-visible:ring-0"
				oninput={() => (activeIndex = 0)}
				onkeydown={handleSearchKeydown}
			/>
			<div id={resultsId} class="min-h-0 overflow-auto p-1" role="listbox" aria-label={studioMessage('preview.findInDiagram')}>
				{#if results.totalMatches === 0}
					<p class="m-0 p-3 text-xs text-muted-foreground" role="status">{studioMessage('preview.noSearchMatches')}</p>
				{:else}
					{#if results.groups.length > 0}
						<section class="mb-1 last:mb-0" role="group" aria-label={studioMessage('preview.groups')}>
							<h2 class="m-0 px-2 py-1 text-[.6875rem] font-semibold text-muted-foreground">{studioMessage('preview.groups')}</h2>
							{#each groupResults as { item: group, activeIndex: optionIndex } (`${group.name}-${optionIndex}`)}
								<Button
									id={`${instanceId}-result-${optionIndex}`}
									variant="ghost"
									class={cn(sceneNavigatorItemClass, activeIndex === optionIndex && 'bg-accent text-accent-foreground')}
									type="button"
									role="option"
									aria-selected={activeIndex === optionIndex}
									tabindex={-1}
									onpointermove={() => (activeIndex = optionIndex)}
									onpointerdown={(event) => event.preventDefault()}
									onclick={() => selectResult(optionIndex)}
								>
									<ListTree size={16} aria-hidden="true" />
									<span class="min-w-0 flex-1 truncate">{group.title}</span>
									<span class="max-w-32 shrink overflow-hidden text-ellipsis whitespace-nowrap text-[.625rem] text-muted-foreground">{studioMessage('preview.group')}</span>
								</Button>
							{/each}
						</section>
					{/if}
					{#if results.nodes.length > 0}
						<section class="mb-1 last:mb-0" role="group" aria-label={studioMessage('preview.nodes')}>
							<h2 class="m-0 px-2 py-1 text-[.6875rem] font-semibold text-muted-foreground">{studioMessage('preview.nodes')}</h2>
							{#each nodeResults as { item: node, activeIndex: optionIndex } (node.id)}
								<Button
									id={`${instanceId}-result-${optionIndex}`}
									variant="ghost"
									class={cn(sceneNavigatorItemClass, activeIndex === optionIndex && 'bg-accent text-accent-foreground')}
									type="button"
									role="option"
									aria-selected={activeIndex === optionIndex}
									tabindex={-1}
									onpointermove={() => (activeIndex = optionIndex)}
									onpointerdown={(event) => event.preventDefault()}
									onclick={() => selectResult(optionIndex)}
								>
									<CircleDot size={16} aria-hidden="true" />
									<span class="min-w-0 flex-1 truncate" title={`${node.label ?? node.id} · ${node.id}`}>{node.label ?? node.id}</span>
									<span class="max-w-32 shrink overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[.625rem] text-muted-foreground">{node.id}</span>
								</Button>
							{/each}
						</section>
					{/if}
					{#if results.connections.length > 0}
						<section class="mb-1 last:mb-0" role="group" aria-label={studioMessage('preview.connections')}>
							<h2 class="m-0 px-2 py-1 text-[.6875rem] font-semibold text-muted-foreground">{studioMessage('preview.connections')}</h2>
							{#each connectionResults as { item: edge, index, activeIndex: optionIndex } (`${edge.from}-${edge.to}-${index}`)}
								{@const source = props.scene.nodes.find(({ id }) => id === edge.from)}
								{@const target = props.scene.nodes.find(({ id }) => id === edge.to)}
								{@const label = edge.label?.trim() || `${source?.label ?? edge.from} → ${target?.label ?? edge.to}`}
								<Button
									id={`${instanceId}-result-${optionIndex}`}
									variant="ghost"
									class={cn(sceneNavigatorItemClass, activeIndex === optionIndex && 'bg-accent text-accent-foreground')}
									type="button"
									role="option"
									aria-selected={activeIndex === optionIndex}
									tabindex={-1}
									onpointermove={() => (activeIndex = optionIndex)}
									onpointerdown={(event) => event.preventDefault()}
									onclick={() => selectResult(optionIndex)}
								>
									<ArrowRight size={16} aria-hidden="true" />
									<span class="min-w-0 flex-1 truncate" title={label}>{label}</span>
									<span class="max-w-32 shrink overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[.625rem] text-muted-foreground" title={`${edge.from} → ${edge.to}`}>{edge.from} → {edge.to}</span>
								</Button>
							{/each}
						</section>
					{/if}
				{/if}
			</div>
			{#if results.totalMatches > results.visibleMatches}
				<p class="m-0 border-t border-border p-3 text-xs text-muted-foreground" role="status">{studioMessage('preview.resultsLimit', { visible: results.visibleMatches, total: results.totalMatches })}</p>
			{/if}
		</Popover.Content>
	</Popover.Portal>
</Popover.Root>
