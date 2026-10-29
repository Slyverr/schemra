import { describe, expect, it } from 'bun:test';
import { createInitialProject, serializeProjectBundle, type StudioProject } from '../../src/lib/project/session.ts';
import {
	BUNDLED_EXAMPLE_PROJECT_REVISION,
	StudioWorkspaceConflictError,
	type StudioWorkspaceSnapshot
} from '../../src/lib/project/storage.ts';
import { createWorkspaceController, type WorkspaceStorageAdapter } from '../../src/lib/project/workspace-controller.ts';

function exampleProject(): StudioProject {
	return createInitialProject('visual traffic_overview "Traffic overview" {}', 'scenario "Request and response" {}', [
		{ id: 'file-everything', path: 'scenarios/everything.schemra', source: 'scenario "Everything" {}' },
		{ id: 'file-data-lifecycle', path: 'scenarios/data-lifecycle.schemra', source: 'scenario "Data lifecycle" {}' },
		{ id: 'file-markers', path: 'scenarios/markers.schemra', source: 'scenario "Marker styles" {}' },
		{ id: 'file-service-flow', path: 'scenarios/service-flow.schemra', source: 'scenario "Service flow" {}' }
	]);
}

function memoryStorage(initial: StudioWorkspaceSnapshot | null = null) {
	let snapshot = initial ? structuredClone(initial) : null;
	const writes: StudioWorkspaceSnapshot[] = [];
	const adapter: WorkspaceStorageAdapter = {
		async loadStudioWorkspace() {
			return snapshot ? structuredClone(snapshot) : null;
		},
		async saveStudioWorkspace(next) {
			snapshot = structuredClone(next);
			writes.push(structuredClone(next));
		}
	};
	return {
		adapter,
		writes,
		read: () => snapshot ? structuredClone(snapshot) : null
	};
}

describe('workspace controller initialization', () => {
	it('seeds the canonical example and persists it without browser-specific APIs', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter });

		const initialized = await controller.initialize();
		expect(initialized.ok).toBe(true);
		if (!initialized.ok) return;
		expect(initialized.value.activeProjectId).toBe('project-example');
		expect(initialized.value.projects[0]?.files.map(({ id, path }) => [id, path])).toEqual([
			['file-traffic-overview', 'visuals/traffic-overview.schemra'],
			['file-request-response', 'scenarios/request-response.schemra'],
			['file-everything', 'scenarios/everything.schemra'],
			['file-data-lifecycle', 'scenarios/data-lifecycle.schemra'],
			['file-markers', 'scenarios/markers.schemra'],
			['file-service-flow', 'scenarios/service-flow.schemra']
		]);

		const saved = await controller.flush();
		expect(saved.ok).toBe(true);
		expect(storage.read()?.exampleProjectRevision).toBe(BUNDLED_EXAMPLE_PROJECT_REVISION);
		expect(storage.writes).toHaveLength(1);
	});

	it('refreshes an outdated bundled seed while retaining user projects and active selection', async () => {
		const oldExample = exampleProject();
		oldExample.files = [
			{ ...oldExample.files[0]!, id: 'file-traffic-overview', path: 'visuals/old-traffic.schemra', source: 'old', savedSource: 'old' }
		];
		const saved: StudioWorkspaceSnapshot = {
			projects: [
				{ id: 'project-example', name: 'Example project', files: [{ id: 'file-traffic-overview', path: 'visuals/old-traffic.schemra', source: 'old' }] },
				{ id: 'project-personal', name: 'Personal project', files: [{ id: 'file-personal', path: 'visuals/personal.schemra', source: 'personal' }] }
			],
			activeProjectId: 'project-personal',
			selectedFileByProject: { 'project-personal': 'file-personal' },
			selectedScenarioByProject: { 'project-example': { traffic_overview: 'old' }, 'project-personal': { personal: 'kept' } },
			exampleProjectRevision: BUNDLED_EXAMPLE_PROJECT_REVISION - 1
		};
		const storage = memoryStorage(saved);
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });

		const initialized = await controller.initialize();
		expect(initialized.ok).toBe(true);
		const state = controller.getState();
		expect(state.projects.find((project) => project.id === 'project-example')?.files).toHaveLength(6);
		expect(state.projects.find((project) => project.id === 'project-personal')?.files[0]?.source).toBe('personal');
		expect(state.activeProjectId).toBe('project-personal');
		expect(state.selectedScenarioByProject).toEqual({ 'project-personal': { personal: 'kept' } });
		await controller.flush();
		expect(storage.read()?.exampleProjectRevision).toBe(BUNDLED_EXAMPLE_PROJECT_REVISION);
	});
});

