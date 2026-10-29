import {
	assertCanAddDocument,
	createStarterDocument,
	createProjectFromBundle,
	createStarterProject,
	normalizeResourceTitle,
	normalizeProjectName,
	normalizeProjectFilePath,
	parseProjectBundle,
	restoreStudioProject,
	resourceSlug,
	serializeProjectBundle,
	uniqueProjectPath,
	uniqueProjectName,
	type StudioProject
} from './session.js';
import type { ProjectDocument, SourceMode, SchemraFileHandle } from './session.js';
import {
	BUNDLED_EXAMPLE_PROJECT_REVISION,
	refreshBundledExampleProject,
	StudioWorkspaceConflictError,
	loadStudioWorkspace,
	saveStudioWorkspace,
	type StudioWorkspaceSnapshot
} from './storage.js';

export type WorkspaceSaveStatus = 'idle' | 'loading' | 'ready' | 'dirty' | 'saving' | 'saved' | 'error' | 'conflict';
export type WorkspaceIssueKind = 'not-initialized' | 'not-found' | 'invalid' | 'storage' | 'conflict';

export interface WorkspaceIssue {
	kind: WorkspaceIssueKind;
	message: string;
}

export interface WorkspaceControllerState {
	projects: StudioProject[];
	activeProjectId: string | null;
	selectedFileByProject: Record<string, string>;
	selectedScenarioByProject: Record<string, Record<string, string | null>>;
	initialized: boolean;
	saveStatus: WorkspaceSaveStatus;
	error: WorkspaceIssue | null;
}

export interface WorkspaceStorageAdapter {
	loadStudioWorkspace(): Promise<StudioWorkspaceSnapshot | null>;
	saveStudioWorkspace(snapshot: StudioWorkspaceSnapshot): Promise<void>;
}

export interface WorkspaceControllerOptions {
	exampleProject: StudioProject;
	storage?: WorkspaceStorageAdapter;
	autosaveDelayMs?: number;
}

export type WorkspaceActionResult<T> =
	| { ok: true; value: T }
	| { ok: false; error: WorkspaceIssue };

export type WorkspaceImportResult =
	| { ok: false; error: WorkspaceIssue }
	| { ok: true; project: StudioProject; persistence: 'saved' | 'unsaved'; warning?: WorkspaceIssue };

export type WorkspaceExportResult = WorkspaceActionResult<{ fileName: string; contents: string }>;
export type WorkspaceFlushResult = WorkspaceActionResult<void>;

export interface ScenarioSelectionToClear {
	visualId: string;
	scenarioId: string;
}

export interface WorkspaceRemovalPlan {
	/** Visuals that are being removed along with their source documents. */
	visualIdsToRemove?: string[];
	/** Visuals that will remain after this removal. Required when removing visuals. */
	remainingVisualIds?: string[];
	/** Selected scenarios removed from a file that also contains other scenarios. */
	scenarioSelectionsToClear?: ScenarioSelectionToClear[];
}

export interface WorkspaceController {
	getState(): WorkspaceControllerState;
	subscribe(listener: (state: WorkspaceControllerState) => void): () => void;
	initialize(): Promise<WorkspaceActionResult<WorkspaceControllerState>>;
	updateSource(projectId: string, fileId: string, source: string): WorkspaceActionResult<void>;
	selectProject(projectId: string): WorkspaceActionResult<void>;
	selectFile(projectId: string, fileId: string): WorkspaceActionResult<void>;
	selectScenario(projectId: string, visualId: string, scenarioId: string | null): WorkspaceActionResult<void>;
	createProject(name: string): WorkspaceActionResult<StudioProject>;
	renameProject(projectId: string, name: string): WorkspaceActionResult<StudioProject>;
	createVisual(projectId: string, title: string): WorkspaceActionResult<StudioProject['files'][number]>;
	createScenario(projectId: string, resource: { visualId: string; scenarioId: string; title: string; source: string }): WorkspaceActionResult<StudioProject['files'][number]>;
	addDocument(projectId: string, document: { path: string; source: string; handle?: SchemraFileHandle | null; mode?: SourceMode }): WorkspaceActionResult<ProjectDocument>;
	markSourceSaved(projectId: string, fileId: string, writtenSource: string, destination: { name: string; handle: SchemraFileHandle | null; mode: 'local' | 'download' }): WorkspaceActionResult<ProjectDocument>;
	removeDocuments(projectId: string, fileIds: string[], plan?: WorkspaceRemovalPlan): WorkspaceActionResult<void>;
	importBundle(contents: string): Promise<WorkspaceImportResult>;
	exportBundle(projectId?: string): WorkspaceExportResult;
	flush(): Promise<WorkspaceFlushResult>;
	dispose(): Promise<WorkspaceFlushResult>;
}

