export const PROJECT_BUNDLE_FORMAT = 'schemra-project';
export const PROJECT_BUNDLE_VERSION = 2;
const LEGACY_PROJECT_BUNDLE_VERSION = 1;

export const MAX_PROJECT_FILES = 100;
export const MAX_SOURCE_FILE_BYTES = 2 * 1024 * 1024;
export const MAX_PROJECT_SOURCE_BYTES = 12 * 1024 * 1024;
export const MAX_PROJECT_BUNDLE_BYTES = 64 * 1024 * 1024;

export type SourceMode = 'example' | 'local' | 'download' | 'memory';

/** A browser-backed handle for a source document opened from the local filesystem. */
export interface SchemraFileHandle {
	name: string;
	getFile(): Promise<File>;
	createWritable(): Promise<{ write(contents: string): Promise<void>; close(): Promise<void>; abort?(): Promise<void> }>;
	queryPermission?(options: { mode: 'readwrite' }): Promise<PermissionState>;
	requestPermission?(options: { mode: 'readwrite' }): Promise<PermissionState>;
}

export interface ProjectDocument {
	id: string;
	path: string;
	source: string;
	savedSource: string;
	handle: SchemraFileHandle | null;
	mode: SourceMode;
}

export interface StudioProject {
	id: string;
	name: string;
	files: ProjectDocument[];
}

export interface RestorableStudioProject {
	id: string;
	name: string;
	files: Array<{ id: string; path: string; source: string }>;
}

export interface ProjectBundleFile {
	path: string;
	source: string;
}

export interface ProjectBundle {
	format: typeof PROJECT_BUNDLE_FORMAT;
	version: typeof PROJECT_BUNDLE_VERSION;
	name: string;
	files: ProjectBundleFile[];
}

let idSequence = 0;

function createId(prefix: 'project' | 'file' | 'visual') {
	idSequence += 1;
	return `${prefix}-${Date.now().toString(36)}-${idSequence.toString(36)}`;
}

export function createInitialProject(
	visualSource: string,
	scenarioSource: string,
	additionalFiles: Array<{ id: string; path: string; source: string }> = []
): StudioProject {
	return {
		id: 'project-example',
		name: 'Example project',
		files: [
			{
				id: 'file-traffic-overview',
				path: 'visuals/traffic-overview.schemra',
				source: visualSource,
				savedSource: visualSource,
				handle: null,
				mode: 'example'
			},
			{
				id: 'file-request-response',
				path: 'scenarios/request-response.schemra',
				source: scenarioSource,
				savedSource: scenarioSource,
				handle: null,
				mode: 'example'
			},
			...additionalFiles.map(({ id, path, source }) => ({
				id,
				path,
				source,
				savedSource: source,
				handle: null,
				mode: 'example' as const
			}))
		]
	};
}

export function createStarterProject(name: string): StudioProject {
	return {
		id: createId('project'),
		name: normalizeProjectName(name),
		files: [createStarterDocument('visuals/untitled.schemra')]
	};
}

export function normalizeResourceTitle(value: string) {
	const title = value.trim().normalize('NFC').replace(/[\u0000-\u001f\u007f]/g, '');
	if (!title) throw new TypeError('Enter a name between 1 and 80 characters.');
	if (Array.from(title).length > 80) throw new TypeError('Names must be 80 characters or fewer.');
	return title;
}

/** Creates a safe, readable file-name stem from a resource title. */
export function resourceSlug(title: string) {
	return normalizeResourceTitle(title)
		.normalize('NFKD')
		.replace(/[\u0300-\u036f]/g, '')
		.replace(/([a-z0-9])([A-Z])/g, '$1-$2')
		.toLowerCase()
		.replace(/[^a-z0-9]+/g, '-')
		.replace(/^-+|-+$/g, '') || 'untitled';
}

/** Creates a stable DSL identifier from a title without colliding with existing IDs. */
export function uniqueScenarioId(title: string, usedIds: Iterable<string>) {
	const base = normalizeResourceTitle(title)
		.normalize('NFKD')
		.toLowerCase()
		.replace(/[\u0300-\u036f]/g, '')
		.replace(/[^a-z0-9_]+/g, '_')
		.replace(/^_+|_+$/g, '') || 'scenario';
	const identifier = /^[a-z_]/.test(base) ? base : `scenario_${base}`;
	const used = new Set(usedIds);
	let candidate = identifier.slice(0, 32).replace(/_+$/g, '') || 'scenario';
	let suffix = 2;
	while (used.has(candidate)) {
		const suffixText = `_${suffix++}`;
		candidate = `${identifier.slice(0, 32 - suffixText.length).replace(/_+$/g, '')}${suffixText}`;
	}
	return candidate;
}

