export interface EditorPosition {
	line: number;
	column: number;
}

export interface EditorSpan {
	start: EditorPosition;
	end: EditorPosition;
}

export interface EditorSyntaxToken {
	kind: string;
	span: EditorSpan;
}

export interface EditorDiagnostic {
	code: string;
	severity: 'error' | 'warning' | 'information';
	message: string;
	hint: string | null;
	span: EditorSpan;
}

export interface EditorTextSegment {
	text: string;
	className: string;
	diagnostic: EditorDiagnostic | null;
}

export interface EditorLine {
	index: number;
	startOffset: number;
	text: string;
	segments: EditorTextSegment[];
	diagnostics: EditorDiagnostic[];
	hasErrors: boolean;
}

/** Map coordinates within the source text area to a one-based source position. */
export function getEditorPositionAtPoint(
	lines: readonly EditorLine[],
	x: number,
	y: number,
	lineHeight: number,
	measureCharacter: (character: string) => number,
	tabWidth: number
): EditorPosition | null {
	if (lines.length === 0 || !Number.isFinite(y) || !Number.isFinite(lineHeight) || lineHeight <= 0) return null;
	if (y < 0 || y >= lines.length * lineHeight) return null;
	const lineIndex = Math.floor(y / lineHeight);
	const line = lines[lineIndex];
	if (!line) return null;
	return {
		line: lineIndex + 1,
		column: getEditorColumnAtX(line.text, x, measureCharacter, tabWidth) + 1
	};
}

/** Map a mouse X coordinate within a rendered source line to a zero-based codepoint column. */
export function getEditorColumnAtX(
	text: string,
	x: number,
	measureCharacter: (character: string) => number,
	tabWidth: number
): number {
	const characters = Array.from(text);
	let width = 0;
	const safeTabWidth = Math.max(1, tabWidth);
	for (const [column, character] of characters.entries()) {
		const tabRemainder = width % safeTabWidth;
		const characterWidth = character === '\t'
			? safeTabWidth - tabRemainder
			: Math.max(0, measureCharacter(character));
		if (x <= width + characterWidth / 2) return column;
		width += characterWidth;
	}
	return characters.length;
}

/** Find the diagnostic occupying a zero-based source column in a rendered line. */
export function getEditorDiagnosticAtColumn(line: EditorLine, column: number): EditorDiagnostic | null {
	const lineNumber = line.index + 1;
	const lineLength = Array.from(line.text).length;
	return line.diagnostics.find((item) => {
		const start = item.span.start.line === lineNumber ? item.span.start.column - 1 : 0;
		const end = item.span.end.line === lineNumber ? item.span.end.column - 1 : lineLength;
		if (start === end) {
			const markerColumn = start < lineLength ? start : Math.max(0, lineLength - 1);
			return lineLength > 0 && column === markerColumn;
		}
		return column >= start && column < end;
	}) ?? null;
}

/** Map a selection through a text replacement, keeping it near the same source content. */
export function mapEditorSelection(
	before: string,
	after: string,
	selection: { start: number; end: number; direction: 'forward' | 'backward' | 'none' }
) {
	let prefix = 0;
	while (prefix < before.length && prefix < after.length && before[prefix] === after[prefix]) prefix += 1;
	let suffix = 0;
	while (
		suffix < before.length - prefix &&
		suffix < after.length - prefix &&
		before[before.length - suffix - 1] === after[after.length - suffix - 1]
	) suffix += 1;
	const mapOffset = (offset: number) => {
		const clamped = Math.min(before.length, Math.max(0, offset));
		if (clamped <= prefix) return clamped;
		if (clamped >= before.length - suffix) return after.length - (before.length - clamped);
		const newChangedLength = after.length - prefix - suffix;
		return prefix + Math.min(clamped - prefix, newChangedLength);
	};
	return {
		start: mapOffset(selection.start),
		end: mapOffset(selection.end),
		direction: selection.direction
	};
}

const syntaxClasses: Readonly<Record<string, string>> = {
	keyword: 'syntax-keyword',
	property: 'syntax-property',
	value: 'syntax-value',
	operator: 'syntax-operator',
	punctuation: 'syntax-punctuation',
	number: 'syntax-number',
	string: 'syntax-string',
	comment: 'syntax-comment',
	identifier: 'syntax-identifier'
};

/** Validate the compiler's untyped JSON token list before using its source positions. */
export function readEditorSyntaxTokens(value: unknown): EditorSyntaxToken[] {
	if (!Array.isArray(value)) return [];
	return value.flatMap((candidate): EditorSyntaxToken[] => {
		if (!isRecord(candidate) || typeof candidate.kind !== 'string' || !isEditorSpan(candidate.span)) return [];
		return [{ kind: candidate.kind, span: candidate.span }];
	});
}

