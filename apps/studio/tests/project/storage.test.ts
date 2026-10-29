import { describe, expect, it } from 'bun:test';

type Handler = ((event: Event) => void) | null;

class FakeRequest<T = unknown> {
	result!: T;
	error: DOMException | null = null;
	onsuccess: Handler = null;
	onerror: Handler = null;
	onupgradeneeded: Handler = null;
	onblocked: Handler = null;

	succeed(result: T) {
		this.result = result;
		this.onsuccess?.(new Event('success'));
	}
}

class FakeTransaction {
	oncomplete: Handler = null;
	onabort: Handler = null;
	error: DOMException | null = null;
	private pending = 0;
	private aborted = false;
	private completionQueued = false;

	constructor(private readonly database: FakeIndexedDb) {}

	objectStore() {
		return {
			get: (key: IDBValidKey) => {
				const request = new FakeRequest();
				this.beginRequest();
				queueMicrotask(() => {
					if (this.aborted) return;
					request.succeed(clone(this.database.records.get(String(key))));
					this.finishRequest();
				});
				return request;
			},
			put: (value: unknown, key: IDBValidKey) => {
				const request = new FakeRequest<IDBValidKey>();
				this.beginRequest();
				queueMicrotask(() => {
					if (this.aborted) return;
					const error = this.database.takeNextPutError();
					if (error) {
						request.error = error;
						this.error = error;
						this.abort();
						return;
					}
					this.database.records.set(String(key), clone(value));
					request.succeed(key);
					this.finishRequest();
				});
				return request;
			}
		};
	}

	abort() {
		if (this.aborted) return;
		this.aborted = true;
		queueMicrotask(() => this.onabort?.(new Event('abort')));
	}

	private beginRequest() {
		this.pending += 1;
	}

	private finishRequest() {
		this.pending -= 1;
		if (this.pending !== 0 || this.aborted || this.completionQueued) return;
		this.completionQueued = true;
		queueMicrotask(() => {
			if (!this.aborted) this.oncomplete?.(new Event('complete'));
		});
	}
}

class FakeIndexedDb {
	readonly records = new Map<string, unknown>();
	nextPutError: DOMException | null = null;
	private readonly database = {
		objectStoreNames: { contains: () => true },
		createObjectStore: () => undefined,
		transaction: () => new FakeTransaction(this),
		close: () => undefined,
		onversionchange: null as Handler,
		onclose: null as Handler
	};

	open() {
		const request = new FakeRequest<typeof this.database>();
		queueMicrotask(() => {
			request.result = this.database;
			request.onupgradeneeded?.(new Event('upgradeneeded'));
			request.onsuccess?.(new Event('success'));
		});
		return request;
	}

	takeNextPutError() {
		const error = this.nextPutError;
		this.nextPutError = null;
		return error;
	}

	close() {
		this.database.onclose?.(new Event('close'));
	}
}

function clone<T>(value: T): T {
	return value === undefined ? value : structuredClone(value);
}

