import type { EditorSymbol, SourceSpan } from '../schemra.js';
import { utf8SpanToEditorSpan } from './source-edits.js';

export interface RefactorDiagnostic {
	code: string;
	severity: 'error' | 'warning' | 'information';
	message: string;
	hint?: string | null;
	span: SourceSpan;
	path?: string;
}

export interface RefactorSymbol {
	name: string;
	kind: 'visual' | 'node' | 'scenario';
	role: 'definition' | 'reference';
	target_key: string;
	span: SourceSpan;
	path?: string;
}

export interface RefactorFile {
	path: string;
	diagnostics: readonly RefactorDiagnostic[];
}

/** Validate compiler diagnostics while attaching their owning source path. */
export function readRefactorDiagnostics(value: unknown, path: string): RefactorDiagnostic[] {
	if (!Array.isArray(value)) return [];
	return value.flatMap((candidate): RefactorDiagnostic[] => {
		if (
			!isRecord(candidate) ||
			typeof candidate.code !== 'string' ||
			typeof candidate.message !== 'string' ||
			!['error', 'warning', 'information'].includes(String(candidate.severity)) ||
			!isSourceSpan(candidate.span)
		) return [];
		return [{
			code: candidate.code,
			severity: candidate.severity as RefactorDiagnostic['severity'],
			message: candidate.message,
			hint: typeof candidate.hint === 'string' ? candidate.hint : null,
			span: candidate.span,
			path
		}];
	});
}

/** Convert one editor-facing UTF-16 symbol span to compiler UTF-8 coordinates. */
export function editorSymbolForRefactor(
	source: string,
	symbol: EditorSymbol,
	path: string
): RefactorSymbol | null {
	const start = editorLocationToSourceLocation(source, symbol.span.start);
	const end = editorLocationToSourceLocation(source, symbol.span.end);
	if (!start || !end || source.slice(symbol.span.start.offset, symbol.span.end.offset) !== symbol.name) return null;
	return {
		...symbol,
		span: { start, end },
		path
	};
}

export interface DiagnosticQuickFix {
	start: number;
	end: number;
	replacement: string;
}

export interface DiagnosticQuickFixEdit {
	start: number;
	end: number;
	text: string;
}

const nodeDiagnosticCodes = new Set([
	'E_UNKNOWN_EDGE_NODE',
	'E_UNKNOWN_MOVE_NODE',
	'E_UNKNOWN_HIGHLIGHT_NODE'
]);

/** Return close node names for a single unresolved endpoint in an edge or scenario action. */
export function endpointQuickFixCandidates(
	source: string,
	symbols: readonly RefactorSymbol[],
	diagnostic: RefactorDiagnostic
): string[] {
	const target = endpointQuickFixTarget(source, symbols, diagnostic);
	if (!target) return [];
	return nearestWords(target.name, target.choices);
}

/** Find the one unresolved node token associated with a node-reference diagnostic. */
export function endpointQuickFixTarget(
	source: string,
	symbols: readonly RefactorSymbol[],
	diagnostic: RefactorDiagnostic
): { name: string; start: number; end: number; choices: string[] } | null {
	if (!nodeDiagnosticCodes.has(diagnostic.code)) return null;

	if (diagnostic.code === 'E_UNKNOWN_EDGE_NODE') {
		return edgeQuickFixTarget(source, symbols, diagnostic);
	}

	const diagnosticSpan = editorSpan(source, diagnostic.span);
	if (!diagnosticSpan) return null;
	const references = symbols
		.filter((symbol) => symbol.kind === 'node' && symbol.role === 'reference' &&
			isInDiagnostic(source, symbol, diagnostic, diagnosticSpan))
		.map((symbol) => validatedSymbol(source, symbol))
		.filter((symbol): symbol is ValidSymbol => symbol !== null)
		.sort((left, right) => left.start - right.start);
	if (references.length === 0) return null;

	const definitions = symbols.filter((symbol) =>
		symbol.kind === 'node' && symbol.role === 'definition'
	);
	const unresolved = references.filter((reference) => !definitions.some((definition) =>
		definition.target_key === reference.target_key
	));
	if (unresolved.length !== 1) return null;

	const reference = unresolved[0]!;
	const scope = nodeScope(reference.target_key);
	if (!scope) return null;
	const choices = [...new Set(definitions
		.filter((definition) => nodeScope(definition.target_key) === scope)
		.map((definition) => definition.name))];
	return { name: reference.name, start: reference.start, end: reference.end, choices };
}

