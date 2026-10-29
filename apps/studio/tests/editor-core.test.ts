import { describe, expect, test } from 'bun:test';
import type { SourceSpan } from '../src/lib/schemra.js';
import { createBracketMatcher } from '../src/lib/editor/bracket-matching.js';
import { getDelimiterEdit } from '../src/lib/editor/delimiters.js';
import { getIndentationEdit } from '../src/lib/editor/indentation.js';
import { getActiveScopeGuideId, getScopeRanges } from '../src/lib/editor/scope-guides.js';
import { applySourceTextEdits, removeSourceDeclaration } from '../src/lib/editor/source-edits.js';

function sourceSpan(source: string, start: number, end: number): SourceSpan {
	const location = (offset: number) => {
		const before = source.slice(0, offset);
		const line = before.split('\n').length;
		return { offset, line, column: offset - before.lastIndexOf('\n') };
	};
	return { start: location(start), end: location(end) };
}

describe('editor indentation edits', () => {
	test('indents at a caret and moves the caret after inserted spaces', () => {
		expect(getIndentationEdit('visual', 3, 3, false)).toEqual({
			rangeStart: 3,
			rangeEnd: 3,
			text: '  ',
			selectionStart: 5,
			selectionEnd: 5
		});
	});

	test('indents each selected line and keeps the block selected', () => {
		const source = 'first\nsecond';
		expect(getIndentationEdit(source, 0, source.length, false)).toEqual({
			rangeStart: 0,
			rangeEnd: source.length,
			text: '  first\n  second',
			selectionStart: 2,
			selectionEnd: source.length + 4
		});
	});

	test('outdents spaces and tabs while preserving the selected block', () => {
		const source = '  first\n\tsecond\nthird';
		expect(getIndentationEdit(source, 0, source.length, true)).toEqual({
			rangeStart: 0,
			rangeEnd: source.length,
			text: 'first\nsecond\nthird',
			selectionStart: 0,
			selectionEnd: source.length - 3
		});
	});

	test('outdents the current line at the caret and maps the cursor position', () => {
		expect(getIndentationEdit('  first\nsecond', 4, 4, true)).toEqual({
			rangeStart: 0,
			rangeEnd: 7,
			text: 'first',
			selectionStart: 2,
			selectionEnd: 2
		});
	});

	test('does nothing when the line has no removable indentation', () => {
		expect(getIndentationEdit('first\nsecond', 2, 2, true)).toBeNull();
	});

	test('supports four-space insertion and outdent', () => {
		expect(getIndentationEdit('first', 0, 0, false, 4)).toMatchObject({ text: '    ', selectionStart: 4 });
		expect(getIndentationEdit('    first', 4, 4, true, 4)).toMatchObject({ text: 'first' });
	});

	test('excludes a following line when selection ends at its start', () => {
		const source = 'first\nsecond';
		expect(getIndentationEdit(source, 0, 6, false)).toEqual({
			rangeStart: 0,
			rangeEnd: 5,
			text: '  first',
			selectionStart: 2,
			selectionEnd: 8
		});
	});
});

describe('editor delimiter edits', () => {
	test.each([
		['(', '()'],
		['[', '[]'],
		['{', '{}'],
		['"', '""']
	])('inserts a closing pair for %s', (opening, text) => {
		expect(getDelimiterEdit('name', 4, 4, opening)).toEqual({
			kind: 'replace-text',
			rangeStart: 4,
			rangeEnd: 4,
			text,
			selectionStart: 5,
			selectionEnd: 5,
			refreshCompletions: true
		});
	});

	test('wraps selected text and preserves the inner selection', () => {
		expect(getDelimiterEdit('node', 0, 4, '{')).toEqual({
			kind: 'replace-text',
			rangeStart: 0,
			rangeEnd: 4,
			text: '{node}',
			selectionStart: 1,
			selectionEnd: 5,
			refreshCompletions: true
		});
	});

	test('skips an existing closer and removes an empty pair with Backspace', () => {
		expect(getDelimiterEdit('()', 1, 1, ')')).toEqual({ kind: 'move-cursor', cursor: 2 });
		expect(getDelimiterEdit('()', 1, 1, 'Backspace')).toEqual({
			kind: 'replace-text',
			rangeStart: 0,
			rangeEnd: 2,
			text: '',
			selectionStart: 0,
			selectionEnd: 0,
			refreshCompletions: false
		});
	});

	test('skips the auto-closed brace after an opened multiline block', () => {
		expect(getDelimiterEdit('{\n  \n}', 4, 4, '}')).toEqual({ kind: 'move-cursor', cursor: 6 });
		expect(getDelimiterEdit('{\n  node\n}', 4, 4, '}')).toBeNull();
	});

	test('does not pair delimiters inside strings or line comments', () => {
		expect(getDelimiterEdit('"value"', 2, 2, '{')).toBeNull();
		expect(getDelimiterEdit('// value', 3, 3, '(')).toBeNull();
		expect(getDelimiterEdit('// value', 3, 3, '"')).toBeNull();
	});

	test('skips the terminator when a quote is typed at the string end', () => {
		expect(getDelimiterEdit('"value"', 6, 6, '"')).toEqual({ kind: 'move-cursor', cursor: 7 });
	});
});

