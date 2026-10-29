interface PaletteDocument {
	id: string;
	path: string;
	source: string;
	savedSource: string;
}

interface PaletteProject {
	id: string;
	name: string;
	files: readonly PaletteDocument[];
}

export interface CommandPaletteAction {
	id: string;
	label: string;
	description?: string;
	keywords?: readonly string[];
	shortcut?: string;
	disabled?: boolean;
}

export interface CommandPaletteContext {
	projects: readonly PaletteProject[];
	activeProjectId: string | null;
	activeFileId: string | null;
	openTabs?: readonly { projectId: string; fileId: string }[];
	actions?: readonly CommandPaletteAction[];
}

interface CommandPaletteEntryBase {
	key: string;
	title: string;
	searchTerms: readonly string[];
	disabled: boolean;
}

export interface CommandPaletteFileEntry extends CommandPaletteEntryBase {
	type: 'file';
	group: 'Open tabs' | 'Project files';
	projectId: string;
	projectName: string;
	fileId: string;
	path: string;
	kind: 'visual' | 'scenario' | 'source';
	current: boolean;
	unsaved: boolean;
}

export interface CommandPaletteProjectEntry extends CommandPaletteEntryBase {
	type: 'project';
	group: 'Projects';
	projectId: string;
	fileCount: number;
	current: boolean;
}

export interface CommandPaletteActionEntry extends CommandPaletteEntryBase {
	type: 'action';
	group: 'Actions';
	actionId: string;
	description: string;
	shortcut: string | null;
}

export type CommandPaletteEntry =
	| CommandPaletteFileEntry
	| CommandPaletteProjectEntry
	| CommandPaletteActionEntry;

/** Build stable, grouped entries for open tabs, project files, projects, and actions. */
export function buildCommandPaletteEntries(context: CommandPaletteContext): CommandPaletteEntry[] {
	const projectsById = new Map(context.projects.map((project) => [project.id, project]));
	const tabs = uniqueTabFiles(context.openTabs ?? [], projectsById);
	const openKeys = new Set(tabs.map(({ projectId, fileId }) => fileKey(projectId, fileId)));
	const entries: CommandPaletteEntry[] = [];

	for (const tab of tabs) {
		const project = projectsById.get(tab.projectId);
		const file = project?.files.find(({ id }) => id === tab.fileId);
		if (project && file) entries.push(fileEntry(project, file, context, 'Open tabs'));
	}

	for (const project of context.projects) {
		for (const file of project.files) {
			if (!openKeys.has(fileKey(project.id, file.id))) {
				entries.push(fileEntry(project, file, context, 'Project files'));
			}
		}
	}

	for (const project of context.projects) {
		entries.push({
			key: `project:${project.id}`,
			type: 'project',
			group: 'Projects',
			projectId: project.id,
			title: project.name,
			fileCount: project.files.length,
			current: project.id === context.activeProjectId,
			disabled: false,
			searchTerms: [project.name, project.id, `${project.files.length} files`]
		});
	}

	for (const action of context.actions ?? []) {
		entries.push({
			key: `action:${action.id}`,
			type: 'action',
			group: 'Actions',
			actionId: action.id,
			title: action.label,
			description: action.description ?? '',
			shortcut: action.shortcut ?? null,
			disabled: action.disabled ?? false,
			searchTerms: [action.label, action.description ?? '', ...(action.keywords ?? [])]
		});
	}

	return entries;
}

/** Filter entries by all query words, preserving stable order for equally good matches. */
export function searchCommandPalette(
	entries: readonly CommandPaletteEntry[],
	query: string
): CommandPaletteEntry[] {
	const terms = normalize(query).split(/\s+/).filter(Boolean);
	if (terms.length === 0) return [...entries];

	return entries
		.map((entry, index) => ({ entry, index, score: scoreEntry(entry, terms) }))
		.filter((item): item is { entry: CommandPaletteEntry; index: number; score: number } => item.score !== null)
		.sort((left, right) => left.score - right.score || left.index - right.index)
		.map(({ entry }) => entry);
}

