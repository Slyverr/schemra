import initializeWasm, {
	ScenarioSampler as BrowserScenarioSampler,
	compile_project_schemra,
	compile_schemra,
	complete_schemra,
	complete_schemra_with_nodes,
	format_schemra,
	rename_project_schemra
} from './generated/schemra_wasm.js';

export interface ProjectSource {
	id: string;
	path: string;
	source: string;
}

export interface SourceLocation {
	offset: number;
	line: number;
	column: number;
}

export interface SourceSpan {
	start: SourceLocation;
	end: SourceLocation;
}

export interface CompletionItem {
	label: string;
	insert_text: string;
	detail: string;
	kind: 'keyword' | 'value' | 'identifier';
}

export interface CompletionList {
	start: number;
	end: number;
	items: CompletionItem[];
}

export interface CompileResult {
	ok: boolean;
	svg: string | null;
	title: string | null;
	stats: { nodes: number; edges: number; groups: number } | null;
	diagnostics: unknown[];
	tokens: unknown[];
	declarations: unknown[];
	scenarios: unknown[];
	scene: unknown | null;
}

export interface ProjectCompileResult {
	error: string | null;
	files: unknown[];
	visuals: { id: string; path: string; title: string; output: CompileResult }[];
}

export interface RenameSelection {
	name: string;
	kind: string;
	span: SourceSpan;
}

export interface RenameEdit {
	path: string;
	span: SourceSpan;
	replacement: string;
}

export interface RenameResult {
	error: string | null;
	selection: RenameSelection | null;
	edits: RenameEdit[];
}

export interface ScenarioPlayback {
	durationMs: number;
	sample(timeMs: number): Record<string, unknown>;
	dispose(): void;
}

let wasmReady: Promise<void> | undefined;

/** Loads the browser WASM module once and shares initialization between calls. */
export function initializeSchemra(): Promise<void> {
	if (!wasmReady) {
		const pending = initializeWasm().then(() => undefined);
		wasmReady = pending.catch((error: unknown) => {
			wasmReady = undefined;
			throw error;
		});
	}
	return wasmReady;
}

export async function compileSource(source: string): Promise<CompileResult> {
	await initializeSchemra();
	return parseJson<CompileResult>(compile_schemra(source));
}

export async function compileProject(sources: ProjectSource[]): Promise<ProjectCompileResult> {
	await initializeSchemra();
	return parseJson<ProjectCompileResult>(compile_project_schemra(JSON.stringify({ sources })));
}

export async function formatSource(source: string): Promise<string> {
	await initializeSchemra();
	return format_schemra(source);
}

/** Cursor offsets use JavaScript/UTF-16 positions; completion ranges are returned in that form. */
export async function completeAt(
	source: string,
	cursorOffset: number,
	explicit = false
): Promise<CompletionList | null> {
	await initializeSchemra();
	const result = parseJson<CompletionList | null>(
		complete_schemra(source, toUtf8Offset(source, cursorOffset), explicit)
	);
	return result ? completionToUtf16(source, result) : null;
}

/** Adds project node IDs to completion context while preserving editor UTF-16 positions. */
export async function completeAtWithNodes(
	source: string,
	cursorOffset: number,
	nodeIds: string[],
	explicit = false
): Promise<CompletionList | null> {
	await initializeSchemra();
	const result = parseJson<CompletionList | null>(
		complete_schemra_with_nodes(
			source,
			toUtf8Offset(source, cursorOffset),
			explicit,
			JSON.stringify({ node_ids: nodeIds })
		)
	);
	return result ? completionToUtf16(source, result) : null;
}

/** Rename offsets and returned spans use editor-friendly JavaScript/UTF-16 positions. */
export async function renameProject(
	sources: ProjectSource[],
	path: string,
	cursorOffset: number,
	newName?: string
): Promise<RenameResult> {
	await initializeSchemra();
	const source = sources.find((item) => item.path === path);
	if (!source) {
		return { error: 'The source file is unavailable.', selection: null, edits: [] };
	}

	const request = JSON.stringify({
		sources,
		path,
		offset: toUtf8Offset(source.source, cursorOffset),
		new_name: newName
	});
	const result = parseJson<RenameResult>(rename_project_schemra(request));
	if (result.selection) {
		result.selection.span = spanToUtf16(source.source, result.selection.span);
	}
	const sourceByPath = new Map(sources.map((item) => [item.path, item.source]));
	for (const edit of result.edits) {
		const editSource = sourceByPath.get(edit.path);
		if (editSource) edit.span = spanToUtf16(editSource, edit.span);
	}
	return result;
}

/** Creates a reusable playback sampler from one linked project scenario. */
export async function createScenarioPlayback(
	sources: ProjectSource[],
	visualId: string,
	scenarioId: string
): Promise<ScenarioPlayback> {
	await initializeSchemra();
	const sampler = new BrowserScenarioSampler(JSON.stringify({ sources }), visualId, scenarioId);
	const durationMs = sampler.durationMs();
	let disposed = false;

	return {
		durationMs,
		sample(timeMs) {
			if (disposed) throw new Error('This scenario playback sampler has been disposed.');
			const boundedTime = Number.isFinite(timeMs)
				? Math.max(0, Math.min(0xffff_ffff, Math.trunc(timeMs)))
				: 0;
			return parseJson<Record<string, unknown>>(sampler.sample(boundedTime));
		},
		dispose() {
			if (!disposed) {
				sampler.free();
				disposed = true;
			}
		}
	};
}

function parseJson<T>(json: string): T {
	return JSON.parse(json) as T;
}

function toUtf8Offset(source: string, utf16Offset: number): number {
	const requested = Number.isFinite(utf16Offset) ? Math.trunc(utf16Offset) : 0;
	let offset = Math.max(0, Math.min(source.length, requested));
	if (offset > 0 && offset < source.length) {
		const previous = source.charCodeAt(offset - 1);
		const next = source.charCodeAt(offset);
		if (previous >= 0xd800 && previous <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) {
			offset -= 1;
		}
	}
	return new TextEncoder().encode(source.slice(0, offset)).length;
}

function toUtf16Offset(source: string, utf8Offset: number): number {
	let byteOffset = 0;
	let editorOffset = 0;
	const encoder = new TextEncoder();
	for (const character of source) {
		if (byteOffset >= utf8Offset) break;
		const characterBytes = encoder.encode(character).length;
		if (byteOffset + characterBytes > utf8Offset) break;
		byteOffset += characterBytes;
		editorOffset += character.length;
	}
	return editorOffset;
}

function completionToUtf16(source: string, completion: CompletionList): CompletionList {
	return {
		...completion,
		start: toUtf16Offset(source, completion.start),
		end: toUtf16Offset(source, completion.end)
	};
}

function spanToUtf16(source: string, span: SourceSpan): SourceSpan {
	const start = toUtf16Offset(source, span.start.offset);
	const end = toUtf16Offset(source, span.end.offset);
	return {
		start: { ...span.start, offset: start, column: utf16Column(source, start, span.start.line) },
		end: { ...span.end, offset: end, column: utf16Column(source, end, span.end.line) }
	};
}

function utf16Column(source: string, offset: number, line: number): number {
	let lineStart = 0;
	for (let currentLine = 1; currentLine < line; currentLine += 1) {
		const lineBreak = source.indexOf('\n', lineStart);
		if (lineBreak < 0) return offset - lineStart + 1;
		lineStart = lineBreak + 1;
	}
	return offset - lineStart + 1;
}
