import {
	MAX_PROJECT_FILES,
	normalizeProjectFilePath,
	normalizeProjectName,
	type StudioProject
} from './session.js';
import { studioMessage, type StudioMessageId } from '../i18n/messages.js';

const DATABASE_NAME = 'schemra-studio-workspace';
const DATABASE_VERSION = 1;
const OBJECT_STORE = 'workspace';
const WORKSPACE_KEY = 'current';
const SNAPSHOT_VERSION = 1;
export const WORKSPACE_STORAGE_TIMEOUT_MS = 10_000;
export const BUNDLED_EXAMPLE_PROJECT_REVISION = 13;

/** Serializable source-file fields; browser file-system handles are intentionally omitted. */
export interface StudioWorkspaceFileSnapshot {
	id: string;
	path: string;
	source: string;
}

export interface StudioWorkspaceProjectSnapshot {
	id: string;
	name: string;
	files: StudioWorkspaceFileSnapshot[];
}

export interface StudioWorkspaceSnapshot {
	projects: StudioWorkspaceProjectSnapshot[];
	activeProjectId: string | null;
	selectedFileByProject: Record<string, string>;
	selectedScenarioByProject: Record<string, Record<string, string | null>>;
	/** Revision of the bundled Example project template already applied to this workspace. */
	exampleProjectRevision?: number;
}

/**
 * Refreshes only the built-in Example project when its seed revision is stale.
 * User projects and the existing selection are retained; the example's files are
 * intentionally replaced because this project is the canonical DSL smoke-test.
 */
export function refreshBundledExampleProject(
	snapshot: StudioWorkspaceSnapshot,
	bundledProject: StudioProject
): StudioWorkspaceSnapshot | null {
	if ((snapshot.exampleProjectRevision ?? 0) >= BUNDLED_EXAMPLE_PROJECT_REVISION) return null;
	const existingProject = snapshot.projects.find((project) => project.id === bundledProject.id);
	if (!existingProject) return null;

	const files = bundledProject.files.map(({ id, path, source }) => ({ id, path, source }));
	const previousSelection = snapshot.selectedFileByProject[bundledProject.id];
	const selectedFile = files.some((file) => file.id === previousSelection)
		? previousSelection
		: files[0]?.id;
	const selectedFileByProject = { ...snapshot.selectedFileByProject };
	if (selectedFile) selectedFileByProject[bundledProject.id] = selectedFile;
	else delete selectedFileByProject[bundledProject.id];
	const selectedScenarioByProject = { ...snapshot.selectedScenarioByProject };
	delete selectedScenarioByProject[bundledProject.id];

	return {
		...snapshot,
		projects: snapshot.projects.map((project) => project.id === bundledProject.id
			? { ...project, name: bundledProject.name, files }
			: project),
		selectedFileByProject,
		selectedScenarioByProject,
		exampleProjectRevision: BUNDLED_EXAMPLE_PROJECT_REVISION
	};
}

export class StudioWorkspaceStorageError extends Error {
	constructor(message: string) {
		super(message);
		this.name = 'StudioWorkspaceStorageError';
	}
}

interface StoredWorkspace {
	schemaVersion: typeof SNAPSHOT_VERSION;
	/** Missing on records written before optimistic concurrency was added. */
	revision?: number;
	snapshot: StudioWorkspaceSnapshot;
}

let databasePromise: Promise<IDBDatabase> | null = null;
let databaseConnection: IDBDatabase | null = null;
let pendingWrites: Promise<void> = Promise.resolve();
let lastPersistedSnapshotKey: string | null = null;
// null means the database had no workspace; undefined means we have no safe baseline.
let workspaceRevision: number | null | undefined;
let pendingWriteSnapshotKey: string | null = null;
let pendingWritePromise: Promise<void> | null = null;

