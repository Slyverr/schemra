<script lang="ts">
	import { tick } from 'svelte';
	import {
		AlignLeft,
		Command,
		Download,
		FileCode2,
		FilePlus2,
		FolderOpen,
		FolderPlus,
		Search,
		Save,
		Settings2,
		Sparkles,
		Workflow,
		X
	} from '@lucide/svelte';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { Button } from '../components/ui/button/index.js';
	import { cn } from '../utils.js';
	import { studioMessage } from '../i18n/messages.js';
	import { commandPaletteClasses as classes } from './command-palette-classes.js';
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
	const actionIcons = {
		reference: FileCode2,
		'new-project': FolderPlus,
		'new-visual': FilePlus2,
		'open-source': FolderOpen,
		save: Save,
		format: AlignLeft,
		'export-diagram': Workflow,
		'import-project': FolderOpen,
		'export-project': Download,
		settings: Settings2,
		editor: FileCode2,
		preview: Workflow,
		default: Sparkles
	} as const;

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
		const groups = [
			{ id: 'openTabs', label: 'command.group.openTabs' },
			{ id: 'projectFiles', label: 'command.group.projectFiles' },
			{ id: 'projects', label: 'command.group.projects' },
			{ id: 'actions', label: 'command.group.actions' }
		] as const;
		return groups.flatMap(({ id, label }) => {
			const items = visibleEntries.flatMap((entry, index) => entry.group === id ? [{ entry, index }] : []);
			return items.length ? [{ id, label, items }] : [];
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

</script>

<Dialog.Root bind:open>
	{#if open}
		<Dialog.Content
			id={dialogId}
			class={classes.content}
			overlayClass={classes.overlay}
			aria-labelledby={`${instanceId}-title`}
			aria-describedby={`${instanceId}-description`}
		onkeydown={handleKeydown}
		>
			<Dialog.Header class={classes.heading}>
				<div class={classes.icon} aria-hidden="true"><Command size={18} /></div>
				<div class={classes.copy}>
					<Dialog.Title id={`${instanceId}-title`} class={classes.title}>{studioMessage('command.title')}</Dialog.Title>
					<Dialog.Description id={`${instanceId}-description`} class={classes.description}>{studioMessage('command.description')}</Dialog.Description>
				</div>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button variant="ghost" size="icon-sm" class={classes.close} aria-label={studioMessage('command.close')} title={studioMessage('command.close')} {...props}>
							<X size={17} aria-hidden="true" />
						</Button>
					{/snippet}
				</Dialog.Close>
			</Dialog.Header>

			<div class={classes.search}>
				<Search size={17} aria-hidden="true" />
				<label class="sr-only" for={searchId}>{studioMessage('command.inputLabel')}</label>
				<input
					id={searchId}
					bind:this={searchInput}
					class={classes.input}
					value={query}
					role="combobox"
					aria-autocomplete="list"
					aria-expanded="true"
					aria-controls={resultsId}
					aria-activedescendant={activeEntry ? optionId(activeEntry) : undefined}
					autocomplete="off"
					placeholder={studioMessage('command.placeholder')}
					oninput={handleQueryInput}
				/>
				<kbd class={classes.keyboardHint}>ESC</kbd>
			</div>

			<div id={resultsId} class={classes.results} role="listbox" aria-label={studioMessage('command.results')}>
				{#if groups.length === 0}
					<p class={classes.empty} role="status">{studioMessage('command.empty')}</p>
				{:else}
					{#each groups as group (group.id)}
						<div class={classes.group} role="group" aria-label={studioMessage(group.label)}>
							<h3 class={classes.groupTitle}>{studioMessage(group.label)}<span class={classes.groupCount}>{group.items.length}</span></h3>
							{#each group.items as item (`${item.entry.key}`)}
								{@const entry = item.entry}
								{@const isActive = activeIndexInResults === item.index}
								{@const Icon = entry.type === 'action' ? actionIcons[entry.icon] : entry.type === 'project' ? FolderOpen : FileCode2}
								<button
									id={optionId(entry)}
									class={cn(classes.option, isActive && 'border-primary/15 bg-accent text-accent-foreground')}
									type="button"
									role="option"
									aria-selected={isActive}
									aria-disabled={entry.disabled ? 'true' : undefined}
									tabindex="-1"
									disabled={entry.disabled}
									onpointermove={() => (activeIndex = item.index)}
									onclick={() => selectEntry(entry)}
								>
									<span class={cn(classes.optionIcon, isActive && classes.optionIconActive)} aria-hidden="true">
										<Icon size={16} />
									</span>
									<span class={classes.optionCopy}>
										<span class={classes.optionTitle}>{entry.title}</span>
										{#if entry.type === 'file'}
											<span class={classes.optionDetail}>{entry.path}</span>
										{:else if entry.type === 'action' && entry.description}
											<span class={classes.optionDetail}>{entry.description}</span>
										{:else if entry.type === 'project'}
											<span class={classes.optionDetail}>{studioMessage(entry.fileCount === 1 ? 'command.fileCount.one' : 'command.fileCount.other', { count: entry.fileCount })}</span>
										{/if}
									</span>
									{#if entry.type === 'file'}
										<span class={cn(classes.badge, classes.resourceKind)}>{studioMessage(entry.kind === 'visual' ? 'sidebar.resourceKindVisual' : entry.kind === 'scenario' ? 'sidebar.resourceKindScenario' : 'sidebar.resourceKindSource')}</span>
										{#if entry.current}<span class={cn(classes.badge, classes.current)}>{studioMessage('command.current')}</span>{/if}
										{#if entry.unsaved}<span class={cn(classes.badge, classes.unsaved)}>{studioMessage('command.unsaved')}</span>{/if}
									{:else if entry.type === 'project' && entry.current}
										<span class={cn(classes.badge, classes.current)}>{studioMessage('command.current')}</span>
									{:else if entry.type === 'action' && entry.shortcut}
										<kbd class={classes.shortcut}>{entry.shortcut}</kbd>
									{/if}
								</button>
							{/each}
						</div>
					{/each}
				{/if}
			</div>

			<footer class={classes.footer}>
				<span class={classes.footerHint}><kbd class={classes.footerKey}>↑</kbd><kbd class={classes.footerKey}>↓</kbd> {studioMessage('command.footerNavigate')}</span>
				<span class={classes.footerHint}><kbd class={classes.footerKey}>Enter</kbd> {studioMessage('command.footerSelect')}</span>
			</footer>
		</Dialog.Content>
	{/if}
</Dialog.Root>