export function firstCommandPaletteSelection(entries: readonly CommandPaletteEntry[]): number {
	return entries.findIndex((entry) => !entry.disabled);
}

export function lastCommandPaletteSelection(entries: readonly CommandPaletteEntry[]): number {
	for (let index = entries.length - 1; index >= 0; index -= 1) {
		if (!entries[index]?.disabled) return index;
	}
	return -1;
}

/** Move the active result with wraparound while skipping unavailable actions. */
export function moveCommandPaletteSelection(
	entries: readonly CommandPaletteEntry[],
	currentIndex: number,
	direction: -1 | 1
): number {
	if (entries.length === 0) return -1;
	const startIndex = Number.isInteger(currentIndex) && currentIndex >= 0 && currentIndex < entries.length
		? currentIndex
		: direction === 1 ? -1 : 0;
	for (let step = 1; step <= entries.length; step += 1) {
		const candidate = (startIndex + direction * step + entries.length) % entries.length;
		if (!entries[candidate]?.disabled) return candidate;
	}
	return -1;
}

/** Return file kind from its DSL declaration, falling back to the conventional project folder. */
export function commandPaletteFileKind(file: Pick<PaletteDocument, 'path' | 'source'>): CommandPaletteFileEntry['kind'] {
	const declared = /^\s*(visual|scenario)\b/m.exec(file.source)?.[1];
	if (declared === 'visual' || declared === 'scenario') return declared;
	const directory = file.path.split('/')[0]?.replace(/s$/, '');
	return directory === 'visual' || directory === 'scenario' ? directory : 'source';
}

export function commandPaletteFileName(path: string): string {
	return path.split('/').at(-1)?.replace(/\.schemra$/i, '') ?? path;
}

function fileEntry(
	project: PaletteProject,
	file: PaletteDocument,
	context: CommandPaletteContext,
	group: CommandPaletteFileEntry['group']
): CommandPaletteFileEntry {
	const kind = commandPaletteFileKind(file);
	const title = commandPaletteFileName(file.path);
	return {
		key: `${group === 'Open tabs' ? 'tab' : 'file'}:${project.id}:${file.id}`,
		type: 'file',
		group,
		projectId: project.id,
		projectName: project.name,
		fileId: file.id,
		path: file.path,
		title,
		kind,
		current: project.id === context.activeProjectId && file.id === context.activeFileId,
		unsaved: file.source !== file.savedSource,
		disabled: false,
		searchTerms: [title, file.path, file.id, kind, project.name, project.id]
	};
}

function uniqueTabFiles(
	tabs: readonly { projectId: string; fileId: string }[],
	projects: ReadonlyMap<string, PaletteProject>
) {
	const seen = new Set<string>();
	return tabs.filter(({ projectId, fileId }) => {
		const key = fileKey(projectId, fileId);
		if (seen.has(key) || !projects.get(projectId)?.files.some((file) => file.id === fileId)) return false;
		seen.add(key);
		return true;
	});
}

function fileKey(projectId: string, fileId: string) {
	return `${projectId}\u0000${fileId}`;
}

function scoreEntry(entry: CommandPaletteEntry, terms: readonly string[]): number | null {
	const title = normalize(entry.title);
	const fields = [title, ...entry.searchTerms.map(normalize)].filter(Boolean);
	let score = 0;
	for (const term of terms) {
		let best = Infinity;
		for (const field of fields) {
			if (field === term) best = Math.min(best, 0);
			else if (field.startsWith(term)) best = Math.min(best, 1);
			else if (field.includes(term)) best = Math.min(best, 2);
			else if (isSubsequence(term, field)) best = Math.min(best, 4);
		}
		if (!Number.isFinite(best)) return null;
		score += best;
	}
	return score;
}

function normalize(value: string) {
	return value.normalize('NFKD').replace(/[\u0300-\u036f]/g, '').toLocaleLowerCase().trim();
}

function isSubsequence(needle: string, haystack: string) {
	let index = 0;
	for (const character of haystack) {
		if (character === needle[index]) index += 1;
		if (index === needle.length) return true;
	}
	return false;
}