/** Rehydrate persisted project source files. */
export function restoreStudioProject(
	project: RestorableStudioProject,
	mode: Extract<SourceMode, 'example' | 'memory'> = 'memory'
): StudioProject {
	return {
		id: project.id,
		name: project.name,
		files: project.files.map(({ id, path, source }) => ({
			id,
			path,
			source,
			savedSource: source,
			handle: null,
			mode
		}))
	};
}

export function createStarterDocument(path: string, title = 'Untitled visual'): ProjectDocument {
	const visualId = createId('visual').replaceAll('-', '_');
	const contents = `visual ${visualId} ${JSON.stringify(title)} {
  direction: left-to-right

	group services "Services" {
	  node service "Service" {
	    icon: "server"
	    detail: "Describe this part of your system"
	  }
	}
}
`;
	return {
		id: createId('file'),
		path,
		source: contents,
		savedSource: contents,
		handle: null,
		mode: 'memory'
	};
}

export function createProjectFromBundle(bundle: ProjectBundle): StudioProject {
	return {
		id: createId('project'),
		name: bundle.name,
		files: bundle.files.map(({ path, source }) => ({
			id: createId('file'),
			path,
			source,
			savedSource: source,
			handle: null,
			mode: 'memory'
		}))
	};
}

export function serializeProjectBundle(project: StudioProject) {
	const bundle: ProjectBundle = {
		format: PROJECT_BUNDLE_FORMAT,
		version: PROJECT_BUNDLE_VERSION,
		name: normalizeProjectName(project.name),
		files: project.files.map(({ path, source }) => ({ path, source }))
	};
	validateProject(bundle);
	const contents = JSON.stringify(bundle, null, 2);
	if (byteLength(contents) > MAX_PROJECT_BUNDLE_BYTES) {
		throw new TypeError(`This project bundle is too large to export (${MAX_PROJECT_BUNDLE_BYTES / 1024 / 1024} MiB maximum).`);
	}
	return contents;
}

export function parseProjectBundle(contents: string): ProjectBundle {
	if (byteLength(contents) > MAX_PROJECT_BUNDLE_BYTES) {
		throw new TypeError(`This project bundle is too large to open (${MAX_PROJECT_BUNDLE_BYTES / 1024 / 1024} MiB maximum).`);
	}

	let value: unknown;
	try {
		value = JSON.parse(contents);
	} catch {
		throw new TypeError('The selected file is not valid project JSON.');
	}

	if (!isRecord(value)) throw new TypeError('This is not a Schemra project bundle.');
	if (
		value.format !== PROJECT_BUNDLE_FORMAT ||
		(value.version !== PROJECT_BUNDLE_VERSION && value.version !== LEGACY_PROJECT_BUNDLE_VERSION)
	) {
		throw new TypeError('This Schemra project bundle uses an unsupported format version.');
	}
	if (typeof value.name !== 'string' || !Array.isArray(value.files)) {
		throw new TypeError('The project bundle is missing its name or source files.');
	}
	if (value.files.length < 1 || value.files.length > MAX_PROJECT_FILES) {
		throw new TypeError(`A project must contain between 1 and ${MAX_PROJECT_FILES} source files.`);
	}

	const bundle: ProjectBundle = {
		format: PROJECT_BUNDLE_FORMAT,
		version: PROJECT_BUNDLE_VERSION,
		name: value.name,
		files: value.files.map((file, index) => {
			if (!isRecord(file) || typeof file.path !== 'string' || typeof file.source !== 'string') {
				throw new TypeError(`Project file ${index + 1} must have a path and source string.`);
			}
			return { path: file.path, source: file.source };
		}),
	};
	validateProject(bundle);
	return bundle;
}