const defaultStorage: WorkspaceStorageAdapter = { loadStudioWorkspace, saveStudioWorkspace };
const DEFAULT_AUTOSAVE_DELAY_MS = 250;

/** Owns Studio project state and safely coordinates it with browser workspace persistence. */
export function createWorkspaceController(options: WorkspaceControllerOptions): WorkspaceController {
	const exampleProject = cloneProject(options.exampleProject);
	const storage = options.storage ?? defaultStorage;
	const autosaveDelayMs = Math.max(0, options.autosaveDelayMs ?? DEFAULT_AUTOSAVE_DELAY_MS);
	const firstFileId = exampleProject.files[0]?.id ?? '';
	const listeners = new Set<(state: WorkspaceControllerState) => void>();
	let state: WorkspaceControllerState = {
		projects: [exampleProject],
		activeProjectId: exampleProject.id,
		selectedFileByProject: firstFileId ? { [exampleProject.id]: firstFileId } : {},
		selectedScenarioByProject: {},
		initialized: false,
		saveStatus: 'idle',
		error: null
	};
	let initializePromise: Promise<WorkspaceActionResult<WorkspaceControllerState>> | null = null;
	let autosaveTimer: ReturnType<typeof setTimeout> | null = null;
	let queuedWrites: Promise<void> = Promise.resolve();
	let lastSavedSnapshotKey: string | null = null;
	let stateRevision = 0;
	const pendingWrites = new Map<string, Promise<void>>();

	function getState(): WorkspaceControllerState {
		return cloneState(state);
	}

	function publish(next: WorkspaceControllerState) {
		state = next;
		for (const listener of listeners) listener(getState());
	}

	function subscribe(listener: (next: WorkspaceControllerState) => void) {
		listeners.add(listener);
		listener(getState());
		return () => listeners.delete(listener);
	}

	function updateState(patch: Partial<WorkspaceControllerState>) {
		publish({ ...state, ...patch });
	}

	function makeSnapshot(): StudioWorkspaceSnapshot {
		return {
			projects: state.projects.map((project) => ({
				id: project.id,
				name: project.name,
				files: project.files.map(({ id, path, source }) => ({ id, path, source }))
			})),
			activeProjectId: state.activeProjectId,
			selectedFileByProject: { ...state.selectedFileByProject },
			selectedScenarioByProject: { ...state.selectedScenarioByProject },
			exampleProjectRevision: BUNDLED_EXAMPLE_PROJECT_REVISION
		};
	}

	function scheduleSave(delay = autosaveDelayMs) {
		if (!state.initialized) return;
		if (autosaveTimer !== null) clearTimeout(autosaveTimer);
		updateState({ saveStatus: 'dirty', error: null });
		autosaveTimer = setTimeout(() => {
			autosaveTimer = null;
			void saveCurrentSnapshot();
		}, delay);
	}

	function saveCurrentSnapshot(): Promise<WorkspaceFlushResult> {
		const snapshot = makeSnapshot();
		const key = JSON.stringify(snapshot);
		const revision = stateRevision;
		if (key === lastSavedSnapshotKey && pendingWrites.size === 0) {
			if (autosaveTimer === null) updateState({ saveStatus: 'saved', error: null });
			return Promise.resolve({ ok: true, value: undefined });
		}
		const existing = pendingWrites.get(key);
		if (existing) return existing.then(() => ({ ok: true, value: undefined }), (error: unknown) => failSave(error, key, revision));

		updateState({ saveStatus: 'saving', error: null });
		const write = queuedWrites.then(async () => {
			if (key === lastSavedSnapshotKey) return;
			await storage.saveStudioWorkspace(snapshot);
			lastSavedSnapshotKey = key;
		});
		queuedWrites = write.catch(() => undefined);
		pendingWrites.set(key, write);
		return write.then(
			() => {
				pendingWrites.delete(key);
				if (revision === stateRevision) updateState({ saveStatus: 'saved', error: null });
				else if (autosaveTimer === null) scheduleSave(0);
				return { ok: true, value: undefined };
			},
			(error: unknown) => {
				pendingWrites.delete(key);
				return failSave(error, key, revision);
			}
		);
	}

	function failSave(error: unknown, key: string, revision: number): WorkspaceFlushResult {
		const issue = toWorkspaceIssue(error, 'Could not save the browser workspace.');
		if (revision === stateRevision && JSON.stringify(makeSnapshot()) === key) {
			updateState({ saveStatus: issue.kind === 'conflict' ? 'conflict' : 'error', error: issue });
		}
		return { ok: false, error: issue };
	}

	async function initialize(): Promise<WorkspaceActionResult<WorkspaceControllerState>> {
		if (initializePromise) return initializePromise;
		if (state.initialized) return { ok: true, value: getState() };
		updateState({ saveStatus: 'loading', error: null });
		initializePromise = (async () => {
			let snapshot: StudioWorkspaceSnapshot | null;
			try {
				snapshot = await storage.loadStudioWorkspace();
			} catch (error) {
				const issue = toWorkspaceIssue(error, 'Could not restore the saved workspace.');
				publish({
					...state,
					projects: [cloneProject(exampleProject)],
					activeProjectId: exampleProject.id,
					selectedFileByProject: firstFileId ? { [exampleProject.id]: firstFileId } : {},
					selectedScenarioByProject: {},
					initialized: true,
					saveStatus: issue.kind === 'conflict' ? 'conflict' : 'error',
					error: issue
				});
				return { ok: false, error: issue };
			}

			const baseSnapshot = snapshot ?? initialSnapshot(exampleProject);
			const refreshed = snapshot ? refreshBundledExampleProject(snapshot, exampleProject) : null;
			const restored = refreshed ?? baseSnapshot;
			lastSavedSnapshotKey = snapshot ? JSON.stringify(snapshot) : null;
			const projects = restored.projects.map((project) =>
				restoreStudioProject(project, project.id === exampleProject.id ? 'example' : 'memory')
			);
			if (projects.length === 0) projects.push(cloneProject(exampleProject));
			const activeProjectId = restored.activeProjectId && projects.some((project) => project.id === restored.activeProjectId)
				? restored.activeProjectId
				: projects[0]?.id ?? null;
			const selectedFileByProject = filterSelectedFiles(restored, projects);
			if (activeProjectId && !selectedFileByProject[activeProjectId]) {
				const first = projects.find((project) => project.id === activeProjectId)?.files[0]?.id;
				if (first) defineValue(selectedFileByProject, activeProjectId, first);
			}
			const selectedScenarioByProject = cloneScenarioSelections(restored.selectedScenarioByProject);
			publish({
				projects,
				activeProjectId,
				selectedFileByProject,
				selectedScenarioByProject,
				initialized: true,
				saveStatus: 'ready',
				error: null
			});
			stateRevision += 1;
			if (!snapshot || refreshed || restored.projects.length === 0) scheduleSave(0);
			return { ok: true, value: getState() };
		})();
		const result = await initializePromise;
		initializePromise = null;
		return result;
	}

	function requireInitialized(): WorkspaceIssue | null {
		return state.initialized ? null : { kind: 'not-initialized', message: 'Initialize the workspace before changing it.' };
	}

	function updateSource(projectId: string, fileId: string, source: string): WorkspaceActionResult<void> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		const fileIndex = state.projects[projectIndex]?.files.findIndex((file) => file.id === fileId) ?? -1;
		if (projectIndex < 0 || fileIndex < 0) return notFound('The selected project document no longer exists.');
		const project = state.projects[projectIndex]!;
		if (project.files[fileIndex]!.source === source) return { ok: true, value: undefined };
		const projects = state.projects.slice();
		const files = project.files.slice();
		files[fileIndex] = { ...files[fileIndex]!, source };
		projects[projectIndex] = { ...project, files };
		stateRevision += 1;
		publish({ ...state, projects, saveStatus: 'dirty', error: null });
		scheduleSave();
		return { ok: true, value: undefined };
	}

	function selectProject(projectId: string): WorkspaceActionResult<void> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const project = state.projects.find((entry) => entry.id === projectId);
		if (!project) return notFound('The selected project no longer exists.');
		const selectedFileByProject = { ...state.selectedFileByProject };
		if (!selectedFileByProject[projectId] && project.files[0]) defineValue(selectedFileByProject, projectId, project.files[0].id);
		if (state.activeProjectId === projectId && selectedFileByProject[projectId] === state.selectedFileByProject[projectId]) {
			return { ok: true, value: undefined };
		}
		stateRevision += 1;
		publish({ ...state, activeProjectId: projectId, selectedFileByProject, saveStatus: 'dirty', error: null });
		scheduleSave();
		return { ok: true, value: undefined };
	}

	function selectFile(projectId: string, fileId: string): WorkspaceActionResult<void> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const project = state.projects.find((entry) => entry.id === projectId);
		if (!project || !project.files.some((file) => file.id === fileId)) {
			return notFound('The selected project document no longer exists.');
		}
		if (state.activeProjectId === projectId && state.selectedFileByProject[projectId] === fileId) {
			return { ok: true, value: undefined };
		}
		const selectedFileByProject = { ...state.selectedFileByProject };
		defineValue(selectedFileByProject, projectId, fileId);
		stateRevision += 1;
		publish({ ...state, activeProjectId: projectId, selectedFileByProject, saveStatus: 'dirty', error: null });
		scheduleSave();
		return { ok: true, value: undefined };
	}

	function selectScenario(projectId: string, visualId: string, scenarioId: string | null): WorkspaceActionResult<void> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		if (!state.projects.some((project) => project.id === projectId)) return notFound('The selected project no longer exists.');
		const projectSelections = { ...(state.selectedScenarioByProject[projectId] ?? {}) };
		if (projectSelections[visualId] === scenarioId && Object.hasOwn(projectSelections, visualId)) {
			return { ok: true, value: undefined };
		}
		defineValue(projectSelections, visualId, scenarioId);
		const selectedScenarioByProject = { ...state.selectedScenarioByProject };
		defineValue(selectedScenarioByProject, projectId, projectSelections);
		stateRevision += 1;
		publish({ ...state, selectedScenarioByProject, saveStatus: 'dirty', error: null });
		scheduleSave();
		return { ok: true, value: undefined };
	}

	function createProject(name: string): WorkspaceActionResult<StudioProject> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		try {
			const project = createStarterProject(uniqueProjectName(state.projects.map((entry) => entry.name), name));
			const projects = [...state.projects, project];
			const selectedFileByProject = { ...state.selectedFileByProject };
			if (project.files[0]) defineValue(selectedFileByProject, project.id, project.files[0].id);
			stateRevision += 1;
			publish({ ...state, projects, activeProjectId: project.id, selectedFileByProject, saveStatus: 'dirty', error: null });
			scheduleSave();
			return { ok: true, value: cloneProject(project) };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not create the project.') };
		}
	}

	function renameProject(projectId: string, name: string): WorkspaceActionResult<StudioProject> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		if (projectIndex < 0) return notFound('The selected project no longer exists.');
		try {
			const current = state.projects[projectIndex]!;
			const renamed = {
				...current,
				name: uniqueProjectName(state.projects.filter((_, index) => index !== projectIndex).map((project) => project.name), normalizeProjectName(name))
			};
			const projects = state.projects.slice();
			projects[projectIndex] = renamed;
			stateRevision += 1;
			publish({ ...state, projects, saveStatus: 'dirty', error: null });
			scheduleSave();
			return { ok: true, value: cloneProject(renamed) };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not rename the project.') };
		}
	}

	function createVisual(projectId: string, title: string): WorkspaceActionResult<StudioProject['files'][number]> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		if (projectIndex < 0) return notFound('The selected project no longer exists.');
		try {
			const project = state.projects[projectIndex]!;
			const normalizedTitle = normalizeResourceTitle(title);
			const path = uniqueProjectPath(project, `visuals/${resourceSlug(normalizedTitle)}.schemra`);
			const file = createStarterDocument(path, normalizedTitle);
			assertCanAddDocument(project, file.source);
			insertDocument(projectIndex, file);
			return { ok: true, value: { ...file } };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not create the visual.') };
		}
	}

	function createScenario(
		projectId: string,
		resource: { visualId: string; scenarioId: string; title: string; source: string }
	): WorkspaceActionResult<StudioProject['files'][number]> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		if (projectIndex < 0) return notFound('The selected project no longer exists.');
		try {
			const project = state.projects[projectIndex]!;
			const title = normalizeResourceTitle(resource.title);
			if (!/^[a-z_][a-z0-9_]{0,31}$/.test(resource.scenarioId)) {
				throw new TypeError('The scenario identifier is not valid.');
			}
			if (!/^[a-z_][a-zA-Z0-9_]*$/.test(resource.visualId)) {
				throw new TypeError('The visual identifier is not valid.');
			}
			if (!resource.source.trim()) throw new TypeError('Scenario source cannot be empty.');
			const path = uniqueProjectPath(
				project,
				`scenarios/${resource.visualId}/${resourceSlug(title)}.schemra`
			);
			const file = {
				id: `file-${crypto.randomUUID()}`,
				path,
				source: resource.source,
				savedSource: resource.source,
				handle: null,
				mode: 'memory' as const
			};
			assertCanAddDocument(project, file.source);
			insertDocument(projectIndex, file);
			const selectedScenarioByProject = cloneScenarioSelections(state.selectedScenarioByProject);
			const projectSelections = { ...(selectedScenarioByProject[projectId] ?? {}) };
			defineValue(projectSelections, resource.visualId, resource.scenarioId);
			defineValue(selectedScenarioByProject, projectId, projectSelections);
			stateRevision += 1;
			publish({ ...state, selectedScenarioByProject, saveStatus: 'dirty', error: null });
			scheduleSave();
			return { ok: true, value: { ...file } };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not create the scenario.') };
		}
	}

	function addDocument(
		projectId: string,
		input: { path: string; source: string; handle?: SchemraFileHandle | null; mode?: SourceMode }
	): WorkspaceActionResult<ProjectDocument> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		if (projectIndex < 0) return notFound('The selected project no longer exists.');
		try {
			const project = state.projects[projectIndex]!;
			const requestedPath = normalizeProjectFilePath(input.path);
			const document: ProjectDocument = {
				id: `file-${crypto.randomUUID()}`,
				path: uniqueProjectPath(project, requestedPath),
				source: input.source,
				savedSource: input.source,
				handle: input.handle ?? null,
				mode: input.mode ?? (input.handle ? 'local' : 'memory')
			};
			assertCanAddDocument(project, document.source);
			insertDocument(projectIndex, document);
			return { ok: true, value: { ...document } };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not add the source document.') };
		}
	}

	function markSourceSaved(
		projectId: string,
		fileId: string,
		writtenSource: string,
		destination: { name: string; handle: SchemraFileHandle | null; mode: 'local' | 'download' }
	): WorkspaceActionResult<ProjectDocument> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		const project = state.projects[projectIndex];
		const fileIndex = project?.files.findIndex((file) => file.id === fileId) ?? -1;
		if (!project || fileIndex < 0) return notFound('The selected project document no longer exists.');
		try {
			const current = project.files[fileIndex]!;
			const directory = current.path.includes('/') ? `${current.path.slice(0, current.path.lastIndexOf('/') + 1)}` : '';
			const otherFilesProject = { ...project, files: project.files.filter(({ id }) => id !== fileId) };
			const path = uniqueProjectPath(otherFilesProject, `${directory}${destination.name}`);
			const saved: ProjectDocument = {
				...current,
				path,
				savedSource: writtenSource,
				handle: destination.handle,
				mode: destination.mode
			};
			const projects = state.projects.slice();
			projects[projectIndex] = {
				...project,
				files: project.files.map((file, index) => index === fileIndex ? saved : file)
			};
			if (path !== current.path) {
				stateRevision += 1;
				publish({ ...state, projects, saveStatus: 'dirty', error: null });
				scheduleSave();
			} else {
				publish({ ...state, projects });
			}
			return { ok: true, value: { ...saved } };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not update the saved source document.') };
		}
	}

	function insertDocument(projectIndex: number, file: StudioProject['files'][number]) {
		const projects = state.projects.slice();
		const project = projects[projectIndex]!;
		projects[projectIndex] = { ...project, files: [...project.files, file] };
		const selectedFileByProject = { ...state.selectedFileByProject };
		defineValue(selectedFileByProject, project.id, file.id);
		stateRevision += 1;
		publish({
			...state,
			projects,
			activeProjectId: project.id,
			selectedFileByProject,
			saveStatus: 'dirty',
			error: null
		});
		scheduleSave();
	}

	function removeDocuments(projectId: string, fileIds: string[], plan: WorkspaceRemovalPlan = {}): WorkspaceActionResult<void> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		const projectIndex = state.projects.findIndex((project) => project.id === projectId);
		if (projectIndex < 0) return notFound('The selected project no longer exists.');
		const project = state.projects[projectIndex]!;
		const removeIds = new Set(fileIds);
		if (removeIds.size === 0) return { ok: false, error: { kind: 'invalid', message: 'Select at least one source document to remove.' } };
		if (removeIds.size !== fileIds.length || fileIds.some((id) => !project.files.some((file) => file.id === id))) {
			return notFound('One or more selected source documents no longer exist.');
		}
		if (project.files.length - removeIds.size < 1) {
			return { ok: false, error: { kind: 'invalid', message: 'A project must keep at least one source document.' } };
		}
		const visualsToRemove = new Set(plan.visualIdsToRemove ?? []);
		if (visualsToRemove.size > 0 && (plan.remainingVisualIds?.length ?? 0) === 0) {
			return { ok: false, error: { kind: 'invalid', message: 'A project must keep at least one visual.' } };
		}

		const projects = state.projects.slice();
		const remainingFiles = project.files.filter((file) => !removeIds.has(file.id));
		projects[projectIndex] = { ...project, files: remainingFiles };
		const selectedFileByProject = { ...state.selectedFileByProject };
		if (removeIds.has(selectedFileByProject[projectId] ?? '')) {
			defineValue(selectedFileByProject, projectId, remainingFiles[0]!.id);
		}
		const selectedScenarioByProject = cloneScenarioSelections(state.selectedScenarioByProject);
		const projectSelections = { ...(selectedScenarioByProject[projectId] ?? {}) };
		for (const visualId of visualsToRemove) delete projectSelections[visualId];
		for (const { visualId, scenarioId } of plan.scenarioSelectionsToClear ?? []) {
			if (projectSelections[visualId] === scenarioId) defineValue(projectSelections, visualId, null);
		}
		defineValue(selectedScenarioByProject, projectId, projectSelections);
		stateRevision += 1;
		publish({ ...state, projects, selectedFileByProject, selectedScenarioByProject, saveStatus: 'dirty', error: null });
		scheduleSave();
		return { ok: true, value: undefined };
	}

	async function importBundle(contents: string): Promise<WorkspaceImportResult> {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		let project: StudioProject;
		try {
			project = createProjectFromBundle(parseProjectBundle(contents));
			project.name = uniqueProjectName(state.projects.map((entry) => entry.name), project.name);
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not import this project bundle.') };
		}
		const projects = [...state.projects, project];
		const selectedFileByProject = { ...state.selectedFileByProject };
		if (project.files[0]) defineValue(selectedFileByProject, project.id, project.files[0].id);
		stateRevision += 1;
		publish({ ...state, projects, activeProjectId: project.id, selectedFileByProject, saveStatus: 'dirty', error: null });
		const saved = await flush();
		return saved.ok
			? { ok: true, project: cloneProject(project), persistence: 'saved' }
			: { ok: true, project: cloneProject(project), persistence: 'unsaved', warning: saved.error };
	}

	function exportBundle(projectId = state.activeProjectId): WorkspaceExportResult {
		const unavailable = requireInitialized();
		if (unavailable) return { ok: false, error: unavailable };
		if (!projectId) return notFound('There is no active project to export.');
		const project = state.projects.find((entry) => entry.id === projectId);
		if (!project) return notFound('The selected project no longer exists.');
		try {
			const safeName = project.name.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'schemra-project';
			return { ok: true, value: { fileName: `${safeName}.schemra-project.json`, contents: serializeProjectBundle(project) } };
		} catch (error) {
			return { ok: false, error: toWorkspaceIssue(error, 'Could not export this project.') };
		}
	}

	async function flush(): Promise<WorkspaceFlushResult> {
		if (!state.initialized) return { ok: false, error: { kind: 'not-initialized', message: 'Initialize the workspace before saving it.' } };
		if (autosaveTimer !== null) {
			clearTimeout(autosaveTimer);
			autosaveTimer = null;
		}
		return saveCurrentSnapshot();
	}

	async function dispose(): Promise<WorkspaceFlushResult> {
		const result = await flush();
		if (autosaveTimer !== null) clearTimeout(autosaveTimer);
		autosaveTimer = null;
		listeners.clear();
		return result;
	}

	return {
		getState,
		subscribe,
		initialize,
		updateSource,
		selectProject,
		selectFile,
		selectScenario,
		createProject,
		renameProject,
		createVisual,
		createScenario,
		addDocument,
		markSourceSaved,
		removeDocuments,
		importBundle,
		exportBundle,
		flush,
		dispose
	};
}

