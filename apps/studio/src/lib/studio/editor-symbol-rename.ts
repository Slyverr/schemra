import { applySourceTextEdits } from '../editor/source-edits.js';
import { findEditorSymbolDefinitionAtOffset, type EditorSymbolFile } from '../editor/symbol-navigation.js';
import { studioMessage } from '../i18n/messages.js';
import type { StudioProject } from '../project/session.js';
import type { WorkspaceController, WorkspaceControllerState } from '../project/workspace-controller.js';
import { makeSourcesKey } from './project-view.js';
import type { ProjectSource, RenameEdit, RenameResult, RenameSelection } from '../schemra.js';
import { renameProject } from '../schemra.js';

export type EditorRenameTarget = {
	projectId: string;
	fileId: string;
	path: string;
	sourcesKey: string;
	offset: number;
	invocationFileId: string;
	invocationOffset: number;
	selection: RenameSelection;
};

export type EditorRenameState = {
	open: boolean;
	target: EditorRenameTarget | null;
	draft: string;
	error: string;
	pending: boolean;
};

export type EditorRenameSnapshot = {
	workspace: WorkspaceControllerState;
	activeProject: StudioProject | null;
	activeFile: StudioProject['files'][number] | null;
	compiledFiles: readonly EditorSymbolFile[];
};

type RenameActionsOptions = {
	controller: WorkspaceController;
	state: EditorRenameState;
	getSnapshot: () => EditorRenameSnapshot;
	snapshotIsCurrent: (projectId: string, sourcesKey: string) => boolean;
	setFileMessage: (message: { kind: 'error' | 'success'; text: string } | null) => void;
	navigate: (fileId: string, offset: number) => void;
	focusEditor: () => Promise<void>;
	prepareRename?: typeof renameProject;
};

export function createValidatedRenameUpdates(
	sources: readonly ProjectSource[],
	targetName: string,
	newName: string,
	selection: RenameSelection,
	plan: RenameResult
) {
	if (plan.error || !plan.selection) throw new Error(plan.error ?? studioMessage('editor.renamePlanFailed'));
	if (
		plan.selection.name !== targetName ||
		plan.selection.kind !== selection.kind ||
		plan.selection.span.start.offset !== selection.span.start.offset ||
		plan.selection.span.end.offset !== selection.span.end.offset
	) throw new Error(studioMessage('editor.renameSelectionChanged'));
	if (plan.edits.length === 0) throw new Error(studioMessage('editor.renamePlanEmpty'));

	const updates = new Map<string, { path: string; source: string }>();
	let validatedEditCount = 0;
	for (const source of sources) {
		const edits = plan.edits.filter((edit) => edit.path === source.path);
		if (edits.length === 0) continue;
		validatedEditCount += edits.length;
		validateEdits(source, edits, targetName, newName);
		updates.set(source.id, { path: source.path, source: applySourceTextEdits(source.source, edits) });
	}
	if (updates.size === 0 || validatedEditCount !== plan.edits.length) {
		throw new Error(studioMessage('editor.renamePlanInvalid'));
	}
	return updates;
}

function validateEdits(source: ProjectSource, edits: readonly RenameEdit[], targetName: string, newName: string) {
	for (const edit of edits) {
		if (
			edit.replacement !== newName ||
			source.source.slice(edit.span.start.offset, edit.span.end.offset) !== targetName
		) throw new Error(studioMessage('editor.renamePlanMismatch'));
	}
}

