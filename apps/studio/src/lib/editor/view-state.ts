export type SelectionDirection = 'forward' | 'backward' | 'none';

export interface EditorSelection {
	start: number;
	end: number;
	direction: SelectionDirection;
}

export interface EditorViewState {
	selection: EditorSelection;
	scrollLeft: number;
	scrollTop: number;
}

export function editorDocumentKey(projectId: string | null | undefined, documentId: string | null | undefined): string | null {
	if (!documentId) return null;
	return JSON.stringify([projectId ?? null, documentId]);
}

export function rememberEditorViewState(
	states: Map<string, EditorViewState>,
	key: string | null,
	state: EditorViewState
): void {
	if (!key) return;
	states.set(key, {
		selection: { ...state.selection },
		scrollLeft: finiteNonNegative(state.scrollLeft),
		scrollTop: finiteNonNegative(state.scrollTop)
	});
}

export function restoreEditorViewState(
	states: ReadonlyMap<string, EditorViewState>,
	key: string | null,
	sourceLength: number
): EditorViewState {
	const stored = key ? states.get(key) : undefined;
	if (!stored) return { selection: { start: 0, end: 0, direction: 'none' }, scrollLeft: 0, scrollTop: 0 };

	const limit = Number.isFinite(sourceLength) ? Math.max(0, Math.trunc(sourceLength)) : 0;
	const start = clampOffset(stored.selection.start, limit);
	const end = clampOffset(stored.selection.end, limit);
	const direction = stored.selection.direction === 'forward' || stored.selection.direction === 'backward'
		? stored.selection.direction
		: 'none';
	return {
		selection: { start: Math.min(start, end), end: Math.max(start, end), direction },
		scrollLeft: finiteNonNegative(stored.scrollLeft),
		scrollTop: finiteNonNegative(stored.scrollTop)
	};
}

function clampOffset(offset: number, limit: number): number {
	return Math.min(limit, Math.max(0, Number.isFinite(offset) ? Math.trunc(offset) : 0));
}

function finiteNonNegative(value: number): number {
	return Number.isFinite(value) ? Math.max(0, value) : 0;
}
