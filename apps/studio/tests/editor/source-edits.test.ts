import { describe, expect, it } from 'bun:test';
import {
	applySourceTextEdits,
	removeSourceDeclaration,
	sourceOffsetAtPosition,
	utf8SpanToEditorSpan
} from '../../src/lib/editor/source-edits.ts';

function editorSpan(start: number, end: number) {
	return {
		start: { offset: start, line: 1, column: start + 1 },
		end: { offset: end, line: 1, column: end + 1 }
	};
}

function utf8Span(source: string, start: number, end: number) {
	const encoder = new TextEncoder();
	return {
		start: { offset: encoder.encode(source.slice(0, start)).byteLength, line: 1, column: start + 1 },
		end: { offset: encoder.encode(source.slice(0, end)).byteLength, line: 1, column: end + 1 }
	};
}

describe('source edits with compiler spans', () => {
	it('applies disjoint source edits from right to left', () => {
		const source = 'alpha -> beta';
		expect(applySourceTextEdits(source, [
			{ span: editorSpan(0, 5), replacement: 'client' },
			{ span: editorSpan(9, 13), replacement: 'proxy' }
		])).toBe('client -> proxy');
	});

	it('rejects overlapping source edits instead of partially applying them', () => {
		const source = 'alpha -> beta';
		expect(() => applySourceTextEdits(source, [
			{ span: editorSpan(0, 7), replacement: 'x' },
			{ span: editorSpan(5, 13), replacement: 'y' }
		])).toThrow('Source text edits overlap.');
	});

	it('converts UTF-8 byte spans to editor UTF-16 offsets', () => {
		const source = '🙂 café';
		const span = utf8Span(source, 3, source.length);
		expect(utf8SpanToEditorSpan(source, span).start.offset).toBe(3);
		expect(utf8SpanToEditorSpan(source, span).end.offset).toBe(source.length);
	});

	it('removes the requested declaration after multibyte text without corrupting adjacent source', () => {
		const source = '// café 🙂\nvisual one {}\nvisual two {}\n';
		const start = source.indexOf('visual two');
		const end = source.indexOf('\n', start);
		const span = utf8Span(source, start, end);
		expect(removeSourceDeclaration(source, span)).toBe('// café 🙂\nvisual one {}\n');
	});

	it('maps one-based Unicode-scalar columns to UTF-16 editor offsets', () => {
		expect(sourceOffsetAtPosition('🙂x\ny', 1, 2)).toBe(2);
		expect(sourceOffsetAtPosition('🙂x\ny', 2, 1)).toBe(4);
	});
});