/** Loads the saved Studio workspace, or returns null when this browser has no snapshot yet. */
export async function loadStudioWorkspace(): Promise<StudioWorkspaceSnapshot | null> {
	await withStorageDeadline(pendingWrites, 'storage.phase.waitingForWrites');
	workspaceRevision = undefined;
	lastPersistedSnapshotKey = null;
	const database = await openDatabase();
	let stored: unknown;

	try {
		stored = await readWorkspaceRecord(database);
	} catch (error) {
		throw asStorageError('read', error);
	}

	if (stored === undefined) {
		workspaceRevision = null;
		return null;
	}
	if (!isRecord(stored) || stored.schemaVersion !== SNAPSHOT_VERSION || !('snapshot' in stored)) {
		throw new StudioWorkspaceStorageError(studioMessage('storage.unsupportedFormat'));
	}

	try {
		const snapshot = normalizeSnapshot(stored.snapshot);
		workspaceRevision = stored.revision === undefined ? 0 : requireRevision(stored.revision);
		lastPersistedSnapshotKey = snapshotKey(snapshot);
		return snapshot;
	} catch (error) {
		throw new StudioWorkspaceStorageError(
			studioMessage('storage.invalidWorkspace', {
				details: errorMessage(error, studioMessage('storage.validationFailed'))
			})
		);
	}
}

/** Saves a detached, validated snapshot; concurrent calls are committed in invocation order. */
export function saveStudioWorkspace(snapshot: StudioWorkspaceSnapshot): Promise<void> {
	let normalized: StudioWorkspaceSnapshot;
	try {
		normalized = normalizeSnapshot(snapshot);
	} catch (error) {
		return Promise.reject(error);
	}

	const key = snapshotKey(normalized);
	if (pendingWritePromise && key === pendingWriteSnapshotKey) return pendingWritePromise;
	if (!pendingWritePromise && key === lastPersistedSnapshotKey) return Promise.resolve();

	const write = pendingWrites.then(async () => {
		if (key === lastPersistedSnapshotKey) return;
		workspaceRevision = await persistWorkspace(normalized, workspaceRevision, lastPersistedSnapshotKey);
		lastPersistedSnapshotKey = key;
	});
	pendingWrites = write.catch(() => undefined);
	pendingWriteSnapshotKey = key;
	pendingWritePromise = write;
	const clearPendingWrite = () => {
		if (pendingWritePromise !== write) return;
		pendingWritePromise = null;
		pendingWriteSnapshotKey = null;
	};
	void write.then(clearPendingWrite, clearPendingWrite);
	return write;
}

function snapshotKey(snapshot: StudioWorkspaceSnapshot): string {
	return JSON.stringify(snapshot);
}

function withStorageDeadline<T>(operation: Promise<T>, phase: RestorePhaseMessage): Promise<T> {
	return new Promise((resolve, reject) => {
		const timeout = setTimeout(() => {
			reject(new StudioWorkspaceStorageError(studioMessage('storage.restoreTimedOut', {
				phase: studioMessage(phase)
			})));
		}, WORKSPACE_STORAGE_TIMEOUT_MS);
		operation.then(
			(value) => {
				clearTimeout(timeout);
				resolve(value);
			},
			(error: unknown) => {
				clearTimeout(timeout);
				reject(error);
			}
		);
	});
}

function openDatabase(): Promise<IDBDatabase> {
	if (databasePromise) return databasePromise;

	const indexedDb = globalThis.indexedDB;
	if (!indexedDb) {
		return Promise.reject(
			new StudioWorkspaceStorageError(studioMessage('storage.indexedDbUnavailable'))
		);
	}

	const opening = new Promise<IDBDatabase>((resolve, reject) => {
		let settled = false;
		let request: IDBOpenDBRequest;
		const timeout = setTimeout(() => {
			fail(new StudioWorkspaceStorageError(studioMessage('storage.restoreTimedOut', {
				phase: studioMessage('storage.phase.openingBrowserStorage')
			})));
		}, WORKSPACE_STORAGE_TIMEOUT_MS);
		const fail = (error: StudioWorkspaceStorageError) => {
			if (settled) return;
			settled = true;
			clearTimeout(timeout);
			reject(error);
		};

		try {
			request = indexedDb.open(DATABASE_NAME, DATABASE_VERSION);
		} catch (error) {
			fail(asStorageError('open', error));
			return;
		}
		request.onupgradeneeded = () => {
			const database = request.result;
			if (!database.objectStoreNames.contains(OBJECT_STORE)) database.createObjectStore(OBJECT_STORE);
		};
		request.onblocked = () => {
			fail(
				new StudioWorkspaceStorageError(
					studioMessage('storage.openBlocked')
				)
			);
		};
		request.onerror = () => {
			fail(
				new StudioWorkspaceStorageError(
					studioMessage('storage.openFailed', {
						reason: request.error?.message ?? studioMessage('storage.checkPermissions')
					})
				)
			);
		};
		request.onsuccess = () => {
			const database = request.result;
			if (settled) {
				database.close();
				return;
			}
			settled = true;
			clearTimeout(timeout);
			databaseConnection = database;
			const clearConnection = () => {
				if (databaseConnection !== database) return;
				databaseConnection = null;
				databasePromise = null;
			};
			database.onversionchange = () => {
				database.close();
				clearConnection();
			};
			database.onclose = clearConnection;
			resolve(database);
		};
	});

	databasePromise = opening.catch((error: unknown) => {
		databasePromise = null;
		throw error;
	});
	return databasePromise;
}