/** Build conservative typo fixes for unknown node references, properties, and enum values. */
export function diagnosticQuickFixOptions(
	source: string,
	symbols: readonly RefactorSymbol[],
	diagnostic: RefactorDiagnostic
): DiagnosticQuickFix[] {
	if (nodeDiagnosticCodes.has(diagnostic.code)) {
		const target = endpointQuickFixTarget(source, symbols, diagnostic);
		if (!target) return [];
		return nearestWords(target.name, target.choices).map((replacement) => ({
			start: target.start,
			end: target.end,
			replacement
		}));
	}

	const span = editorSpan(source, diagnostic.span);
	if (!span || span.start.offset === span.end.offset) return [];
	const raw = source.slice(span.start.offset, span.end.offset);
	const value = raw.startsWith('"') && raw.endsWith('"') ? raw.slice(1, -1) : raw;
	if (!isQuickFixWord(value)) return [];

	const choices = quickFixWords(diagnostic);
	return nearestWords(value, choices).map((replacement) => ({
		start: span.start.offset,
		end: span.end.offset,
		replacement
	}));
}

/** Resolve a visible suggestion to a validated source edit. */
export function diagnosticQuickFixEdit(
	source: string,
	symbols: readonly RefactorSymbol[],
	diagnostic: RefactorDiagnostic,
	replacement: string
): DiagnosticQuickFixEdit | null {
	const option = diagnosticQuickFixOptions(source, symbols, diagnostic)
		.find((candidate) => candidate.replacement === replacement);
	return option ? { start: option.start, end: option.end, text: option.replacement } : null;
}

/** Count errors by stable path/code/message identity for a before/after comparison. */
export function diagnosticCounts(files: readonly RefactorFile[]): Map<string, number> {
	const counts = new Map<string, number>();
	for (const file of files) {
		for (const diagnostic of file.diagnostics) {
			if (diagnostic.severity !== 'error') continue;
			const key = diagnosticKey(file.path, diagnostic);
			counts.set(key, (counts.get(key) ?? 0) + 1);
		}
	}
	return counts;
}

/** Accept a candidate only when it removes the selected error and creates no new errors. */
export function isSafeQuickFix(
	filePath: string,
	diagnostic: RefactorDiagnostic,
	before: ReadonlyMap<string, number>,
	afterFiles: readonly RefactorFile[]
): boolean {
	if (diagnostic.severity !== 'error') return false;
	const after = diagnosticCounts(afterFiles);
	const key = diagnosticKey(filePath, diagnostic);
	if ((after.get(key) ?? 0) >= (before.get(key) ?? 0)) return false;
	return introducesNoErrors(before, after);
}

/** Check that a candidate project state introduces no additional errors. */
export function projectIntroducesNoErrors(
	beforeFiles: readonly RefactorFile[],
	afterFiles: readonly RefactorFile[]
): boolean {
	return introducesNoErrors(diagnosticCounts(beforeFiles), diagnosticCounts(afterFiles));
}

interface ValidSymbol {
	name: string;
	target_key: string;
	start: number;
	end: number;
}

function edgeQuickFixTarget(
	source: string,
	symbols: readonly RefactorSymbol[],
	diagnostic: RefactorDiagnostic
) {
	const span = editorSpan(source, diagnostic.span);
	if (!span || span.start.offset === span.end.offset) return null;
	const edgeText = source.slice(span.start.offset, span.end.offset);
	const match = /^\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:->|--)\s*([A-Za-z_][A-Za-z0-9_]*)/.exec(edgeText);
	if (!match) return null;

	const from = match[1]!;
	const to = match[2]!;
	const fromStart = span.start.offset + edgeText.indexOf(from);
	const toStart = span.start.offset + edgeText.indexOf(to, fromStart - span.start.offset + from.length);
	if (
		source.slice(fromStart, fromStart + from.length) !== from ||
		source.slice(toStart, toStart + to.length) !== to
	) return null;

	const code = maskCommentsAndStrings(source);
	const declared = new Set(readNodeDeclarations(code));
	const visualId = readVisualId(code);
	for (const symbol of symbols) {
		if (symbol.kind !== 'node' || symbol.role !== 'definition' || !samePath(symbol, diagnostic)) continue;
		if (visualId && nodeScope(symbol.target_key) !== `node:${visualId}`) continue;
		declared.add(symbol.name);
	}
	const missing = [
		{ name: from, start: fromStart, end: fromStart + from.length },
		{ name: to, start: toStart, end: toStart + to.length }
	].filter((endpoint) => !declared.has(endpoint.name));
	if (missing.length !== 1) return null;

	const target = missing[0]!;
	return {
		...target,
		choices: [...declared].filter((name) => name !== target.name)
	};
}

