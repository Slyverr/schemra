<script lang="ts">
	import { onDestroy, tick } from 'svelte';
	import { Command, FileCode2, FolderOpen, Search, Sparkles, X } from '@lucide/svelte';
	import {
		buildCommandPaletteEntries,
		firstCommandPaletteSelection,
		lastCommandPaletteSelection,
		moveCommandPaletteSelection,
		searchCommandPalette,
		type CommandPaletteAction,
		type CommandPaletteContext,
		type CommandPaletteEntry
	} from './command-palette.js';

	type PaletteProject = CommandPaletteContext['projects'][number];

	interface Props {
		open?: boolean;
		projects: readonly PaletteProject[];
		activeProjectId: string | null;
		activeFileId: string | null;
		openTabs?: readonly { projectId: string; fileId: string }[];
		actions?: readonly CommandPaletteAction[];
		onSelectProjectFile: (projectId: string, fileId: string) => void;
		onSelectProject: (projectId: string) => void;
		onAction: (actionId: string) => void;
	}

	let {
		open = $bindable(false),
		projects,
		activeProjectId,
		activeFileId,
		openTabs,
		actions,
		onSelectProjectFile,
		onSelectProject,
		onAction
	}: Props = $props();
	const instanceId = $props.id();
	const dialogId = `${instanceId}-dialog`;
	const searchId = `${instanceId}-search`;
	const resultsId = `${instanceId}-results`;
	let searchInput = $state<HTMLInputElement>();
	let closeButton = $state<HTMLButtonElement>();
	let query = $state('');
	let activeIndex = $state(0);
	let wasOpen = false;
	let previousFocus: HTMLElement | null = null;

	const context = $derived.by((): CommandPaletteContext => ({
		projects,
		activeProjectId,
		activeFileId,
		openTabs,
		actions
	}));
	const entries = $derived(buildCommandPaletteEntries(context));
	const visibleEntries = $derived(searchCommandPalette(entries, query));
	const activeIndexInResults = $derived(
		visibleEntries[activeIndex] && !visibleEntries[activeIndex]?.disabled
			? activeIndex
			: firstCommandPaletteSelection(visibleEntries)
	);
	const activeEntry = $derived(visibleEntries[activeIndexInResults] ?? null);
	const groups = $derived.by(() => {
		const names = ['Open tabs', 'Project files', 'Projects', 'Actions'] as const;
		return names.flatMap((name) => {
			const items = visibleEntries.flatMap((entry, index) => entry.group === name ? [{ entry, index }] : []);
			return items.length ? [{ name, items }] : [];
		});
	});

	$effect(() => {
		if (open && !wasOpen) {
			wasOpen = true;
			previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
			query = '';
			activeIndex = firstCommandPaletteSelection(entries);
			void tick().then(() => searchInput?.focus());
		} else if (!open && wasOpen) {
			wasOpen = false;
			const restore = previousFocus;
			previousFocus = null;
			void tick().then(() => restore?.focus());
		}
	});

	function optionId(entry: CommandPaletteEntry) {
		return `${instanceId}-option-${encodeURIComponent(entry.key)}`;
	}

	function handleQueryInput(event: Event) {
		query = (event.currentTarget as HTMLInputElement).value;
		activeIndex = firstCommandPaletteSelection(searchCommandPalette(entries, query));
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.preventDefault();
			event.stopPropagation();
			close();
			return;
		}
		if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
			event.preventDefault();
			moveSelection(event.key === 'ArrowDown' ? 1 : -1);
			return;
		}
		if (event.key === 'Home' || event.key === 'End') {
			event.preventDefault();
			activeIndex = event.key === 'Home'
				? firstCommandPaletteSelection(visibleEntries)
				: lastCommandPaletteSelection(visibleEntries);
			ensureActiveOptionVisible();
			return;
		}
		if (event.key === 'Enter') {
			event.preventDefault();
			if (activeEntry && !activeEntry.disabled) selectEntry(activeEntry);
			return;
		}
		if (event.key === 'Tab') {
			event.preventDefault();
			if (event.shiftKey) {
				if (document.activeElement === searchInput) closeButton?.focus();
				else searchInput?.focus();
			} else if (document.activeElement === closeButton) {
				searchInput?.focus();
			} else {
				closeButton?.focus();
			}
		}
	}

	function moveSelection(direction: -1 | 1) {
		activeIndex = moveCommandPaletteSelection(visibleEntries, activeIndexInResults, direction);
		ensureActiveOptionVisible();
	}

	function ensureActiveOptionVisible() {
		const entry = visibleEntries[activeIndexInResults];
		if (entry) void tick().then(() => document.getElementById(optionId(entry))?.scrollIntoView({ block: 'nearest' }));
	}

	function selectEntry(entry: CommandPaletteEntry) {
		if (entry.disabled) return;
		previousFocus = null;
		close();
		if (entry.type === 'file') onSelectProjectFile(entry.projectId, entry.fileId);
		else if (entry.type === 'project') onSelectProject(entry.projectId);
		else onAction(entry.actionId);
	}

	function close() {
		open = false;
	}

	onDestroy(() => {
		wasOpen = false;
		previousFocus = null;
	});
