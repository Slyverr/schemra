import type { LocalSchemraSource, OpenLocalSourceResult, SaveLocalSourceResult } from '../project/local-source.js';
import {
	openLocalSchemraSource,
	projectPathForLocalSource,
	readSchemraUpload,
	saveLocalSchemraSource
} from '../project/local-source.js';
import type { SaveLocalSourceOptions } from '../project/local-source.js';
import type { StudioProject } from '../project/session.js';
import type { WorkspaceController, WorkspaceControllerState } from '../project/workspace-controller.js';
import { studioMessage } from '../i18n/messages.js';
import { sourceLabel } from './project-sidebar.js';

export type SourceActionState = {
	operation: 'opening' | 'saving' | null;
	pendingProjectId: string | null;
};

export type SourceActionMessage = { kind: 'error' | 'success'; text: string } | null;

export type SourceActionSnapshot = {
	workspace: WorkspaceControllerState;
	activeProject: StudioProject | null;
	activeFile: StudioProject['files'][number] | null;
};

export type SourceFileOperations = {
	open: () => Promise<OpenLocalSourceResult>;
	readUpload: (file: File | null | undefined) => Promise<OpenLocalSourceResult>;
	save: (options: SaveLocalSourceOptions) => Promise<SaveLocalSourceResult>;
};

type SourceActionsOptions = {
	controller: WorkspaceController;
	state: SourceActionState;
	getSnapshot: () => SourceActionSnapshot;
	getSourceInput: () => HTMLInputElement | undefined;
	setMessage: (message: SourceActionMessage) => void;
	setFocusedPane: (pane: 'editor' | 'preview' | null) => void;
	operations?: SourceFileOperations;
	supportsNativeOpen?: () => boolean;
};

const browserOperations: SourceFileOperations = {
	open: openLocalSchemraSource,
	readUpload: readSchemraUpload,
	save: saveLocalSchemraSource
};

export function createSourceActions({
	controller,
	state,
	getSnapshot,
	getSourceInput,
	setMessage,
	setFocusedPane,
	operations = browserOperations,
	supportsNativeOpen = supportsNativeSourceOpen
}: SourceActionsOptions) {
	function openSource() {
		const { workspace, activeProject } = getSnapshot();
		if (!workspace.initialized || !activeProject || state.operation) return;
		setMessage(null);
		state.pendingProjectId = activeProject.id;
		if (!supportsNativeOpen()) {
			getSourceInput()?.click();
			return;
		}

		state.operation = 'opening';
		let awaitingUpload = false;
		void operations.open().then((result) => {
			if (result.status === 'cancelled') return;
			if (result.status === 'upload-required') {
				awaitingUpload = true;
				state.operation = null;
				getSourceInput()?.click();
				return;
			}
			addOpenedSource(activeProject.id, result.document);
		}).catch((error: unknown) => {
			setMessage({ kind: 'error', text: errorMessage(error, 'project.openFailed') });
		}).finally(() => {
			state.operation = null;
			if (!awaitingUpload) state.pendingProjectId = null;
		});
	}

	function cancelUpload() {
		state.pendingProjectId = null;
	}

	async function openUpload(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0] ?? null;
		input.value = '';
		const projectId = state.pendingProjectId ?? getSnapshot().activeProject?.id ?? null;
		state.pendingProjectId = null;
		if (!file || !projectId) return;
		setMessage(null);
		state.operation = 'opening';
		try {
			const result = await operations.readUpload(file);
			if (result.status === 'opened') addOpenedSource(projectId, result.document);
		} catch (error) {
			setMessage({ kind: 'error', text: errorMessage(error, 'project.readFailed') });
		} finally {
			state.operation = null;
		}
	}

	function addOpenedSource(projectId: string, source: LocalSchemraSource) {
		const result = controller.addDocument(projectId, {
			path: projectPathForLocalSource(source.name, source.source),
			source: source.source,
			handle: source.handle,
			mode: source.mode
		});
		if (!result.ok) {
			setMessage({ kind: 'error', text: result.error.message });
			return;
		}
		setFocusedPane(null);
		setMessage({ kind: 'success', text: studioMessage('project.openedSource', { name: sourceLabel(result.value.path) }) });
	}

	async function saveSource(source?: string) {
		const { workspace, activeProject, activeFile } = getSnapshot();
		if (!workspace.initialized || !activeProject || !activeFile || state.operation) return;
		const projectId = activeProject.id;
		const fileId = activeFile.id;
		const fileName = activeFile.handle?.name ?? activeFile.path.split('/').at(-1) ?? 'source.schemra';
		const writtenSource = source ?? activeFile.source;
		setMessage(null);
		state.operation = 'saving';
		try {
			const result = await operations.save({ name: fileName, source: writtenSource, handle: activeFile.handle });
			if (result.status === 'cancelled') return;
			const marked = controller.markSourceSaved(projectId, fileId, writtenSource, {
				name: result.name,
				handle: result.handle,
				mode: result.mode
			});
			if (!marked.ok) {
				setMessage({ kind: 'error', text: studioMessage('project.savedSourceSyncFailed', { name: result.name, error: marked.error.message }) });
				return;
			}
			const current = controller.getState().projects.find(({ id }) => id === projectId)?.files.find(({ id }) => id === fileId);
			setMessage({
				kind: 'success',
				text: current && current.source !== writtenSource
					? studioMessage('project.savedSourceNewerChanges', { name: result.name })
					: studioMessage('project.savedSource', { name: result.name })
			});
		} catch (error) {
			setMessage({ kind: 'error', text: errorMessage(error, 'project.saveFailed') });
		} finally {
			state.operation = null;
		}
	}

	async function importProject(event: Event) {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		input.value = '';
		if (!file) return;
		const result = await controller.importBundle(await file.text());
		setMessage(!result.ok
			? { kind: 'error', text: result.error.message }
			: result.persistence === 'saved'
				? { kind: 'success', text: studioMessage('project.imported', { name: result.project.name }) }
				: { kind: 'error', text: result.warning?.message ?? studioMessage('project.importedStorageUnavailable', { name: result.project.name }) });
		setFocusedPane(null);
	}

	function exportProject() {
		const result = controller.exportBundle();
		if (!result.ok) {
			setMessage({ kind: 'error', text: result.error.message });
			return;
		}
		const { fileName, contents } = result.value;
		const url = URL.createObjectURL(new Blob([contents], { type: 'application/json;charset=utf-8' }));
		const link = document.createElement('a');
		link.href = url;
		link.download = fileName;
		link.click();
		window.setTimeout(() => URL.revokeObjectURL(url), 0);
		const projectName = getSnapshot().activeProject?.name ?? studioMessage('project.select');
		setMessage({ kind: 'success', text: studioMessage('project.exported', { name: projectName }) });
	}

	return { openSource, cancelUpload, openUpload, saveSource, importProject, exportProject };
}

function supportsNativeSourceOpen() {
	const browser = globalThis as typeof globalThis & { showOpenFilePicker?: unknown };
	return typeof browser.showOpenFilePicker === 'function';
}

function errorMessage(error: unknown, key: Parameters<typeof studioMessage>[0]) {
	return error instanceof Error ? error.message : studioMessage(key);
}