function isInDiagnostic(
	source: string,
	symbol: RefactorSymbol,
	diagnostic: RefactorDiagnostic,
	span: SourceSpan
): boolean {
	if (!samePath(symbol, diagnostic)) return false;
	const symbolSpan = editorSpan(source, symbol.span);
	return symbolSpan !== null &&
		symbolSpan.start.offset >= span.start.offset &&
		symbolSpan.end.offset <= span.end.offset;
}

function validatedSymbol(source: string, symbol: RefactorSymbol): ValidSymbol | null {
	const span = editorSpan(source, symbol.span);
	if (!span || source.slice(span.start.offset, span.end.offset) !== symbol.name) return null;
	return {
		name: symbol.name,
		target_key: symbol.target_key,
		start: span.start.offset,
		end: span.end.offset
	};
}

function editorSpan(source: string, span: SourceSpan): SourceSpan | null {
	try {
		const converted = utf8SpanToEditorSpan(source, span);
		const { start, end } = converted;
		if (
			!Number.isInteger(start.offset) || !Number.isInteger(end.offset) ||
			start.offset < 0 || end.offset <= start.offset || end.offset > source.length ||
			!isUtf16Boundary(source, start.offset) || !isUtf16Boundary(source, end.offset) ||
			!matchesSourcePosition(source, start) || !matchesSourcePosition(source, end)
		) return null;
		return converted;
	} catch {
		return null;
	}
}

function samePath(symbol: RefactorSymbol, diagnostic: RefactorDiagnostic): boolean {
	return !symbol.path || !diagnostic.path || symbol.path === diagnostic.path;
}

function nodeScope(targetKey: string): string | null {
	if (!targetKey.startsWith('node:')) return null;
	const lastColon = targetKey.lastIndexOf(':');
	return lastColon > 5 ? targetKey.slice(0, lastColon) : null;
}

function readVisualId(source: string): string | null {
	return /(?:^|[{};\n])\s*visual\s+([A-Za-z_][A-Za-z0-9_]*)\b/.exec(source)?.[1] ?? null;
}

function readNodeDeclarations(source: string): string[] {
	const names: string[] = [];
	const pattern = /(?:^|[{};\n])\s*node\s+([A-Za-z_][A-Za-z0-9_]*)\b/g;
	for (const match of source.matchAll(pattern)) {
		if (match[1]) names.push(match[1]);
	}
	return names;
}

function maskCommentsAndStrings(source: string): string {
	const characters = source.split('');
	let inString = false;
	let escaped = false;
	let inComment = false;
	for (let index = 0; index < source.length; index += 1) {
		const character = source[index]!;
		if (inComment) {
			if (character === '\n') inComment = false;
			else characters[index] = ' ';
			continue;
		}
		if (inString) {
			if (character === '\n') inString = false;
			else characters[index] = ' ';
			if (escaped) escaped = false;
			else if (character === '\\') escaped = true;
			else if (character === '"') inString = false;
			continue;
		}
		if (character === '/' && source[index + 1] === '/') {
			characters[index] = ' ';
			characters[index + 1] = ' ';
			index += 1;
			inComment = true;
		} else if (character === '"') {
			characters[index] = ' ';
			inString = true;
		}
	}
	return characters.join('');
}

