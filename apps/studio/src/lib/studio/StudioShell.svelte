<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import { AlertCircle, Download, Plus, Save } from '@lucide/svelte';
	import { Button } from '../components/ui/button/index.js';
	import { editorOffsetAtPosition, readEditorDiagnostics, readEditorSyntaxTokens } from '../editor/source-model.js';
	import { studioMessage } from '../i18n/messages.js';
	import {
		diagnosticCounts,
		diagnosticQuickFixEdit,
		editorSymbolForRefactor,
		isSafeQuickFix,
		readRefactorDiagnostics,
		type RefactorDiagnostic,
		type RefactorFile
	} from '../editor/editor-refactors.js';
	import { findEditorSymbolDefinition } from '../editor/symbol-navigation.js';
	import {
		createWorkspaceController,
		type WorkspaceControllerState
	} from '../project/workspace-controller.js';
	import { shouldWarnBeforeUnload } from '../project/workspace-lifecycle.js';
	import { createExampleProject } from '../project/example-project.js';
	import { MAX_PROJECT_FILES } from '../project/session.js';
	import {
		DEFAULT_EDITOR_PREFERENCES,
		readEditorPreferences,
		stepEditorZoom,
		writeEditorPreferences,
		type EditorPreferences
	} from '../settings/editor.js';
	import {
		DEFAULT_THEME_PREFERENCES,
		editorThemeStyles,
		readThemePreferences,
		resolveEditorTheme,
		resolveStudioTheme,
		writeThemePreferences,
		type ThemePreferences
	} from '../settings/appearance.js';
	import CommandPalette from './CommandPalette.svelte';
	import LanguageReferenceDialog from './LanguageReferenceDialog.svelte';
	import type { CommandPaletteAction } from './command-palette.js';
	import ProjectSidebar from './ProjectSidebar.svelte';
	import ResourceDialog from './ResourceDialog.svelte';
	import SymbolRenameDialog from './SymbolRenameDialog.svelte';
	import WorkspaceTabs from './WorkspaceTabs.svelte';
	import StudioHeader from './StudioHeader.svelte';
	import WorkspaceLoading from './WorkspaceLoading.svelte';
	import WorkspacePanels from './WorkspacePanels.svelte';
	import StudioSettingsDialog from './StudioSettingsDialog.svelte';
	import StudioToast from './StudioToast.svelte';
	import { buildProjectSidebarModel, sourceLabel } from './project-sidebar.js';
	import {
		compareSourceFiles,
		isValidProjectCompilation,
		makeSourcesKey,
		stabilizeFileIdentity
	} from './project-view.js';
	import { buildWorkspaceTabIds, buildWorkspaceTabViews } from './workspace-tabs.js';
	import { isInitialWorkspaceViewReady } from './initial-workspace-view.js';
	import { resourceTabMenuPolicy, type ResourceTabKind } from './resource-tab-menu.js';
	import { createEditorSymbolRenameActions, type EditorRenameState } from './editor-symbol-rename.js';
	import { createSourceActions, type SourceActionState } from './source-actions.js';
	import {
		createResourceActions,
		type CompiledScenario,
		type ProjectDeclaration,
		type ProjectVisual,
		type ResourceCompileFile,
		type ResourceDialogState
	} from './resource-actions.js';
	import { studioBrandAssets } from './brand-assets.js';
	import type { DiagramExportFormat } from '../preview/export.js';
	import {
		compileProject,
		type CompileResult,
		type EditorSymbol,
		type ProjectCompileResult,
		type ProjectSource
	} from '../schemra.js';
	import { readPreviewScene } from '../preview/model.js';

	type WorkspacePanelsHandle = {
		exportDiagram: (format: DiagramExportFormat) => Promise<void>;
		formatSource: () => Promise<'formatted' | 'unchanged' | 'stale'>;
	};
	type LastValidCompile = { projectId: string; sourcesKey: string; sources: ProjectSource[]; result: ProjectCompileResult };
	const controller = createWorkspaceController({ exampleProject: createExampleProject() });
	let workspace = $state<WorkspaceControllerState>(controller.getState());
	let compilation = $state<{ projectId: string; sourcesKey: string; result: ProjectCompileResult } | null>(null);
	let initialCompilationSettled = $state(false);
	let lastValid = $state<LastValidCompile | null>(null);
	let compilePending = $state(false);
	let compileError = $state<string | null>(null);
	let settingsOpen = $state(false);
	let commandPaletteOpen = $state(false);
	let languageReferenceOpen = $state(false);
	let paneLayoutWide = $state(false);
	let editorPaneRatio = $state(50);
	let focusedPane = $state<'editor' | 'preview' | null>(null);
	let paneResizeOrigin: { coordinate: number; ratio: number; size: number; pointerId: number } | null = null;
	let themePreferences = $state<ThemePreferences>({ ...DEFAULT_THEME_PREFERENCES });
	let editorPreferences = $state<EditorPreferences>(readEditorPreferences());
	let systemPrefersDark = $state(false);
	let importInput = $state<HTMLInputElement>();
	let sourceInput = $state<HTMLInputElement>();
	let workspacePanels = $state<WorkspacePanelsHandle | null>(null);
	let sourceActionState = $state<SourceActionState>({ operation: null, pendingProjectId: null });
	let fileMessage = $state<{ kind: 'error' | 'success'; text: string } | null>(null);
	let quickFixPending = $state(false);
	let resourceForm = $state<ResourceDialogState>({ kind: null, nameDraft: '', error: '', target: null });
	let editorNavigation = $state<{ fileId: string; offset: number; requestId: number } | null>(null);
	let editorRename = $state<EditorRenameState>({ open: false, target: null, draft: '', error: '', pending: false });
	let symbolRenameInput = $state<HTMLInputElement | null>(null);
	let compileSequence = 0;
	let navigationSequence = 0;

	const activeProject = $derived(workspace.projects.find(({ id }) => id === workspace.activeProjectId) ?? null);
	const resourceCreationDisabled = $derived(
		!workspace.initialized || !activeProject || sourceActionState.operation !== null || activeProject.files.length >= MAX_PROJECT_FILES
	);
	const activeFileId = $derived(activeProject ? workspace.selectedFileByProject[activeProject.id] ?? activeProject.files[0]?.id ?? null : null);
	const activeFile = $derived(activeProject?.files.find(({ id }) => id === activeFileId) ?? null);
	const sourceFiles = $derived(activeProject?.files.map(({ id, path, source }) => ({ id, path, source })) ?? []);
	const sourcesKey = $derived(makeSourcesKey(sourceFiles));
	const orderedFiles = $derived(activeProject ? [...activeProject.files].sort(compareSourceFiles) : []);
	const currentCompilation = $derived(
		compilation && compilation.projectId === activeProject?.id && compilation.sourcesKey === sourcesKey ? compilation.result : null
	);
	const currentFiles = $derived((currentCompilation?.files ?? []) as ResourceCompileFile[]);
	const lastValidForProject = $derived(lastValid && lastValid.projectId === activeProject?.id ? lastValid : null);
	const lastGoodFiles = $derived((lastValidForProject ? lastValidForProject.result.files : []) as ResourceCompileFile[]);
	const currentFileViews = $derived(stabilizeFileIdentity(
		currentFiles,
		lastGoodFiles,
		activeProject?.files.map(({ id }) => id) ?? []
	));
	const sidebarModel = $derived(buildProjectSidebarModel(activeProject?.files ?? [], currentFileViews));
	const sidebarVisuals = $derived(sidebarModel.visuals);
	const sidebarOtherSources = $derived(sidebarModel.otherSources);
	const currentFileCompilation = $derived(currentFileViews.find(({ id }) => id === activeFileId) ?? null);
	const latestActiveFileCompilation = $derived(currentFiles.find(({ id }) => id === activeFileId) ?? null);
	const currentRefactorFiles = $derived(currentFiles.map((file) => ({
		path: file.path,
		diagnostics: readRefactorDiagnostics(file.diagnostics, file.path)
	})));
	const currentRefactorDiagnostics = $derived(
		currentRefactorFiles.find(({ path }) => path === activeFile?.path)?.diagnostics ?? []
	);
	const currentRefactorSymbols = $derived.by(() => currentFiles.flatMap((file) => {
		const source = activeProject?.files.find(({ id }) => id === file.id)?.source;
		if (source === undefined) return [];
		return (file.editor_symbols ?? []).flatMap((symbol) => {
			const converted = editorSymbolForRefactor(source, symbol, file.path);
			return converted ? [converted] : [];
		});
	}));
	const lastGoodActiveFile = $derived(lastGoodFiles.find(({ id }) => id === activeFileId) ?? null);
	const visualId = $derived(
		currentFileCompilation?.visual_id ?? lastGoodActiveFile?.visual_id ??
		(currentCompilation?.visuals as ProjectVisual[] | undefined)?.[0]?.id ??
		(lastValidForProject ? (lastValidForProject.result.visuals as ProjectVisual[])[0]?.id : null) ?? null
	);
	const workspaceTabIds = $derived(buildWorkspaceTabIds(orderedFiles, currentFileViews, visualId, activeFileId));
	function sidebarDeclaration(fileId: string, kind: ResourceTabKind, resourceId: string | null) {
		const file = currentFiles.find(({ id }) => id === fileId);
		return file?.declarations.find((declaration) =>
			declaration.kind === kind && (resourceId === null || declaration.id === resourceId)
		) ?? null;
	}

	function getSidebarResourceMenuPolicy(fileId: string, kind: ResourceTabKind, resourceId: string | null) {
		const file = currentFiles.find(({ id }) => id === fileId);
		const declarationCount = file?.declarations.filter(({ kind: declarationKind }) => declarationKind === kind).length ?? 0;
		return resourceTabMenuPolicy({
			kind,
			declarationCount,
			projectFileCount: activeProject?.files.length ?? 0,
			initialized: workspace.initialized,
			compiling: compilePending,
			projectValid: currentProjectValid
		});
	}

	function renameSidebarResource(fileId: string, kind: ResourceTabKind, resourceId: string | null) {
		const declaration = sidebarDeclaration(fileId, kind, resourceId);
		if (declaration) resourceActions.openResourceRenameForFile(fileId, declaration);
	}

	function removeSidebarResource(fileId: string, kind: ResourceTabKind, resourceId: string | null) {
		resourceActions.openResourceRemovalForFile(fileId, kind, sidebarDeclaration(fileId, kind, resourceId));
	}
	const paletteTabs = $derived(activeProject ? workspaceTabIds.map((fileId) => ({ projectId: activeProject.id, fileId })) : []);
	const currentVisual = $derived(
		((currentCompilation?.visuals ?? []) as ProjectVisual[]).find(({ id }) => id === visualId) ?? null
	);
	const lastGoodVisual = $derived(
		(lastValidForProject ? (lastValidForProject.result.visuals as ProjectVisual[]) : []).find(({ id }) => id === visualId) ?? null
	);
	const currentProjectValid = $derived(currentCompilation ? isValidProjectCompilation(currentCompilation) : false);
	const paletteActions = $derived<CommandPaletteAction[]>([
		{ id: 'language-reference', label: studioMessage('command.action.languageReference'), description: studioMessage('command.action.languageReferenceDescription'), keywords: ['DSL', 'syntax', 'visual', 'node', 'link', 'scenario', 'token', 'move', 'highlight', 'repeat'], icon: 'reference' },
		{ id: 'new-project', label: studioMessage('command.action.newProject'), description: studioMessage('command.action.newProjectDescription'), icon: 'new-project' },
		{ id: 'rename-project', label: studioMessage('command.action.renameProject'), description: activeProject?.name ?? studioMessage('command.action.renameProjectDescription'), disabled: !activeProject },
		{ id: 'new-visual', label: studioMessage('command.action.newVisual'), description: studioMessage('command.action.newVisualDescription'), disabled: resourceCreationDisabled, icon: 'new-visual' },
		{ id: 'new-scenario', label: studioMessage('command.action.newScenario'), description: studioMessage('command.action.newScenarioDescription'), disabled: resourceCreationDisabled || !currentProjectValid || !currentVisual },
		{ id: 'open-source', label: studioMessage('command.action.openSource'), description: studioMessage('command.action.openSourceDescription'), disabled: !workspace.initialized || sourceActionState.operation !== null, shortcut: studioMessage('command.shortcut.openSource'), icon: 'open-source' },
		{ id: 'save-workspace', label: studioMessage('command.action.saveWorkspace'), description: studioMessage('command.action.saveWorkspaceDescription'), disabled: !workspace.initialized || !activeFile, shortcut: studioMessage('command.shortcut.save'), icon: 'save' },
		{ id: 'save-source', label: studioMessage('command.action.saveSource'), description: studioMessage('command.action.saveSourceDescription'), disabled: !workspace.initialized || !activeFile || sourceActionState.operation !== null, icon: 'save' },
		{ id: 'format-source', label: studioMessage('command.action.formatSource'), description: studioMessage('command.action.formatSourceDescription'), disabled: !workspace.initialized || !activeFile || sourceActionState.operation !== null, shortcut: studioMessage('command.shortcut.format'), icon: 'format' },
		{ id: 'export-svg', label: studioMessage('command.action.exportSvg'), description: studioMessage('command.action.exportSvgDescription'), disabled: !currentProjectValid || !currentVisual?.output.svg, icon: 'export-diagram' },
		{ id: 'export-png', label: studioMessage('command.action.exportPng'), description: studioMessage('command.action.exportPngDescription'), disabled: !currentProjectValid || !currentVisual?.output.svg, icon: 'export-diagram' },
		{ id: 'export-pdf', label: studioMessage('command.action.exportPdf'), description: studioMessage('command.action.exportPdfDescription'), disabled: !currentProjectValid || !currentVisual?.output.svg, icon: 'export-diagram' },
		{ id: 'export-project', label: studioMessage('command.action.exportProject'), description: studioMessage('command.action.exportProjectDescription'), disabled: !workspace.initialized || !activeProject, icon: 'export-project' },
		{ id: 'import-project', label: studioMessage('command.action.importProject'), description: studioMessage('command.action.importProjectDescription'), disabled: !workspace.initialized, icon: 'import-project' },
		{ id: 'open-settings', label: studioMessage('command.action.openSettings'), description: studioMessage('command.action.openSettingsDescription'), icon: 'settings' },
		{ id: 'show-editor', label: studioMessage('command.action.showEditor'), description: studioMessage('command.action.showEditorDescription'), icon: 'editor' },
		{ id: 'show-preview', label: studioMessage('command.action.showPreview'), description: studioMessage('command.action.showPreviewDescription'), icon: 'preview' }
	]);
	const previewVisual = $derived(currentVisual?.output.svg && currentVisual.output.scene ? currentVisual : lastGoodVisual);
	const currentPreviewScenarios = $derived(
		currentVisual?.output.svg && currentVisual.output.scene
			? (currentVisual.output.scenarios as CompiledScenario[] | undefined) ?? []
			: []
	);
	const previewScenarios = $derived.by(() => {
		const currentIds = new Set(currentPreviewScenarios.map(({ id }) => id));
		const lastValidScenarios = (lastGoodVisual?.output.scenarios as CompiledScenario[] | undefined) ?? [];
		return [
			...currentPreviewScenarios,
			...lastValidScenarios.filter(({ id }) => !currentIds.has(id))
		];
	});
	const selectedScenarioPreference = $derived.by(() => {
		if (!activeProject || !visualId) return { present: false, id: null as string | null };
		const byVisual = workspace.selectedScenarioByProject[activeProject.id];
		return {
			present: Object.hasOwn(byVisual ?? {}, visualId),
			id: byVisual?.[visualId] ?? null
		};
	});
	const requestedScenarioId = $derived(
		selectedScenarioPreference.present
			? selectedScenarioPreference.id
			: activeFile?.path.startsWith('scenarios/')
				? currentFileCompilation?.declarations.find(({ kind }) => kind === 'scenario')?.id ?? null
				: null
	);
	const selectedScenario = $derived(
		requestedScenarioId ? previewScenarios.find(({ id }) => id === requestedScenarioId) ?? null : null
	);
	const selectedScenarioIsStale = $derived(
		Boolean(selectedScenario && !currentPreviewScenarios.some(({ id }) => id === selectedScenario.id))
	);
	const previewSources = $derived.by(() => {
		if (selectedScenarioIsStale && selectedScenario && lastValidForProject) {
			const previousFile = lastGoodFiles.find(({ declarations }) => declarations.some(
				({ kind, id }) => kind === 'scenario' && id === selectedScenario.id
			));
			const previousSource = lastValidForProject.sources.find(({ id }) => id === previousFile?.id);
			if (previousSource) {
				return sourceFiles.map((source) => source.id === previousSource.id ? previousSource : source);
			}
		}
		return previewVisual === currentVisual ? sourceFiles : lastValidForProject?.sources ?? sourceFiles;
	});
	const currentResourceDeclaration = $derived.by(() => {
		const kind = activeFile?.path.startsWith('visuals/') ? 'visual' : activeFile?.path.startsWith('scenarios/') ? 'scenario' : null;
		if (!kind || !currentFileCompilation) return null;
		const declarations = currentFileCompilation.declarations.filter((item) => item.kind === kind);
		return kind === 'scenario'
			? declarations.find(({ id }) => id === selectedScenario?.id) ?? declarations[0] ?? null
			: declarations[0] ?? null;
	});
	const workspaceTabViews = $derived(buildWorkspaceTabViews({
		files: activeProject?.files ?? [],
		compilation: currentFileViews,
		tabIds: workspaceTabIds,
		activeFileId,
		currentDeclaration: currentResourceDeclaration,
		projectFileCount: activeProject?.files.length ?? 0,
		initialized: workspace.initialized,
		compiling: compilePending,
		projectValid: currentProjectValid,
		onRename: (fileId, declaration) => resourceActions.openResourceRenameForFile(fileId, declaration),
		onRemove: (fileId, kind, declaration) => resourceActions.openResourceRemovalForFile(fileId, kind, declaration)
	}));
	const fileDiagnostics = $derived(
		latestActiveFileCompilation ? readEditorDiagnostics(latestActiveFileCompilation.diagnostics) : []
	);
	const linkedVisualCompilation = $derived(
		currentFileViews.find((file) => file.kind === 'visual' && file.visual_id === visualId) ?? null
	);
	const linkedVisualDiagnostics = $derived(
		currentFileCompilation?.kind === 'scenario' && linkedVisualCompilation
			? readEditorDiagnostics(linkedVisualCompilation.diagnostics)
			: []
	);
	const errorCount = $derived(fileDiagnostics.filter(({ severity }) => severity === 'error').length + linkedVisualDiagnostics.filter(({ severity }) => severity === 'error').length);
	const previewScene = $derived(readPreviewScene(previewVisual?.output.scene));
	const resolvedTheme = $derived(resolveStudioTheme(themePreferences.studio, systemPrefersDark));
	const editorPalette = $derived(editorThemeStyles(resolveEditorTheme(themePreferences.editor, resolvedTheme)));
	const sourceEditorPreferences = $derived(editorPreferences);
	const workspaceEditor = $derived(activeFile ? {
		projectId: activeProject?.id,
		documentId: activeFile.id,
		disabled: !workspace.initialized,
		navigation: editorNavigation,
		source: activeFile.source,
		onChange: changeSource,
		tokens: latestActiveFileCompilation?.tokens.length
			? readEditorSyntaxTokens(latestActiveFileCompilation.tokens)
			: undefined,
		diagnostics: latestActiveFileCompilation ? fileDiagnostics : undefined,
		editorSymbols: latestActiveFileCompilation?.editor_symbols ?? [],
		refactorDiagnostics: currentRefactorDiagnostics,
		refactorSymbols: currentRefactorSymbols,
		quickFixDisabled: compilePending || quickFixPending || !currentCompilation,
		nodeIds: previewScene?.nodes.map(({ id }) => id) ?? [],
		label: studioMessage('editor.label', { path: activeFile.path }),
		preferences: sourceEditorPreferences,
		onSave: saveWorkspace,
		onNavigate: navigateToSymbol,
		onRenameSymbol: (offset: number) => editorRenameActions.open(offset),
		onQuickFix: applyEditorQuickFix,
		onNavigationHandled: handleEditorNavigation
	} : null);
	const workspaceDiagnostics = $derived({
		activeDiagnostics: fileDiagnostics,
		activeFileId,
		linkedDiagnostics: linkedVisualDiagnostics,
		linkedFileId: linkedVisualCompilation?.id ?? null,
		compileError,
		errorCount,
		onNavigate: navigateToDiagnostic
	});
	const showLastValidPreview = $derived(Boolean(
		!compilePending && ((lastGoodVisual && previewVisual !== currentVisual) || selectedScenarioIsStale)
	));
	const workspacePreview = $derived(
		previewVisual?.output.svg && previewVisual.output.scene ? {
			svg: previewVisual.output.svg,
			scene: previewVisual.output.scene,
			scenario: selectedScenario,
			sources: previewSources,
			visualId,
			scenarios: previewScenarios,
			stale: showLastValidPreview,
			onScenarioSelect: selectScenario
		} : null
	);
	const workspacePreviewStats = $derived(previewVisual?.output.stats ? {
		nodes: previewVisual.output.stats.nodes,
		links: previewVisual.output.stats.edges,
		groups: previewVisual.output.stats.groups
	} : null);
	const workspaceActions = {
		togglePaneFocus,
		startPaneResize,
		movePaneResize,
		endPaneResize,
		paneResizeKeydown: handlePaneResizeKeydown,
		resetPaneResize: () => (editorPaneRatio = 50),
		adjustEditorZoom,
		resetEditorZoom
	};
	const resourceActions = createResourceActions({
		controller,
		form: resourceForm,
		getSnapshot: () => ({
			workspace,
			activeProject,
			currentFiles,
			currentCompilation,
			currentVisual,
			selectedScenario,
			sourceFiles,
			visualId,
			compilePending,
			currentProjectValid
		}),
		setFileMessage: (message) => (fileMessage = message),
		setFocusedPane: (pane) => (focusedPane = pane)
	});
	const sourceActions = createSourceActions({
		controller,
		state: sourceActionState,
		getSnapshot: () => ({ workspace, activeProject, activeFile }),
		getSourceInput: () => sourceInput,
		setMessage: (message) => (fileMessage = message),
		setFocusedPane: (pane) => (focusedPane = pane)
	});
	const editorRenameActions = createEditorSymbolRenameActions({
		controller,
		state: editorRename,
		getSnapshot: () => ({ workspace, activeProject, activeFile, compiledFiles: currentFiles }),
		snapshotIsCurrent: projectSnapshotMatches,
		setFileMessage: (message) => (fileMessage = message),
		navigate: (fileId, offset) => {
			editorNavigation = { fileId, offset, requestId: ++navigationSequence };
		},
		focusEditor: async () => {
			await tick();
			symbolRenameInput?.focus();
			symbolRenameInput?.select();
		}
	});

	$effect(() => {
		const projectId = workspace.activeProjectId;
		const key = sourcesKey;
		if (!workspace.initialized || !projectId) return;
		const sources = untrack(() => {
			const project = workspace.projects.find(({ id }) => id === projectId);
			return project?.files.map(({ id, path, source }) => ({ id, path, source })) ?? [];
		});
		const sequence = ++compileSequence;
		let cancelled = false;
		const abortController = new AbortController();
		compilePending = true;
		compileError = null;
		const timer = window.setTimeout(() => {
			void compileProject(sources, {
				signal: abortController.signal,
				queueKey: 'studio-live-project-compile'
			}).then((result) => {
				if (cancelled || sequence !== compileSequence) return;
				compilation = { projectId, sourcesKey: key, result };
				compilePending = false;
				compileError = result.error;
				initialCompilationSettled = true;
				if (isValidProjectCompilation(result)) lastValid = { projectId, sourcesKey: key, sources, result };
			}, (error: unknown) => {
				if (cancelled || sequence !== compileSequence) return;
				compilePending = false;
				compileError = error instanceof Error ? error.message : studioMessage('project.compileFailed');
				initialCompilationSettled = true;
			});
		}, 90);
		return () => {
			cancelled = true;
			window.clearTimeout(timer);
			abortController.abort();
		};
	});

	onMount(() => {
		const stop = controller.subscribe((state) => { workspace = state; });
		themePreferences = readThemePreferences();
		editorPreferences = readEditorPreferences();
		const themeQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const paneLayoutQuery = window.matchMedia('(min-width: 1024px)');
		const syncTheme = () => { systemPrefersDark = themeQuery.matches; };
		const syncPaneLayout = () => { paneLayoutWide = paneLayoutQuery.matches; };
		const flushWhenHidden = () => {
			if (document.visibilityState === 'hidden') void controller.flush().catch(() => undefined);
		};
		const flushOnPageHide = () => { void controller.flush().catch(() => undefined); };
		const warnBeforeUnload = (event: BeforeUnloadEvent) => {
			if (!shouldWarnBeforeUnload(controller.getState())) return;
			void controller.flush().catch(() => undefined);
			event.preventDefault();
			event.returnValue = '';
		};
		syncTheme();
		syncPaneLayout();
		themeQuery.addEventListener('change', syncTheme);
		paneLayoutQuery.addEventListener('change', syncPaneLayout);
		document.addEventListener('visibilitychange', flushWhenHidden);
		window.addEventListener('pagehide', flushOnPageHide);
		window.addEventListener('beforeunload', warnBeforeUnload);
		void controller.initialize();
		return () => {
			stop();
			themeQuery.removeEventListener('change', syncTheme);
			paneLayoutQuery.removeEventListener('change', syncPaneLayout);
			document.removeEventListener('visibilitychange', flushWhenHidden);
			window.removeEventListener('pagehide', flushOnPageHide);
			window.removeEventListener('beforeunload', warnBeforeUnload);
			void controller.dispose();
		};
	});

	function changeSource(value: string) {
		if (activeProject && activeFile) controller.updateSource(activeProject.id, activeFile.id, value);
	}

	function openFile(fileId: string) {
		if (!activeProject) return;
		editorNavigation = null;
		controller.selectFile(activeProject.id, fileId);
		const file = activeProject.files.find(({ id }) => id === fileId);
		const compiled = currentFiles.find(({ id }) => id === fileId);
		if (file?.path.startsWith('scenarios/') && compiled?.visual_id) {
			const scenario = (compiled.output?.scenarios as CompiledScenario[] | undefined)?.[0];
			if (scenario) controller.selectScenario(activeProject.id, compiled.visual_id, scenario.id);
		}
		fileMessage = null;
		focusedPane = null;
	}

	function selectSidebarScenario(fileId: string, selectedVisualId: string, scenarioId: string) {
		if (!activeProject) return;
		openFile(fileId);
		controller.selectScenario(activeProject.id, selectedVisualId, scenarioId);
		focusedPane = null;
	}

	function selectPaletteFile(projectId: string, fileId: string) {
		const result = controller.selectFile(projectId, fileId);
		if (!result.ok) {
			fileMessage = { kind: 'error', text: result.error.message };
			return;
		}
		editorNavigation = null;
		fileMessage = null;
		focusedPane = null;
	}

	function selectPaletteProject(projectId: string) {
		const result = controller.selectProject(projectId);
		if (!result.ok) {
			fileMessage = { kind: 'error', text: result.error.message };
			return;
		}
		editorNavigation = null;
		fileMessage = null;
		focusedPane = null;
	}

	function runPaletteAction(actionId: string) {
		switch (actionId) {
			case 'language-reference': languageReferenceOpen = true; break;
			case 'new-project': resourceActions.openProjectCreation(); break;
			case 'rename-project': resourceActions.openProjectRename(); break;
			case 'new-visual': resourceActions.openResourceCreation('visual'); break;
			case 'new-scenario': resourceActions.openResourceCreation('scenario'); break;
			case 'open-source': sourceActions.openSource(); break;
			case 'save-workspace': void saveWorkspace(); break;
			case 'save-source': void sourceActions.saveSource(); break;
			case 'format-source': void formatActiveSource(); break;
			case 'export-svg': void workspacePanels?.exportDiagram('svg'); break;
			case 'export-png': void workspacePanels?.exportDiagram('png'); break;
			case 'export-pdf': void workspacePanels?.exportDiagram('pdf'); break;
			case 'export-project': sourceActions.exportProject(); break;
			case 'import-project': importInput?.click(); break;
			case 'open-settings': settingsOpen = true; break;
			case 'show-editor': focusedPane = 'editor'; break;
			case 'show-preview': focusedPane = 'preview'; break;
		}
	}

	async function formatActiveSource() {
		fileMessage = null;
		try {
			const result = await workspacePanels?.formatSource();
			if (!result || result === 'stale') return;
			fileMessage = {
				kind: 'success',
				text: studioMessage(result === 'formatted' ? 'command.sourceFormatted' : 'command.sourceAlreadyFormatted')
			};
		} catch {
			fileMessage = { kind: 'error', text: studioMessage('command.sourceFormatFailed') };
		}
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (event.defaultPrevented || event.isComposing || !workspace.initialized || settingsOpen || languageReferenceOpen || resourceForm.kind || editorRename.open) return;
		if (!event.metaKey && !event.ctrlKey && event.altKey && event.shiftKey && event.key.toLowerCase() === 'f') {
			event.preventDefault();
			commandPaletteOpen = false;
			void formatActiveSource();
			return;
		}
		if ((event.metaKey || event.ctrlKey) && !event.altKey && !event.shiftKey && !event.repeat) {
			if (event.key.toLowerCase() === 'o' && sourceActionState.operation === null) {
				event.preventDefault();
				commandPaletteOpen = false;
				sourceActions.openSource();
				return;
			}
			if (event.key.toLowerCase() === 's' && activeFile) {
				event.preventDefault();
				commandPaletteOpen = false;
				void saveWorkspace();
				return;
			}
		}
		if (
			(event.metaKey || event.ctrlKey) &&
			!event.altKey &&
			!event.shiftKey &&
			!event.repeat &&
			event.key.toLowerCase() === 'k' &&
			!commandPaletteOpen &&
			!settingsOpen &&
			!resourceForm.kind &&
			!editorRename.open
		) {
			event.preventDefault();
			event.stopPropagation();
			commandPaletteOpen = true;
		}
	}

	function navigateToSymbol(reference: EditorSymbol) {
		const project = activeProject;
		if (!project || !currentCompilation) {
			fileMessage = { kind: 'error', text: studioMessage('editor.projectCompiling') };
			return;
		}
		const definition = findEditorSymbolDefinition(currentFiles, reference);
		const file = definition && project.files.find((candidate) => candidate.id === definition.fileId);
		if (
			!definition ||
			!file ||
			file.path !== definition.path ||
			file.source.slice(definition.symbol.span.start.offset, definition.symbol.span.end.offset) !== definition.symbol.name
		) {
			fileMessage = { kind: 'error', text: studioMessage('editor.symbolDefinitionMissing') };
			return;
		}
		controller.selectFile(project.id, file.id);
		focusedPane = null;
		fileMessage = null;
		editorNavigation = {
			fileId: file.id,
			offset: definition.symbol.span.start.offset,
			requestId: ++navigationSequence
		};
	}

	function handleEditorNavigation(requestId: number) {
		if (editorNavigation?.requestId === requestId) editorNavigation = null;
	}

	function navigateToDiagnostic(fileId: string, diagnostic: ReturnType<typeof readEditorDiagnostics>[number]) {
		if (!activeProject) return;
		const source = activeProject.files.find((file) => file.id === fileId)?.source;
		if (source === undefined) return;
		controller.selectFile(activeProject.id, fileId);
		focusedPane = 'editor';
		fileMessage = null;
		editorNavigation = {
			fileId,
			offset: editorOffsetAtPosition(source, diagnostic.span.start),
			requestId: ++navigationSequence
		};
	}

	function startPaneResize(event: PointerEvent) {
		if (event.button !== 0 || !event.isPrimary || focusedPane) return;
		const bounds = (event.currentTarget as HTMLElement).parentElement?.getBoundingClientRect();
		if (!bounds) return;
		event.preventDefault();
		paneResizeOrigin = {
			coordinate: paneLayoutWide ? event.clientX : event.clientY,
			ratio: editorPaneRatio,
			size: paneLayoutWide ? bounds.width : bounds.height,
			pointerId: event.pointerId
		};
		(event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
	}

	function movePaneResize(event: PointerEvent) {
		if (!paneResizeOrigin || event.pointerId !== paneResizeOrigin.pointerId) return;
		const coordinate = paneLayoutWide ? event.clientX : event.clientY;
		const delta = ((coordinate - paneResizeOrigin.coordinate) / paneResizeOrigin.size) * 100;
		editorPaneRatio = Math.max(25, Math.min(75, paneResizeOrigin.ratio + delta));
	}

	function endPaneResize(event: PointerEvent) {
		if (!paneResizeOrigin || event.pointerId !== paneResizeOrigin.pointerId) return;
		paneResizeOrigin = null;
	}

	function handlePaneResizeKeydown(event: KeyboardEvent) {
		const step = event.shiftKey ? 10 : 4;
		const decreaseKey = paneLayoutWide ? 'ArrowLeft' : 'ArrowUp';
		const increaseKey = paneLayoutWide ? 'ArrowRight' : 'ArrowDown';
		if (event.key === 'Home') editorPaneRatio = 25;
		else if (event.key === 'End') editorPaneRatio = 75;
		else if (event.key === decreaseKey) editorPaneRatio = Math.max(25, editorPaneRatio - step);
		else if (event.key === increaseKey) editorPaneRatio = Math.min(75, editorPaneRatio + step);
		else return;
		event.preventDefault();
	}

	function togglePaneFocus(pane: 'editor' | 'preview') {
		focusedPane = focusedPane === pane ? null : pane;
	}

	async function applyEditorQuickFix(diagnostic: RefactorDiagnostic, replacement: string) {
		const project = activeProject;
		const file = activeFile;
		if (!project || !file || !currentCompilation || compilePending || quickFixPending) return;
		if (diagnostic.path !== file.path) return;
		const target = diagnosticQuickFixEdit(file.source, currentRefactorSymbols, diagnostic, replacement);
		if (!target) return;

		const sources = project.files.map(({ id, path, source }) => ({ id, path, source }));
		const snapshotKey = makeSourcesKey(sources);
		const beforeFiles = currentRefactorFiles;
		const requestProjectId = project.id;
		const requestFileId = file.id;
		quickFixPending = true;
		fileMessage = null;
		try {
			const candidateSources = sources.map((source) => source.id === file.id
				? { ...source, source: source.source.slice(0, target.start) + target.text + source.source.slice(target.end) }
				: source);
			const validation = await compileProject(candidateSources);
			if (validation.error) throw new Error(validation.error);
			const afterFiles = validation.files.map((candidate) => ({
				path: candidate.path,
				diagnostics: readRefactorDiagnostics(candidate.diagnostics, candidate.path)
			}));
			if (!isSafeQuickFix(file.path, diagnostic, diagnosticCounts(beforeFiles), afterFiles)) {
				throw new Error(studioMessage('editor.quickFixNotSafe'));
			}
			if (
				workspace.activeProjectId !== requestProjectId ||
				workspace.selectedFileByProject[requestProjectId] !== requestFileId ||
				!projectSnapshotMatches(requestProjectId, snapshotKey)
			) throw new Error(studioMessage('editor.projectChangedQuickFix'));

			const updated = candidateSources.find(({ id }) => id === requestFileId);
			if (!updated) throw new Error(studioMessage('editor.quickFixSourceMissing'));
			const result = controller.updateSource(requestProjectId, requestFileId, updated.source);
			if (!result.ok) throw new Error(result.error.message);
		} catch (error) {
			fileMessage = { kind: 'error', text: error instanceof Error ? error.message : studioMessage('editor.quickFixFailed') };
		} finally {
			quickFixPending = false;
		}
	}

	function selectScenario(scenarioId: string | null) {
		if (!activeProject || !visualId) return;
		controller.selectScenario(activeProject.id, visualId, scenarioId);
	}

	function updateThemePreferences(value: ThemePreferences) {
		themePreferences = value;
		writeThemePreferences(value);
	}

	function updateEditorPreferences(value: EditorPreferences) {
		editorPreferences = value;
		writeEditorPreferences(value);
	}

	function adjustEditorZoom(direction: -1 | 1) {
		updateEditorPreferences({
			...editorPreferences,
			zoom: stepEditorZoom(editorPreferences.zoom, direction)
		});
	}

	function resetEditorZoom() {
		updateEditorPreferences({ ...editorPreferences, zoom: DEFAULT_EDITOR_PREFERENCES.zoom });
	}

	async function saveWorkspace(
		source = activeFile?.source ?? '',
		context?: { projectId: string | null; fileId: string | null }
	) {
		const projectId = context?.projectId ?? activeProject?.id;
		const fileId = context?.fileId ?? activeFile?.id;
		if (!projectId || !fileId) return;
		const project = workspace.projects.find(({ id }) => id === projectId);
		if (!project?.files.some(({ id }) => id === fileId)) {
			if (workspace.activeProjectId === projectId) {
				fileMessage = { kind: 'error', text: studioMessage('project.saveChanged') };
			}
			return;
		}
		try {
			if (workspace.activeProjectId === projectId && activeFileId === fileId) {
				controller.updateSource(projectId, fileId, source);
			}
			const saved = await controller.flush();
			if (saved.ok) controller.markWorkspaceSourceSaved(projectId, fileId, source);
			if (workspace.activeProjectId === projectId) {
				fileMessage = saved.ok ? null : { kind: 'error', text: saved.error.message };
			}
		} catch (error) {
			if (workspace.activeProjectId === projectId) {
				fileMessage = { kind: 'error', text: error instanceof Error ? error.message : studioMessage('project.workspaceSaveFailed') };
			}
		}
	}

	function prettySaveError() {
		return workspace.restoreError?.message ?? workspace.error?.message ?? studioMessage('project.storageSaveFailed');
	}

	function retryWorkspaceSave() {
		void controller.flush();
	}

	function projectSnapshotMatches(projectId: string, snapshotKey: string) {
		const project = workspace.projects.find((candidate) => candidate.id === projectId);
		return Boolean(project && makeSourcesKey(project.files.map(({ id, path, source }) => ({ id, path, source }))) === snapshotKey);
	}

</script>

<svelte:head>
	<title>Schemra Studio</title>
	<meta name="description" content="A local workspace for authoring and previewing Schemra diagrams." />
	<link rel="icon" href={studioBrandAssets.mark} />
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} />