async function persistWorkspace(
	snapshot: StudioWorkspaceSnapshot,
	expectedRevision: number | null | undefined,
	expectedSnapshotKey: string | null
): Promise<number> {
	if (expectedRevision === undefined) {
		throw new StudioWorkspaceConflictError(studioMessage('storage.unsafeSave'));
	}

	try {
		const database = await openDatabase();
		return await writeWorkspaceRecord(
			database,
			{ schemaVersion: SNAPSHOT_VERSION, snapshot },
			expectedRevision,
			expectedSnapshotKey
		);
	} catch (error) {
		throw asStorageError('save', error);
	}
}

function readWorkspaceRecord(database: IDBDatabase): Promise<unknown> {
	return new Promise((resolve, reject) => {
		let transaction: IDBTransaction;
		try {
			transaction = database.transaction(OBJECT_STORE, 'readonly');
		} catch (error) {
			clearClosedDatabase(database);
			reject(asStorageError('read', error));
			return;
		}

		const request = transaction.objectStore(OBJECT_STORE).get(WORKSPACE_KEY);
		let value: unknown;
		let requestFailed = false;
		let settled = false;
		const finish = (callback: () => void) => {
			if (settled) return;
			settled = true;
			clearTimeout(timeout);
			callback();
		};
		const timeout = setTimeout(() => {
			finish(() => {
				reject(new StudioWorkspaceStorageError(studioMessage('storage.restoreTimedOut', {
					phase: studioMessage('storage.phase.readingSavedData')
				})));
				try {
					transaction.abort();
				} catch {
					// The transaction may have completed between the timer and abort.
				}
			});
		}, WORKSPACE_STORAGE_TIMEOUT_MS);
		request.onsuccess = () => {
			value = request.result;
		};
		request.onerror = () => {
			requestFailed = true;
		};
		transaction.oncomplete = () => {
			finish(() => {
				if (requestFailed) reject(asStorageError('read', request.error));
				else resolve(value);
			});
		};
		transaction.onabort = () => {
			finish(() => reject(asStorageError('read', transaction.error ?? request.error)));
		};
	});
}

function writeWorkspaceRecord(
	database: IDBDatabase,
	record: StoredWorkspace,
	expectedRevision: number | null,
	expectedSnapshotKey: string | null
): Promise<number> {
	return new Promise((resolve, reject) => {
		let transaction: IDBTransaction;
		try {
			transaction = createDurableWriteTransaction(database);
		} catch (error) {
			clearClosedDatabase(database);
			reject(asStorageError('save', error));
			return;
		}

		const store = transaction.objectStore(OBJECT_STORE);
		const request = store.get(WORKSPACE_KEY);
		let nextRevision = 0;
		let conflict = false;
		request.onsuccess = () => {
			try {
				const current = request.result;
				const currentRevision = current === undefined ? null : storedRevision(current);
				if (currentRevision !== expectedRevision) {
					conflict = true;
					transaction.abort();
					return;
				}

				// A pre-revision Studio tab can still write an unversioned record. Compare
				// its content to our loaded baseline so that this first version-aware save
				// cannot silently replace a concurrent legacy-tab edit.
				if (
					current !== undefined &&
					currentRevision === 0 &&
					expectedSnapshotKey !== null
				) {
					let currentSnapshotKey: string;
					try {
						currentSnapshotKey = snapshotKey(normalizeSnapshot((current as StoredWorkspace).snapshot));
					} catch {
						currentSnapshotKey = '';
					}
					if (currentSnapshotKey !== expectedSnapshotKey) {
						conflict = true;
						transaction.abort();
						return;
					}
				}

				nextRevision = (expectedRevision ?? 0) + 1;
				store.put({ ...record, revision: nextRevision }, WORKSPACE_KEY);
			} catch (error) {
				conflict = error instanceof StudioWorkspaceConflictError;
				if (!conflict) reject(asStorageError('save', error));
				transaction.abort();
			}
		};
		transaction.oncomplete = () => resolve(nextRevision);
		transaction.onabort = () => {
			if (conflict) reject(workspaceConflictError());
			else reject(asStorageError('save', transaction.error ?? request.error));
		};
	});
}

