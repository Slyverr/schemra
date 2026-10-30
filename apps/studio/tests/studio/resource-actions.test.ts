import { describe, expect, it } from 'bun:test';
import { MAX_PROJECT_FILES, createInitialProject } from '../../src/lib/project/session.ts';
import { studioMessage } from '../../src/lib/i18n/messages.ts';
import { createWorkspaceController, type WorkspaceStorageAdapter } from '../../src/lib/project/workspace-controller.ts';
import { createResourceActions, type ProjectVisual, type ResourceActionSnapshot, type ResourceCompileFile, type ResourceDialogState } from '../../src/lib/studio/resource-actions.ts';

function setup(currentVisual: ProjectVisual | null = null, currentFiles: ResourceCompileFile[] = [], projectFileCount?: number) {
	const exampleProject = createInitialProject(
		'visual traffic_overview "Traffic overview" {}',
		'scenario "Request and response" {}'
	);
	const storage: WorkspaceStorageAdapter = {
		async loadStudioWorkspace() { return null; },
		async saveStudioWorkspace() {}
	};
	const controller = createWorkspaceController({ exampleProject, storage, autosaveDelayMs: 60_000 });
	const form: ResourceDialogState = { kind: null, nameDraft: '', error: '', target: null };
	let focusedPane: 'editor' | 'preview' | null = 'preview';
	let fileMessage = null;
	const getSnapshot = (): ResourceActionSnapshot => {
		const workspace = controller.getState();
		const project = workspace.projects.find(({ id }) => id === workspace.activeProjectId) ?? null;
		const activeProject = project && projectFileCount !== undefined
			? { ...project, files: Array.from({ length: projectFileCount }, (_, index) => project.files[index] ?? { ...project.files[0]!, id: `limit-file-${index}` }) }
			: project;
		return {
			workspace,
			activeProject,
			currentFiles,
			currentCompilation: null,
			currentVisual,
			selectedScenario: null,
			sourceFiles: activeProject?.files.map(({ id, path, source }) => ({ id, path, source })) ?? [],
			visualId: currentVisual?.id ?? null,
			compilePending: false,
			currentProjectValid: true
		};
	};
	const actions = createResourceActions({
		controller,
		form,
		getSnapshot,
		setFileMessage: (next) => { fileMessage = next; },
		setFocusedPane: (next) => { focusedPane = next; }
	});
	return { controller, form, actions, getSnapshot, getFocusedPane: () => focusedPane, getFileMessage: () => fileMessage };
}

describe('resource actions', () => {
	it('creates and selects a project, then closes the dialog', async () => {
		const app = setup();
		await app.controller.initialize();

		app.actions.openProjectCreation();
		expect(app.form.kind).toBe('create-project');
		app.form.nameDraft = 'Operations';
		await app.actions.save();

		const workspace = app.controller.getState();
		expect(workspace.projects.map(({ name }) => name)).toContain('Operations');
		expect(workspace.projects.find(({ id }) => id === workspace.activeProjectId)?.name).toBe('Operations');
		expect(app.form.kind).toBeNull();
		expect(app.getFocusedPane()).toBeNull();
		expect(app.getFileMessage()).toBeNull();
		await app.controller.dispose();
	});

	it('removes a selected source file and closes the dialog', async () => {
		const app = setup();
		await app.controller.initialize();
		const before = app.getSnapshot().activeProject!.files.length;

		app.actions.openResourceRemovalForFile('file-request-response', 'source', null);
		expect(app.form.kind).toBe('remove-resource');
		app.actions.remove();

		const files = app.getSnapshot().activeProject!.files;
		expect(files).toHaveLength(before - 1);
		expect(files.some(({ id }) => id === 'file-request-response')).toBe(false);
		expect(app.form.kind).toBeNull();
		expect(app.getFileMessage()).toBeNull();
		await app.controller.dispose();
	});

	it('writes the scenario display title into the generated DSL source', async () => {
		const currentVisual = {
			id: 'traffic_overview',
			output: { scenarios: [], scene: { edges: [{ from: 'client', to: 'proxy' }] } }
		} as unknown as ProjectVisual;
		const app = setup(currentVisual);
		await app.controller.initialize();

		app.actions.openResourceCreation('scenario');
		app.form.nameDraft = 'Operations flow';
		await app.actions.save();

		const scenario = app.getSnapshot().activeProject?.files.find(({ path }) => path.startsWith('scenarios/traffic_overview/'));
		expect(scenario?.source).toContain('scenario operations_flow "Operations flow" for visual traffic_overview');
		expect(scenario?.source).toContain('move request from client -> proxy');
		await app.controller.dispose();
	});

	it('rejects visual and scenario creation when the project file limit is reached', async () => {
		const app = setup(null, [], MAX_PROJECT_FILES);
		await app.controller.initialize();

		for (const kind of ['visual', 'scenario'] as const) {
			app.actions.openResourceCreation(kind);
			expect(app.form.kind).toBeNull();
			expect(app.getFileMessage()).toEqual({
				kind: 'error',
				text: studioMessage('project.sourceFileCountMaximum', { max: MAX_PROJECT_FILES })
			});
		}

		await app.controller.dispose();
	});

	it('removes only the visual source file and leaves linked scenario files in the project', async () => {
		const span = {
			start: { line: 1, column: 1, offset: 0 },
			end: { line: 1, column: 2, offset: 1 }
		};
		const currentFiles = [
			{
				id: 'file-traffic-overview',
				kind: 'visual',
				visual_id: 'traffic_overview',
				declarations: [{ kind: 'visual', id: 'traffic_overview', title: 'Traffic overview', visual_id: null, span }]
			},
			{
				id: 'file-request-response',
				kind: 'scenario',
				visual_id: 'traffic_overview',
				declarations: [{ kind: 'scenario', id: 'request-response', title: 'Request and response', visual_id: 'traffic_overview', span }]
			}
		] as unknown as ResourceCompileFile[];
		const app = setup(null, currentFiles);
		await app.controller.initialize();

		app.actions.openResourceRemovalForFile('file-traffic-overview', 'visual', currentFiles[0]!.declarations[0]!);
		app.actions.remove();

		const project = app.getSnapshot().activeProject!;
		expect(project.files.map(({ id }) => id)).toEqual(['file-request-response']);
		expect(project.files[0]?.path).toBe('scenarios/request-response.schemra');
		expect(app.form.kind).toBeNull();
		await app.controller.dispose();
	});
});