{#if !isInitialWorkspaceViewReady(workspace.initialized, activeProject !== null, initialCompilationSettled)}
	<WorkspaceLoading theme={resolvedTheme} reduceMotion={themePreferences.reduceMotion} />
{:else}
<main class={`flex h-dvh w-full min-h-0 min-w-0 overflow-hidden bg-background text-foreground ${resolvedTheme} ${themePreferences.reduceMotion ? '[&_*]:!animate-none [&_*]:!scroll-auto [&_*]:!transition-none' : ''}`}>
	<ProjectSidebar
		wordmark={studioBrandAssets.wordmark}
		mark={studioBrandAssets.mark}
		reduceMotion={themePreferences.reduceMotion}
		primaryVisuals={sidebarModel.primaryVisuals}
		exampleVisuals={sidebarModel.exampleVisuals}
		sources={sidebarOtherSources}
		resourcesDisabled={!workspace.initialized}
		creationDisabled={resourceCreationDisabled}
		getResourceMenuPolicy={getSidebarResourceMenuPolicy}
		activeFileId={activeFileId}
		activeVisualId={visualId}
		selectedScenarioId={selectedScenario?.id ?? null}
		onSelectFile={openFile}
		onSelectScenario={selectSidebarScenario}
		onCreateVisual={() => resourceActions.openResourceCreation('visual')}
		onRenameResource={renameSidebarResource}
		onRemoveResource={removeSidebarResource}
		onOpenSettings={() => (settingsOpen = true)}
	/>
	<div class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
	<StudioHeader
		projects={workspace.projects}
		activeProjectId={activeProject?.id ?? null}
		activeProjectName={activeProject?.name ?? null}
		activeProjectDirty={Boolean(activeProject?.files.some((file) => file.source !== file.savedSource))}
		{activeFileId}
		activeVisualId={visualId}
		visuals={sidebarVisuals}
		otherSources={sidebarOtherSources}
		initialized={workspace.initialized}
		creationDisabled={resourceCreationDisabled}
		compileStatus={compilePending ? 'compiling' : compileError ? 'error' : currentVisual?.output.svg ? 'ready' : 'attention'}
		{commandPaletteOpen}
		sourceOperation={sourceActionState.operation}
		canSaveSource={Boolean(workspace.initialized && activeFile)}
		canExportDiagram={Boolean(previewVisual?.output.svg)}
		onSelectProject={(projectId) => {
			editorNavigation = null;
			controller.selectProject(projectId);
		}}
		onCreateProject={resourceActions.openProjectCreation}
		onRenameProject={resourceActions.openProjectRename}
		onImportProject={() => importInput?.click()}
		onExportProject={sourceActions.exportProject}
		onOpenSearch={() => (commandPaletteOpen = true)}
		onSelectFile={openFile}
		onCreateVisual={() => resourceActions.openResourceCreation('visual')}
		onOpenSettings={() => (settingsOpen = true)}
		onOpenSource={sourceActions.openSource}
		onSaveSource={() => void sourceActions.saveSource()}
		onExportDiagram={(format) => void workspacePanels?.exportDiagram(format)}
	/>
	<input bind:this={importInput} class="sr-only" type="file" accept="application/json,.json" aria-label={studioMessage('file.chooseProjectBundle')} onchange={sourceActions.importProject} />
	<input bind:this={sourceInput} class="sr-only" type="file" accept=".schemra,text/plain" aria-label={studioMessage('file.chooseSource')} onchange={sourceActions.openUpload} oncancel={sourceActions.cancelUpload} />

	<nav class="flex h-11 min-h-11 flex-[0_0_auto] items-stretch justify-between gap-2 border-b border-border bg-background px-3 sm:px-6" aria-label={studioMessage('tabs.projectSourceFiles')}>
		<WorkspaceTabs tabs={workspaceTabViews} activeTabId={activeFileId} initialized={workspace.initialized} onSelect={openFile} />
		<Button variant="ghost" size="xs" class="h-8 w-8 shrink-0 self-center justify-center gap-0 rounded-md p-0 text-primary hover:bg-accent hover:text-foreground disabled:cursor-not-allowed disabled:opacity-50 lg:w-auto lg:gap-2 lg:px-[.65rem]" type="button" aria-label={studioMessage('resource.newScenario')} title={studioMessage('resource.newScenario')} onclick={() => resourceActions.openResourceCreation('scenario')} disabled={resourceCreationDisabled || !currentProjectValid || !currentVisual}>
			<Plus size={16} class="size-4" aria-hidden="true" /><span class="hidden lg:inline">{studioMessage('resource.newScenario')}</span>
		</Button>
	</nav>

	{#if workspace.restoreError || (workspace.error && (workspace.saveStatus === 'error' || workspace.saveStatus === 'conflict'))}
		<div class="fixed right-4 top-4 z-[1000] flex max-h-[calc(100dvh-2rem)] w-[min(32rem,calc(100vw-2rem))] flex-col gap-3 overflow-auto rounded-lg border border-destructive/35 bg-popover/90 p-3 text-xs text-destructive shadow-lg backdrop-blur-sm" role="alert" aria-live="assertive">
			<div class="flex min-w-0 items-start gap-2">
				<AlertCircle size={15} class="mt-0.5 size-[15px] shrink-0" aria-hidden="true" />
				<div class="flex min-w-0 flex-wrap items-baseline gap-x-2 gap-y-1">
				<strong>{workspace.restoreError ? studioMessage('workspace.restoreFailed') : workspace.saveStatus === 'conflict' ? studioMessage('workspace.conflict') : studioMessage('workspace.saveFailed')}</strong>
				<span>{prettySaveError()}</span>
				</div>
			</div>
			<div class="flex flex-wrap gap-[.4rem]">
				{#if !workspace.restoreError && workspace.saveStatus === 'error'}
					<Button variant="outline" size="xs" class="min-h-8 gap-[.35rem] border-current/30 px-[.55rem] text-[.72rem] text-current hover:bg-current/[.08] hover:text-current disabled:cursor-not-allowed disabled:opacity-50" type="button" disabled={!activeProject || !workspace.initialized} onclick={retryWorkspaceSave}><Save size={14} aria-hidden="true" />{studioMessage('workspace.retrySave')}</Button>
				{/if}
				<Button variant="outline" size="xs" class="min-h-8 gap-[.35rem] border-current/30 px-[.55rem] text-[.72rem] text-current hover:bg-current/[.08] hover:text-current disabled:cursor-not-allowed disabled:opacity-50" type="button" disabled={!activeProject || !workspace.initialized} onclick={sourceActions.exportProject}><Download size={14} aria-hidden="true" />{studioMessage('workspace.exportActiveProject')}</Button>
			</div>
		</div>
	{/if}
	<ResourceDialog
		kind={resourceForm.kind}
		targetKind={resourceForm.target?.kind ?? null}
		scenarioVisualTitle={sidebarVisuals.find(({ id }) => id === visualId)?.title ?? ''}
		bind:nameDraft={resourceForm.nameDraft}
		error={resourceForm.error}
		removeLabel={resourceActions.removalLabel(resourceForm.target)}
		onClose={resourceActions.close}
		onSave={() => void resourceActions.save()}
		onRemove={resourceActions.remove}
	/>
	<SymbolRenameDialog
		open={Boolean(editorRename.open && editorRename.target)}
		kind={editorRename.target?.selection.kind ?? ''}
		bind:draft={editorRename.draft}
		error={editorRename.error}
		pending={editorRename.pending}
		bind:inputElement={symbolRenameInput}
		onClose={editorRenameActions.close}
		onSave={() => void editorRenameActions.save()}
	/>

		<WorkspacePanels
			bind:this={workspacePanels}
			editor={workspaceEditor}
			diagnostics={workspaceDiagnostics}
			preview={workspacePreview}
			editorFileId={activeFileId}
			focusedPane={focusedPane}
			{editorPaneRatio}
			{paneLayoutWide}
			stats={workspacePreviewStats}
			editorZoom={editorPreferences.zoom}
			{compilePending}
			{showLastValidPreview}
			editorPalette={editorPalette}
			actions={workspaceActions}
		/>

		<StudioSettingsDialog
			bind:open={settingsOpen}
			themePreferences={themePreferences}
			editorPreferences={editorPreferences}
			onThemePreferencesChange={updateThemePreferences}
			onEditorPreferencesChange={updateEditorPreferences}
		/>

	<CommandPalette
		bind:open={commandPaletteOpen}
		projects={workspace.projects}
		activeProjectId={activeProject?.id ?? null}
		activeFileId={activeFileId}
		openTabs={paletteTabs}
		actions={paletteActions}
		onSelectProjectFile={selectPaletteFile}
		onSelectProject={selectPaletteProject}
		onAction={runPaletteAction}
	/>
	<LanguageReferenceDialog bind:open={languageReferenceOpen} />
	<StudioToast message={fileMessage} onDismiss={() => (fileMessage = null)} />
	</div>
</main>
{/if}
