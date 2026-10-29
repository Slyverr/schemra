import { applySourceTextEdits, removeSourceDeclaration, utf8SpanToEditorSpan } from '../editor/source-edits.js';
import { studioMessage } from '../i18n/messages.js';
import type { WorkspaceController, WorkspaceControllerState } from '../project/workspace-controller.js';
import type { StudioProject } from '../project/session.js';
import { normalizeResourceTitle, uniqueProjectName, uniqueScenarioId } from '../project/session.js';
import { renameProject as renameDslSymbol, type CompileResult, type ProjectCompileResult, type ProjectSource, type SourceSpan } from '../schemra.js';
import { findDeclarationIdentifier, renameVisualTitle } from './resource-source-edits.js';
import type { ResourceTabKind } from './resource-tab-menu.js';

export type ResourceDialogKind = 'create-project' | 'create-visual' | 'create-scenario' | 'rename-project' | 'rename-resource' | 'remove-resource' | null;

export type ProjectDeclaration = {
	kind: 'visual' | 'scenario' | string;
	id: string | null;
	title: string;
	visual_id: string | null;
	span: SourceSpan;
};

export type ResourceCompileFile = Omit<ProjectCompileResult['files'][number], 'tokens' | 'declarations' | 'output'> & {
	tokens: { kind: string; span: SourceSpan }[];
	declarations: ProjectDeclaration[];
	output: CompileResult | null;
	editor_symbols?: ProjectCompileResult['files'][number]['editor_symbols'];
};

export type ResourceTarget = {
	projectId: string;
	fileId: string;
	kind: ResourceTabKind;
	declaration: ProjectDeclaration | null;
};

export type CompiledScenario = { id: string; title: string; duration_ms: number; tokens?: unknown[] };
export type ProjectVisual = ProjectCompileResult['visuals'][number];

export type ResourceDialogState = {
	kind: ResourceDialogKind;
	nameDraft: string;
	error: string;
	target: ResourceTarget | null;
};

export type ResourceActionSnapshot = {
	workspace: WorkspaceControllerState;
	activeProject: StudioProject | null;
	currentFiles: ResourceCompileFile[];
	currentCompilation: ProjectCompileResult | null;
	currentVisual: ProjectCompileResult['visuals'][number] | null;
	selectedScenario: { id: string; title: string; duration_ms: number } | null;
	sourceFiles: ProjectSource[];
	visualId: string | null;
	compilePending: boolean;
	currentProjectValid: boolean;
};

export type FileMessage = { kind: 'error' | 'success'; text: string } | null;

type ResourceActionsOptions = {
	controller: WorkspaceController;
	form: ResourceDialogState;
	getSnapshot: () => ResourceActionSnapshot;
	setFileMessage: (message: FileMessage) => void;
	setFocusedPane: (pane: 'editor' | 'preview' | null) => void;
};