describe('workspace controller actions', () => {
	it('renames projects and resolves case-insensitive name collisions', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		controller.createProject('Personal');
		const personal = controller.getState().projects.find(({ name }) => name === 'Personal')!;

		const renamed = controller.renameProject(personal.id, 'example PROJECT');
		expect(renamed.ok).toBe(true);
		if (renamed.ok) expect(renamed.value.name).toBe('example PROJECT 2');
		const invalid = controller.renameProject(personal.id, '   ');
		expect(invalid.ok).toBe(false);
		if (!invalid.ok) expect(invalid.error.kind).toBe('invalid');
		expect(controller.getState().projects.find(({ id }) => id === personal.id)?.name).toBe('example PROJECT 2');
		await controller.flush();
		expect(storage.read()?.projects.find(({ id }) => id === personal.id)?.name).toBe('example PROJECT 2');
	});

	it('creates visuals with unique paths and selects the new document', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();

		const first = controller.createVisual('project-example', 'Payments API');
		const second = controller.createVisual('project-example', 'Payments API');
		expect(first.ok).toBe(true);
		expect(second.ok).toBe(true);
		if (!first.ok || !second.ok) return;
		expect(first.value.path).toBe('visuals/payments-api.schemra');
		expect(second.value.path).toBe('visuals/payments-api-2.schemra');
		expect(second.value.source).toContain('"Payments API"');
		expect(controller.getState().selectedFileByProject['project-example']).toBe(second.value.id);
		await controller.flush();
		expect(storage.read()?.projects[0]?.files.some(({ path }) => path === 'visuals/payments-api-2.schemra')).toBe(true);
	});

	it('creates a linked scenario and selects its visual scenario identity', async () => {
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: memoryStorage().adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		const created = controller.createScenario('project-example', {
			visualId: 'traffic_overview',
			scenarioId: 'request_2',
			title: 'Request 2',
			source: 'scenario request_2 "Request 2" for visual traffic_overview { wait 500ms }'
		});
		expect(created.ok).toBe(true);
		if (!created.ok) return;
		expect(created.value.path).toBe('scenarios/traffic_overview/request-2.schemra');
		expect(controller.getState().selectedFileByProject['project-example']).toBe(created.value.id);
		expect(controller.getState().selectedScenarioByProject['project-example']?.traffic_overview).toBe('request_2');
		const collision = controller.createScenario('project-example', {
			visualId: 'traffic_overview',
			scenarioId: 'request_2_2',
			title: 'Request 2',
			source: 'scenario request_2_2 for visual traffic_overview { wait 500ms }'
		});
		expect(collision.ok).toBe(true);
		if (collision.ok) expect(collision.value.path).toBe('scenarios/traffic_overview/request-2-2.schemra');
	});

	it('adds local source documents without serializing browser file handles', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		const handle = {
			name: 'traffic-overview.schemra',
			async getFile() { return new File(['visual traffic_overview {}'], this.name); },
			async createWritable() { return { async write() {}, async close() {} }; }
		};

		const added = controller.addDocument('project-example', {
			path: 'visuals/traffic-overview.schemra',
			source: 'visual local_copy {}',
			handle,
			mode: 'local'
		});
		expect(added.ok).toBe(true);
		if (!added.ok) return;
		expect(added.value).toMatchObject({
			path: 'visuals/traffic-overview-2.schemra',
			source: 'visual local_copy {}',
			savedSource: 'visual local_copy {}',
			handle,
			mode: 'local'
		});
		await controller.flush();
		const persisted = storage.read()?.projects[0]?.files.at(-1);
		expect(persisted).toEqual({ path: added.value.path, source: added.value.source, id: added.value.id });
		expect(persisted).not.toHaveProperty('handle');
	});

	it('records the exact source snapshot written without replacing edits made during the write', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		await controller.flush();
		const handle = {
			name: 'renamed.schemra',
			async getFile() { return new File(['written snapshot'], this.name); },
			async createWritable() { return { async write() {}, async close() {} }; }
		};

		controller.updateSource('project-example', 'file-traffic-overview', 'newer editor contents');
		const marked = controller.markSourceSaved('project-example', 'file-traffic-overview', 'written snapshot', {
			name: handle.name, handle, mode: 'local'
		});

		expect(marked.ok).toBe(true);
		const file = controller.getState().projects[0]?.files.find(({ id }) => id === 'file-traffic-overview');
		expect(file).toMatchObject({
			path: 'visuals/renamed.schemra',
			source: 'newer editor contents',
			savedSource: 'written snapshot',
			handle,
			mode: 'local'
		});
		await controller.flush();
		const persisted = storage.read()?.projects[0]?.files.find(({ id }) => id === 'file-traffic-overview');
		expect(persisted).toEqual({ id: 'file-traffic-overview', path: 'visuals/renamed.schemra', source: 'newer editor contents' });
		expect(persisted).not.toHaveProperty('handle');
	});

	it('removes linked scenario files without clearing a different selected scenario', async () => {
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: memoryStorage().adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		controller.selectScenario('project-example', 'traffic_overview', 'request-response');
		const added = controller.createScenario('project-example', {
			visualId: 'traffic_overview', scenarioId: 'extra', title: 'Extra',
			source: 'scenario extra "Extra" for visual traffic_overview { wait 500ms }'
		});
		expect(added.ok).toBe(true);
		if (!added.ok) return;
		controller.selectScenario('project-example', 'traffic_overview', 'request-response');
		const result = controller.removeDocuments('project-example', [added.value.id], {
			scenarioSelectionsToClear: [{ visualId: 'traffic_overview', scenarioId: 'extra' }]
		});
		expect(result.ok).toBe(true);
		expect(controller.getState().projects[0]?.files.some(({ id }) => id === added.value.id)).toBe(false);
		expect(controller.getState().selectedScenarioByProject['project-example']?.traffic_overview).toBe('request-response');

		const selected = controller.createScenario('project-example', {
			visualId: 'traffic_overview', scenarioId: 'extra', title: 'Extra',
			source: 'scenario extra "Extra" for visual traffic_overview { wait 500ms }'
		});
		expect(selected.ok).toBe(true);
		if (!selected.ok) return;
		const removeSelected = controller.removeDocuments('project-example', [selected.value.id], {
			scenarioSelectionsToClear: [{ visualId: 'traffic_overview', scenarioId: 'extra' }]
		});
		expect(removeSelected.ok).toBe(true);
		expect(controller.getState().selectedScenarioByProject['project-example']?.traffic_overview).toBeNull();
	});

	it('removes a visual with its linked scenarios but refuses to remove the last visual', async () => {
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: memoryStorage().adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		const otherVisual = controller.createVisual('project-example', 'Other visual');
		expect(otherVisual.ok).toBe(true);
		if (!otherVisual.ok) return;
		const before = controller.getState().projects[0]!;
		const blocked = controller.removeDocuments('project-example', ['file-traffic-overview'], {
			visualIdsToRemove: ['traffic_overview'],
			remainingVisualIds: []
		});
		expect(blocked.ok).toBe(false);
		if (!blocked.ok) expect(blocked.error.message).toContain('at least one visual');
		expect(controller.getState().projects[0]?.files.map(({ id }) => id)).toEqual(before.files.map(({ id }) => id));

		const removed = controller.removeDocuments('project-example', ['file-traffic-overview', 'file-request-response'], {
			visualIdsToRemove: ['traffic_overview'],
			remainingVisualIds: ['other_visual']
		});
		expect(removed.ok).toBe(true);
		const state = controller.getState();
		expect(state.projects[0]?.files.some(({ id }) => id === 'file-traffic-overview')).toBe(false);
		expect(state.projects[0]?.files.some(({ id }) => id === 'file-request-response')).toBe(false);
		expect(state.selectedScenarioByProject['project-example']?.traffic_overview).toBeUndefined();
		expect(state.selectedFileByProject['project-example']).toBe(otherVisual.value.id);
	});

	it('debounces edits, preserves the file baseline, and exports current project source', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		await controller.flush();

		const edit = controller.updateSource('project-example', 'file-traffic-overview', 'first edit');
		expect(edit.ok).toBe(true);
		controller.updateSource('project-example', 'file-traffic-overview', 'second edit');
		expect(storage.writes).toHaveLength(1);
		const current = controller.getState().projects[0]!.files[0]!;
		expect(current.source).toBe('second edit');
		expect(current.savedSource).toBe('visual traffic_overview "Traffic overview" {}');

		const exported = controller.exportBundle();
		expect(exported.ok).toBe(true);
		if (exported.ok) {
			expect(exported.value.fileName).toBe('example-project.schemra-project.json');
			expect(JSON.parse(exported.value.contents).files[0].source).toBe('second edit');
		}
		await controller.flush();
		expect(storage.read()?.projects[0]?.files[0]?.source).toBe('second edit');
		await controller.dispose();
	});

	it('serializes overlapping saves in order and leaves the newest edit persisted', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		await controller.flush();

		let writeCount = 1;
		let releaseFirst!: () => void;
		let releaseSecond!: () => void;
		storage.adapter.saveStudioWorkspace = async (snapshot) => {
			writeCount += 1;
			if (writeCount === 1) return;
			if (writeCount === 2) await new Promise<void>((resolve) => { releaseFirst = resolve; });
			if (writeCount === 3) await new Promise<void>((resolve) => { releaseSecond = resolve; });
			storage.writes.push(structuredClone(snapshot));
		};

		controller.updateSource('project-example', 'file-traffic-overview', 'first concurrent edit');
		const firstSave = controller.flush();
		await new Promise((resolve) => setTimeout(resolve, 0));
		controller.updateSource('project-example', 'file-traffic-overview', 'latest concurrent edit');
		const latestSave = controller.flush();
		releaseFirst();
		await new Promise((resolve) => setTimeout(resolve, 0));
		releaseSecond();

		expect((await firstSave).ok).toBe(true);
		expect((await latestSave).ok).toBe(true);
		expect(storage.writes.at(-1)?.projects[0]?.files[0]?.source).toBe('latest concurrent edit');
		expect(controller.getState().saveStatus).toBe('saved');
	});

	it('returns import validation and persistence outcomes without losing imported state', async () => {
		const storage = memoryStorage();
		const controller = createWorkspaceController({ exampleProject: exampleProject(), storage: storage.adapter });
		await controller.initialize();
		await controller.flush();

		const before = controller.getState().projects.length;
		const invalid = await controller.importBundle('not json');
		expect(invalid.ok).toBe(false);
		if (!invalid.ok) expect(invalid.error.kind).toBe('invalid');
		expect(controller.getState().projects).toHaveLength(before);

		const source = serializeProjectBundle({
			id: 'project-import',
			name: 'Example project',
			files: [{ id: 'file-import', path: 'visuals/import.schemra', source: 'visual imported "Imported" {}', savedSource: '', handle: null, mode: 'memory' }]
		});
		const imported = await controller.importBundle(source);
		expect(imported.ok).toBe(true);
		if (imported.ok) {
			expect(imported.project.name).toBe('Example project 2');
			expect(imported.persistence).toBe('saved');
		}
		expect(controller.getState().activeProjectId).toBe(imported.ok ? imported.project.id : null);
	});

	it('reports cross-tab conflicts as a distinct state and result', async () => {
		const example = exampleProject();
		const initial: StudioWorkspaceSnapshot = {
			projects: [{
				id: example.id,
				name: example.name,
				files: example.files.map(({ id, path, source }) => ({ id, path, source }))
			}],
			activeProjectId: example.id,
			selectedFileByProject: { [example.id]: example.files[0]!.id },
			selectedScenarioByProject: {},
			exampleProjectRevision: BUNDLED_EXAMPLE_PROJECT_REVISION
		};
		const storage = memoryStorage(initial);
		storage.adapter.saveStudioWorkspace = async () => {
			throw new StudioWorkspaceConflictError('workspace conflict');
		};
		const controller = createWorkspaceController({ exampleProject: example, storage: storage.adapter, autosaveDelayMs: 60_000 });
		await controller.initialize();
		controller.updateSource(example.id, example.files[0]!.id, 'updated');

		const result = await controller.flush();
		expect(result.ok).toBe(false);
		if (!result.ok) expect(result.error.kind).toBe('conflict');
		expect(controller.getState().saveStatus).toBe('conflict');
		expect(controller.getState().error?.message).toBe('workspace conflict');
	});
});
