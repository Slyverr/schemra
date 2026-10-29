<script lang="ts">
	import { onMount, tick, untrack } from 'svelte';
	import {
		AlertCircle,
		Check,
		ChevronDown,
		Download,
		FileCode2,
		FolderOpen,
		LoaderCircle,
		Pencil,
		Plus,
		Search,
		Save,
		Settings2,
		Trash2,
		Upload,
		X
	} from '@lucide/svelte';
	import DiagramPreview from '../preview/DiagramPreview.svelte';
	import { readEditorDiagnostics, readEditorSyntaxTokens } from '../editor/source-model.js';
	import { applySourceTextEdits, removeSourceDeclaration, utf8SpanToEditorSpan } from '../editor/source-edits.js';
	import {
		diagnosticCounts,
		diagnosticQuickFixEdit,
		editorSymbolForRefactor,
		isSafeQuickFix,
		readRefactorDiagnostics,
		type RefactorDiagnostic,
		type RefactorFile
	} from '../editor/editor-refactors.js';
	import { findEditorSymbolDefinition, findEditorSymbolDefinitionAtOffset } from '../editor/symbol-navigation.js';
	import SourceEditor from '../editor/SourceEditor.svelte';
	import {
		createWorkspaceController,
		type WorkspaceControllerState
	} from '../project/workspace-controller.js';
