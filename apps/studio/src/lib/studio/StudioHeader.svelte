<script lang="ts">
	import { AlertCircle, Check, LoaderCircle, Search } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import FileActionsMenu from './FileActionsMenu.svelte';
	import MobileMoreActionsMenu from './MobileMoreActionsMenu.svelte';
	import MobileResourceBrowser from './MobileResourceBrowser.svelte';
	import ProjectPicker from './ProjectPicker.svelte';
	import type { SidebarResource, SidebarVisual } from './project-sidebar.js';
	import type { DiagramExportFormat } from '../preview/export.js';

	type ProjectOption = { id: string; name: string; files: readonly unknown[] };
	type CompileStatus = 'compiling' | 'error' | 'ready' | 'attention';

	interface Props {
		projects: readonly ProjectOption[];
		activeProjectId: string | null;
		activeProjectName: string | null;
		activeProjectDirty: boolean;
		activeFileId: string | null;
		activeVisualId: string | null;
		visuals: SidebarVisual[];
		otherSources: SidebarResource[];
		initialized: boolean;
		creationDisabled: boolean;
		compileStatus: CompileStatus;
		commandPaletteOpen: boolean;
		sourceOperation: 'opening' | 'saving' | null;
		canSaveSource: boolean;
		canExportDiagram: boolean;
		onSelectProject: (projectId: string) => void;
		onCreateProject: () => void;
		onRenameProject: () => void;
		onImportProject: () => void;
		onExportProject: () => void;
		onOpenSearch: () => void;
		onSelectFile: (fileId: string) => void;
		onCreateVisual: () => void;
		onOpenSettings: () => void;
		onOpenSource: () => void;
		onSaveSource: () => void;
		onExportDiagram: (format: DiagramExportFormat) => void;
	}

	let {
		projects,
		activeProjectId,
		activeProjectName,
		activeProjectDirty,
		activeFileId,
		activeVisualId,
		visuals,
		otherSources,
		initialized,
		creationDisabled,
		compileStatus,
		commandPaletteOpen,
		sourceOperation,
		canSaveSource,
		canExportDiagram,
		onSelectProject,
		onCreateProject,
		onRenameProject,
		onImportProject,
		onExportProject,
		onOpenSearch,
		onSelectFile,
		onCreateVisual,
		onOpenSettings,
		onOpenSource,
		onSaveSource,
		onExportDiagram
	}: Props = $props();
</script>

<header class="flex h-14 min-h-14 shrink-0 items-center gap-2 border-b bg-background px-3 sm:px-4 max-sm:grid max-sm:h-auto max-sm:min-h-14 max-sm:grid-cols-[minmax(0,1fr)_auto] max-sm:gap-y-1 max-sm:py-1.5">
	<nav class="min-w-0 flex-1 overflow-hidden max-sm:col-start-1 max-sm:row-start-1" aria-label={studioMessage('header.projectLocation')}>
		<ol class="m-0 flex min-w-0 list-none items-center gap-2 p-0">
			<li class="flex min-w-0 flex-[0_1_auto] items-center">
				<ProjectPicker
					{projects}
					{activeProjectId}
					{activeProjectName}
					{activeProjectDirty}
					disabled={!initialized}
					onSelect={onSelectProject}
					onCreate={onCreateProject}
					onRename={onRenameProject}
					onImport={onImportProject}
					onExport={onExportProject}
				/>
			</li>
			<li class="flex shrink-0 items-center">
				<MobileResourceBrowser
					{visuals}
					{otherSources}
					{activeFileId}
					{activeVisualId}
					resourcesDisabled={!initialized}
					{creationDisabled}
					{onSelectFile}
					{onCreateVisual}
				/>
			</li>
		</ol>
	</nav>
	<div class="col-start-2 row-start-1 flex shrink-0 items-center gap-2 max-sm:col-start-2 max-sm:row-start-1">
		<div class="hidden items-center gap-1.5 whitespace-nowrap text-[.72rem] text-muted-foreground xl:inline-flex" aria-live="polite">
			{#if compileStatus === 'compiling'}
				<LoaderCircle size={14} class="animate-spin" aria-hidden="true" /><span>{studioMessage('header.compiling')}</span>
			{:else if compileStatus === 'error'}
				<AlertCircle size={14} class="shrink-0 text-destructive" aria-hidden="true" /><span>{studioMessage('header.compilationError')}</span>
			{:else if compileStatus === 'ready'}
				<Check size={14} class="shrink-0 text-primary" aria-hidden="true" /><span>{studioMessage('header.compiled')}</span>
			{:else}
				<AlertCircle size={14} class="shrink-0 text-destructive" aria-hidden="true" /><span>{studioMessage('header.needsAttention')}</span>
			{/if}
		</div>
		<Button
			variant="outline"
			class="h-9 w-9 gap-[.4rem] rounded-[.55rem] px-0 text-[.77rem] shadow-none hover:bg-accent hover:text-accent-foreground lg:w-auto lg:gap-[.45rem] lg:px-3"
			type="button"
			aria-label={studioMessage('header.search')}
			title={studioMessage('header.searchHint')}
			aria-keyshortcuts="Control+K Meta+K"
			aria-haspopup="dialog"
			aria-expanded={commandPaletteOpen}
			onclick={onOpenSearch}
			disabled={!initialized}
		>
			<Search size={17} class="size-[17px]" aria-hidden="true" /><span class="hidden lg:inline">{studioMessage('header.searchLabel')}</span>
		</Button>
		<FileActionsMenu
			disabled={!initialized}
			{canSaveSource}
			{canExportDiagram}
			{sourceOperation}
			{onOpenSource}
			{onSaveSource}
			{onExportDiagram}
		/>
		<MobileMoreActionsMenu disabled={!initialized} {creationDisabled} {onOpenSettings} {onCreateVisual} />
	</div>
</header>