function initialSnapshot(exampleProject: StudioProject): StudioWorkspaceSnapshot {
	const project = cloneProject(exampleProject);
	return {
		projects: [{ id: project.id, name: project.name, files: project.files.map(({ id, path, source }) => ({ id, path, source })) }],
		activeProjectId: project.id,
		selectedFileByProject: project.files[0] ? { [project.id]: project.files[0].id } : {},
		selectedScenarioByProject: {},
		exampleProjectRevision: BUNDLED_EXAMPLE_PROJECT_REVISION
	};
}

function filterSelectedFiles(snapshot: StudioWorkspaceSnapshot, projects: StudioProject[]): Record<string, string> {
	const selected: Record<string, string> = {};
	for (const [projectId, fileId] of Object.entries(snapshot.selectedFileByProject)) {
		if (projects.find((project) => project.id === projectId)?.files.some((file) => file.id === fileId)) {
			defineValue(selected, projectId, fileId);
		}
	}
	return selected;
}

function cloneScenarioSelections(
	selections: Record<string, Record<string, string | null>>
): Record<string, Record<string, string | null>> {
	const clone: Record<string, Record<string, string | null>> = {};
	for (const [projectId, byVisual] of Object.entries(selections)) {
		const visualSelections: Record<string, string | null> = {};
		for (const [visualId, scenarioId] of Object.entries(byVisual)) defineValue(visualSelections, visualId, scenarioId);
		defineValue(clone, projectId, visualSelections);
	}
	return clone;
}

function defineValue<T>(record: Record<string, T>, key: string, value: T) {
	Object.defineProperty(record, key, { value, enumerable: true, configurable: true, writable: true });
}

function cloneProject(project: StudioProject): StudioProject {
	return { ...project, files: project.files.map((file) => ({ ...file })) };
}

function cloneState(state: WorkspaceControllerState): WorkspaceControllerState {
	return {
		...state,
		projects: state.projects.map(cloneProject),
		selectedFileByProject: { ...state.selectedFileByProject },
		selectedScenarioByProject: cloneScenarioSelections(state.selectedScenarioByProject),
		error: state.error ? { ...state.error } : null
	};
}

function notFound(message: string): WorkspaceActionResult<never> {
	return { ok: false, error: { kind: 'not-found', message } };
}

function toWorkspaceIssue(error: unknown, fallback: string): WorkspaceIssue {
	const message = error instanceof Error && error.message ? error.message : fallback;
	return {
		kind: error instanceof StudioWorkspaceConflictError ? 'conflict' : error instanceof TypeError ? 'invalid' : 'storage',
		message
	};
}