describe('editor bracket matching', () => {
	test('matches nested pairs at either side of the caret', () => {
		const match = createBracketMatcher();
		const source = 'visual map "Map" { node api "API" { route(api, [0]) } }';
		const opening = source.indexOf('{', source.indexOf('node'));
		const closing = source.indexOf('}', opening);
		expect(match(source, opening + 1, opening + 1)).toEqual(new Set([opening, closing]));
		expect(match(source, closing, closing)).toEqual(new Set([closing, opening]));
	});

	test('ignores brackets in strings, escaped quotes, and comments', () => {
		const match = createBracketMatcher();
		const source = 'visual map "Map } and \\\" quote" { // }\n  node api "{" {} }';
		const opening = source.indexOf('{');
		const closing = source.lastIndexOf('}');
		expect(match(source, opening + 1, opening + 1)).toEqual(new Set([opening, closing]));
	});

	test('returns no pair for selections, unmatched delimiters, or crossed pairs', () => {
		const match = createBracketMatcher();
		expect(match('({}', 1, 2)).toEqual(new Set());
		expect(match('([)]', 0, 0)).toEqual(new Set());
		expect(match('([)]', 2, 2)).toEqual(new Set());
		expect(match('([)]{}', 5, 5)).toEqual(new Set([4, 5]));
	});

	test('rebuilds its index when the source changes', () => {
		const match = createBracketMatcher();
		match('{}', 1, 1);
		expect(match('[]', 0, 0)).toEqual(new Set([0, 1]));
	});
});

describe('brace scope guides', () => {
	test('anchors guides at the first code character and carries them through the closing brace', () => {
		const source = [
			'visual map "Map" {',
			'  node api "API" {',
			'    icon: "lucide/server"',
			'  }',
			'}'
		].join('\n');
		expect(getScopeRanges(source).map(({ anchorOffset, openLine, endLine }) => ({ anchorOffset, openLine, endLine }))).toEqual([
			{ anchorOffset: 0, openLine: 0, endLine: 4 },
			{ anchorOffset: source.indexOf('node'), openLine: 1, endLine: 3 }
		]);
	});

	test('ignores braces in strings and comments', () => {
		const source = [
			'visual map "Map { not a block }" {',
			'  // { ignored',
			'  node api "API" {}',
			'}'
		].join('\n');
		expect(getScopeRanges(source).map(({ anchorOffset, openLine, endLine }) => ({ anchorOffset, openLine, endLine }))).toEqual([
			{ anchorOffset: 0, openLine: 0, endLine: 3 }
		]);
	});

	test('keeps UTF-16 offsets when supplementary characters occur earlier in the source', () => {
		const source = '// 🧭\nvisual map "Map" {\n  node api "API" {}\n}';
		expect(getScopeRanges(source)[0]?.anchorOffset).toBe(source.indexOf('visual'));
	});

	test('selects the deepest scope under the cursor, including its closing brace', () => {
		const source = [
			'visual map "Map" {',
			'  node api "API" {',
			'    icon: "lucide/server"',
			'  }',
			'}'
		].join('\n');
		const scopes = getScopeRanges(source);
		const outer = scopes.find((scope) => scope.depth === 0)!;
		const inner = scopes.find((scope) => scope.depth === 1)!;
		const innerClose = source.indexOf('\n  }') + 3;
		expect(getActiveScopeGuideId(scopes, source.indexOf('visual'))).toBe(outer.id);
		expect(getActiveScopeGuideId(scopes, source.indexOf('icon'))).toBe(inner.id);
		expect(getActiveScopeGuideId(scopes, innerClose)).toBe(inner.id);
		expect(getActiveScopeGuideId(scopes, innerClose + 1)).toBe(outer.id);
		expect(getActiveScopeGuideId(scopes, source.length)).toBeNull();
	});

	test('extends unclosed scopes to EOF and omits empty scopes', () => {
		expect(getScopeRanges('visual map "Map" {\n  node api "API" {\n    icon: "lucide/server"')).toMatchObject([
			{ openLine: 0, endLine: 2, closed: false },
			{ openLine: 1, endLine: 2, closed: false }
		]);
		expect(getScopeRanges('visual map "Map" {}')).toEqual([]);
	});
});

describe('source text edits', () => {
	test('applies multiple non-overlapping edits from right to left', () => {
		const source = 'visual a\nvisual b';
		expect(applySourceTextEdits(source, [
			{ span: sourceSpan(source, 7, 8), replacement: 'x' },
			{ span: sourceSpan(source, 16, 17), replacement: 'y' }
		])).toBe('visual x\nvisual y');
	});

	test('rejects overlapping and reversed spans', () => {
		const source = 'abcdef';
		expect(() => applySourceTextEdits(source, [
			{ span: sourceSpan(source, 1, 4), replacement: 'x' },
			{ span: sourceSpan(source, 3, 5), replacement: 'y' }
		])).toThrow(RangeError);
		expect(() => applySourceTextEdits(source, [{ span: sourceSpan(source, 5, 2), replacement: 'x' }])).toThrow(RangeError);
	});

	test('removes a declaration and its LF or CRLF line ending', () => {
		const lf = 'first\nremove\nlast';
		const crlf = 'first\r\nremove\r\nlast';
		expect(removeSourceDeclaration(lf, sourceSpan(lf, 6, 12))).toBe('first\nlast');
		expect(removeSourceDeclaration(crlf, sourceSpan(crlf, 7, 13))).toBe('first\r\nlast');
	});
});