import { shouldWarnBeforeUnload } from '../project/workspace-lifecycle.js';
	import { createExampleProject } from '../project/example-project.js';
	import {
		readEditorPreferences,
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
	import SettingsPanel from '../settings/SettingsPanel.svelte';
	import CommandPalette from './CommandPalette.svelte';
	import type { CommandPaletteAction } from './command-palette.js';
	import ResourceContextMenu from './ResourceContextMenu.svelte';
	import { resourceTabMenuPolicy, type ResourceTabKind } from './resource-tab-menu.js';
	import { studioBrandAssets } from './brand-assets.js';
	import {
		compileProject,
		renameProject as renameDslSymbol,
		type CompileResult,
		type EditorSymbol,
		type ProjectCompileResult,
		type ProjectSource,
		type RenameSelection,
		type SourceSpan
	} from '../schemra.js';
	import { readPreviewScene } from '../preview/model.js';
	import { normalizeResourceTitle, uniqueScenarioId } from '../project/session.js';
	import {
		openLocalSchemraSource,
		projectPathForLocalSource,
		readSchemraUpload,
		saveLocalSchemraSource,
		type LocalSchemraSource
	} from '../project/local-source.js';

	type ProjectCompileFile = {
		id: string;
		path: string;
		kind: string;
		visual_id: string | null;
		tokens: SourceToken[];
		diagnostics: unknown[];
		declarations: ProjectDeclaration[];
		editor_symbols?: EditorSymbol[];
		output: CompileResult | null;
	};
	type ProjectDeclaration = { kind: 'visual' | 'scenario' | string; id: string | null; title: string; visual_id: string | null; span: SourceSpan };
	type SourceToken = { kind: string; span: SourceSpan };

	type CompiledScenario = { id: string; title: string; duration_ms: number; tokens?: unknown[] };
	type ProjectVisual = { id: string; path: string; title: string; output: CompileResult };
	type LastValidCompile = { projectId: string; sourcesKey: string; sources: ProjectSource[]; result: ProjectCompileResult };
	type Surface = 'editor' | 'preview';
	type ResourceDialog = 'create-visual' | 'create-scenario' | 'rename-project' | 'rename-resource' | 'remove-resource' | null;
	type ResourceTarget = { projectId: string; fileId: string; kind: ResourceTabKind; declaration: ProjectDeclaration | null };
	type EditorRenameTarget = {
		projectId: string;
		fileId: string;
		path: string;
		sourcesKey: string;
		offset: number;
		invocationFileId: string;
		invocationOffset: number;
		selection: RenameSelection;
	};

	const controller = createWorkspaceController({ exampleProject: createExampleProject() });
	let workspace = $state<WorkspaceControllerState>(controller.getState());
	let compilation = $state<{ projectId: string; sourcesKey: string; result: ProjectCompileResult } | null>(null);
	let lastValid = $state<LastValidCompile | null>(null);
	let compilePending = $state(false);
	let compileError = $state<string | null>(null);
	let settingsOpen = $state(false);
	let commandPaletteOpen = $state(false);
	let activeSurface = $state<Surface>('editor');
	let themePreferences = $state<ThemePreferences>({ ...DEFAULT_THEME_PREFERENCES });
	let editorPreferences = $state<EditorPreferences>(readEditorPreferences());
	let systemPrefersDark = $state(false);
	let importInput = $state<HTMLInputElement>();
	let sourceInput = $state<HTMLInputElement>();
	let sourceOperation = $state<'opening' | 'saving' | null>(null);
	let pendingSourceProjectId: string | null = null;
	let fileMessage = $state<{ kind: 'error' | 'success'; text: string } | null>(null);
	let quickFixPending = $state(false);
	let resourceDialog = $state<ResourceDialog>(null);
	let resourceNameDraft = $state('');
	let resourceDialogError = $state('');
	let resourceTarget = $state<ResourceTarget | null>(null);
	let editorNavigation = $state<{ fileId: string; offset: number; requestId: number } | null>(null);
	let symbolRenameOpen = $state(false);
	let symbolRenameTarget = $state<EditorRenameTarget | null>(null);
	let symbolRenameDraft = $state('');
	let symbolRenameError = $state('');
	let symbolRenamePending = $state(false);
	let symbolRenameInput = $state<HTMLInputElement>();
	let compileSequence = 0;
	let symbolRenameRequestId = 0;
	let navigationSequence = 0;

	const activeProject = $derived(workspace.projects.find(({ id }) => id === workspace.activeProjectId) ?? null);
	const activeFileId = $derived(activeProject ? workspace.selectedFileByProject[activeProject.id] ?? activeProject.files[0]?.id ?? null : null);
	const activeFile = $derived(activeProject?.files.find(({ id }) => id === activeFileId) ?? null);
	const sourceFiles = $derived(activeProject?.files.map(({ id, path, source }) => ({ id, path, source })) ?? []);
	const sourcesKey = $derived(makeSourcesKey(sourceFiles));
	const orderedFiles = $derived(activeProject ? [...activeProject.files].sort(compareSourceFiles) : []);
	const paletteTabs = $derived(activeProject ? orderedFiles.map(({ id }) => ({ projectId: activeProject.id, fileId: id })) : []);
	const currentCompilation = $derived(
		compilation && compilation.projectId === activeProject?.id && compilation.sourcesKey === sourcesKey ? compilation.result : null
	);
	const currentFiles = $derived((currentCompilation?.files ?? []) as ProjectCompileFile[]);
	const currentFileCompilation = $derived(currentFiles.find(({ id }) => id === activeFileId) ?? null);
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
	const lastValidForProject = $derived(lastValid && lastValid.projectId === activeProject?.id ? lastValid : null);
	const lastGoodFiles = $derived((lastValidForProject ? lastValidForProject.result.files : []) as ProjectCompileFile[]);
	const lastGoodActiveFile = $derived(lastGoodFiles.find(({ id }) => id === activeFileId) ?? null);
	const visualId = $derived(
		currentFileCompilation?.visual_id ?? lastGoodActiveFile?.visual_id ??
		(currentCompilation?.visuals as ProjectVisual[] | undefined)?.[0]?.id ??
		(lastValidForProject ? (lastValidForProject.result.visuals as ProjectVisual[])[0]?.id : null) ?? null
	);
	const currentVisual = $derived(
		((currentCompilation?.visuals ?? []) as ProjectVisual[]).find(({ id }) => id === visualId) ?? null
	);
	const lastGoodVisual = $derived(
		(lastValidForProject ? (lastValidForProject.result.visuals as ProjectVisual[]) : []).find(({ id }) => id === visualId) ?? null
	);
	const currentProjectValid = $derived(currentCompilation ? isValidProjectCompilation(currentCompilation) : false);
	const paletteActions = $derived<CommandPaletteAction[]>([
		{ id: 'new-project', label: 'Create project', description: 'Start a new local project', keywords: ['project', 'new', 'create'] },
		{ id: 'rename-project', label: 'Rename project', description: activeProject?.name ?? 'Rename the active project', keywords: ['project', 'name'], disabled: !activeProject },
		{ id: 'new-visual', label: 'Create visual', description: 'Add a visual to the active project', keywords: ['diagram', 'visual', 'new'], disabled: !workspace.initialized || !activeProject },
		{ id: 'new-scenario', label: 'Create scenario', description: 'Add a scenario to the current visual', keywords: ['animation', 'scenario', 'new'], disabled: !workspace.initialized || !activeProject || !currentProjectValid || !currentVisual },
		{ id: 'open-source', label: 'Open source file', description: 'Open a local Schemra source file', keywords: ['browse', 'file', 'import'], disabled: !workspace.initialized || sourceOperation !== null },
		{ id: 'save-workspace', label: 'Save workspace', description: 'Save the current project in this browser', keywords: ['persist', 'save'], disabled: !workspace.initialized || !activeFile },
		{ id: 'save-source', label: 'Save source file', description: 'Save the current source to a local file', keywords: ['download', 'export', 'save'], disabled: !workspace.initialized || !activeFile || sourceOperation !== null },
		{ id: 'export-project', label: 'Export project', description: 'Download the active project bundle', keywords: ['backup', 'download', 'json'], disabled: !workspace.initialized || !activeProject },
		{ id: 'import-project', label: 'Import project', description: 'Load a project bundle', keywords: ['bundle', 'json', 'open'], disabled: !workspace.initialized },
		{ id: 'open-settings', label: 'Open settings', description: 'Configure appearance and editor behavior', keywords: ['preferences', 'theme', 'editor'] },
		{ id: 'show-editor', label: 'Show editor', description: 'Switch to the source editor', keywords: ['code', 'source'] },
		{ id: 'show-preview', label: 'Show preview', description: 'Switch to the diagram preview', keywords: ['diagram', 'visual'] }
	]);
	const previewVisual = $derived(currentProjectValid && currentVisual?.output.svg && currentVisual.output.scene ? currentVisual : lastGoodVisual);
	const previewSources = $derived(
		currentProjectValid && previewVisual === currentVisual ? sourceFiles :
		lastValidForProject ? lastValidForProject.sources : sourceFiles
	);
	const previewScenarios = $derived.by(() => {
		const current = currentVisual?.output.scenarios as CompiledScenario[] | undefined;
		const last = lastGoodVisual?.output.scenarios as CompiledScenario[] | undefined;
		const list = currentProjectValid && currentVisual?.output.svg && currentVisual.output.scene ? current : last;
		return list ?? [];
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
				? ((currentFileCompilation?.output?.scenarios as CompiledScenario[] | undefined)?.[0]?.id ?? previewScenarios[0]?.id ?? null)
				: previewScenarios[0]?.id ?? null
	);
	const selectedScenario = $derived(
		requestedScenarioId ? previewScenarios.find(({ id }) => id === requestedScenarioId) ?? previewScenarios[0] ?? null : null
	);
	const currentResourceDeclaration = $derived.by(() => {
		const kind = activeFile?.path.startsWith('visuals/') ? 'visual' : activeFile?.path.startsWith('scenarios/') ? 'scenario' : null;
		if (!kind || !currentFileCompilation) return null;
		const declarations = currentFileCompilation.declarations.filter((item) => item.kind === kind);
		return kind === 'scenario'
			? declarations.find(({ id }) => id === selectedScenario?.id) ?? declarations[0] ?? null
			: declarations[0] ?? null;
	});
	const fileDiagnostics = $derived(
		currentFileCompilation ? readEditorDiagnostics(currentFileCompilation.diagnostics) : []
	);
	const linkedVisualCompilation = $derived(
		currentFiles.find((file) => file.kind === 'visual' && file.visual_id === visualId) ?? null
	);
	const linkedVisualDiagnostics = $derived(
		currentFileCompilation?.kind === 'scenario' && linkedVisualCompilation
			? readEditorDiagnostics(linkedVisualCompilation.diagnostics)
			: []
	);
	const diagnosticCount = $derived(fileDiagnostics.length + linkedVisualDiagnostics.length);
	const errorCount = $derived(fileDiagnostics.filter(({ severity }) => severity === 'error').length + linkedVisualDiagnostics.filter(({ severity }) => severity === 'error').length);
	const previewScene = $derived(readPreviewScene(previewVisual?.output.scene));
	const hasFreshCompilation = $derived(currentCompilation !== null);
	const resolvedTheme = $derived(resolveStudioTheme(themePreferences.studio, systemPrefersDark));
	const editorPalette = $derived(editorThemeStyles(resolveEditorTheme(themePreferences.editor, resolvedTheme)));
	const sourceEditorPreferences = $derived(editorPreferences);
	const saveStatusLabel = $derived(
		workspace.saveStatus === 'saving' ? 'Saving' :
		workspace.saveStatus === 'dirty' ? 'Unsaved changes' :
		workspace.saveStatus === 'saved' ? 'Saved locally' :
		workspace.saveStatus === 'error' || workspace.saveStatus === 'conflict' ? 'Save issue' :
		workspace.saveStatus === 'loading' ? 'Restoring' : 'Ready'
	);

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
				if (isValidProjectCompilation(result)) lastValid = { projectId, sourcesKey: key, sources, result };
			}, (error: unknown) => {
				if (cancelled || sequence !== compileSequence) return;
				compilePending = false;
				compileError = error instanceof Error ? error.message : 'The project could not be compiled.';
			});
		}, 90);
		return () => {
			cancelled = true;
			window.clearTimeout(timer);
			abortController.abort();
		};
	});

	$effect(() => {
		if (!currentCompilation || !activeProject || !visualId || Object.hasOwn(workspace.selectedScenarioByProject[activeProject.id] ?? {}, visualId)) return;
		const scenarioId = activeFile?.path.startsWith('scenarios/')
			? ((currentFileCompilation?.output?.scenarios as CompiledScenario[] | undefined)?.[0]?.id ?? null)
			: previewScenarios[0]?.id ?? null;
		if (scenarioId) controller.selectScenario(activeProject.id, visualId, scenarioId);
	});

	onMount(() => {
		const stop = controller.subscribe((state) => { workspace = state; });
		themePreferences = readThemePreferences();
		editorPreferences = readEditorPreferences();
		const themeQuery = window.matchMedia('(prefers-color-scheme: dark)');
		const syncTheme = () => { systemPrefersDark = themeQuery.matches; };
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
		themeQuery.addEventListener('change', syncTheme);
		document.addEventListener('visibilitychange', flushWhenHidden);
		window.addEventListener('pagehide', flushOnPageHide);
		window.addEventListener('beforeunload', warnBeforeUnload);
		void controller.initialize();
		return () => {
			stop();
			themeQuery.removeEventListener('change', syncTheme);
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
		activeSurface = 'editor';
	}

	function selectPaletteFile(projectId: string, fileId: string) {
		const result = controller.selectFile(projectId, fileId);
		if (!result.ok) {
			fileMessage = { kind: 'error', text: result.error.message };
			return;
		}
		editorNavigation = null;
		fileMessage = null;
		activeSurface = 'editor';
	}

	function selectPaletteProject(projectId: string) {
		const result = controller.selectProject(projectId);
		if (!result.ok) {
			fileMessage = { kind: 'error', text: result.error.message };
			return;
		}
		editorNavigation = null;
		fileMessage = null;
		activeSurface = 'editor';
	}

	function runPaletteAction(actionId: string) {
		switch (actionId) {
			case 'new-project': createProject(); break;
			case 'rename-project': openProjectRename(); break;
			case 'new-visual': openResourceCreation('visual'); break;
			case 'new-scenario': openResourceCreation('scenario'); break;
			case 'open-source': openSourceFile(); break;
			case 'save-workspace': void saveWorkspace(); break;
			case 'save-source': void saveSourceFile(); break;
			case 'export-project': exportProject(); break;
			case 'import-project': importInput?.click(); break;
			case 'open-settings': settingsOpen = true; break;
			case 'show-editor': activeSurface = 'editor'; break;
			case 'show-preview': activeSurface = 'preview'; break;
		}
	}

	function handleWindowKeydown(event: KeyboardEvent) {
		if (
			(event.metaKey || event.ctrlKey) &&
			!event.altKey &&
			!event.shiftKey &&
			!event.repeat &&
			event.key.toLowerCase() === 'k' &&
			!commandPaletteOpen &&
			!settingsOpen &&
			!resourceDialog &&
			!symbolRenameOpen
		) {
			event.preventDefault();
			event.stopPropagation();
			commandPaletteOpen = true;
		}
	}

	function navigateToSymbol(reference: EditorSymbol) {
		const project = activeProject;
		if (!project || !currentCompilation) {
			fileMessage = { kind: 'error', text: 'Wait for the project to compile before navigating symbols.' };
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
			fileMessage = { kind: 'error', text: 'The referenced symbol has no current, unambiguous definition.' };
			return;
		}
		controller.selectFile(project.id, file.id);
		activeSurface = 'editor';
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
				throw new Error('This suggestion did not resolve the issue cleanly; no source was changed.');
			}
			if (
				workspace.activeProjectId !== requestProjectId ||
				workspace.selectedFileByProject[requestProjectId] !== requestFileId ||
				!projectSnapshotMatches(requestProjectId, snapshotKey)
			) throw new Error('The project changed while validating the suggestion. Try again.');

			const updated = candidateSources.find(({ id }) => id === requestFileId);
			if (!updated) throw new Error('The edited source is no longer available.');
			const result = controller.updateSource(requestProjectId, requestFileId, updated.source);
			if (!result.ok) throw new Error(result.error.message);
		} catch (error) {
			fileMessage = { kind: 'error', text: error instanceof Error ? error.message : 'Could not apply this diagnostic fix.' };
		} finally {
			quickFixPending = false;
		}
	}

	async function openEditorSymbolRename(offset: number) {
		const project = activeProject;
		const file = activeFile;
		if (!project || !file || symbolRenamePending) return;
		const definition = findEditorSymbolDefinitionAtOffset(currentFiles, file.id, offset);
		const definitionFile = definition && project.files.find((candidate) => candidate.id === definition.fileId);
		if (!definition || !definitionFile || definitionFile.path !== definition.path) {
			fileMessage = { kind: 'error', text: 'Place the cursor on a compiled symbol to rename it.' };
			return;
		}
		const sources = project.files.map(({ id, path, source }) => ({ id, path, source }));
		const snapshotKey = makeSourcesKey(sources);
		const requestId = ++symbolRenameRequestId;
		symbolRenamePending = true;
		symbolRenameError = '';
		fileMessage = null;
		try {
			const prepared = await renameDslSymbol(sources, definition.path, definition.symbol.span.start.offset);
			if (prepared.error || !prepared.selection) {
				throw new Error(prepared.error ?? 'No renameable symbol is at this position.');
			}
			if (
				requestId !== symbolRenameRequestId ||
				!projectSnapshotMatches(project.id, snapshotKey) ||
				workspace.activeProjectId !== project.id ||
				workspace.selectedFileByProject[project.id] !== file.id
			) throw new Error('The project changed while preparing the rename. Try again.');

			symbolRenameTarget = {
				projectId: project.id,
				fileId: definitionFile.id,
				path: definitionFile.path,
				sourcesKey: snapshotKey,
				offset: definition.symbol.span.start.offset,
				invocationFileId: file.id,
				invocationOffset: offset,
				selection: prepared.selection
			};
			symbolRenameDraft = prepared.selection.name;
			symbolRenamePending = false;
			symbolRenameOpen = true;
			await tick();
			symbolRenameInput?.focus();
			symbolRenameInput?.select();
		} catch (error) {
			if (requestId === symbolRenameRequestId) {
				fileMessage = {
					kind: 'error',
					text: error instanceof Error ? error.message : 'Could not prepare the symbol rename.'
				};
			}
		} finally {
			if (requestId === symbolRenameRequestId) symbolRenamePending = false;
		}
	}

	function closeEditorSymbolRename() {
		if (symbolRenamePending) return;
		const target = symbolRenameTarget;
		symbolRenameRequestId += 1;
		symbolRenameOpen = false;
		symbolRenameTarget = null;
		symbolRenameError = '';
		if (target) {
			editorNavigation = {
				fileId: target.invocationFileId,
				offset: target.invocationOffset,
				requestId: ++navigationSequence
			};
		}
	}

	async function saveEditorSymbolRename() {
		const target = symbolRenameTarget;
		const project = target && workspace.projects.find((candidate) => candidate.id === target.projectId);
		if (!target || !project || symbolRenamePending) return;
		const newName = symbolRenameDraft.trim();
		if (!newName) {
			symbolRenameError = 'Enter a valid identifier.';
			return;
		}
		if (newName === target.selection.name) {
			closeEditorSymbolRename();
			return;
		}

		const sources = project.files.map(({ id, path, source }) => ({ id, path, source }));
		const targetSource = sources.find((source) => source.id === target.fileId && source.path === target.path);
		if (
			workspace.activeProjectId !== target.projectId ||
			workspace.selectedFileByProject[target.projectId] !== target.invocationFileId ||
			makeSourcesKey(sources) !== target.sourcesKey ||
			!targetSource ||
			targetSource.source.slice(target.selection.span.start.offset, target.selection.span.end.offset) !== target.selection.name
		) {
			symbolRenameError = 'The project changed after rename was opened. Reopen rename and try again.';
			return;
		}

		symbolRenamePending = true;
		symbolRenameError = '';
		try {
			const plan = await renameDslSymbol(sources, target.path, target.offset, newName);
			if (plan.error || !plan.selection) throw new Error(plan.error ?? 'Could not safely rename this symbol.');
			if (
				plan.selection.name !== target.selection.name ||
				plan.selection.kind !== target.selection.kind ||
				plan.selection.span.start.offset !== target.selection.span.start.offset ||
				plan.selection.span.end.offset !== target.selection.span.end.offset
			) throw new Error('The selected symbol changed. Reopen rename and try again.');
			if (plan.edits.length === 0) throw new Error('The compiler returned no edits for this rename.');

			const updates = new Map<string, { path: string; source: string }>();
			let validatedEditCount = 0;
			for (const source of sources) {
				const edits = plan.edits.filter((edit) => edit.path === source.path);
				if (edits.length === 0) continue;
				validatedEditCount += edits.length;
				for (const edit of edits) {
					if (
						edit.replacement !== newName ||
						source.source.slice(edit.span.start.offset, edit.span.end.offset) !== target.selection.name
					) throw new Error('The rename plan no longer matches the project source.');
				}
				updates.set(source.id, { path: source.path, source: applySourceTextEdits(source.source, edits) });
			}
			if (updates.size === 0 || validatedEditCount !== plan.edits.length) {
				throw new Error('The compiler returned an invalid cross-file rename plan.');
			}
			if (
				!projectSnapshotMatches(target.projectId, target.sourcesKey) ||
				workspace.activeProjectId !== target.projectId ||
				workspace.selectedFileByProject[target.projectId] !== target.invocationFileId
			) throw new Error('The project changed while validating the rename. Reopen rename and try again.');

			for (const [fileId, update] of updates) {
				const result = controller.updateSource(target.projectId, fileId, update.source);
				if (!result.ok) throw new Error(result.error.message);
			}
			symbolRenameOpen = false;
			symbolRenameTarget = null;
			symbolRenameError = '';
			fileMessage = null;
			editorNavigation = {
				fileId: target.fileId,
				offset: plan.selection.span.start.offset,
				requestId: ++navigationSequence
			};
		} catch (error) {
			symbolRenameError = error instanceof Error ? error.message : 'Could not safely rename this symbol.';
		} finally {
			symbolRenamePending = false;
		}
	}

	function selectScenario(event: Event) {
		if (!activeProject || !visualId) return;
		const scenarioId = (event.currentTarget as HTMLSelectElement).value || null;
		controller.selectScenario(activeProject.id, visualId, scenarioId);
	}

	function selectProject(event: Event) {
		editorNavigation = null;
		controller.selectProject((event.currentTarget as HTMLSelectElement).value);
	}

	function openProjectRename() {
		if (!activeProject) return;
		resourceDialog = 'rename-project';
		resourceNameDraft = activeProject.name;
		resourceDialogError = '';
	}

	function openResourceCreation(kind: 'visual' | 'scenario') {
		if (!activeProject) return;
		if (kind === 'scenario' && (!visualId || !currentProjectValid)) {
			fileMessage = { kind: 'error', text: 'Compile a valid visual before creating a scenario.' };
			return;
		}
		resourceDialog = kind === 'visual' ? 'create-visual' : 'create-scenario';
		resourceNameDraft = kind === 'visual' ? 'Untitled visual' : 'New scenario';
		resourceDialogError = '';
	}

	function openResourceRename() {
		if (!activeProject || !activeFile || !currentResourceDeclaration) return;
		openResourceRenameForFile(activeFile.id, currentResourceDeclaration);
	}

	function openResourceRenameForFile(fileId: string, declaration: ProjectDeclaration | null) {
		if (!activeProject || compilePending || !currentProjectValid || !declaration) return;
		const kind = declaration.kind;
		if (kind !== 'visual' && kind !== 'scenario') return;
		resourceTarget = { projectId: activeProject.id, fileId, kind, declaration };
		resourceDialog = 'rename-resource';
		resourceNameDraft = declaration.title;
		resourceDialogError = '';
	}

	function openResourceRemoval() {
		if (!activeProject || !activeFile || !currentResourceDeclaration) return;
		const kind = currentResourceDeclaration.kind;
		if (kind !== 'visual' && kind !== 'scenario') return;
		openResourceRemovalForFile(activeFile.id, kind, currentResourceDeclaration);
	}

	function openResourceRemovalForFile(fileId: string, kind: ResourceTabKind, declaration: ProjectDeclaration | null) {
		if (!activeProject || compilePending || !currentProjectValid || !activeProject.files.some((file) => file.id === fileId)) return;
		resourceTarget = { projectId: activeProject.id, fileId, kind, declaration };
		resourceDialog = 'remove-resource';
		resourceDialogError = '';
	}

	function closeResourceDialog() {
		resourceDialog = null;
		resourceTarget = null;
		resourceDialogError = '';
	}

	async function saveResourceDialog() {
		if (!activeProject || !resourceDialog) return;
		try {
			const title = normalizeResourceTitle(resourceNameDraft);
			if (resourceDialog === 'rename-project') {
				const result = controller.renameProject(activeProject.id, title);
				if (!result.ok) throw new Error(result.error.message);
				fileMessage = null;
				closeResourceDialog();
				return;
			}
			if (resourceDialog === 'create-visual') {
				const result = controller.createVisual(activeProject.id, title);
				if (!result.ok) throw new Error(result.error.message);
				fileMessage = null;
				closeResourceDialog();
				return;
			}
			if (resourceDialog === 'create-scenario') {
				const targetVisualId = currentVisual?.id;
				if (!targetVisualId || !currentVisual) throw new Error('The selected visual is unavailable.');
				const usedScenarioIds = (currentVisual.output.scenarios as CompiledScenario[]).map((scenario) => scenario.id);
				const scenarioId = uniqueScenarioId(title, usedScenarioIds);
				const edge = firstVisualEdge(currentVisual.output.scene);
				const body = edge
					? `  token request {\n    icon: "mail"\n  }\n  move request from ${edge.from} -> ${edge.to} {\n    duration: 700ms\n  }`
					: '  wait 500ms';
				const source = `scenario ${scenarioId} for visual ${targetVisualId} {\n${body}\n}\n`;
				const result = controller.createScenario(activeProject.id, { visualId: targetVisualId, scenarioId, title, source });
				if (!result.ok) throw new Error(result.error.message);
				fileMessage = null;
				closeResourceDialog();
				return;
			}
			if (resourceDialog === 'rename-resource') {
				const target = resourceTarget;
				const document = target && activeProject.files.find(({ id }) => id === target.fileId);
				const current = target && currentFiles.find(({ id }) => id === target.fileId);
				if (!target || target.projectId !== activeProject.id || target.kind === 'source' || !target.declaration || !document || !current) throw new Error('The source changed while this dialog was open. Close it and try again.');
				const declaration = target.declaration;
				const liveDeclaration = current.declarations.find((item) => item.kind === target.kind && item.id === declaration.id && item.span.start.offset === declaration.span.start.offset);
				if (!liveDeclaration) throw new Error('The source changed while this dialog was open. Close it and try again.');
				if (target.kind === 'visual') {
					const source = renameVisualTitle(document.source, current, liveDeclaration, title);
					const result = controller.updateSource(activeProject.id, document.id, source);
					if (!result.ok) throw new Error(result.error.message);
				} else {
					const scenarioId = liveDeclaration.id;
					if (!scenarioId) throw new Error('The scenario identifier is unavailable.');
					const targetVisual = currentCompilation?.visuals.find(({ id }) => id === liveDeclaration.visual_id);
					const otherScenarioIds = (targetVisual?.output.scenarios as CompiledScenario[] | undefined)?.map(({ id }) => id).filter((id) => id !== scenarioId) ?? [];
					const replacementId = uniqueScenarioId(title, otherScenarioIds);
					const token = findDeclarationIdentifier(document.source, current, liveDeclaration, scenarioId);
					if (!token) throw new Error('The scenario identifier could not be located in the current source.');
					const renamed = await renameDslSymbol(sourceFiles, document.path, utf8SpanToEditorSpan(document.source, token.span).start.offset, replacementId);
					if (renamed.error) throw new Error(renamed.error);
					for (const edit of renamed.edits) {
						const editedFile = activeProject.files.find(({ path }) => path === edit.path);
						if (!editedFile) throw new Error('A linked scenario source is unavailable.');
						const result = controller.updateSource(activeProject.id, editedFile.id, applySourceTextEdits(editedFile.source, [edit]));
						if (!result.ok) throw new Error(result.error.message);
					}
				}
				fileMessage = null;
				closeResourceDialog();
				return;
			}
		} catch (error) {
			resourceDialogError = error instanceof Error ? error.message : 'Could not complete this project action.';
		}
	}

	function firstVisualEdge(scene: unknown): { from: string; to: string } | null {
		if (!isRecord(scene) || !Array.isArray(scene.edges)) return null;
		for (const edge of scene.edges) {
			if (isRecord(edge) && typeof edge.from === 'string' && typeof edge.to === 'string') return { from: edge.from, to: edge.to };
		}
		return null;
	}

	function removeResource() {
		if (!activeProject || !resourceTarget) return;
		const target = resourceTarget;
		if (target.projectId !== activeProject.id) {
			resourceDialogError = 'The active project changed while this dialog was open. Close it and try again.';
			return;
		}
		const file = activeProject.files.find(({ id }) => id === target.fileId);
		if (!file) {
			resourceDialogError = 'The source changed while this dialog was open. Close it and try again.';
			return;
		}
		if (target.kind === 'source') {
			if (activeProject.files.length <= 1) {
				resourceDialogError = 'A project must keep at least one source file.';
				return;
			}
			const removed = controller.removeDocuments(activeProject.id, [file.id]);
			if (!removed.ok) {
				resourceDialogError = removed.error.message;
				return;
			}
			fileMessage = null;
			closeResourceDialog();
			return;
		}
		const compiled = currentFiles.find(({ id }) => id === target.fileId);
		if (!compiled || !target.declaration) {
			resourceDialogError = 'The source changed while this dialog was open. Close it and try again.';
			return;
		}
		const declaration = target.declaration;
		const liveDeclaration = compiled.declarations.find((item) => item.kind === target.kind && item.id === declaration.id && item.span.start.offset === declaration.span.start.offset);
		if (!liveDeclaration) {
			resourceDialogError = 'The source changed while this dialog was open. Close it and try again.';
			return;
		}
		if (target.kind === 'visual') {
			const visualId = liveDeclaration.id;
			if (!visualId) {
				resourceDialogError = 'The visual identifier is unavailable.';
				return;
			}
			const remainingVisualIds = (currentCompilation?.visuals ?? []).map(({ id }) => id).filter((id) => id !== visualId);
			if (remainingVisualIds.length === 0) {
				resourceDialogError = 'A project must keep at least one visual.';
				return;
			}
			const linkedScenarioFiles = currentFiles.filter((item) => item.kind === 'scenario' && item.visual_id === visualId);
			const visualDeclarations = compiled.declarations.filter((item) => item.kind === 'visual');
			const removeVisualFile = visualDeclarations.length <= 1;
			const fileIds = [...linkedScenarioFiles.map(({ id }) => id), ...(removeVisualFile ? [file.id] : [])];
			if (fileIds.length > 0) {
				const removed = controller.removeDocuments(activeProject.id, [...new Set(fileIds)], {
					visualIdsToRemove: [visualId],
					remainingVisualIds
				});
				if (!removed.ok) {
					resourceDialogError = removed.error.message;
					return;
				}
			}
			if (!removeVisualFile) {
				const source = removeSourceDeclaration(file.source, liveDeclaration.span);
				const updated = controller.updateSource(activeProject.id, file.id, source);
				if (!updated.ok) {
					resourceDialogError = updated.error.message;
					return;
				}
				controller.selectScenario(activeProject.id, visualId, null);
			}
		} else {
			const scenarioId = liveDeclaration.id;
			const scenarioDeclarations = compiled.declarations.filter((item) => item.kind === 'scenario');
			if (!scenarioId) {
				resourceDialogError = 'The scenario identifier is unavailable.';
				return;
			}
			if (scenarioDeclarations.length <= 1) {
				const removed = controller.removeDocuments(activeProject.id, [file.id], {
					scenarioSelectionsToClear: liveDeclaration.visual_id ? [{ visualId: liveDeclaration.visual_id, scenarioId }] : []
				});
				if (!removed.ok) {
					resourceDialogError = removed.error.message;
					return;
				}
			} else {
				const updated = controller.updateSource(activeProject.id, file.id, removeSourceDeclaration(file.source, liveDeclaration.span));
				if (!updated.ok) {
					resourceDialogError = updated.error.message;
					return;
				}
				if (liveDeclaration.visual_id && selectedScenario?.id === scenarioId) {
					controller.selectScenario(activeProject.id, liveDeclaration.visual_id, null);
				}
			}
		}
		fileMessage = null;
		closeResourceDialog();
	}

	function resourceRemovalLabel(target: ResourceTarget | null) {
		if (target?.kind !== 'scenario') return 'Remove from project';
		const file = currentFiles.find(({ id }) => id === target.fileId);
		const declarationCount = file?.declarations.filter((item) => item.kind === 'scenario').length ?? 0;
		return declarationCount > 1 ? 'Remove scenario' : 'Remove from project';
	}

	function updateThemePreferences(value: ThemePreferences) {
		themePreferences = value;
		writeThemePreferences(value);
	}

	function updateEditorPreferences(value: EditorPreferences) {
		editorPreferences = value;
		writeEditorPreferences(value);
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
				fileMessage = { kind: 'error', text: 'The source changed before it could be saved.' };
			}
			return;
		}
		try {
			if (workspace.activeProjectId === projectId && activeFileId === fileId) {
				controller.updateSource(projectId, fileId, source);
			}
			const saved = await controller.flush();
			if (workspace.activeProjectId === projectId) {
				fileMessage = saved.ok
					? { kind: 'success', text: 'Workspace saved in this browser.' }
					: { kind: 'error', text: saved.error.message };
			}
		} catch (error) {
			if (workspace.activeProjectId === projectId) {
				fileMessage = { kind: 'error', text: error instanceof Error ? error.message : 'Could not save the workspace.' };
			}
		}
	}

	function openSourceFile() {
		const projectId = activeProject?.id;
		if (!workspace.initialized || !projectId || sourceOperation) return;
		fileMessage = null;
		pendingSourceProjectId = projectId;
		if (!supportsNativeSourceOpen()) {
			sourceInput?.click();
			return;
		}
		sourceOperation = 'opening';
		let awaitingUpload = false;
		void openLocalSchemraSource().then((result) => {
			if (result.status === 'cancelled') return;
			if (result.status === 'upload-required') {
				awaitingUpload = true;
				sourceOperation = null;
				sourceInput?.click();
				return;
			}
			addOpenedSource(projectId, result.document);
		}).catch((error: unknown) => {
			fileMessage = { kind: 'error', text: error instanceof Error ? error.message : 'Could not open the source file.' };
		}).finally(() => {
			sourceOperation = null;
			if (!awaitingUpload) pendingSourceProjectId = null;
		});
	}

	function cancelSourceUpload() {
		pendingSourceProjectId = null;
	}

	async function openUploadedSource(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0] ?? null;
		input.value = '';
		const projectId = pendingSourceProjectId ?? activeProject?.id ?? null;
		pendingSourceProjectId = null;
		if (!file || !projectId) return;
		fileMessage = null;
		sourceOperation = 'opening';
		try {
			const result = await readSchemraUpload(file);
			if (result.status === 'opened') addOpenedSource(projectId, result.document);
		} catch (error) {
			fileMessage = { kind: 'error', text: error instanceof Error ? error.message : 'Could not read the source file.' };
		} finally {
			sourceOperation = null;
		}
	}

	function addOpenedSource(projectId: string, document: LocalSchemraSource) {
		const added = controller.addDocument(projectId, {
			path: projectPathForLocalSource(document.name, document.source),
			source: document.source,
			handle: document.handle,
			mode: document.mode
		});
		if (!added.ok) {
			fileMessage = { kind: 'error', text: added.error.message };
			return;
		}
		activeSurface = 'editor';
		fileMessage = { kind: 'success', text: `Opened ${fileLabel(added.value.path)}.` };
	}

	async function saveSourceFile(source = activeFile?.source ?? '') {
		if (!workspace.initialized || !activeProject || !activeFile || sourceOperation) return;
		const projectId = activeProject.id;
		const fileId = activeFile.id;
		const fileName = activeFile.handle?.name ?? activeFile.path.split('/').at(-1) ?? 'source.schemra';
		const writtenSource = source;
		const handle = activeFile.handle;
		fileMessage = null;
		sourceOperation = 'saving';
		try {
			const result = await saveLocalSchemraSource({ name: fileName, source: writtenSource, handle });
			if (result.status === 'cancelled') return;
			const marked = controller.markSourceSaved(projectId, fileId, writtenSource, {
				name: result.name,
				handle: result.handle,
				mode: result.mode
			});
			if (!marked.ok) {
				fileMessage = { kind: 'error', text: `Saved ${result.name}, but could not update the workspace: ${marked.error.message}` };
				return;
			}
			const current = controller.getState().projects.find(({ id }) => id === projectId)?.files.find(({ id }) => id === fileId);
			fileMessage = {
				kind: 'success',
				text: current && current.source !== writtenSource
					? `Saved ${result.name}. Newer editor changes remain unsaved to this file.`
					: `Saved ${result.name}.`
			};
		} catch (error) {
			fileMessage = { kind: 'error', text: error instanceof Error ? error.message : 'Could not save the source file.' };
		} finally {
			sourceOperation = null;
		}
	}

	function supportsNativeSourceOpen() {
		const browser = globalThis as typeof globalThis & { showOpenFilePicker?: unknown };
		return typeof browser.showOpenFilePicker === 'function';
	}

	async function importProject(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		const result = await controller.importBundle(await file.text());
		fileMessage = !result.ok
			? { kind: 'error', text: result.error.message }
			: result.persistence === 'saved'
				? { kind: 'success', text: `Imported ${result.project.name}.` }
				: { kind: 'error', text: result.warning?.message ?? `Imported ${result.project.name}, but browser storage is unavailable.` };
		activeSurface = 'editor';
	}

	function exportProject() {
		const result = controller.exportBundle();
		if (!result.ok) {
			fileMessage = { kind: 'error', text: result.error.message };
			return;
		}
		const { fileName, contents } = result.value;
		const url = URL.createObjectURL(new Blob([contents], { type: 'application/json;charset=utf-8' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = fileName;
		link.click();
		window.setTimeout(() => URL.revokeObjectURL(url), 0);
		fileMessage = { kind: 'success', text: `Exported ${activeProject?.name ?? 'project'}.` };
	}

	function createProject() {
		const result = controller.createProject('Untitled project');
		fileMessage = result.ok ? null : { kind: 'error', text: result.error.message };
		activeSurface = 'editor';
	}

	function isVisualFile(path: string) {
		return path.startsWith('visuals/');
	}

	function fileLabel(path: string) {
		return path.split('/').at(-1)?.replace(/\.schemra$/i, '') ?? path;
	}

	function fileKind(path: string) {
		return isVisualFile(path) ? 'Visual' : path.startsWith('scenarios/') ? 'Scenario' : 'Source';
	}

	function prettySaveError() {
		return workspace.error?.message ?? 'Changes could not be saved to browser storage.';
	}

	function makeSourcesKey(sources: ProjectSource[]) {
		return JSON.stringify(sources.map(({ id, path, source }) => [id, path, source]));
	}

	function projectSnapshotMatches(projectId: string, snapshotKey: string) {
		const project = workspace.projects.find((candidate) => candidate.id === projectId);
		return Boolean(project && makeSourcesKey(project.files.map(({ id, path, source }) => ({ id, path, source }))) === snapshotKey);
	}

	function compareSourceFiles(left: { path: string }, right: { path: string }) {
		const leftRank = isVisualFile(left.path) ? 0 : left.path.startsWith('scenarios/') ? 1 : 2;
		const rightRank = isVisualFile(right.path) ? 0 : right.path.startsWith('scenarios/') ? 1 : 2;
		return leftRank - rightRank || left.path.localeCompare(right.path);
	}

	function isValidProjectCompilation(result: ProjectCompileResult) {
		if (result.error || result.visuals.length === 0) return false;
		const files = result.files as ProjectCompileFile[];
		if (files.some((file) => file.diagnostics.some((diagnostic) => isErrorDiagnostic(diagnostic)))) return false;
		return result.visuals.some(({ output }) => Boolean(output.svg && output.scene));
	}

	function isErrorDiagnostic(value: unknown) {
		return typeof value === 'object' && value !== null && 'severity' in value && value.severity === 'error';
	}

	function findDeclarationIdentifier(source: string, file: ProjectCompileFile, declaration: ProjectDeclaration, identifier: string) {
		return file.tokens.find((token) => {
			if (token.kind !== 'identifier' || token.span.start.line !== declaration.span.start.line) return false;
			const editorSpan = utf8SpanToEditorSpan(source, token.span);
			return source.slice(editorSpan.start.offset, editorSpan.end.offset) === identifier;
		}) ?? null;
	}

	function renameVisualTitle(source: string, file: ProjectCompileFile, declaration: ProjectDeclaration, title: string) {
		const titleToken = file.tokens.find((token) => token.kind === 'string' && token.span.start.line === declaration.span.start.line);
		if (titleToken) {
			return applySourceTextEdits(source, [{ span: utf8SpanToEditorSpan(source, titleToken.span), replacement: JSON.stringify(title) }]);
		}
		const identifier = declaration.id && findDeclarationIdentifier(source, file, declaration, declaration.id);
		if (!identifier) throw new Error('The visual identifier could not be located in the current source.');
		const point = utf8SpanToEditorSpan(source, identifier.span).end;
		return applySourceTextEdits(source, [{ span: { start: point, end: point }, replacement: ` ${JSON.stringify(title)}` }]);
	}

	function isRecord(value: unknown): value is Record<string, unknown> {
		return typeof value === 'object' && value !== null && !Array.isArray(value);
	}
</script>

<svelte:head>
	<title>Schemra Studio</title>
	<meta name="description" content="A local workspace for authoring and previewing Schemra diagrams." />
	<link rel="icon" href={studioBrandAssets.mark} />
</svelte:head>

<svelte:window onkeydown={handleWindowKeydown} />

<main class="studio-shell" class:schemra-light={resolvedTheme === 'schemra-light'} class:schemra-dark={resolvedTheme === 'schemra-dark'} class:reduce-motion={themePreferences.reduceMotion}>
	<header class="topbar">
		<a class="brand" href="/" aria-label="Schemra Studio home">
			<picture aria-hidden="true">
				<source media="(max-width: 700px)" srcset={studioBrandAssets.mark} />
				<img src={studioBrandAssets.wordmark} alt="" />
			</picture>
		</a>
		<div class="project-controls">
			<label class="project-select-wrap">
				<FolderOpen size={17} aria-hidden="true" />
				<select aria-label="Project" value={activeProject?.id ?? ''} onchange={selectProject} disabled={!workspace.initialized}>
					{#each workspace.projects as project (project.id)}<option value={project.id}>{project.name}</option>{/each}
				</select>
				<ChevronDown size={14} aria-hidden="true" />
			</label>
			<button class="icon-button" type="button" aria-label="Rename project" title="Rename project" onclick={openProjectRename} disabled={!activeProject}><Pencil size={16} aria-hidden="true" /></button>
			<button class="icon-button" type="button" aria-label="Create project" title="Create project" onclick={createProject}>＋</button>
		</div>
		<div class="topbar-actions">
			<button class="icon-button" type="button" aria-label="Search files and actions" title="Search files and actions (Ctrl/⌘ K)" aria-haspopup="dialog" aria-expanded={commandPaletteOpen} onclick={() => (commandPaletteOpen = true)} disabled={!workspace.initialized}><Search size={17} aria-hidden="true" /></button>
			<span class="save-state" class:save-error={workspace.saveStatus === 'error' || workspace.saveStatus === 'conflict'} title={workspace.error?.message ?? saveStatusLabel}>
				{#if workspace.saveStatus === 'saving' || workspace.saveStatus === 'loading'}<LoaderCircle size={15} class="spin" aria-hidden="true" />
				{:else if workspace.saveStatus === 'saved'}<Check size={15} aria-hidden="true" />
				{:else if workspace.saveStatus === 'error' || workspace.saveStatus === 'conflict'}<AlertCircle size={15} aria-hidden="true" />
				{:else}<Save size={15} aria-hidden="true" />{/if}
				<span>{saveStatusLabel}</span>
			</span>
			<button class="text-button" type="button" aria-label="Save workspace" title="Save workspace in this browser" onclick={() => void saveWorkspace()} disabled={!workspace.initialized || !activeFile}><Save size={15} aria-hidden="true" /><span>Save workspace</span></button>
			<button class="icon-button" type="button" aria-label="Export project" title="Export project" onclick={exportProject} disabled={!workspace.initialized}><Download size={17} aria-hidden="true" /></button>
			<button class="icon-button" type="button" aria-label="Import project" title="Import project" onclick={() => importInput?.click()} disabled={!workspace.initialized}><Upload size={17} aria-hidden="true" /></button>
			<input bind:this={importInput} class="visually-hidden" type="file" accept="application/json,.json" aria-label="Choose project bundle" onchange={importProject} />
			<input bind:this={sourceInput} class="visually-hidden" type="file" accept=".schemra,text/plain" aria-label="Choose a Schemra source file" onchange={openUploadedSource} oncancel={cancelSourceUpload} />
			<button class="icon-button settings-trigger" type="button" aria-label="Settings" title="Settings" onclick={() => (settingsOpen = true)}><Settings2 size={17} aria-hidden="true" /></button>
		</div>
	</header>

	<nav class="file-tabs" aria-label="Project source files">
		<div class="tabs-scroll">
			{#each orderedFiles as file (file.id)}
				{@const fileCompilation = currentFiles.find((item) => item.id === file.id)}
				{@const tabKind: ResourceTabKind = file.path.startsWith('visuals/') ? 'visual' : file.path.startsWith('scenarios/') ? 'scenario' : 'source'}
				{@const declarations = fileCompilation?.declarations.filter((item) => item.kind === tabKind) ?? []}
				{@const tabDeclaration = file.id === activeFileId && currentResourceDeclaration?.kind === tabKind
					? currentResourceDeclaration
					: declarations[0] ?? null}
				{@const menuPolicy = resourceTabMenuPolicy({
					kind: tabKind,
					declarationCount: declarations.length,
					projectFileCount: activeProject?.files.length ?? 0,
					initialized: workspace.initialized,
					compiling: compilePending,
					projectValid: currentProjectValid
				})}
				<ResourceContextMenu
					onRename={menuPolicy.showRename && tabDeclaration ? () => openResourceRenameForFile(file.id, tabDeclaration) : undefined}
					onRemove={() => openResourceRemovalForFile(file.id, tabKind, tabDeclaration)}
					renameDisabled={menuPolicy.renameDisabled}
					removeDisabled={menuPolicy.removeDisabled}
				>
					<button
						class="file-tab"
						class:active={file.id === activeFileId}
						class:tab-invalid={fileCompilation?.diagnostics.some(isErrorDiagnostic)}
						type="button"
						disabled={!workspace.initialized}
						aria-current={file.id === activeFileId ? 'page' : undefined}
						aria-haspopup="menu"
						aria-keyshortcuts="Shift+F10"
						title={file.path}
						onclick={() => openFile(file.id)}
					>
						<FileCode2 size={15} aria-hidden="true" />
						<span class="tab-kind">{fileKind(file.path)}</span>
						<span class="tab-label">{fileLabel(file.path)}</span>
						{#if file.source !== file.savedSource}<span class="unsaved-dot" aria-label="Unsaved changes"></span>{/if}
					</button>
				</ResourceContextMenu>
			{/each}
		</div>
		<div class="resource-actions" aria-label="Resource actions">
			<button class="icon-button" type="button" aria-label="Open source" title={sourceOperation === 'opening' ? 'Opening source' : 'Open source'} onclick={openSourceFile} disabled={!workspace.initialized || sourceOperation !== null}><FolderOpen size={16} aria-hidden="true" /></button>
			<button class="icon-button" type="button" aria-label="Save source" title={sourceOperation === 'saving' ? 'Saving source' : 'Save source'} onclick={() => void saveSourceFile()} disabled={!workspace.initialized || !activeFile || sourceOperation !== null}>{#if sourceOperation === 'saving'}<LoaderCircle size={15} class="spin" aria-hidden="true" />{:else}<Save size={15} aria-hidden="true" />{/if}</button>
			<button class="icon-button" type="button" aria-label="Create visual" title="Create visual" onclick={() => openResourceCreation('visual')} disabled={!workspace.initialized}><Plus size={16} aria-hidden="true" /></button>
			<button class="icon-button" type="button" aria-label="Create scenario" title="Create scenario" onclick={() => openResourceCreation('scenario')} disabled={!workspace.initialized || !currentProjectValid || !currentVisual}><Plus size={16} aria-hidden="true" /></button>
			{#if currentResourceDeclaration}
				<button class="icon-button" type="button" aria-label="Rename selected resource" title="Rename selected resource" onclick={openResourceRename} disabled={compilePending || !currentProjectValid}><Pencil size={15} aria-hidden="true" /></button>
				<button class="icon-button danger-button" type="button" aria-label="Remove selected resource" title="Remove selected resource" onclick={openResourceRemoval} disabled={compilePending || !currentProjectValid}><Trash2 size={15} aria-hidden="true" /></button>
			{/if}
		</div>
		{#if activeFile}<span class="file-path" title={activeFile.path}>{activeFile.path}</span>{/if}
	</nav>

	{#if fileMessage}
		<div class="file-message" class:message-error={fileMessage.kind === 'error'} role={fileMessage.kind === 'error' ? 'alert' : 'status'}>
			<span>{fileMessage.text}</span><button type="button" aria-label="Dismiss message" onclick={() => (fileMessage = null)}><X size={14} /></button>
		</div>
	{/if}
	{#if workspace.error && (workspace.saveStatus === 'error' || workspace.saveStatus === 'conflict')}
		<div class="file-message message-error" role="alert"><span>{prettySaveError()}</span><button type="button" aria-label="Dismiss message" onclick={() => controller.flush().then(() => (fileMessage = null))}><X size={14} /></button></div>
	{/if}
	{#if resourceDialog}
		<div class="resource-layer">
			<dialog open class="resource-dialog" aria-modal="true" aria-labelledby="resource-dialog-title">
				<header class="resource-dialog-heading">
					<h2 id="resource-dialog-title">{resourceDialog === 'create-visual' ? 'Create visual' : resourceDialog === 'create-scenario' ? 'Create scenario' : resourceDialog === 'rename-project' ? 'Rename project' : resourceDialog === 'rename-resource' ? `Rename ${resourceTarget?.kind ?? 'resource'}` : `Remove ${resourceTarget?.kind ?? 'resource'}`}</h2>
					<button class="icon-button" type="button" aria-label="Close" onclick={closeResourceDialog}><X size={16} aria-hidden="true" /></button>
				</header>
				{#if resourceDialog === 'remove-resource'}
					<p class="resource-dialog-description">
						{#if resourceTarget?.kind === 'visual'}This removes the visual and its linked scenario files. The project must keep at least one visual.{:else if resourceTarget?.kind === 'scenario'}This removes the selected scenario from the project. The source file stays in the project if it contains other scenarios.{:else}This removes the source file from the current project. The file itself remains on disk.{/if}
					</p>
				{:else}
					<form onsubmit={(event) => { event.preventDefault(); void saveResourceDialog(); }}>
						<label class="resource-field-label" for="resource-name">{resourceDialog === 'rename-project' ? 'Project name' : resourceTarget?.kind === 'scenario' || resourceDialog === 'create-scenario' ? 'Scenario name' : 'Visual name'}</label>
						<input id="resource-name" bind:value={resourceNameDraft} maxlength="80" autocomplete="off" spellcheck="false" />
					</form>
				{/if}
				{#if resourceDialogError}<p class="resource-dialog-error" role="alert">{resourceDialogError}</p>{/if}
				<footer class="resource-dialog-actions">
					<button class="text-button secondary-button" type="button" onclick={closeResourceDialog}>Cancel</button>
					{#if resourceDialog === 'remove-resource'}
						<button class="text-button danger-action" type="button" onclick={removeResource}>{resourceRemovalLabel(resourceTarget)}</button>
					{:else}
						<button class="text-button" type="button" onclick={() => void saveResourceDialog()}>{resourceDialog === 'rename-resource' || resourceDialog === 'rename-project' ? 'Save' : 'Create'}</button>
					{/if}
				</footer>
			</dialog>
		</div>
	{/if}
	{#if symbolRenameOpen && symbolRenameTarget}
		<div class="resource-layer">
			<dialog open class="resource-dialog" aria-modal="true" aria-labelledby="symbol-rename-title">
				<header class="resource-dialog-heading">
					<h2 id="symbol-rename-title">Rename {symbolRenameTarget.selection.kind}</h2>
					<button class="icon-button" type="button" aria-label="Close rename" onclick={closeEditorSymbolRename} disabled={symbolRenamePending}><X size={16} aria-hidden="true" /></button>
				</header>
				<p class="resource-dialog-description">Rename this identifier and its linked references throughout the project.</p>
				<form id="symbol-rename-form" onsubmit={(event) => { event.preventDefault(); void saveEditorSymbolRename(); }}>
					<label class="resource-field-label" for="symbol-rename-input">New identifier</label>
					<input
						id="symbol-rename-input"
						bind:this={symbolRenameInput}
						bind:value={symbolRenameDraft}
						autocomplete="off"
						autocapitalize="off"
						spellcheck="false"
						aria-invalid={Boolean(symbolRenameError)}
						disabled={symbolRenamePending}
					/>
				</form>
				{#if symbolRenameError}<p class="resource-dialog-error" role="alert">{symbolRenameError}</p>{/if}
				<footer class="resource-dialog-actions">
					<button class="text-button secondary-button" type="button" onclick={closeEditorSymbolRename} disabled={symbolRenamePending}>Cancel</button>
					<button class="text-button" type="submit" form="symbol-rename-form" disabled={symbolRenamePending || !symbolRenameDraft.trim()}>{symbolRenamePending ? 'Checking…' : 'Rename'}</button>
				</footer>
			</dialog>
		</div>
	{/if}

	<div class="mobile-surface-switch" role="group" aria-label="Workspace view">
		<button type="button" class:active={activeSurface === 'editor'} aria-pressed={activeSurface === 'editor'} onclick={() => (activeSurface = 'editor')}>Editor</button>
		<button type="button" class:active={activeSurface === 'preview'} aria-pressed={activeSurface === 'preview'} onclick={() => (activeSurface = 'preview')}>Preview</button>
	</div>

	<section class="workspace-grid" aria-label="Schemra workspace" class:show-editor={activeSurface === 'editor'} class:show-preview={activeSurface === 'preview'}>
		<section class="editor-panel" aria-label="Editor panel">
			<header class="panel-heading">
				<div class="panel-title"><FileCode2 size={16} aria-hidden="true" /><h1>Editor</h1>{#if diagnosticCount > 0}<span class="issue-count" class:has-error={errorCount > 0}>{diagnosticCount} {diagnosticCount === 1 ? 'issue' : 'issues'}</span>{/if}</div>
				<div class="editor-heading-actions">
					{#if compilePending}<span class="compiling"><LoaderCircle size={14} class="spin" aria-hidden="true" /> Compiling</span>
					{:else if compileError}<span class="compile-problem" title={compileError}><AlertCircle size={14} aria-hidden="true" /> Invalid project</span>
					{:else if hasFreshCompilation}<span class="compiled"><Check size={14} aria-hidden="true" /> Ready</span>{/if}
				</div>
			</header>
			<div class="editor-workspace" style={editorPalette}>
				{#if activeFile}
					<SourceEditor
						projectId={activeProject?.id}
						documentId={activeFile.id}
						disabled={!workspace.initialized}
						navigation={editorNavigation}
						source={activeFile.source}
						onChange={changeSource}
						tokens={currentFileCompilation ? readEditorSyntaxTokens(currentFileCompilation.tokens) : undefined}
						diagnostics={currentFileCompilation ? fileDiagnostics : undefined}
						editorSymbols={currentFileCompilation?.editor_symbols ?? []}
						refactorDiagnostics={currentRefactorDiagnostics}
						refactorSymbols={currentRefactorSymbols}
						quickFixDisabled={compilePending || quickFixPending || !currentCompilation}
						nodeIds={previewScene?.nodes.map(({ id }) => id) ?? []}
						label={`${activeFile.path} editor`}
						preferences={sourceEditorPreferences}
						onSave={saveWorkspace}
						onNavigate={navigateToSymbol}
						onRenameSymbol={openEditorSymbolRename}
						onQuickFix={applyEditorQuickFix}
						onNavigationHandled={handleEditorNavigation}
					/>
				{:else}
					<div class="empty-editor"><FileCode2 size={22} aria-hidden="true" /><p>{workspace.initialized ? 'Create or import a project to start.' : 'Restoring your local workspace…'}</p></div>
				{/if}
			</div>
		</section>

		<section class="preview-panel" aria-label="Preview panel">
			<header class="preview-heading">
				<div class="panel-title"><span class="preview-mark" aria-hidden="true"></span><h2>Preview</h2></div>
				<div class="preview-options">
					{#if previewScenarios.length > 0}
						<label class="scenario-select-wrap">
							<span class="sr-only">Scenario</span>
							<select value={selectedScenario?.id ?? ''} onchange={selectScenario} aria-label="Preview scenario">
								{#each previewScenarios as scenario (scenario.id)}<option value={scenario.id}>{scenario.title}</option>{/each}
							</select><ChevronDown size={13} aria-hidden="true" />
						</label>
					{/if}
					{#if previewVisual?.output.stats}<span class="diagram-stats">{previewVisual.output.stats.nodes} nodes · {previewVisual.output.stats.edges} links · {previewVisual.output.stats.groups} groups</span>{/if}
				</div>
			</header>
			<div class="preview-content">
				{#if previewVisual?.output.svg && previewVisual.output.scene}
					<DiagramPreview
						svg={previewVisual.output.svg}
						scene={previewVisual.output.scene}
						scenario={selectedScenario}
						sources={previewSources}
						visualId={visualId}
					/>
				{:else}
					<div class="empty-preview"><div class="empty-preview-icon"><FileCode2 size={21} aria-hidden="true" /></div><strong>{compilePending ? 'Building preview' : 'No valid visual yet'}</strong><p>{compilePending ? 'The compiler is updating this diagram.' : 'Add a visual declaration to the active project source.'}</p></div>
				{/if}
				{#if lastGoodVisual && !currentVisual?.output.svg && !compilePending}<span class="last-valid-badge" title="Showing the last valid preview"><AlertCircle size={14} aria-hidden="true" /><span>Last valid preview</span></span>{/if}
			</div>
		</section>
	</section>

	{#if settingsOpen}
		<div class="settings-layer">
				<dialog open class="settings-dialog" aria-labelledby="settings-title">
				<header class="settings-heading"><div><span>Preferences</span><h2 id="settings-title">Settings</h2></div><button class="icon-button" type="button" aria-label="Close settings" onclick={() => (settingsOpen = false)}><X size={17} /></button></header>
				<div class="settings-scroll"><SettingsPanel
					themePreferences={themePreferences}
					editorPreferences={editorPreferences}
					onThemePreferencesChange={updateThemePreferences}
					onEditorPreferencesChange={updateEditorPreferences}
				/></div>
				</dialog>
		</div>
	{/if}

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
</main>

<style>
	.studio-shell { --shell-bg: var(--background); --shell-fg: var(--foreground); display: flex; width: 100%; height: 100dvh; min-width: 20rem; min-height: 28rem; flex-direction: column; overflow: hidden; color: var(--shell-fg); background: var(--shell-bg); }
	.topbar { display: flex; min-height: 3.75rem; flex: 0 0 auto; align-items: center; gap: 1.25rem; padding: 0 1.25rem; border-bottom: 1px solid var(--border); background: var(--background); }
	.brand { display: inline-flex; flex: 0 0 auto; align-items: center; text-decoration: none; }
	.brand picture { display: block; width: 8.75rem; height: 2rem; }
	.brand img { display: block; width: 100%; height: 100%; object-fit: contain; }
	.project-controls { display: flex; min-width: 0; flex: 1 1 auto; align-items: center; gap: .5rem; }
	.project-select-wrap { display: flex; min-width: min(16rem, 55vw); max-width: 22rem; height: 2.4rem; align-items: center; gap: .55rem; padding: 0 .65rem; border: 1px solid var(--border); border-radius: .55rem; color: var(--muted-foreground); background: var(--background); }
	.project-select-wrap:focus-within, .scenario-select-wrap:focus-within { outline: 2px solid color-mix(in srgb, var(--ring) 55%, transparent); outline-offset: 1px; }
	.project-select-wrap select, .scenario-select-wrap select { width: 100%; min-width: 0; appearance: none; border: 0; outline: 0; color: var(--foreground); background: transparent; font: inherit; font-size: .83rem; }
	.project-select-wrap select { font-weight: 600; }
	.project-select-wrap option, .scenario-select-wrap option { color: #102442; background: #fff; }
	.project-select-wrap :global(svg:last-child), .scenario-select-wrap :global(svg) { flex: 0 0 auto; pointer-events: none; }
	.topbar-actions { display: flex; flex: 0 0 auto; align-items: center; gap: .4rem; }
	.save-state { display: inline-flex; min-width: 7rem; align-items: center; gap: .35rem; padding: 0 .55rem; color: var(--muted-foreground); font-size: .72rem; white-space: nowrap; }
	.save-state :global(svg) { color: var(--primary); }
	.save-state.save-error, .compile-problem { color: var(--destructive); }
	.icon-button, .text-button { display: inline-flex; min-height: 2.25rem; align-items: center; justify-content: center; gap: .4rem; padding: .45rem .6rem; border: 1px solid var(--border); border-radius: .55rem; color: var(--foreground); background: var(--background); font: inherit; font-size: .77rem; cursor: pointer; }
	.icon-button { width: 2.25rem; padding: 0; }
	.icon-button:hover:not(:disabled), .text-button:hover:not(:disabled) { background: var(--accent); color: var(--accent-foreground); }
	.icon-button:disabled, .text-button:disabled { cursor: not-allowed; opacity: .48; }
	.text-button { color: var(--primary); font-weight: 620; }
	.file-tabs { display: flex; min-height: 3rem; flex: 0 0 auto; align-items: stretch; justify-content: space-between; gap: .75rem; padding: 0 1rem; border-bottom: 1px solid var(--border); background: var(--background); }
	.tabs-scroll { display: flex; min-width: 0; flex: 1 1 auto; gap: .18rem; overflow: auto hidden; scrollbar-width: thin; scrollbar-color: var(--input) transparent; }
	.resource-actions { display: flex; flex: 0 0 auto; align-items: center; gap: .25rem; }
	.resource-actions .icon-button { width: 2rem; min-height: 2rem; height: 2rem; }
	.danger-button:hover:not(:disabled), .danger-action { color: var(--destructive); }
	.file-tab { position: relative; display: inline-flex; max-width: 19rem; min-width: max-content; flex: 0 1 auto; align-items: center; gap: .45rem; padding: 0 .78rem; border: 0; border-bottom: 2px solid transparent; color: var(--muted-foreground); background: transparent; font: inherit; font-size: .77rem; cursor: pointer; }
	.file-tab:hover { color: var(--foreground); background: color-mix(in srgb, var(--muted) 55%, transparent); }
	.file-tab:disabled { color: var(--muted-foreground); cursor: wait; opacity: 1; }
	.file-tab.active { border-bottom-color: var(--primary); color: var(--foreground); background: color-mix(in srgb, var(--accent) 38%, transparent); }
	.file-tab.tab-invalid { color: var(--destructive); }
	.file-tab :global(> svg) { flex: 0 0 auto; }
	.tab-kind { color: var(--muted-foreground); font-size: .65rem; font-weight: 600; text-transform: uppercase; letter-spacing: .045em; }
	.tab-label { overflow: hidden; max-width: 10rem; text-overflow: ellipsis; white-space: nowrap; }
	.unsaved-dot { width: .38rem; height: .38rem; flex: 0 0 auto; border-radius: 50%; background: var(--primary); }
	.file-path { max-width: 18rem; overflow: hidden; align-self: center; color: var(--muted-foreground); font-family: ui-monospace, monospace; font-size: .68rem; text-overflow: ellipsis; white-space: nowrap; }
	.file-message { display: flex; min-height: 2.2rem; flex: 0 0 auto; align-items: center; justify-content: space-between; gap: 1rem; padding: .35rem 1rem; border-bottom: 1px solid color-mix(in srgb, var(--primary) 28%, var(--border)); color: var(--primary); background: color-mix(in srgb, var(--primary) 7%, var(--background)); font-size: .75rem; }
	.file-message.message-error { border-color: color-mix(in srgb, var(--destructive) 35%, var(--border)); color: var(--destructive); background: color-mix(in srgb, var(--destructive) 6%, var(--background)); }
	.file-message button { display: grid; width: 1.6rem; height: 1.6rem; flex: 0 0 auto; place-items: center; border: 0; border-radius: .4rem; color: inherit; background: transparent; cursor: pointer; }
	.file-message button:hover { background: color-mix(in srgb, currentColor 10%, transparent); }
	.mobile-surface-switch { display: none; }
	.workspace-grid { display: grid; min-height: 0; flex: 1 1 auto; grid-template-columns: minmax(0, 1.06fr) minmax(0, .94fr); }
	.editor-panel, .preview-panel { display: flex; min-width: 0; min-height: 0; flex-direction: column; }
	.editor-panel { border-right: 1px solid var(--border); }
	.panel-heading, .preview-heading { display: flex; min-height: 2.75rem; flex: 0 0 auto; align-items: center; justify-content: space-between; gap: .8rem; padding: 0 .9rem; border-bottom: 1px solid var(--border); background: var(--background); }
	.panel-title { display: flex; min-width: 0; align-items: center; gap: .5rem; }
	.panel-title h1, .panel-title h2 { overflow: hidden; margin: 0; color: var(--foreground); font-size: .82rem; font-weight: 650; text-overflow: ellipsis; white-space: nowrap; }
	.panel-title :global(> svg) { color: var(--muted-foreground); }
	.issue-count { margin-left: .25rem; color: var(--muted-foreground); font-size: .69rem; }
	.issue-count.has-error { color: var(--destructive); }
	.editor-heading-actions, .preview-options { display: flex; min-width: 0; align-items: center; justify-content: flex-end; gap: .75rem; }
	.compiling, .compiled, .compile-problem { display: inline-flex; align-items: center; gap: .3rem; color: var(--muted-foreground); font-size: .68rem; white-space: nowrap; }
	.compiled :global(svg) { color: var(--primary); }
	.editor-workspace { display: flex; min-height: 0; flex: 1 1 auto; overflow: hidden; color-scheme: var(--editor-color-scheme, light); background: var(--editor-background); color: var(--editor-foreground); }
	.editor-workspace :global(.source-editor) { flex: 1 1 auto; }
	.empty-editor { display: grid; min-height: 0; flex: 1 1 auto; align-content: center; justify-items: center; gap: .75rem; color: var(--muted-foreground); font-size: .84rem; }
	.empty-editor p { margin: 0; }
	.preview-mark { width: .5rem; height: .5rem; border-radius: 50%; background: var(--primary); box-shadow: 0 0 0 .22rem color-mix(in srgb, var(--primary) 13%, transparent); }
	.scenario-select-wrap { position: relative; display: flex; min-width: 8rem; max-width: 15rem; align-items: center; gap: .25rem; padding: .35rem .5rem; border: 1px solid var(--border); border-radius: .45rem; color: var(--muted-foreground); }
	.scenario-select-wrap select { padding-right: .25rem; font-size: .72rem; }
	.diagram-stats { color: var(--muted-foreground); font-size: .65rem; white-space: nowrap; }
	.preview-content { position: relative; display: flex; min-height: 0; flex: 1 1 auto; overflow: hidden; background: var(--muted); }
	.preview-content :global(.preview) { min-height: 0; }
	.empty-preview { display: flex; max-width: 22rem; margin: auto; flex-direction: column; align-items: center; gap: .5rem; padding: 2rem; color: var(--muted-foreground); text-align: center; }
	.empty-preview-icon { display: grid; width: 2.8rem; height: 2.8rem; margin-bottom: .3rem; place-items: center; border: 1px solid var(--border); border-radius: .9rem; color: var(--primary); background: var(--background); }
	.empty-preview strong { color: var(--foreground); font-size: .9rem; }
	.empty-preview p { margin: 0; font-size: .75rem; line-height: 1.5; }
	.last-valid-badge { position: absolute; top: .75rem; left: .75rem; display: inline-flex; align-items: center; gap: .35rem; padding: .35rem .55rem; border: 1px solid var(--border); border-radius: .45rem; color: var(--muted-foreground); background: var(--background); box-shadow: 0 2px 8px rgb(16 36 66 / 7%); font-size: .68rem; }
	.settings-layer { position: fixed; z-index: 30; inset: 0; display: flex; justify-content: flex-end; background: rgb(7 21 41 / 38%); }
	.resource-layer { position: fixed; z-index: 40; inset: 0; display: grid; place-items: center; padding: 1rem; background: rgb(7 21 41 / 38%); }
	.resource-dialog { display: flex; width: min(28rem, 100%); flex-direction: column; gap: 1rem; margin: 0; padding: 1.1rem; border: 1px solid var(--border); border-radius: .8rem; color: var(--foreground); background: var(--background); box-shadow: 0 1rem 3rem rgb(7 21 41 / 18%); }
	.resource-dialog-heading { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
	.resource-dialog-heading h2 { margin: 0; font-size: .98rem; font-weight: 650; }
	.resource-dialog form { display: grid; gap: .45rem; }
	.resource-field-label { color: var(--muted-foreground); font-size: .72rem; }
	.resource-dialog input { width: 100%; min-height: 2.5rem; padding: .45rem .65rem; border: 1px solid var(--border); border-radius: .5rem; outline: 0; color: var(--foreground); background: var(--background); font: inherit; font-size: .82rem; }
	.resource-dialog input:focus { border-color: var(--ring); box-shadow: 0 0 0 2px color-mix(in srgb, var(--ring) 25%, transparent); }
	.resource-dialog-description { margin: 0; color: var(--muted-foreground); font-size: .78rem; line-height: 1.5; }
	.resource-dialog-error { margin: -.25rem 0 0; color: var(--destructive); font-size: .75rem; line-height: 1.4; }
	.resource-dialog-actions { display: flex; justify-content: flex-end; gap: .5rem; padding-top: .25rem; }
	.secondary-button { color: var(--foreground); }
	.settings-dialog { display: flex; width: min(39rem, 100%); height: 100%; flex-direction: column; overflow: hidden; border-left: 1px solid var(--border); color: var(--foreground); background: var(--background); box-shadow: -1rem 0 3rem rgb(7 21 41 / 12%); }
	.settings-heading { display: flex; min-height: 4.25rem; flex: 0 0 auto; align-items: center; justify-content: space-between; padding: .65rem 1.1rem; border-bottom: 1px solid var(--border); }
	.settings-heading > div { display: grid; gap: .15rem; }
	.settings-heading span { color: var(--muted-foreground); font-size: .65rem; text-transform: uppercase; letter-spacing: .08em; }
	.settings-heading h2 { margin: 0; font-size: 1rem; }
	.settings-scroll { min-height: 0; flex: 1 1 auto; overflow-y: auto; padding: 1rem; }
	.settings-scroll :global(.settings-panel) { max-width: 34rem; margin-inline: auto; }
	:global(.spin) { animation: spin .9s linear infinite; }
	.visually-hidden, .sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0, 0, 0, 0); white-space: nowrap; clip-path: inset(50%); }
	@keyframes spin { to { transform: rotate(360deg); } }
	.reduce-motion *, .reduce-motion *::before, .reduce-motion *::after { scroll-behavior: auto !important; animation-duration: .01ms !important; animation-iteration-count: 1 !important; transition-duration: .01ms !important; }
	@media (max-width: 900px) {
		.topbar { gap: .8rem; padding-inline: .75rem; }
		.save-state { min-width: 0; }
		.save-state span { display: none; }
		.file-path { display: none; }
	}
	@media (max-width: 700px) {
		.studio-shell { min-height: 24rem; }
		.topbar { min-height: 3.25rem; gap: .5rem; }
		.brand picture { width: 1.8rem; height: 1.8rem; }
		.project-select-wrap { min-width: 0; max-width: none; flex: 1 1 auto; }
		.project-controls { flex: 1 1 auto; }
		.topbar-actions { gap: .25rem; }
		.text-button { width: 2.2rem; padding: 0; font-size: 0; }
		.text-button :global(svg) { width: 1rem; }
		.topbar-actions > .icon-button[aria-label='Export project'], .topbar-actions > .icon-button[aria-label='Import project'] { display: none; }
		.file-tabs { min-height: 2.7rem; padding-inline: .3rem; }
		.resource-actions { gap: .15rem; }
		.resource-actions .icon-button { width: 1.8rem; height: 1.8rem; min-height: 1.8rem; }
		.file-tab { gap: .35rem; padding-inline: .55rem; font-size: .7rem; }
		.file-tab :global(> svg) { width: .85rem; }
		.tab-kind { font-size: .58rem; }
		.mobile-surface-switch { display: flex; flex: 0 0 auto; gap: .3rem; padding: .35rem .55rem; border-bottom: 1px solid var(--border); background: var(--background); }
		.mobile-surface-switch button { flex: 1 1 50%; min-height: 2rem; border: 1px solid transparent; border-radius: .45rem; color: var(--muted-foreground); background: transparent; font: inherit; font-size: .75rem; }
		.mobile-surface-switch button.active { border-color: var(--border); color: var(--foreground); background: var(--muted); font-weight: 600; }
		.workspace-grid { display: block; }
		.editor-panel, .preview-panel { height: 100%; border-right: 0; }
		.workspace-grid.show-editor .preview-panel, .workspace-grid.show-preview .editor-panel { display: none; }
		.workspace-grid.show-editor .editor-panel, .workspace-grid.show-preview .preview-panel { display: flex; }
		.panel-heading, .preview-heading { padding-inline: .6rem; }
		.preview-options { gap: .4rem; }
		.diagram-stats { display: none; }
		.scenario-select-wrap { min-width: 6rem; max-width: 9rem; }
	}
</style>