export function uniqueProjectPath(project: StudioProject, proposedPath: string) {
	const path = normalizeProjectFilePath(proposedPath);
	const existing = new Set(project.files.map((file) => file.path.toLowerCase()));
	if (!existing.has(path.toLowerCase())) return path;

	const parts = path.split('/');
	const fileName = parts.pop() ?? 'visual.schemra';
	const directory = parts.length > 0 ? `${parts.join('/')}/` : '';
	const stem = fileName.replace(/\.schemra$/i, '');
	let suffix = 2;
	while (existing.has(`${directory}${stem}-${suffix}.schemra`.toLowerCase())) suffix += 1;
	return `${directory}${stem}-${suffix}.schemra`;
}

export function assertCanAddDocument(project: StudioProject, source: string) {
	if (project.files.length >= MAX_PROJECT_FILES) {
		throw new TypeError(`A project can contain up to ${MAX_PROJECT_FILES} source files.`);
	}
	const sourceBytes = byteLength(source);
	if (sourceBytes > MAX_SOURCE_FILE_BYTES) {
		throw new TypeError(`A source file must be no larger than ${MAX_SOURCE_FILE_BYTES / 1024 / 1024} MiB.`);
	}
	const projectBytes = project.files.reduce((sum, file) => sum + byteLength(file.source), 0);
	if (projectBytes + sourceBytes > MAX_PROJECT_SOURCE_BYTES) {
		throw new TypeError(`Project sources must be no larger than ${MAX_PROJECT_SOURCE_BYTES / 1024 / 1024} MiB in total.`);
	}
}

export function normalizeProjectName(value: string) {
	const name = value.trim().normalize('NFC').replace(/[\u0000-\u001f\u007f]/g, '');
	if (!name) throw new TypeError('Enter a project name.');
	if (Array.from(name).length > 80) throw new TypeError('Project names must be 80 characters or fewer.');
	return name;
}

export function uniqueProjectName(existingNames: Iterable<string>, proposedName: string) {
	const name = normalizeProjectName(proposedName);
	const existing = new Set(Array.from(existingNames, (value) => value.normalize('NFC').toLowerCase()));
	if (!existing.has(name.toLowerCase())) return name;

	const characters = Array.from(name);
	for (let suffix = 2; ; suffix += 1) {
		const suffixText = ` ${suffix}`;
		const stem = characters.slice(0, 80 - Array.from(suffixText).length).join('').trimEnd();
		const candidate = `${stem}${suffixText}`;
		if (!existing.has(candidate.toLowerCase())) return candidate;
	}
}

function validateProject(bundle: ProjectBundle) {
	bundle.name = normalizeProjectName(bundle.name);
	if (bundle.files.length < 1 || bundle.files.length > MAX_PROJECT_FILES) {
		throw new TypeError(`A project must contain between 1 and ${MAX_PROJECT_FILES} source files.`);
	}

	const paths = new Set<string>();
	let totalBytes = 0;
	for (const file of bundle.files) {
		file.path = normalizeProjectFilePath(file.path);
		const key = file.path.toLowerCase();
		if (paths.has(key)) throw new TypeError(`The project contains duplicate file path "${file.path}".`);
		paths.add(key);
		const sourceBytes = byteLength(file.source);
		if (sourceBytes > MAX_SOURCE_FILE_BYTES) {
			throw new TypeError(`${file.path} exceeds the 2 MiB per-file limit.`);
		}
		totalBytes += sourceBytes;
	}
	if (totalBytes > MAX_PROJECT_SOURCE_BYTES) {
		throw new TypeError('The project sources exceed the 12 MiB total limit.');
	}
}

export function normalizeProjectFilePath(value: string) {
	value = value.normalize('NFC');
	if (!value || value.includes('\\') || value.startsWith('/') || /^[a-z]:/i.test(value)) {
		throw new TypeError(`"${value}" is not a safe project-relative path.`);
	}
	const segments = value.split('/');
	if (
		segments.some(
			(segment) =>
				!segment || segment === '.' || segment === '..' || segment.trim() !== segment || /[<>:"|?*\u0000-\u001f\u007f]/.test(segment)
		)
	) {
		throw new TypeError(`"${value}" is not a safe project-relative path.`);
	}
	if (!value.toLowerCase().endsWith('.schemra')) {
		throw new TypeError(`"${value}" must point to a .schemra source file.`);
	}
	return value;
}

function byteLength(value: string) {
	return new TextEncoder().encode(value).byteLength;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}
