import type { SourceSpan } from '../schemra.js';

export interface SourceTextEdit {
	span: SourceSpan;
	replacement: string;
}

/** Apply editor-coordinate edits from right to left so earlier ranges stay stable. */
export function applySourceTextEdits(source: string, edits: readonly SourceTextEdit[]): string {
	const ranges = edits
		.map(({ span, replacement }) => ({ ...toStringRange(source, span), replacement }))
		.sort((left, right) => right.start - left.start);

	let output = source;
	let previousStart = source.length;
	for (const range of ranges) {
		if (range.end > previousStart) throw new RangeError('Source text edits overlap.');
		output = output.slice(0, range.start) + range.replacement + output.slice(range.end);
		previousStart = range.start;
	}
	return output;
}

/** Remove one declaration and its line ending, if present. */
export function removeSourceDeclaration(source: string, span: SourceSpan): string {
	const range = toStringRange(source, utf8SpanToEditorSpan(source, span));
	let end = range.end;
	if (source.startsWith('\r\n', end)) end += 2;
	else if (source[end] === '\n') end += 1;
	return source.slice(0, range.start) + source.slice(end);
}

/** Convert compiler byte offsets to the UTF-16 offsets used by the editor. */
export function utf8SpanToEditorSpan(source: string, span: SourceSpan): SourceSpan {
	const start = utf8OffsetToUtf16(source, span.start.offset);
	const end = utf8OffsetToUtf16(source, span.end.offset);
	return {
		start: { ...span.start, offset: start },
		end: { ...span.end, offset: end }
	};
}

/** Return a UTF-16 source offset for a one-based line and Unicode-scalar column. */
export function sourceOffsetAtPosition(source: string, line: number, column: number) {
	if (!Number.isInteger(line) || line < 1 || !Number.isInteger(column) || column < 1) {
		throw new RangeError('Source position must use positive one-based coordinates.');
	}
	let lineStart = 0;
	for (let currentLine = 1; currentLine < line; currentLine += 1) {
		const lineBreak = source.indexOf('\n', lineStart);
		if (lineBreak < 0) throw new RangeError('Source line is outside the current text.');
		lineStart = lineBreak + 1;
	}
	const lineBreak = source.indexOf('\n', lineStart);
	const lineEnd = lineBreak < 0 ? source.length : lineBreak;
	const scalarOffset = Array.from(source.slice(lineStart, lineEnd)).slice(0, column - 1).join('').length;
	return Math.min(lineStart + scalarOffset, lineEnd);
}

function utf8OffsetToUtf16(source: string, byteOffset: number) {
	if (!Number.isInteger(byteOffset) || byteOffset < 0) throw new RangeError('Source span is outside the current text.');
	let bytes = 0;
	let utf16 = 0;
	const encoder = new TextEncoder();
	for (const character of source) {
		if (bytes >= byteOffset) break;
		const length = encoder.encode(character).length;
		if (bytes + length > byteOffset) throw new RangeError('Source span splits a UTF-8 character.');
		bytes += length;
		utf16 += character.length;
	}
	if (bytes !== byteOffset) throw new RangeError('Source span is outside the current text.');
	return utf16;
}

function toStringRange(source: string, span: SourceSpan) {
	const start = span.start.offset;
	const end = span.end.offset;
	if (!Number.isInteger(start) || !Number.isInteger(end) || start < 0 || end > source.length) {
		throw new RangeError('Source span is outside the current text.');
	}
	if (start > end) throw new RangeError('Source span is reversed.');
	return { start, end };
}
