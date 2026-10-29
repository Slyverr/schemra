import { describe, expect, test } from 'bun:test';
import {
	buildEditorLines,
	editorOffsetAtPosition,
	getEditorColumnAtX,
	getEditorDiagnosticAtColumn,
	getEditorPositionAtPoint,
	mapEditorSelection,
	readEditorDiagnostics,
	readEditorSyntaxTokens,
	type EditorDiagnostic,
	type EditorSyntaxToken
} from '../src/lib/editor/source-model.js';

function token(kind: string, startColumn: number, endColumn: number): EditorSyntaxToken {
	return {
		kind,
		span: {
			start: { line: 1, column: startColumn },
			end: { line: 1, column: endColumn }
		}
	};
}

function diagnostic(startColumn: number, endColumn: number): EditorDiagnostic {
	return {
		code: 'E_TEST',
		severity: 'error',
		message: 'Invalid source',
		hint: null,
		span: {
			start: { line: 1, column: startColumn },
			end: { line: 1, column: endColumn }
		}
	};
}

describe('source editor presentation model', () => {
	test('maps editor line and Unicode columns to JavaScript offsets', () => {
		expect(editorOffsetAtPosition('😀x\nabc', { line: 1, column: 2 })).toBe(2);
		expect(editorOffsetAtPosition('😀x\nabc', { line: 2, column: 2 })).toBe(5);
	});

	test('preserves source text while applying compiler token and diagnostic styles', () => {
		const source = 'visual app';
		const lines = buildEditorLines(source, [token('keyword', 1, 7), token('identifier', 8, 11)], [diagnostic(8, 11)], new Set());

		expect(lines.map((line) => line.segments.map((segment) => segment.text).join('')).join('\n')).toBe(source);
		expect(lines[0]?.segments[0]?.className).toContain('syntax-keyword');
		expect(lines[0]?.segments.at(-1)?.className).toContain('editor-diagnostic-error');
		expect(lines[0]?.hasErrors).toBe(true);
	});

	test('matches UTF-16 bracket offsets after a supplementary Unicode character', () => {
		const source = '😀{}';
		const lines = buildEditorLines(source, [], [], new Set([2, 3]));
		expect(lines[0]?.segments.map((segment) => segment.text).join('')).toBe(source);
		expect(lines[0]?.segments.at(-1)?.className).toContain('editor-matching-bracket');
	});

	test('creates one visible line for an empty source and retains blank lines', () => {
		expect(buildEditorLines('', [], [], new Set())).toHaveLength(1);
		expect(buildEditorLines('one\n\nthree', [], [], new Set()).map((line) => line.text)).toEqual(['one', '', 'three']);
	});

	test('accepts valid compiler JSON and rejects malformed ranges', () => {
		expect(readEditorSyntaxTokens([
			{ kind: 'keyword', span: { start: { line: 1, column: 1 }, end: { line: 1, column: 4 } } },
			{ kind: 'invalid', span: { start: { line: 0, column: 1 }, end: { line: 1, column: 2 } } }
		])).toHaveLength(1);
		expect(readEditorDiagnostics([
			{ code: 'E_TEST', severity: 'error', message: 'Broken', hint: null, span: { start: { line: 1, column: 1 }, end: { line: 1, column: 2 } } },
			{ code: 'E_TEST', severity: 'fatal', message: 'Broken', hint: null, span: { start: { line: 1, column: 1 }, end: { line: 1, column: 2 } } }
		])).toHaveLength(1);
	});

	test('maps horizontal pointer positions through proportional glyphs and tab stops', () => {
		const width = (character: string) => character === 'W' ? 20 : 10;
		expect(getEditorColumnAtX('ab', 4, width, 20)).toBe(0);
		expect(getEditorColumnAtX('ab', 6, width, 20)).toBe(1);
		expect(getEditorColumnAtX('a\tb', 24, width, 20)).toBe(2);
		expect(getEditorColumnAtX('W😀', 25, width, 20)).toBe(1);
		expect(getEditorColumnAtX('abc', 100, width, 20)).toBe(3);
	});

	test('maps pointer coordinates to one-based source positions across rows', () => {
		const lines = buildEditorLines('ab\nWxy', [], [], new Set());
		const width = (character: string) => character === 'W' ? 20 : 10;
		expect(getEditorPositionAtPoint(lines, 6, 4, 24, width, 20)).toEqual({ line: 1, column: 2 });
		expect(getEditorPositionAtPoint(lines, 21, 25, 24, width, 20)).toEqual({ line: 2, column: 2 });
		expect(getEditorPositionAtPoint(lines, 0, -4, 24, width, 20)).toBeNull();
		expect(getEditorPositionAtPoint(lines, 0, 48, 24, width, 20)).toBeNull();
	});

	test('maps the pointer column to the covered diagnostic and respects exclusive ends', () => {
		const issue = diagnostic(2, 4);
		const line = buildEditorLines('abcd', [], [issue], new Set())[0]!;
		expect(getEditorDiagnosticAtColumn(line, 0)).toBeNull();
		expect(getEditorDiagnosticAtColumn(line, 1)).toBe(issue);
		expect(getEditorDiagnosticAtColumn(line, 2)).toBe(issue);
		expect(getEditorDiagnosticAtColumn(line, 3)).toBeNull();
		expect(getEditorDiagnosticAtColumn(line, 4)).toBeNull();
	});

	test('keeps the selection near its content when formatting changes indentation', () => {
		const before = 'visual app {\nnode api {}\n}';
		const after = 'visual app {\n  node api {}\n}';
		const offset = before.indexOf('api') + 1;
		expect(mapEditorSelection(before, after, { start: offset, end: offset, direction: 'none' })).toEqual({
			start: offset + 2,
			end: offset + 2,
			direction: 'none'
		});
	});
});
