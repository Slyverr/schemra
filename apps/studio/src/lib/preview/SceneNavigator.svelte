<script lang="ts">
	import { tick } from 'svelte';
	import { ArrowRight, CircleDot, ListTree, Search } from '@lucide/svelte';
	import { computeCommandScore, Popover } from 'bits-ui';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { sceneNavigatorItemClass } from '../studio/ui-classes.js';
	import { searchPreviewScene, type PreviewScene } from './model.js';

	interface Props {
		scene: PreviewScene;
		onFocusTarget: (bounds: PreviewScene['nodes'][number]['bounds']) => void;
		onFocusNode: (nodeId: string) => void;
		onFocusConnection: (edgeIndex: number) => void;
	}

	const props: Props = $props();
	const instanceId = $props.id();
	const searchId = `${instanceId}-search`;
	let searchInput = $state<HTMLInputElement | null>(null);
	let open = $state(false);
	let searchQuery = $state('');
	const results = $derived(searchPreviewScene(props.scene, searchQuery, 30, computeCommandScore));

	$effect(() => {
		if (open) void tick().then(() => searchInput?.focus());
	});

	function close() {
		open = false;
	}

	function handleSearchKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close();
		}
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

<Popover.Root bind:open>
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
				placeholder={studioMessage('preview.searchScene')}
				autocomplete="off"
				class="h-10 min-w-0 rounded-none border-0 border-b border-border bg-transparent px-3 text-[.8125rem] shadow-none focus-visible:border-ring focus-visible:ring-0"
				onkeydown={handleSearchKeydown}
			/>
			<div class="min-h-0 overflow-auto p-1">
				{#if results.totalMatches === 0}
					<p class="m-0 p-3 text-xs text-muted-foreground" role="status">{studioMessage('preview.noSearchMatches')}</p>
				{:else}
					{#if results.groups.length > 0}
						<section class="mb-1 last:mb-0" aria-label={studioMessage('preview.groups')}>
							<h2 class="m-0 px-2 py-1 text-[.6875rem] font-semibold text-muted-foreground">{studioMessage('preview.groups')}</h2>
							{#each results.groups as { item: group, index } (`${group.name}-${index}`)}
								<Button variant="ghost" class={sceneNavigatorItemClass} type="button" onclick={() => focusTarget(group.bounds)}>
									<ListTree size={16} aria-hidden="true" />
									<span class="min-w-0 flex-1 truncate">{group.title}</span>
									<span class="max-w-32 shrink overflow-hidden text-ellipsis whitespace-nowrap text-[.625rem] text-muted-foreground">{studioMessage('preview.group')}</span>
								</Button>
							{/each}
						</section>
					{/if}
					{#if results.nodes.length > 0}
						<section class="mb-1 last:mb-0" aria-label={studioMessage('preview.nodes')}>
							<h2 class="m-0 px-2 py-1 text-[.6875rem] font-semibold text-muted-foreground">{studioMessage('preview.nodes')}</h2>
							{#each results.nodes as { item: node } (node.id)}
								<Button variant="ghost" class={sceneNavigatorItemClass} type="button" onclick={() => focusNode(node.id)}>
									<CircleDot size={16} aria-hidden="true" />
									<span class="min-w-0 flex-1 truncate" title={`${node.label ?? node.id} · ${node.id}`}>{node.label ?? node.id}</span>
									<span class="max-w-32 shrink overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[.625rem] text-muted-foreground">{node.id}</span>
								</Button>
							{/each}
						</section>
					{/if}
					{#if results.connections.length > 0}
						<section class="mb-1 last:mb-0" aria-label={studioMessage('preview.connections')}>
							<h2 class="m-0 px-2 py-1 text-[.6875rem] font-semibold text-muted-foreground">{studioMessage('preview.connections')}</h2>
							{#each results.connections as { item: edge, index } (`${edge.from}-${edge.to}-${index}`)}
								{@const source = props.scene.nodes.find(({ id }) => id === edge.from)}
								{@const target = props.scene.nodes.find(({ id }) => id === edge.to)}
								{@const label = edge.label?.trim() || `${source?.label ?? edge.from} → ${target?.label ?? edge.to}`}
								<Button variant="ghost" class={sceneNavigatorItemClass} type="button" onclick={() => focusConnection(index)}>
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
