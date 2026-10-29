import { describe, expect, it } from 'bun:test';
import { createInitialProject } from '../../src/lib/project/session.ts';
import type { OpenLocalSourceResult, SaveLocalSourceOptions, SaveLocalSourceResult } from '../../src/lib/project/local-source.ts';
import { createWorkspaceController, type WorkspaceStorageAdapter } from '../../src/lib/project/workspace-controller.ts';
import { createSourceActions, type SourceActionMessage, type SourceActionState, type SourceFileOperations } from '../../src/lib/studio/source-actions.ts';

function setup() {
	const exampleProject = createInitialProject('visual traffic_overview "Traffic overview" {}', 'scenario "Request and response" {}');
	const storage: WorkspaceStorageAdapter = {
		async loadStudioWorkspace() { return null; },
		async saveStudioWorkspace() {}
	};
	const controller = createWorkspaceController({ exampleProject, storage, autosaveDelayMs: 60_000 });
	const state: SourceActionState = { operation: null, pendingProjectId: null };
	let focusedPane: 'editor' | 'preview' | null = 'preview';
	let message: SourceActionMessage = null;
	const operations: SourceFileOperations = {
		async open(): Promise<OpenLocalSourceResult> { return { status: 'cancelled' }; },
		async readUpload(file) {
			return file
				? { status: 'opened', document: { name: file.name, source: 'visual imported "Imported" {}', handle: null, mode: 'download' } }
				: { status: 'cancelled' };
		},
		async save(options: SaveLocalSourceOptions): Promise<SaveLocalSourceResult> {
			return { status: 'saved', name: options.name, handle: null, mode: 'download' };
		}
	};
	const getSnapshot = () => {
		const workspace = controller.getState();
		const activeProject = workspace.projects.find(({ id }) => id === workspace.activeProjectId) ?? null;
		const fileId = activeProject ? workspace.selectedFileByProject[activeProject.id] : null;
		const activeFile = activeProject?.files.find(({ id }) => id === fileId) ?? null;
		return { workspace, activeProject, activeFile };
	};
	const actions = createSourceActions({
		controller,
		state,
		getSnapshot,
		getSourceInput: () => undefined,
		setMessage: (next) => { message = next; },
		setFocusedPane: (next) => { focusedPane = next; },
		operations,
		supportsNativeOpen: () => true
	});
	return { controller, state, actions, getSnapshot, getMessage: () => message, getFocusedPane: () => focusedPane };
}

describe('source actions', () => {
	it('adds an uploaded source to the active project and focuses the editor', async () => {
		const app = setup();
		await app.controller.initialize();
		const input = { files: [{ name: 'imported.schemra' }], value: 'imported.schemra' };

		await app.actions.openUpload({ currentTarget: input } as unknown as Event);

		const { activeProject } = app.getSnapshot();
		expect(activeProject?.files.some(({ path, source }) => path === 'visuals/imported.schemra' && source === 'visual imported "Imported" {}')).toBe(true);
		expect(app.getFocusedPane()).toBeNull();
		expect(app.state.operation).toBeNull();
		expect(app.getMessage()?.kind).toBe('success');
		await app.controller.dispose();
	});

	it('marks the exact written source as saved', async () => {
		const app = setup();
		await app.controller.initialize();
		const { activeFile } = app.getSnapshot();
		if (!activeFile) throw new Error('Expected an active source file');
		const writtenSource = `${activeFile.source}\n// saved`;
		app.controller.updateSource(app.getSnapshot().activeProject!.id, activeFile.id, writtenSource);

		await app.actions.saveSource();

		const savedFile = app.getSnapshot().activeProject?.files.find(({ id }) => id === activeFile.id);
		expect(savedFile?.savedSource).toBe(writtenSource);
		expect(app.state.operation).toBeNull();
		expect(app.getMessage()?.kind).toBe('success');
		await app.controller.dispose();
	});
});