function quickFixWords(diagnostic: RefactorDiagnostic): string[] {
	if (diagnostic.code === 'E_UNKNOWN_PROPERTY') {
		return wordsAfterPrefix(diagnostic.hint ?? null, 'Supported properties:');
	}
	if (diagnostic.code === 'E_PROPERTY_TYPE') {
		return wordsAfterPrefix(diagnostic.hint ?? null, 'Expected one of');
	}
	return [];
}

function wordsAfterPrefix(hint: string | null, prefix: string): string[] {
	if (!hint?.startsWith(prefix)) return [];
	return hint.slice(prefix.length)
		.replace(/[.]$/, '')
		.split(',')
		.map((word) => word.trim())
		.filter(isQuickFixWord);
}

function nearestWords(input: string, choices: readonly string[]): string[] {
	const candidates = [...new Set(choices)]
		.filter((choice) => choice !== input && isQuickFixWord(choice))
		.map((name) => ({ name, distance: editDistance(input.toLowerCase(), name.toLowerCase()) }))
		.filter((candidate) => candidate.distance <= 2)
		.sort((left, right) => left.distance - right.distance || compareText(left.name, right.name));
	const bestDistance = candidates[0]?.distance;
	if (bestDistance === undefined) return [];
	return [...new Set(candidates.filter((candidate) => candidate.distance === bestDistance)
		.map((candidate) => candidate.name))].slice(0, 3);
}

function isQuickFixWord(word: string): boolean {
	return /^[A-Za-z_][A-Za-z0-9_-]*$/.test(word);
}

function editDistance(left: string, right: string): number {
	const previous = Array.from({ length: right.length + 1 }, (_, index) => index);
	for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
		const current = [leftIndex];
		for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
			current[rightIndex] = Math.min(
				current[rightIndex - 1]! + 1,
				previous[rightIndex]! + 1,
				previous[rightIndex - 1]! + (left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1)
			);
		}
		previous.splice(0, previous.length, ...current);
	}
	return previous[right.length] ?? Math.max(left.length, right.length);
}

function compareText(left: string, right: string): number {
	return left < right ? -1 : left > right ? 1 : 0;
}

function isUtf16Boundary(source: string, offset: number): boolean {
	return !(offset > 0 && offset < source.length &&
		/^[\uDC00-\uDFFF]$/.test(source[offset]!) && /[\uD800-\uDBFF]/.test(source[offset - 1]!));
}

function editorLocationToSourceLocation(
	source: string,
	location: SourceSpan['start']
): SourceSpan['start'] | null {
	if (!Number.isInteger(location.offset) || location.offset < 0 || location.offset > source.length || !isUtf16Boundary(source, location.offset)) return null;
	const prefix = source.slice(0, location.offset);
	const lineStart = prefix.lastIndexOf('\n') + 1;
	const line = prefix.split('\n').length;
	if (line !== location.line) return null;
	return {
		offset: new TextEncoder().encode(prefix).length,
		line,
		column: Array.from(prefix.slice(lineStart)).length + 1
	};
}

function isSourceSpan(value: unknown): value is SourceSpan {
	if (!isRecord(value) || !isSourceLocation(value.start) || !isSourceLocation(value.end)) return false;
	return value.start.offset <= value.end.offset;
}

function isSourceLocation(value: unknown): value is SourceSpan['start'] {
	return isRecord(value) &&
		typeof value.offset === 'number' && Number.isInteger(value.offset) && value.offset >= 0 &&
		typeof value.line === 'number' && Number.isInteger(value.line) && value.line > 0 &&
		typeof value.column === 'number' && Number.isInteger(value.column) && value.column > 0;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function matchesSourcePosition(source: string, location: SourceSpan['start']): boolean {
	const prefix = source.slice(0, location.offset);
	const line = prefix.split('\n').length;
	const column = Array.from(prefix.slice(prefix.lastIndexOf('\n') + 1)).length + 1;
	return location.line === line && location.column === column;
}

function diagnosticKey(path: string, diagnostic: RefactorDiagnostic): string {
	return `${path}\u0000${diagnostic.code}\u0000${diagnostic.message}`;
}

function introducesNoErrors(before: ReadonlyMap<string, number>, after: ReadonlyMap<string, number>): boolean {
	for (const [key, count] of after) {
		if (count > (before.get(key) ?? 0)) return false;
	}
	return true;
}
