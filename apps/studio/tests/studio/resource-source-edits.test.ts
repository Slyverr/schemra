import { describe, expect, test } from 'bun:test';
import type { SourceSpan } from '../../src/lib/schemra.js';
import { findDeclarationIdentifier, renameVisualTitle, type SourceToken } from '../../src/lib/studio/resource-source-edits.js';

function span(source: string, start: number, end: number): SourceSpan {
	const location = (offset: number) => {
		const before = source.slice(0, offset);
		return {
			offset: new TextEncoder().encode(before).length,
			line: before.split('\n').length,
			column: before.length - before.lastIndexOf('\n')
		};
	};
	return { start: location(start), end: location(end) };
}

describe('resource declaration source edits', () => {
	test('renames a visual title without changing surrounding source', () => {
		const source = 'visual "Old" { node api "API" {} }';
		const titleStart = source.indexOf('"Old"');
		const tokens: SourceToken[] = [{ kind: 'string', span: span(source, titleStart, titleStart + '"Old"'.length) }];

		expect(renameVisualTitle(source, tokens, span(source, 0, source.length), null, 'New "gateway"'))
			.toBe('visual "New \\"gateway\\"" { node api "API" {} }');
	});

	test('inserts a display title after an identifier-only visual declaration', () => {
		const source = 'visual gateway {}';
		const idStart = source.indexOf('gateway');
		const declarationSpan = span(source, 0, source.length);
		const tokens: SourceToken[] = [{ kind: 'identifier', span: span(source, idStart, idStart + 'gateway'.length) }];

		expect(renameVisualTitle(source, tokens, declarationSpan, 'gateway', 'Main gateway'))
			.toBe('visual gateway "Main gateway" {}');
	});

	test('does not match an identifier from a different declaration line', () => {
		const source = 'visual main {}\nnode main {}';
		const firstLine = span(source, 0, source.indexOf('\n'));
		const secondId = source.lastIndexOf('main');
		const tokens: SourceToken[] = [{ kind: 'identifier', span: span(source, secondId, secondId + 4) }];

		expect(findDeclarationIdentifier(source, tokens, firstLine, 'main')).toBeNull();
		expect(renameVisualTitle(source, tokens, firstLine, 'main', 'Main')).toBeNull();
	});
});