export function createEditorSymbolRenameActions({
	controller,
	state,
	getSnapshot,
	snapshotIsCurrent,
	setFileMessage,
	navigate,
	focusEditor,
	prepareRename = renameProject
}: RenameActionsOptions) {
	let requestId = 0;

	async function open(offset: number) {
		const { activeProject: project, activeFile: file, compiledFiles } = getSnapshot();
		if (!project || !file || state.pending) return;
		const definition = findEditorSymbolDefinitionAtOffset(compiledFiles, file.id, offset);
		const definitionFile = definition && project.files.find(({ id }) => id === definition.fileId);
		if (!definition || !definitionFile || definitionFile.path !== definition.path) {
			setFileMessage({ kind: 'error', text: studioMessage('editor.symbolAtCaretRequired') });
			return;
		}

		const sources = project.files.map(({ id, path, source }) => ({ id, path, source }));
		const sourcesKey = makeSourcesKey(sources);
		const currentRequest = ++requestId;
		state.pending = true;
		state.error = '';
		setFileMessage(null);
		try {
			const prepared = await prepareRename(sources, definition.path, definition.symbol.span.start.offset);
			if (prepared.error || !prepared.selection) throw new Error(prepared.error ?? studioMessage('editor.renameMissingTarget'));
			if (!isCurrentInvocation(currentRequest, project.id, sourcesKey, file.id)) {
				throw new Error(studioMessage('editor.projectChangedRenamePrepare'));
			}
			state.target = {
				projectId: project.id,
				fileId: definitionFile.id,
				path: definitionFile.path,
				sourcesKey,
				offset: definition.symbol.span.start.offset,
				invocationFileId: file.id,
				invocationOffset: offset,
				selection: prepared.selection
			};
			state.draft = prepared.selection.name;
			state.open = true;
			state.pending = false;
			await focusEditor();
		} catch (error) {
			if (currentRequest === requestId) {
				setFileMessage({
					kind: 'error',
					text: error instanceof Error ? error.message : studioMessage('editor.renamePrepareFailed')
				});
			}
		} finally {
			if (currentRequest === requestId) state.pending = false;
		}
	}

	function close() {
		if (state.pending) return;
		const target = state.target;
		requestId += 1;
		state.open = false;
		state.target = null;
		state.error = '';
		if (target) navigate(target.invocationFileId, target.invocationOffset);
	}

	async function save() {
		const target = state.target;
		const { workspace } = getSnapshot();
		const project = target && workspace.projects.find(({ id }) => id === target.projectId);
		if (!target || !project || state.pending) return;
		const newName = state.draft.trim();
		if (!newName) {
			state.error = studioMessage('editor.identifierRequired');
			return;
		}
		if (newName === target.selection.name) {
			close();
			return;
		}

		const sources = project.files.map(({ id, path, source }) => ({ id, path, source }));
		if (!isTargetCurrent(target, sources, workspace)) {
			state.error = studioMessage('editor.projectChangedRenameOpened');
			return;
		}

		state.pending = true;
		state.error = '';
		try {
			const plan = await prepareRename(sources, target.path, target.offset, newName);
			const updates = createValidatedRenameUpdates(sources, target.selection.name, newName, target.selection, plan);
			if (
				!snapshotIsCurrent(target.projectId, target.sourcesKey) ||
				getSnapshot().workspace.activeProjectId !== target.projectId ||
				getSnapshot().workspace.selectedFileByProject[target.projectId] !== target.invocationFileId
			) throw new Error(studioMessage('editor.projectChangedRenameApply'));

			for (const [fileId, update] of updates) {
				const result = controller.updateSource(target.projectId, fileId, update.source);
				if (!result.ok) throw new Error(result.error.message);
			}
			state.open = false;
			state.target = null;
			state.error = '';
			setFileMessage(null);
			navigate(target.fileId, plan.selection!.span.start.offset);
		} catch (error) {
			state.error = error instanceof Error ? error.message : studioMessage('editor.renamePlanFailed');
		} finally {
			state.pending = false;
		}
	}

	function isCurrentInvocation(currentRequest: number, projectId: string, sourcesKey: string, fileId: string) {
		const workspace = getSnapshot().workspace;
		return currentRequest === requestId && snapshotIsCurrent(projectId, sourcesKey) &&
			workspace.activeProjectId === projectId && workspace.selectedFileByProject[projectId] === fileId;
	}

	return { open, close, save };
}

function isTargetCurrent(target: EditorRenameTarget, sources: readonly ProjectSource[], workspace: WorkspaceControllerState) {
	const targetSource = sources.find(({ id, path }) => id === target.fileId && path === target.path);
	return workspace.activeProjectId === target.projectId &&
		workspace.selectedFileByProject[target.projectId] === target.invocationFileId &&
		makeSourcesKey(sources) === target.sourcesKey &&
		targetSource?.source.slice(target.selection.span.start.offset, target.selection.span.end.offset) === target.selection.name;
}
