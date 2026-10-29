import type { ResourceTabKind } from './resource-tab-menu.js';

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