function storedRevision(value: unknown): number {
	if (!isRecord(value) || value.schemaVersion !== SNAPSHOT_VERSION) {
		throw workspaceConflictError();
	}
	return value.revision === undefined ? 0 : requireRevision(value.revision);
}

function requireRevision(value: unknown): number {
	if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) {
		throw new StudioWorkspaceConflictError(studioMessage('storage.invalidRevision'));
	}
	return value;
}

function workspaceConflictError(): StudioWorkspaceConflictError {
	return new StudioWorkspaceConflictError(studioMessage('storage.workspaceConflict'));
}

export class StudioWorkspaceConflictError extends StudioWorkspaceStorageError {
	constructor(message: string) {
		super(message);
		this.name = 'StudioWorkspaceConflictError';
	}
}

function createDurableWriteTransaction(database: IDBDatabase): IDBTransaction {
	try {
		return database.transaction(OBJECT_STORE, 'readwrite', { durability: 'strict' });
	} catch (error) {
		if (!isUnsupportedDurabilityOption(error)) throw error;
		// Older engines may reject the transaction options overload; retain atomic IDB writes there.
		return database.transaction(OBJECT_STORE, 'readwrite');
	}
}

function isUnsupportedDurabilityOption(error: unknown): boolean {
	return error instanceof TypeError || (isRecord(error) && error.name === 'NotSupportedError');
}

function clearClosedDatabase(database: IDBDatabase) {
	if (databaseConnection !== database) return;
	databaseConnection = null;
	databasePromise = null;
}

