<script lang="ts">
	import { tick } from 'svelte';
	import {
		AlignLeft,
		Download,
		FileCode2,
		FilePlus2,
		FolderOpen,
		FolderPlus,
		Play,
		Search,
		Save,
		Settings2,
		Sparkles,
		Workflow,
	} from '@lucide/svelte';
	import { Command as CommandPrimitive } from 'bits-ui';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { cn } from '../utils.js';
	import { studioMessage } from '../i18n/messages.js';
	import { commandPaletteClasses as classes } from './command-palette-classes.js';
	import {
		buildCommandPaletteEntries,
		firstCommandPaletteSelection,
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

	function entryIcon(entry: CommandPaletteEntry) {
		if (entry.type === 'action') return actionIcons[entry.icon];
		if (entry.type === 'project') return FolderOpen;
		if (entry.kind === 'visual') return Workflow;
		if (entry.kind === 'scenario') return Play;
		return FileCode2;
	}

	function fileKindLabel(kind: 'visual' | 'scenario' | 'source') {
		return studioMessage(
			kind === 'visual' ? 'sidebar.resourceKindVisual' : kind === 'scenario' ? 'sidebar.resourceKindScenario' : 'sidebar.resourceKindSource'
		);
	}

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
	let selectedKey = $state('');
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
	const groups = $derived.by(() => {
		const groups = [
			{ id: 'openTabs', label: 'command.group.openTabs' },
			{ id: 'projectFiles', label: 'command.group.projectFiles' },
			{ id: 'projects', label: 'command.group.projects' },
			{ id: 'actions', label: 'command.group.actions' }
		] as const;
		return groups.flatMap(({ id, label }) => {
			const items = visibleEntries.filter((entry) => entry.group === id);
			return items.length ? [{ id, label, items }] : [];
		});
	});

	$effect(() => {
		if (open && !wasOpen) {
			wasOpen = true;
			previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
			query = '';
			selectedKey = entries[firstCommandPaletteSelection(entries)]?.key ?? '';
			void tick().then(() => searchInput?.focus());
		} else if (!open && wasOpen) {
			wasOpen = false;
			const restore = previousFocus;
			previousFocus = null;
			void tick().then(() => restore?.focus());
		}
	});

	function handleQueryInput(event: Event) {
		const nextQuery = (event.currentTarget as HTMLInputElement).value;
		query = nextQuery;
		const results = searchCommandPalette(entries, nextQuery);
		selectedKey = results[firstCommandPaletteSelection(results)]?.key ?? '';
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
		>
			<Dialog.Header class="sr-only">
				<Dialog.Title id={`${instanceId}-title`}>{studioMessage('command.title')}</Dialog.Title>
				<Dialog.Description id={`${instanceId}-description`}>{studioMessage('command.description')}</Dialog.Description>
			</Dialog.Header>

			<CommandPrimitive.Root
				bind:value={selectedKey}
				label={studioMessage('command.results')}
				loop
				shouldFilter={false}
				class={classes.root}
			>
				<div class={classes.search}>
					<Search size={17} aria-hidden="true" />
					<CommandPrimitive.Input
						id={searchId}
						bind:ref={searchInput}
						bind:value={query}
						class={classes.input}
						aria-label={studioMessage('command.inputLabel')}
						autocomplete="off"
						placeholder={studioMessage('command.placeholder')}
						oninput={handleQueryInput}
					/>
				</div>

				<CommandPrimitive.List id={resultsId} class={classes.results}>
					<CommandPrimitive.Empty class={classes.empty}>{studioMessage('command.empty')}</CommandPrimitive.Empty>
					{#each groups as group (group.id)}
						<CommandPrimitive.Group value={group.id} class={classes.group}>
							<CommandPrimitive.GroupHeading class={classes.groupTitle}>
								{studioMessage(group.label)}
								<span class={classes.groupCount}>{group.items.length}</span>
							</CommandPrimitive.GroupHeading>
							<CommandPrimitive.GroupItems>
								{#each group.items as entry (entry.key)}
									{@const isActive = selectedKey === entry.key}
									{@const Icon = entryIcon(entry)}
									<CommandPrimitive.Item
										value={entry.key}
										disabled={entry.disabled}
										onSelect={() => selectEntry(entry)}
										class={cn(classes.option, isActive && 'bg-muted text-foreground')}
									>
										<span class={cn(classes.optionIcon, isActive && classes.optionIconActive)} aria-hidden="true">
											<Icon size={16} />
										</span>
										<span class={classes.optionCopy}>
											{#if entry.type === 'file'}
												<span class="sr-only">{fileKindLabel(entry.kind)}</span>
											{/if}
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
											{#if entry.current}<span class={cn(classes.badge, classes.current)}>{studioMessage('command.current')}</span>{/if}
											{#if entry.unsaved}<span class={cn(classes.badge, classes.unsaved)}>{studioMessage('command.unsaved')}</span>{/if}
										{:else if entry.type === 'project' && entry.current}
											<span class={cn(classes.badge, classes.current)}>{studioMessage('command.current')}</span>
										{:else if entry.type === 'action' && entry.shortcut}
											<kbd class={classes.shortcut}>{entry.shortcut}</kbd>
										{/if}
									</CommandPrimitive.Item>
								{/each}
							</CommandPrimitive.GroupItems>
						</CommandPrimitive.Group>
					{/each}
				</CommandPrimitive.List>

			<footer class={classes.footer}>
				<span class={classes.footerHint}><kbd class={classes.footerKey}>↑</kbd><kbd class={classes.footerKey}>↓</kbd> {studioMessage('command.footerNavigate')}</span>
				<span class={classes.footerHint}><kbd class={classes.footerKey}>Enter</kbd> {studioMessage('command.footerSelect')}</span>
			</footer>
			</CommandPrimitive.Root>
		</Dialog.Content>
	{/if}
</Dialog.Root>