</script>

{#if open}
	<div class="palette-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && close()}>
		<div
			id={dialogId}
			class="palette-dialog"
			role="dialog"
			tabindex="-1"
			aria-modal="true"
			aria-labelledby={`${instanceId}-title`}
			aria-describedby={`${instanceId}-description`}
			onkeydown={handleKeydown}
		>
			<header class="palette-heading">
				<div class="palette-heading-icon" aria-hidden="true"><Command size={18} /></div>
				<div class="palette-heading-copy">
					<h2 id={`${instanceId}-title`}>Search files and actions</h2>
					<p id={`${instanceId}-description`}>Jump to a tab, open a project file, switch projects, or run a Studio action.</p>
				</div>
				<button bind:this={closeButton} class="palette-close" type="button" aria-label="Close search" onclick={close}>
					<X size={17} aria-hidden="true" />
				</button>
			</header>

			<div class="palette-search-wrap">
				<Search size={17} aria-hidden="true" />
				<label class="sr-only" for={searchId}>Search open tabs, project files, projects, and actions</label>
				<input
					id={searchId}
					bind:this={searchInput}
					value={query}
					role="combobox"
					aria-autocomplete="list"
					aria-expanded="true"
					aria-controls={resultsId}
					aria-activedescendant={activeEntry ? optionId(activeEntry) : undefined}
					autocomplete="off"
					placeholder="Search tabs, files, projects, and actions…"
					oninput={handleQueryInput}
				/>
				<kbd class="keyboard-hint">ESC</kbd>
			</div>

			<div id={resultsId} class="palette-results" role="listbox" aria-label="Search results">
				{#if groups.length === 0}
					<p class="palette-empty" role="status">No matching tabs, files, projects, or actions.</p>
				{:else}
					{#each groups as group (group.name)}
						<div class="palette-group" role="group" aria-label={group.name}>
							<h3>{group.name}<span>{group.items.length}</span></h3>
							{#each group.items as item (`${item.entry.key}`)}
								{@const entry = item.entry}
								<button
									id={optionId(entry)}
									class="palette-option"
									class:active={activeIndexInResults === item.index}
									class:disabled={entry.disabled}
									type="button"
									role="option"
									aria-selected={activeIndexInResults === item.index}
									aria-disabled={entry.disabled ? 'true' : undefined}
									tabindex="-1"
									disabled={entry.disabled}
									onpointermove={() => (activeIndex = item.index)}
									onclick={() => selectEntry(entry)}
								>
									<span class="option-icon" aria-hidden="true">
										{#if entry.type === 'action'}<Sparkles size={16} />
										{:else if entry.type === 'project'}<FolderOpen size={16} />
										{:else}<FileCode2 size={16} />{/if}
									</span>
									<span class="option-copy">
										<span class="option-title">{entry.title}</span>
										{#if entry.type === 'file'}
											<span class="option-detail">{entry.path}</span>
										{:else if entry.type === 'action' && entry.description}
											<span class="option-detail">{entry.description}</span>
										{:else if entry.type === 'project'}
											<span class="option-detail">{entry.fileCount} {entry.fileCount === 1 ? 'file' : 'files'}</span>
										{/if}
									</span>
									{#if entry.type === 'file'}
										<span class="option-badge">{entry.kind}</span>
						{#if entry.current}<span class="option-current">Current</span>{/if}
										{#if entry.unsaved}<span class="option-unsaved">Unsaved</span>{/if}
									{:else if entry.type === 'project' && entry.current}
										<span class="option-current">Current</span>
									{:else if entry.type === 'action' && entry.shortcut}
										<kbd class="option-shortcut">{entry.shortcut}</kbd>
									{/if}
								</button>
							{/each}
						</div>
					{/each}
				{/if}
			</div>

			<footer class="palette-footer">
				<span><kbd>↑</kbd><kbd>↓</kbd> to navigate</span>
				<span><kbd>Enter</kbd> to select</span>
			</footer>
		</div>
	</div>
{/if}

<style>
	.palette-backdrop {
		position: fixed;
		z-index: 1000;
		inset: 0;
		display: grid;
		align-items: start;
		justify-items: center;
		padding: min(14vh, 7rem) 1rem 1rem;
		background: rgb(12 25 43 / 48%);
		backdrop-filter: blur(2px);
	}
	.palette-dialog {
		width: min(100%, 42rem);
		max-height: min(78vh, 46rem);
		display: flex;
		flex-direction: column;
		overflow: hidden;
		border: 1px solid var(--border);
		border-radius: calc(var(--radius) + 0.25rem);
		background: var(--popover);
		color: var(--popover-foreground);
		box-shadow: 0 1.5rem 4rem rgb(10 30 56 / 26%);
	}
	.palette-heading {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 1rem 1rem 0.85rem;
	}
	.palette-heading-icon, .option-icon {
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		color: var(--muted-foreground);
	}
	.palette-heading-icon {
		width: 2.25rem;
		height: 2.25rem;
		border: 1px solid var(--border);
		border-radius: 0.65rem;
		background: var(--muted);
		color: var(--primary);
	}
	.palette-heading-copy { min-width: 0; flex: 1; }
	.palette-heading h2 { margin: 0; font-size: 0.95rem; font-weight: 650; letter-spacing: -0.01em; }
	.palette-heading p { margin: 0.2rem 0 0; color: var(--muted-foreground); font-size: 0.76rem; line-height: 1.4; }
	.palette-close {
		width: 2rem;
		height: 2rem;
		flex: 0 0 auto;
		display: grid;
		place-items: center;
		border: 1px solid transparent;
		border-radius: 0.5rem;
		background: transparent;
		color: var(--muted-foreground);
		cursor: pointer;
	}
	.palette-close:hover, .palette-close:focus-visible { border-color: var(--border); background: var(--muted); color: var(--foreground); outline: none; }
	.palette-search-wrap {
		min-height: 3rem;
		margin: 0 0.75rem;
		padding: 0 0.75rem;
		display: flex;
		align-items: center;
		gap: 0.6rem;
		border: 1px solid var(--input);
		border-radius: 0.65rem;
		background: var(--background);
		color: var(--muted-foreground);
	}
	.palette-search-wrap:focus-within { border-color: var(--ring); box-shadow: 0 0 0 2px color-mix(in srgb, var(--ring) 20%, transparent); }
	.palette-search-wrap input { min-width: 0; flex: 1; height: 2.8rem; border: 0; outline: none; background: transparent; color: var(--foreground); font: inherit; font-size: 0.875rem; }
	.palette-search-wrap input::placeholder { color: var(--muted-foreground); opacity: 0.85; }
	.keyboard-hint, .option-shortcut, .palette-footer kbd { border: 1px solid var(--border); border-radius: 0.3rem; background: var(--muted); color: var(--muted-foreground); font: 500 0.65rem/1.35 ui-monospace, SFMono-Regular, Menlo, monospace; }
	.keyboard-hint { padding: 0.12rem 0.35rem; }
	.palette-results { min-height: 4rem; overflow: auto; padding: 0.5rem 0.5rem 0.7rem; overscroll-behavior: contain; }
	.palette-empty { margin: 0; padding: 2rem 1rem; color: var(--muted-foreground); text-align: center; font-size: 0.82rem; }
	.palette-group + .palette-group { margin-top: 0.5rem; }
	.palette-group h3 { display: flex; justify-content: space-between; margin: 0; padding: 0.45rem 0.6rem 0.35rem; color: var(--muted-foreground); font-size: 0.68rem; font-weight: 650; letter-spacing: 0.06em; text-transform: uppercase; }
	.palette-group h3 span { font-variant-numeric: tabular-nums; font-weight: 500; letter-spacing: 0; }
	.palette-option {
		width: 100%;
		min-height: 3rem;
		padding: 0.42rem 0.55rem;
		display: flex;
		align-items: center;
		gap: 0.65rem;
		border: 1px solid transparent;
		border-radius: 0.55rem;
		background: transparent;
		color: var(--foreground);
		text-align: left;
		cursor: pointer;
	}
	.palette-option:hover, .palette-option.active { border-color: color-mix(in srgb, var(--primary) 16%, transparent); background: var(--accent); color: var(--accent-foreground); }
	.palette-option:focus-visible { outline: 2px solid var(--ring); outline-offset: 1px; }
	.palette-option.disabled { opacity: 0.45; cursor: not-allowed; }
	.option-icon { width: 1.8rem; height: 1.8rem; border-radius: 0.45rem; background: var(--muted); }
	.palette-option.active .option-icon { background: color-mix(in srgb, var(--primary) 12%, var(--background)); color: var(--primary); }
	.option-copy { min-width: 0; flex: 1; display: flex; flex-direction: column; gap: 0.13rem; }
	.option-title, .option-detail { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.option-title { font-size: 0.82rem; font-weight: 550; }
	.option-detail { color: var(--muted-foreground); font-size: 0.7rem; }
	.option-badge, .option-current, .option-unsaved { flex: 0 0 auto; border-radius: 999px; padding: 0.16rem 0.5rem; font-size: 0.64rem; line-height: 1.25; text-transform: capitalize; }
	.option-badge { border: 1px solid var(--border); color: var(--muted-foreground); }
	.option-current { background: var(--secondary); color: var(--secondary-foreground); text-transform: none; }
	.option-unsaved { color: #986300; background: color-mix(in srgb, #e6a700 14%, var(--background)); text-transform: none; }
	.option-shortcut { padding: 0.16rem 0.4rem; }
	.palette-footer { min-height: 2.5rem; padding: 0.4rem 0.85rem; display: flex; align-items: center; justify-content: space-between; gap: 1rem; border-top: 1px solid var(--border); color: var(--muted-foreground); font-size: 0.68rem; }
	.palette-footer span { display: inline-flex; align-items: center; gap: 0.25rem; }
	.palette-footer kbd { min-width: 1.25rem; padding: 0.08rem 0.25rem; text-align: center; }
	.sr-only { position: absolute; width: 1px; height: 1px; padding: 0; margin: -1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; border: 0; }
	@media (max-width: 36rem) {
		.palette-backdrop { padding: 0.5rem; align-items: start; }
		.palette-dialog { max-height: calc(100dvh - 1rem); border-radius: 0.8rem; }
		.palette-heading { padding: 0.75rem 0.75rem 0.65rem; }
		.palette-heading p { font-size: 0.69rem; }
		.palette-option { gap: 0.45rem; padding-inline: 0.4rem; }
		.option-detail { max-width: min(70vw, 22rem); }
	}
</style>