function normalizeSnapshot(value: unknown): StudioWorkspaceSnapshot {
	if (!isRecord(value)) throw new TypeError(studioMessage('storage.snapshotMustBeObject'));
	if (!Array.isArray(value.projects)) throw new TypeError(studioMessage('storage.snapshotMissingProjects'));
	if (value.activeProjectId !== null && typeof value.activeProjectId !== 'string') {
		throw new TypeError(studioMessage('storage.activeProjectIdInvalid'));
	}
	if (!isRecord(value.selectedFileByProject)) {
		throw new TypeError(studioMessage('storage.selectedFileMapInvalid'));
	}
	const rawScenarioSelections = value.selectedScenarioByProject === undefined
		? {}
		: value.selectedScenarioByProject;
	if (!isRecord(rawScenarioSelections)) {
		throw new TypeError(studioMessage('storage.selectedScenarioMapInvalid'));
	}

	const projectIds = new Set<string>();
	const fileIds = new Set<string>();
	const projects = value.projects.map((entry, projectIndex): StudioWorkspaceProjectSnapshot => {
		const label = studioMessage('storage.projectLabel', { index: projectIndex + 1 });
		if (!isRecord(entry)) throw new TypeError(studioMessage('storage.projectInvalid', { project: label }));
		const id = requireIdentifier(entry.id);
		if (projectIds.has(id)) throw new TypeError(studioMessage('storage.duplicateProjectId', { id }));
		projectIds.add(id);
		if (typeof entry.name !== 'string') throw new TypeError(studioMessage('storage.projectNameInvalid', { project: label }));
		if (!Array.isArray(entry.files)) throw new TypeError(studioMessage('storage.projectFilesInvalid', { project: label }));
		// Local snapshots can exceed portable bundle size limits; IndexedDB and its quota govern storage.
		if (entry.files.length > MAX_PROJECT_FILES) {
			throw new TypeError(studioMessage('storage.projectFileLimit', { project: label, max: MAX_PROJECT_FILES }));
		}

		const paths = new Set<string>();
		const files = entry.files.map((file, fileIndex): StudioWorkspaceFileSnapshot => {
			if (!isRecord(file)) throw new TypeError(studioMessage('storage.fileInvalid', { project: label, index: fileIndex + 1 }));
			const fileId = requireIdentifier(file.id);
			if (fileIds.has(fileId)) throw new TypeError(studioMessage('storage.duplicateFileId', { id: fileId }));
			fileIds.add(fileId);
			if (typeof file.path !== 'string' || typeof file.source !== 'string') {
				throw new TypeError(studioMessage('storage.fileSourceInvalid', { project: label, index: fileIndex + 1 }));
			}
			const path = normalizeProjectFilePath(file.path);
			const pathKey = path.toLowerCase();
			if (paths.has(pathKey)) throw new TypeError(studioMessage('storage.duplicateFilePath', { path }));
			paths.add(pathKey);
			return { id: fileId, path, source: file.source };
		});

		return {
			id,
			name: normalizeProjectName(entry.name),
			files
		};
	});

	const projectById = new Map(projects.map((project) => [project.id, project]));
	const selectedFileByProject: Record<string, string> = {};
	for (const [projectId, fileIdValue] of Object.entries(value.selectedFileByProject)) {
		const fileId = requireIdentifier(fileIdValue);
		const project = projectById.get(projectId);
		if (project?.files.some((file) => file.id === fileId)) {
			Object.defineProperty(selectedFileByProject, projectId, {
				value: fileId,
				enumerable: true,
				configurable: true,
				writable: true
			});
		}
	}

	const selectedScenarioByProject: Record<string, Record<string, string | null>> = {};
	for (const [projectIdValue, selectionsValue] of Object.entries(rawScenarioSelections)) {
		const projectId = requireIdentifier(projectIdValue);
		const project = projectById.get(projectId);
		if (!project) continue;
		if (!isRecord(selectionsValue)) {
			throw new TypeError(studioMessage('storage.selectedScenariosInvalid', { projectId }));
		}

		const visualSelections: Record<string, string | null> = {};
		for (const [visualIdValue, scenarioId] of Object.entries(selectionsValue)) {
			const visualId = requireIdentifier(visualIdValue);
			if (scenarioId !== null && typeof scenarioId !== 'string') {
				throw new TypeError(studioMessage('storage.selectedScenarioInvalid', { visualId }));
			}
			Object.defineProperty(visualSelections, visualId, {
				value: scenarioId,
				enumerable: true,
				configurable: true,
				writable: true
			});
		}
		Object.defineProperty(selectedScenarioByProject, projectId, {
			value: visualSelections,
			enumerable: true,
			configurable: true,
			writable: true
		});
	}

	const requestedActiveProjectId = value.activeProjectId === null
		? null
		: requireIdentifier(value.activeProjectId);
	const activeProjectId = requestedActiveProjectId === null
		? null
		: projectById.has(requestedActiveProjectId)
			? requestedActiveProjectId
			: projects[0]?.id ?? null;

	const exampleProjectRevision = value.exampleProjectRevision === undefined
		? undefined
		: requireNonNegativeInteger(value.exampleProjectRevision);
	return {
		projects,
		activeProjectId,
		selectedFileByProject,
		selectedScenarioByProject,
		...(exampleProjectRevision === undefined ? {} : { exampleProjectRevision })
	};
}

function requireNonNegativeInteger(value: unknown): number {
	if (typeof value !== 'number' || !Number.isSafeInteger(value) || value < 0) {
		throw new TypeError(studioMessage('storage.nonNegativeIntegerInvalid'));
	}
	return value;
}

function requireIdentifier(value: unknown): string {
	if (
		typeof value !== 'string' ||
		!value.trim() ||
		value.length > 256 ||
		/[\u0000-\u001f\u007f]/.test(value)
	) {
		throw new TypeError(studioMessage('storage.identifierInvalid'));
	}
	return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function asStorageError(operation: 'open' | 'read' | 'save', error: unknown): StudioWorkspaceStorageError {
	if (error instanceof StudioWorkspaceStorageError) return error;
	const reason = errorMessage(error, studioMessage('storage.unavailableOrFull'));
	const messageId = ({
		open: 'storage.openFailed',
		read: 'storage.readFailed',
		save: 'storage.saveFailed'
	} as const satisfies Record<typeof operation, StudioMessageId>)[operation];
	return new StudioWorkspaceStorageError(studioMessage(messageId, { reason }));
}

function errorMessage(error: unknown, fallback: string): string {
	return error instanceof Error && error.message ? error.message : fallback;
}

type RestorePhaseMessage = Extract<StudioMessageId,
	| 'storage.phase.waitingForWrites'
	| 'storage.phase.openingBrowserStorage'
	| 'storage.phase.readingSavedData'
>;
