<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { ArrowRight, CircleDot, ListTree, Search } from '@lucide/svelte';
import { computeCommandScore } from 'bits-ui';
import { searchPreviewScene, type PreviewScene } from './model';

	interface Props {
		scene: PreviewScene;
		onFocusTarget: (bounds: PreviewScene['nodes'][number]['bounds']) => void;
		onFocusNode: (nodeId: string) => void;
		onFocusConnection: (edgeIndex: number) => void;
	}

	const props: Props = $props();
	const instanceId = $props.id();
	const searchId = `${instanceId}-search`;
	let root = $state<HTMLDivElement>();
	let trigger = $state<HTMLButtonElement>();
	let searchInput = $state<HTMLInputElement>();
	let open = $state(false);
	let searchQuery = $state('');
	const results = $derived(searchPreviewScene(props.scene, searchQuery, 30, computeCommandScore));

	onMount(() => {
		const closeOutside = (event: PointerEvent) => {
			if (open && !root?.contains(event.target as Node)) open = false;
		};
		document.addEventListener('pointerdown', closeOutside);
		return () => document.removeEventListener('pointerdown', closeOutside);
	});

	$effect(() => {
		if (open) void tick().then(() => searchInput?.focus());
	});

	function toggle() {
		open = !open;
	}

	function close() {
		open = false;
		void tick().then(() => trigger?.focus());
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

<div class="navigator" bind:this={root}>
	<button
		bind:this={trigger}
		type="button"
		class="trigger"
		aria-label="Find a node, group, or connection in the diagram"
		aria-expanded={open}
		aria-controls={`${instanceId}-panel`}
		title="Find in diagram"
		onclick={toggle}
	>
		<Search size={16} aria-hidden="true" />
	</button>

	{#if open}
		<div id={`${instanceId}-panel`} class="panel" role="dialog" aria-label="Find in diagram">
			<label class="sr-only" for={searchId}>Search nodes, groups, and connections</label>
			<input
				id={searchId}
				bind:this={searchInput}
				bind:value={searchQuery}
				type="search"
				placeholder="Search nodes, groups, and connections…"
				autocomplete="off"
				onkeydown={handleSearchKeydown}
			/>
			<div class="results">
				{#if results.totalMatches === 0}
					<p class="empty" role="status">No matching diagram items.</p>
				{:else}
					{#if results.groups.length > 0}
						<section aria-label="Groups">
							<h2>Groups</h2>
							{#each results.groups as { item: group, index } (`${group.name}-${index}`)}
								<button class="result" type="button" onclick={() => focusTarget(group.bounds)}>
									<ListTree size={16} aria-hidden="true" />
									<span class="result-label">{group.title}</span>
									<span class="result-kind">Group</span>
								</button>
							{/each}
						</section>
					{/if}
					{#if results.nodes.length > 0}
						<section aria-label="Nodes">
							<h2>Nodes</h2>
							{#each results.nodes as { item: node } (node.id)}
								<button class="result" type="button" onclick={() => focusNode(node.id)}>
									<CircleDot size={16} aria-hidden="true" />
									<span class="result-label" title={`${node.label ?? node.id} · ${node.id}`}>{node.label ?? node.id}</span>
									<span class="result-detail">{node.id}</span>
								</button>
							{/each}
						</section>
					{/if}
					{#if results.connections.length > 0}
						<section aria-label="Connections">
							<h2>Connections</h2>
							{#each results.connections as { item: edge, index } (`${edge.from}-${edge.to}-${index}`)}
								{@const source = props.scene.nodes.find(({ id }) => id === edge.from)}
								{@const target = props.scene.nodes.find(({ id }) => id === edge.to)}
								{@const label = edge.label?.trim() || `${source?.label ?? edge.from} → ${target?.label ?? edge.to}`}
								<button class="result" type="button" onclick={() => focusConnection(index)}>
									<ArrowRight size={16} aria-hidden="true" />
									<span class="result-label" title={label}>{label}</span>
									<span class="result-detail" title={`${edge.from} → ${edge.to}`}>{edge.from} → {edge.to}</span>
								</button>
							{/each}
						</section>
					{/if}
				{/if}
			</div>
			{#if results.totalMatches > results.visibleMatches}
				<p class="result-limit" role="status">Showing {results.visibleMatches} of {results.totalMatches}. Narrow your search to see more.</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	.navigator { position: relative; flex: 0 0 auto; }
	.trigger, .result {
		display: flex;
		align-items: center;
		border: 1px solid var(--border);
		color: var(--foreground);
		background: var(--background);
		font: inherit;
		cursor: pointer;
	}
	.trigger { justify-content: center; width: 2rem; height: 2rem; border-radius: .375rem; }
	.trigger:hover, .result:hover { background: var(--muted); }
	.trigger:focus-visible, .result:focus-visible, input:focus-visible { outline: 2px solid var(--ring); outline-offset: 2px; }
	.panel {
		position: absolute;
		z-index: 20;
		top: calc(100% + .5rem);
		left: 0;
		display: flex;
		width: min(24rem, calc(100vw - 2rem));
		max-height: min(24rem, 60vh);
		flex-direction: column;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: .6rem;
		background: var(--background);
		box-shadow: 0 12px 32px rgb(15 23 42 / .16);
	}
	.panel input {
		min-width: 0;
		min-height: 2.5rem;
		padding: .5rem .75rem;
		border: 0;
		border-bottom: 1px solid var(--border);
		color: var(--foreground);
		background: transparent;
		font: inherit;
		font-size: .8125rem;
	}
	.results { min-height: 0; overflow: auto; padding: .25rem; }
	.results section + section { margin-top: .25rem; }
	.results h2 { margin: 0; padding: .4rem .5rem .25rem; color: var(--muted-foreground); font-size: .6875rem; font-weight: 600; }
	.result { width: 100%; min-height: 2.25rem; gap: .5rem; padding: .4rem .5rem; border-color: transparent; border-radius: .35rem; text-align: left; font-size: .75rem; }
	.result > :global(svg) { flex: 0 0 auto; color: var(--muted-foreground); }
	.result-label { min-width: 0; flex: 1 1 auto; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.result-kind, .result-detail { max-width: 8rem; flex: 0 1 auto; overflow: hidden; color: var(--muted-foreground); font-size: .625rem; text-overflow: ellipsis; white-space: nowrap; }
	.result-detail { font-family: ui-monospace, monospace; }
	.empty, .result-limit { margin: 0; padding: .75rem; color: var(--muted-foreground); font-size: .75rem; }
	.result-limit { border-top: 1px solid var(--border); }
	.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
</style>
