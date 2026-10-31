import type { ResourceTabKind } from './resource-tab-menu.js';
import { resourceTabMenuPolicy } from './resource-tab-menu.js';
import { isErrorDiagnostic } from './project-view.js';
import { sourceLabel } from './project-sidebar.js';

export type WorkspaceTabDocument = {
	id: string;
	path: string;
};

export type WorkspaceTabCompilation = {
	id: string;
	kind: string;
	visual_id: string | null;
};

export type WorkspaceTabNavigationKey = 'ArrowLeft' | 'ArrowRight' | 'Home' | 'End';

export type WorkspaceTabView = {
	id: string;
	path: string;
	title: string;
	resourceKind: ResourceTabKind;
	displayKind: 'visual' | 'scenario' | 'component' | 'source';
	hasErrors: boolean;
	unsaved: boolean;
	rename?: () => void;
	remove: () => void;
	renameDisabled: boolean;
	removeDisabled: boolean;
};

type WorkspaceTabDeclaration = { kind: string; id: string | null; title: string };

type WorkspaceTabViewInput<Declaration extends WorkspaceTabDeclaration> = {
	files: readonly { id: string; path: string; source: string; savedSource: string }[];
	compilation: readonly {
		id: string;
		kind: string;
		diagnostics: readonly unknown[];
		declarations: readonly Declaration[];
	}[];
	tabIds: readonly string[];
	activeFileId: string | null;
	currentDeclaration: Declaration | null;
	projectFileCount: number;
	initialized: boolean;
	compiling: boolean;
	projectValid: boolean;
	onRename: (fileId: string, declaration: Declaration) => void;
	onRemove: (fileId: string, kind: ResourceTabKind, declaration: Declaration | null) => void;
};

/** Returns the active visual's documents first, followed by an independently selected source if needed. */
export function buildWorkspaceTabIds(
	documents: readonly WorkspaceTabDocument[],
	compilation: readonly WorkspaceTabCompilation[],
	activeVisualId: string | null,
	activeFileId: string | null
) {
	if (!documents.length) return [];
	const compiledById = new Map(compilation.map((file) => [file.id, file]));
	const visualDocument = documents.find((document) => {
		const compiled = compiledById.get(document.id);
		return compiled?.kind === 'visual' && compiled.visual_id === activeVisualId;
	}) ?? documents.find((document) => document.path.startsWith('visuals/'));
	const linkedScenarioIds = new Set(compilation
		.filter((file) => file.kind === 'scenario' && file.visual_id === activeVisualId)
		.map((file) => file.id));
	const tabs = documents
		.filter((document) => document.id === visualDocument?.id || linkedScenarioIds.has(document.id))
		.map(({ id }) => id);
	if (activeFileId && documents.some(({ id }) => id === activeFileId) && !tabs.includes(activeFileId)) {
		tabs.push(activeFileId);
	}
	return tabs;
}

/** Build tab display and menu data from project files and their latest compilation. */
export function buildWorkspaceTabViews<Declaration extends WorkspaceTabDeclaration>({
	files,
	compilation,
	tabIds,
	activeFileId,
	currentDeclaration,
	projectFileCount,
	initialized,
	compiling,
	projectValid,
	onRename,
	onRemove
}: WorkspaceTabViewInput<Declaration>): WorkspaceTabView[] {
	const filesById = new Map(files.map((file) => [file.id, file]));
	const compilationById = new Map(compilation.map((file) => [file.id, file]));

	return tabIds.flatMap((id) => {
		const file = filesById.get(id);
		if (!file) return [];
		const fileCompilation = compilationById.get(id);
		const resourceKind: ResourceTabKind = file.path.startsWith('visuals/')
			? 'visual'
			: file.path.startsWith('scenarios/') ? 'scenario' : 'source';
		const displayKind = fileCompilation?.kind === 'visual'
			? 'visual'
			: fileCompilation?.kind === 'scenario'
				? 'scenario'
				: file.path.startsWith('components/') ? 'component' : 'source';
		const declarations = fileCompilation?.declarations.filter(({ kind }) => kind === resourceKind) ?? [];
		const declaration = id === activeFileId && currentDeclaration?.kind === resourceKind
			? currentDeclaration
			: declarations[0] ?? fileCompilation?.declarations[0] ?? null;
		const policy = resourceTabMenuPolicy({
			kind: resourceKind,
			declarationCount: declarations.length,
			projectFileCount,
			initialized,
			compiling,
			projectValid
		});

		return [{
			id: file.id,
			path: file.path,
			title: declaration?.title ?? sourceLabel(file.path),
			resourceKind,
			displayKind,
			hasErrors: fileCompilation?.diagnostics.some(isErrorDiagnostic) ?? false,
			unsaved: file.source !== file.savedSource,
			rename: policy.showRename && declaration ? () => onRename(file.id, declaration) : undefined,
			remove: () => onRemove(file.id, resourceKind, declaration),
			renameDisabled: policy.renameDisabled,
			removeDisabled: policy.removeDisabled
		}];
	});
}

export function nextWorkspaceTabIndex(
	currentIndex: number,
	key: WorkspaceTabNavigationKey,
	tabCount: number
): number | null {
	if (!Number.isInteger(tabCount) || tabCount <= 0) return null;
	if (key === 'Home') return 0;
	if (key === 'End') return tabCount - 1;
	const current = Number.isInteger(currentIndex) ? currentIndex : 0;
	const delta = key === 'ArrowRight' ? 1 : -1;
	return (current + delta + tabCount) % tabCount;
}
