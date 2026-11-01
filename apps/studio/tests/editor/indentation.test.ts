import { describe, expect, it } from 'bun:test';
import { getIndentedNewlineEdit } from '../../src/lib/editor/indentation.js';

describe('editor newline indentation', () => {
	it('places the caret inside an empty block pair', () => {
		const source = 'group {}';
		const edit = getIndentedNewlineEdit(source, 7, 7);

		expect(source.slice(0, edit.rangeStart) + edit.text + source.slice(edit.rangeEnd)).toBe('group {\n  \n}');
		expect(edit.selectionStart).toBe(10);
		expect(edit.selectionEnd).toBe(10);
	});

	it('carries the current indentation onto a new line', () => {
		const source = '    move token';
		const edit = getIndentedNewlineEdit(source, source.length, source.length, 4);

		expect(edit.text).toBe('\n    ');
		expect(edit.selectionStart).toBe(source.length + edit.text.length);
	});

	it('dedents when inserting a line before a closing brace', () => {
		const source = '  }';
		const edit = getIndentedNewlineEdit(source, 2, 2);

		expect(source.slice(0, edit.rangeStart) + edit.text + source.slice(edit.rangeEnd)).toBe('  \n}');
		expect(edit.selectionStart).toBe(3);
	});

	it('replaces a selected range when inserting a new line', () => {
		const source = '  token';
		const edit = getIndentedNewlineEdit(source, 2, source.length);

		expect(source.slice(0, edit.rangeStart) + edit.text + source.slice(edit.rangeEnd)).toBe('  \n  ');
	});
});