export function createResourceActions({ controller, form, getSnapshot, setFileMessage, setFocusedPane }: ResourceActionsOptions) {
	function open(kind: ResourceDialogKind, nameDraft = '', target: ResourceTarget | null = null) {
		form.kind = kind;
		form.nameDraft = nameDraft;
		form.error = '';
		form.target = target;
	}

	function openProjectRename() {
		const { activeProject } = getSnapshot();
		if (activeProject) open('rename-project', activeProject.name);
	}

	function openProjectCreation() {
		const { workspace } = getSnapshot();
		open('create-project', uniqueProjectName(workspace.projects.map(({ name }) => name), studioMessage('project.defaultName')));
	}

	function openResourceCreation(kind: 'visual' | 'scenario') {
		const { activeProject, currentProjectValid, visualId } = getSnapshot();
		if (!activeProject) return;
		if (kind === 'scenario' && (!visualId || !currentProjectValid)) {
			setFileMessage({ kind: 'error', text: studioMessage('resource.compileBeforeScenario') });
			return;
		}
		open(kind === 'visual' ? 'create-visual' : 'create-scenario', studioMessage(kind === 'visual' ? 'resource.untitledVisual' : 'resource.newScenarioName'));
	}

	function openResourceRenameForFile(fileId: string, declaration: ProjectDeclaration | null) {
		const { activeProject, compilePending, currentProjectValid } = getSnapshot();
		if (!activeProject || compilePending || !currentProjectValid || !declaration) return;
		if (declaration.kind !== 'visual' && declaration.kind !== 'scenario') return;
		open('rename-resource', declaration.title, { projectId: activeProject.id, fileId, kind: declaration.kind, declaration });
	}

	function openResourceRemovalForFile(fileId: string, kind: ResourceTabKind, declaration: ProjectDeclaration | null) {
		const { activeProject, compilePending, currentProjectValid } = getSnapshot();
		if (!activeProject || compilePending || !currentProjectValid || !activeProject.files.some((file) => file.id === fileId)) return;
		open('remove-resource', '', { projectId: activeProject.id, fileId, kind, declaration });
	}

	function close() {
		form.kind = null;
		form.target = null;
		form.error = '';
	}

	async function save() {
		if (!form.kind) return;
		try {
			const title = normalizeResourceTitle(form.nameDraft);
			switch (form.kind) {
				case 'create-project': return createProject(title);
				case 'rename-project': return renameProject(title);
				case 'create-visual': return createVisual(title);
				case 'create-scenario': return createScenario(title);
				case 'rename-resource': return renameResource(title);
				case 'remove-resource':
				case null: return;
			}
		} catch (error) {
			form.error = error instanceof Error ? error.message : studioMessage('resource.actionFailed');
		}
	}

	function createProject(title: string) {
		const result = controller.createProject(title);
		if (!result.ok) throw new Error(result.error.message);
		setFileMessage(null);
		setFocusedPane(null);
		close();
	}

	function renameProject(title: string) {
		const { activeProject } = getSnapshot();
		if (!activeProject) return;
		const result = controller.renameProject(activeProject.id, title);
		if (!result.ok) throw new Error(result.error.message);
		setFileMessage(null);
		close();
	}

	function createVisual(title: string) {
		const { activeProject } = getSnapshot();
		if (!activeProject) return;
		const result = controller.createVisual(activeProject.id, title);
		if (!result.ok) throw new Error(result.error.message);
		setFileMessage(null);
		close();
	}

	function createScenario(title: string) {
		const { activeProject, currentVisual } = getSnapshot();
		if (!activeProject) return;
		if (!currentVisual) throw new Error(studioMessage('resource.selectedVisualUnavailable'));
		const usedScenarioIds = (currentVisual.output.scenarios as { id: string }[]).map(({ id }) => id);
		const scenarioId = uniqueScenarioId(title, usedScenarioIds);
		const edge = firstVisualEdge(currentVisual.output.scene);
		const body = edge
			? ['  token request {', '    icon: "mail"', '  }', `  move request from ${edge.from} -> ${edge.to} {`, '    duration: 700ms', '  }'].join('\n')
			: '  wait 500ms';
		const source = `scenario ${scenarioId} ${JSON.stringify(title)} for visual ${currentVisual.id} {\n${body}\n}\n`;
		const result = controller.createScenario(activeProject.id, { visualId: currentVisual.id, scenarioId, title, source });
		if (!result.ok) throw new Error(result.error.message);
		setFileMessage(null);
		close();
	}

	async function renameResource(title: string) {
		const snapshot = getSnapshot();
		const target = form.target;
		const document = target && snapshot.activeProject?.files.find(({ id }) => id === target.fileId);
		const current = target && snapshot.currentFiles.find(({ id }) => id === target.fileId);
		if (!target || target.projectId !== snapshot.activeProject?.id || target.kind === 'source' || !target.declaration || !document || !current) {
			throw new Error(studioMessage('project.sourceChangedDialog'));
		}
		const declaration = liveDeclaration(current, target.kind, target.declaration);
		if (target.kind === 'visual') {
			const source = renameVisualTitle(document.source, current.tokens, declaration.span, declaration.id, title);
			if (!source) throw new Error(studioMessage('project.visualIdNotFound'));
			const result = controller.updateSource(snapshot.activeProject!.id, document.id, source);
			if (!result.ok) throw new Error(result.error.message);
		} else {
			await renameScenario(snapshot, document, current, declaration, title);
		}
		setFileMessage(null);
		close();
	}

	async function renameScenario(
		snapshot: ResourceActionSnapshot,
		document: StudioProject['files'][number],
		current: ResourceCompileFile,
		declaration: ProjectDeclaration,
		title: string
	) {
		const scenarioId = declaration.id;
		if (!scenarioId || !snapshot.activeProject) throw new Error(studioMessage('project.scenarioIdUnavailable'));
		const targetVisual = snapshot.currentCompilation?.visuals.find(({ id }) => id === declaration.visual_id);
		const otherScenarioIds = (targetVisual?.output.scenarios as { id: string }[] | undefined)?.map(({ id }) => id).filter((id) => id !== scenarioId) ?? [];
		const replacementId = uniqueScenarioId(title, otherScenarioIds);
		const token = findDeclarationIdentifier(document.source, current.tokens, declaration.span, scenarioId);
		if (!token) throw new Error(studioMessage('resource.scenarioIdNotFound'));
		const renamed = await renameDslSymbol(snapshot.sourceFiles, document.path, utf8SpanToEditorSpan(document.source, token.span).start.offset, replacementId);
		if (renamed.error) throw new Error(renamed.error);
		for (const edit of renamed.edits) {
			const editedFile = snapshot.activeProject.files.find(({ path }) => path === edit.path);
			if (!editedFile) throw new Error(studioMessage('resource.linkedScenarioMissing'));
			const result = controller.updateSource(snapshot.activeProject.id, editedFile.id, applySourceTextEdits(editedFile.source, [edit]));
			if (!result.ok) throw new Error(result.error.message);
		}
	}

	function remove() {
		const snapshot = getSnapshot();
		const target = form.target;
		const project = snapshot.activeProject;
		if (!project || !target) return;
		if (target.projectId !== project.id) return fail('resource.activeProjectChanged');
		const file = project.files.find(({ id }) => id === target.fileId);
		if (!file) return fail('project.sourceChangedDialog');
		if (target.kind === 'source') return removeSource(project, file);
		const compiled = snapshot.currentFiles.find(({ id }) => id === target.fileId);
		if (!compiled || !target.declaration) return fail('project.sourceChangedDialog');
		const declaration = liveDeclaration(compiled, target.kind, target.declaration);
		if (target.kind === 'visual') removeVisual(snapshot, project, file, compiled, declaration);
		else removeScenario(snapshot, project, file, compiled, declaration);
	}

	function removeSource(project: StudioProject, file: StudioProject['files'][number]) {
		if (project.files.length <= 1) return fail('resource.minimumSources');
		const result = controller.removeDocuments(project.id, [file.id]);
		if (!result.ok) return failWith(result.error.message);
		finishRemoval();
	}

	function removeVisual(_snapshot: ResourceActionSnapshot, project: StudioProject, file: StudioProject['files'][number], _compiled: ResourceCompileFile, declaration: ProjectDeclaration) {
		if (!declaration.id) return fail('project.visualIdUnavailable');
		const result = controller.removeDocuments(project.id, [file.id]);
		if (!result.ok) return failWith(result.error.message);
		finishRemoval();
	}

	function removeScenario(snapshot: ResourceActionSnapshot, project: StudioProject, file: StudioProject['files'][number], compiled: ResourceCompileFile, declaration: ProjectDeclaration) {
		const scenarioId = declaration.id;
		if (!scenarioId) return fail('project.scenarioIdUnavailable');
		const declarations = compiled.declarations.filter((item) => item.kind === 'scenario');
		if (declarations.length <= 1) {
			const selections = declaration.visual_id ? [{ visualId: declaration.visual_id, scenarioId }] : [];
			const result = controller.removeDocuments(project.id, [file.id], { scenarioSelectionsToClear: selections });
			if (!result.ok) return failWith(result.error.message);
		} else {
			const result = controller.updateSource(project.id, file.id, removeSourceDeclaration(file.source, declaration.span));
			if (!result.ok) return failWith(result.error.message);
			if (declaration.visual_id && snapshot.selectedScenario?.id === scenarioId) controller.selectScenario(project.id, declaration.visual_id, null);
		}
		finishRemoval();
	}

	function liveDeclaration(file: ResourceCompileFile, kind: ResourceTabKind, target: ProjectDeclaration) {
		const declaration = file.declarations.find((item) => item.kind === kind && item.id === target.id && item.span.start.offset === target.span.start.offset);
		if (!declaration) throw new Error(studioMessage('project.sourceChangedDialog'));
		return declaration;
	}

	function finishRemoval() {
		setFileMessage(null);
		close();
	}

	function fail(key: Parameters<typeof studioMessage>[0]) {
		form.error = studioMessage(key);
	}

	function failWith(message: string) {
		form.error = message;
	}

	function removalLabel(target: ResourceTarget | null) {
		if (target?.kind !== 'scenario') return studioMessage('resource.dialog.removeFromProject');
		const file = getSnapshot().currentFiles.find(({ id }) => id === target.fileId);
		const count = file?.declarations.filter(({ kind }) => kind === 'scenario').length ?? 0;
		return studioMessage(count > 1 ? 'resource.dialog.removeScenario' : 'resource.dialog.removeFromProject');
	}

	return {
		openProjectRename,
		openProjectCreation,
		openResourceCreation,
		openResourceRenameForFile,
		openResourceRemovalForFile,
		close,
		save,
		remove,
		removalLabel
	};
}

function firstVisualEdge(scene: unknown): { from: string; to: string } | null {
	if (!isRecord(scene) || !Array.isArray(scene.edges)) return null;
	for (const edge of scene.edges) {
		if (isRecord(edge) && typeof edge.from === 'string' && typeof edge.to === 'string') return { from: edge.from, to: edge.to };
	}
	return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
