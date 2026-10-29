import type { EditorSymbol, EditorSymbolRole } from '../schemra.js';

export interface EditorSymbolFile {
	id: string;
	path: string;
	editor_symbols?: readonly EditorSymbol[];
}

/** Resolve a semantic symbol under an editor caret, preferring the narrowest span. */
export function findEditorSymbolAtOffset(
	symbols: readonly EditorSymbol[],
	offset: number,
	role?: EditorSymbolRole
): EditorSymbol | null {
	if (!Number.isInteger(offset) || offset < 0) return null;
	let match: EditorSymbol | null = null;
	for (const symbol of symbols) {
		if (role && symbol.role !== role) continue;
		if (offset < symbol.span.start.offset || offset > symbol.span.end.offset) continue;
		if (!match || symbol.span.end.offset - symbol.span.start.offset < match.span.end.offset - match.span.start.offset) {
			match = symbol;
		}
	}
	return match;
}

/** Return one unambiguous definition for a linked symbol. */
export function findEditorSymbolDefinition(
	files: readonly EditorSymbolFile[],
	reference: Pick<EditorSymbol, 'kind' | 'target_key'>
): { fileId: string; path: string; symbol: EditorSymbol } | null {
	let definition: { fileId: string; path: string; symbol: EditorSymbol } | null = null;
	for (const file of files) {
		for (const symbol of file.editor_symbols ?? []) {
			if (
				symbol.role !== 'definition' ||
				symbol.kind !== reference.kind ||
				symbol.target_key !== reference.target_key
			) continue;
			if (definition) return null;
			definition = { fileId: file.id, path: file.path, symbol };
		}
	}
	return definition;
}

/** Resolve any definition or reference at an offset to its unique editable definition. */
export function findEditorSymbolDefinitionAtOffset(
	files: readonly EditorSymbolFile[],
	fileId: string,
	offset: number
): { fileId: string; path: string; symbol: EditorSymbol } | null {
	const file = files.find((candidate) => candidate.id === fileId);
	if (!file) return null;
	const symbol = findEditorSymbolAtOffset(file.editor_symbols ?? [], offset);
	return symbol ? findEditorSymbolDefinition(files, symbol) : null;
}