describe('Studio workspace storage', () => {
	it('refreshes only the bundled Example project once for the current DSL seed', async () => {
		const { refreshBundledExampleProject, BUNDLED_EXAMPLE_PROJECT_REVISION } = await import(
			'../../src/lib/project/storage.ts'
		);
		const bundledExample = {
			id: 'project-example',
			name: 'Example project',
			files: [
				{ id: 'file-traffic-overview', path: 'visuals/traffic-overview.schemra', source: 'visual traffic_overview "Traffic overview" {}' },
				{ id: 'file-request-response', path: 'scenarios/request-response.schemra', source: 'scenario "Request and response" {}' },
				{ id: 'file-everything', path: 'scenarios/everything.schemra', source: 'scenario "Everything" {}' },
				{ id: 'file-data-lifecycle', path: 'scenarios/data-lifecycle.schemra', source: 'scenario "Data lifecycle" {}' },
				{ id: 'file-markers', path: 'scenarios/markers.schemra', source: 'scenario "Marker styles" {}' },
				{ id: 'file-service-flow', path: 'scenarios/service-flow.schemra', source: 'scenario "Service flow" {}' }
			].map((file) => ({ ...file, savedSource: file.source, handle: null, mode: 'example' as const }))
		};
		const saved = {
			projects: [
				{
					id: 'project-example',
					name: 'Example project',
					files: [{
						id: 'file-traffic-overview',
						path: 'visuals/traffic-overview.schemra',
						source: 'visual "old source" {}'
					}]
				},
				{
					id: 'project-personal',
					name: 'Personal project',
					files: [{ id: 'personal-visual', path: 'visuals/personal.schemra', source: 'kept' }]
				}
			],
			activeProjectId: 'project-personal',
			selectedFileByProject: {
				'project-example': 'file-continuous-traffic',
				'project-personal': 'personal-visual'
			},
			selectedScenarioByProject: {
				'project-example': { continuous_traffic: 'Old scenario title' },
				'project-personal': { personal_visual: 'Personal scenario' }
			}
		};

		const refreshed = refreshBundledExampleProject(saved, bundledExample);
		expect(refreshed?.projects[0]?.id).toBe('project-example');
		expect(refreshed?.projects[0]?.files[0]).toMatchObject({
			id: 'file-traffic-overview',
			path: 'visuals/traffic-overview.schemra',
			source: 'visual traffic_overview "Traffic overview" {}'
		});
		expect(refreshed?.projects[0]?.files).toHaveLength(6);
		expect(refreshed?.projects[1]).toEqual(saved.projects[1]);
		expect(refreshed?.activeProjectId).toBe('project-personal');
		expect(refreshed?.selectedFileByProject['project-example']).toBe('file-traffic-overview');
		expect(refreshed?.selectedScenarioByProject).toEqual({
			'project-personal': { personal_visual: 'Personal scenario' }
		});
		expect(refreshed?.exampleProjectRevision).toBe(BUNDLED_EXAMPLE_PROJECT_REVISION);
		expect(refreshBundledExampleProject(refreshed!, bundledExample)).toBeNull();
		expect(saved.projects[0].files[0].source).toBe('visual "old source" {}');
		expect(refreshBundledExampleProject({ ...saved, projects: [saved.projects[1]] }, bundledExample)).toBeNull();
	});

	it('recovers from temporary storage failures and refuses to overwrite a newer tab revision', async () => {
		const indexedDb = new FakeIndexedDb();
		Object.defineProperty(globalThis, 'indexedDB', {
			configurable: true,
			value: indexedDb as unknown as IDBFactory
		});

		const storage = await import('../../src/lib/project/storage.ts');
		const initial = {
			projects: [{
				id: 'project-one',
				name: 'Project one',
				files: [{ id: 'visual-one', path: 'visuals/one.schemra', source: 'visual "One" {}' }]
			}],
			activeProjectId: 'project-one',
			selectedFileByProject: { 'project-one': 'visual-one' },
			selectedScenarioByProject: {}
		};

		expect(await storage.loadStudioWorkspace()).toBeNull();
		await storage.saveStudioWorkspace(initial);
		expect(await storage.loadStudioWorkspace()).toEqual(initial);

		const savedRecord = indexedDb.records.get('current') as {
			schemaVersion: number;
			revision: number;
			snapshot: typeof initial;
		};
		const { selectedScenarioByProject: _scenarioSelection, ...legacySnapshot } = initial;
		indexedDb.records.set('current', { ...savedRecord, snapshot: legacySnapshot });
		expect(await storage.loadStudioWorkspace()).toEqual(initial);

		const scenarioSelection = {
			...initial,
			projects: [
				...initial.projects,
				{
					id: 'project-two',
					name: 'Project two',
					files: [{ id: 'visual-two', path: 'visuals/one.schemra', source: 'visual "One" {}' }]
				}
			],
			selectedFileByProject: { ...initial.selectedFileByProject, 'project-two': 'visual-two' },
			selectedScenarioByProject: {
				'project-one': { 'visual-one': 'First request' },
				'project-two': { 'visual-one': 'Second request' }
			}
		};
		await storage.saveStudioWorkspace(scenarioSelection);
		expect(await storage.loadStudioWorkspace()).toEqual(scenarioSelection);

		const locallyEdited = {
			...scenarioSelection,
			projects: scenarioSelection.projects.map((project, index) =>
				index === 0 ? { ...project, name: 'Local edit' } : project
			)
		};
		await storage.saveStudioWorkspace(locallyEdited);
		expect(await storage.loadStudioWorkspace()).toEqual(locallyEdited);

		const retrySnapshot = {
			...locallyEdited,
			projects: locallyEdited.projects.map((project) => ({ ...project, name: 'Recovered edit' }))
		};
		indexedDb.nextPutError = new DOMException('Storage quota exceeded', 'QuotaExceededError');
		await expect(storage.saveStudioWorkspace(retrySnapshot)).rejects.toBeInstanceOf(
			storage.StudioWorkspaceStorageError
		);
		expect((indexedDb.records.get('current') as { snapshot: typeof retrySnapshot }).snapshot).toEqual(locallyEdited);
		await storage.saveStudioWorkspace(retrySnapshot);
		expect(await storage.loadStudioWorkspace()).toEqual(retrySnapshot);

		const externalRecord = indexedDb.records.get('current') as {
			schemaVersion: number;
			revision: number;
			snapshot: typeof retrySnapshot;
		};
		const externalSnapshot = { ...retrySnapshot, activeProjectId: null };
		indexedDb.records.set('current', {
			...externalRecord,
			revision: externalRecord.revision + 1,
			snapshot: externalSnapshot
		});

		await expect(storage.saveStudioWorkspace({
			...retrySnapshot,
			projects: [{ ...retrySnapshot.projects[0], name: 'Stale tab edit' }]
		}))
			.rejects.toBeInstanceOf(storage.StudioWorkspaceConflictError);
		expect((indexedDb.records.get('current') as typeof externalRecord).snapshot.activeProjectId).toBeNull();

		indexedDb.close();
		Object.defineProperty(globalThis, 'indexedDB', { configurable: true, value: undefined });
		await expect(storage.loadStudioWorkspace()).rejects.toBeInstanceOf(storage.StudioWorkspaceStorageError);
		Object.defineProperty(globalThis, 'indexedDB', {
			configurable: true,
			value: indexedDb as unknown as IDBFactory
		});
		expect(await storage.loadStudioWorkspace()).toEqual(externalSnapshot);

		indexedDb.close();
		Object.defineProperty(globalThis, 'indexedDB', {
			configurable: true,
			value: { open: () => new FakeRequest() } as unknown as IDBFactory
		});
		const originalSetTimeout = globalThis.setTimeout;
		globalThis.setTimeout = ((callback: TimerHandler, delay?: number, ...args: unknown[]) =>
			originalSetTimeout(callback, delay === storage.WORKSPACE_STORAGE_TIMEOUT_MS ? 0 : delay, ...args)
		) as typeof globalThis.setTimeout;
		try {
			const stuckRestore = storage.loadStudioWorkspace();
			await expect(stuckRestore).rejects.toThrow('timed out while opening browser storage');

			const stuckTransaction = {
				objectStoreNames: { contains: () => true },
				transaction: () => ({
					objectStore: () => ({ get: () => new FakeRequest() }),
					abort: () => undefined,
					oncomplete: null as Handler,
					onabort: null as Handler,
					error: null as DOMException | null
				}),
				close: () => undefined,
				onversionchange: null as Handler,
				onclose: null as Handler
			};
			Object.defineProperty(globalThis, 'indexedDB', {
				configurable: true,
				value: {
					open: () => {
						const request = new FakeRequest<typeof stuckTransaction>();
						queueMicrotask(() => request.succeed(stuckTransaction));
						return request;
					}
				} as unknown as IDBFactory
			});
			const stuckRead = storage.loadStudioWorkspace();
			await expect(stuckRead).rejects.toThrow('timed out while reading saved data');
		} finally {
			globalThis.setTimeout = originalSetTimeout;
			Object.defineProperty(globalThis, 'indexedDB', { configurable: true, value: indexedDb });
		}
	});
});