/** Validate the compiler's untyped JSON diagnostics before rendering them. */
export function readEditorDiagnostics(value: unknown): EditorDiagnostic[] {
	if (!Array.isArray(value)) return [];
	return value.flatMap((candidate): EditorDiagnostic[] => {
		if (
			!isRecord(candidate) ||
			typeof candidate.code !== 'string' ||
			typeof candidate.message !== 'string' ||
			!['error', 'warning', 'information'].includes(String(candidate.severity)) ||
			!isEditorSpan(candidate.span)
		) return [];
		return [{
			code: candidate.code,
			severity: candidate.severity as EditorDiagnostic['severity'],
			message: candidate.message,
			hint: typeof candidate.hint === 'string' ? candidate.hint : null,
			span: candidate.span
		}];
	});
}

/** Split compiler tokens and diagnostics into source-preserving highlighted lines. */
export function buildEditorLines(
	source: string,
	tokens: readonly EditorSyntaxToken[],
	diagnostics: readonly EditorDiagnostic[],
	matchingOffsets: ReadonlySet<number>
): EditorLine[] {
	let startOffset = 0;
	return source.split('\n').map((text, index) => {
		const lineNumber = index + 1;
		const characters = Array.from(text);
		const lineTokens = tokens.filter((token) => token.span.start.line <= lineNumber && token.span.end.line >= lineNumber);
		const lineDiagnostics = diagnostics.filter((item) => item.span.start.line <= lineNumber && item.span.end.line >= lineNumber);
		const diagnosticsByCharacter = characters.map((_, column) =>
			lineDiagnostics.find((item) => diagnosticCoversColumn(item, lineNumber, column, characters.length)) ?? null
		);
		const tokenKinds = characters.map((_, column) =>
			lineTokens.find((token) => tokenCoversColumn(token, lineNumber, column, characters.length))?.kind ?? 'identifier'
		);

		const segments: EditorTextSegment[] = [];
		let characterOffset = 0;
		for (let column = 0; column < characters.length; column += 1) {
			const character = characters[column] ?? '';
			const diagnostic = diagnosticsByCharacter[column] ?? null;
			const matching = matchingOffsets.has(startOffset + characterOffset);
			const classNames = [syntaxClasses[tokenKinds[column] ?? ''] ?? 'syntax-identifier'];
			if (matching) classNames.push('editor-matching-bracket');
			if (diagnostic) classNames.push(`editor-diagnostic-${diagnostic.severity}`);
			const className = classNames.join(' ');
			const previous = segments.at(-1);
			if (previous?.className === className && previous.diagnostic === diagnostic) {
				previous.text += character;
			} else {
				segments.push({ text: character, className, diagnostic });
			}
			characterOffset += character.length;
		}
		if (segments.length === 0) segments.push({ text: ' ', className: 'editor-empty-line', diagnostic: null });

		const line: EditorLine = {
			index,
			startOffset,
			text,
			segments,
			diagnostics: lineDiagnostics,
			hasErrors: lineDiagnostics.some((item) => item.severity === 'error')
		};
		startOffset += text.length + 1;
		return line;
	});
}

function tokenCoversColumn(token: EditorSyntaxToken, line: number, column: number, lineLength: number): boolean {
	const start = token.span.start.line === line ? token.span.start.column - 1 : 0;
	const end = token.span.end.line === line ? token.span.end.column - 1 : lineLength;
	return column >= start && column < end;
}

function diagnosticCoversColumn(item: EditorDiagnostic, line: number, column: number, lineLength: number): boolean {
	const start = item.span.start.line === line ? item.span.start.column - 1 : 0;
	const end = item.span.end.line === line ? item.span.end.column - 1 : lineLength;
	if (start === end) {
		const markerColumn = start < lineLength ? start : Math.max(0, lineLength - 1);
		return lineLength > 0 && column === markerColumn;
	}
	return column >= start && column < end;
}

function isEditorSpan(value: unknown): value is EditorSpan {
	if (!isRecord(value) || !isRecord(value.start) || !isRecord(value.end)) return false;
	return [value.start, value.end].every((position) => {
		if (typeof position.line !== 'number' || typeof position.column !== 'number') return false;
		return Number.isInteger(position.line) && position.line > 0 &&
			Number.isInteger(position.column) && position.column > 0;
	});
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null;
}
