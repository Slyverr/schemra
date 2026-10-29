import { describe, expect, it } from 'bun:test';
import {
	diagnosticCounts,
	diagnosticQuickFixEdit,
	diagnosticQuickFixOptions,
	editorSymbolForRefactor,
	endpointQuickFixCandidates,
	isSafeQuickFix,
	projectIntroducesNoErrors,
	readRefactorDiagnostics,
	type RefactorDiagnostic,
	type RefactorFile,
	type RefactorSymbol
} from '../../src/lib/editor/editor-refactors.ts';

function span(source: string, start: number, end: number) {
	const encoder = new TextEncoder();
	const lineColumn = (offset: number) => {
		const prefix = source.slice(0, offset);
		const line = prefix.split('\n').length;
		const lineText = prefix.slice(prefix.lastIndexOf('\n') + 1);
		return { offset: encoder.encode(prefix).length, line, column: Array.from(lineText).length + 1 };
	};
	return { start: lineColumn(start), end: lineColumn(end) };
}

function diagnostic(
	source: string,
	code: string,
	message: string,
	start: number,
	end: number,
	hint: string | null = null,
	path = 'visual.schemra'
): RefactorDiagnostic {
	return { code, severity: 'error', message, hint, span: span(source, start, end), path };
}

function nodeSymbol(
	source: string,
	name: string,
	role: RefactorSymbol['role'],
	visual: string,
	start: number,
	path = 'visual.schemra'
): RefactorSymbol {
	return {
		name,
		kind: 'node',
		role,
		target_key: `node:${visual}:${name}`,
		span: span(source, start, start + name.length),
		path
	};
}

describe('diagnostic quick fixes', () => {
	it('validates source diagnostics and retains their source path', () => {
		const source = 'visual app {\n  node proxy {}\n  cliet -> proxy\n}';
		const start = source.indexOf('cliet ->');
		const raw = diagnostic(source, 'E_UNKNOWN_EDGE_NODE', 'Unknown node.', start, start + 'cliet -> proxy'.length);
		expect(readRefactorDiagnostics([raw, { code: 4 }], 'visual.schemra')).toEqual([{
			...raw,
			path: 'visual.schemra'
		}]);
	});

	it('converts editor UTF-16 symbol positions back to compiler UTF-8 coordinates', () => {
		const source = '// café 🙂\nvisual app {\n  node worker {}\n}';
		const start = source.indexOf('worker');
		const end = start + 'worker'.length;
		const editorPosition = (offset: number) => {
			const prefix = source.slice(0, offset);
			return { offset, line: prefix.split('\n').length, column: prefix.length - prefix.lastIndexOf('\n') };
		};
		const converted = editorSymbolForRefactor(source, {
			name: 'worker', kind: 'node', role: 'definition', target_key: 'node:app:worker',
			span: { start: editorPosition(start), end: editorPosition(end) }
		}, 'visual.schemra');

		expect(converted?.span).toEqual({
			start: {
				offset: new TextEncoder().encode(source.slice(0, start)).length,
				line: 3,
				column: 8
			},
			end: {
				offset: new TextEncoder().encode(source.slice(0, end)).length,
				line: 3,
				column: 14
			}
		});
	});

	it('suggests only the closest declared node typo for an edge endpoint', () => {
		const source = 'visual app {\n  node client {}\n  node proxy {}\n  cliet -> proxy\n}';
		const start = source.indexOf('cliet ->');
		const issue = diagnostic(
			source,
			'E_UNKNOWN_EDGE_NODE',
			'An edge references an undeclared node.',
			start,
			start + 'cliet -> proxy'.length
		);

		expect(endpointQuickFixCandidates(source, [], issue)).toEqual(['client']);
		expect(diagnosticQuickFixOptions(source, [], issue)).toEqual([{
			start: source.indexOf('cliet'),
			end: source.indexOf('cliet') + 'cliet'.length,
			replacement: 'client'
		}]);
	});

	it('does not guess when both edge endpoints are unresolved or a comment only mentions a node', () => {
		const source = '// node client {}\nvisual app {\n  node proxy {}\n  cliet -> proxy\n}';
		const start = source.indexOf('cliet ->');
		const issue = diagnostic(
			source,
			'E_UNKNOWN_EDGE_NODE',
			'Unknown edge endpoint.',
			start,
			start + 'cliet -> proxy'.length
		);

		expect(endpointQuickFixCandidates(source, [], issue)).toEqual([]);
	});

	it('suggests a typo for one unresolved scenario node within its visual scope', () => {
		const source = '// café 🙂\nscenario flow for visual app {\n  move packet from cliet -> proxy\n}';
		const moveStart = source.indexOf('move packet');
		const missingStart = source.indexOf('cliet ->');
		const targetStart = source.indexOf('proxy', missingStart);
		const symbols = [
			nodeSymbol(source, 'cliet', 'reference', 'app', missingStart, 'scenario.schemra'),
			nodeSymbol(source, 'proxy', 'reference', 'app', targetStart, 'scenario.schemra'),
			nodeSymbol(source, 'client', 'definition', 'app', 0),
			nodeSymbol(source, 'proxy', 'definition', 'app', 0)
		];
		const issue = diagnostic(
			source,
			'E_UNKNOWN_MOVE_NODE',
			'A move references an unknown node.',
			moveStart,
			source.indexOf('\n}', moveStart),
			null,
			'scenario.schemra'
		);

		expect(endpointQuickFixCandidates(source, symbols, issue)).toEqual(['client']);
		const edit = diagnosticQuickFixEdit(source, symbols, issue, 'client');
		expect(edit).toEqual({ start: missingStart, end: missingStart + 'cliet'.length, text: 'client' });
	});

	it('rejects multiple unresolved scenario references and references from a different file', () => {
		const source = 'scenario flow for visual app {\n  move packet from cliet -> servce\n}';
		const moveStart = source.indexOf('move packet');
		const from = source.indexOf('cliet ->');
		const to = source.indexOf('servce', from);
		const symbols = [
			nodeSymbol(source, 'cliet', 'reference', 'app', from, 'scenario.schemra'),
			nodeSymbol(source, 'servce', 'reference', 'app', to, 'scenario.schemra'),
			nodeSymbol(source, 'client', 'definition', 'app', 0),
			nodeSymbol(source, 'service', 'definition', 'app', 0),
			nodeSymbol(source, 'client', 'reference', 'app', from, 'other.schemra')
		];
		const issue = diagnostic(
			source,
			'E_UNKNOWN_MOVE_NODE',
			'A move references an unknown node.',
			moveStart,
			source.indexOf('\n}', moveStart),
			null,
			'scenario.schemra'
		);

		expect(endpointQuickFixCandidates(source, symbols, issue)).toEqual([]);
	});

	it('suggests property and enum names from compiler hints', () => {
		const source = 'visual app {\n  directon: left-to-right\n}';
		const propertyStart = source.indexOf('directon');
		const propertyIssue = diagnostic(
			source,
			'E_UNKNOWN_PROPERTY',
			'Property `directon` is not valid in this scope.',
			propertyStart,
			propertyStart + 'directon'.length,
			'Supported properties: direction.'
		);
		expect(
			diagnosticQuickFixOptions(source, [], propertyIssue).map((option) => option.replacement)
		).toEqual(['direction']);

		const enumValueStart = source.indexOf('left-to-right');
		const enumIssue = diagnostic(
			source,
			'E_PROPERTY_TYPE',
			'Property `direction` has a value of the wrong type.',
			enumValueStart,
			enumValueStart + 'left-to-right'.length,
			'Expected one of left-to-right, right-to-left, top-to-bottom, bottom-to-top.'
		);
		expect(diagnosticQuickFixOptions(source, [], enumIssue)).toEqual([]);

		const wrongValueSource = source.replace('left-to-right', 'left-to-rigth');
		const wrongValueStart = wrongValueSource.indexOf('left-to-rigth');
		const wrongValueIssue = diagnostic(
			wrongValueSource,
			'E_PROPERTY_TYPE',
			'Property `direction` has a value of the wrong type.',
			wrongValueStart,
			wrongValueStart + 'left-to-rigth'.length,
			'Expected one of left-to-right, right-to-left, top-to-bottom, bottom-to-top.'
		);
		expect(
			diagnosticQuickFixOptions(wrongValueSource, [], wrongValueIssue).map((option) => option.replacement)
		).toEqual(['left-to-right']);
	});

	it('rejects invalid, split-character, and mismatched source spans', () => {
		const source = '// 🙂\nvisual app {\n  node client {}\n  cliet -> client\n}';
		const edgeStart = source.indexOf('cliet ->');
		const invalid = diagnostic(source, 'E_UNKNOWN_EDGE_NODE', 'Unknown endpoint.', edgeStart, source.length);
		invalid.span.end.offset += 1;
		expect(diagnosticQuickFixOptions(source, [], invalid)).toEqual([]);

		const stalePosition = diagnostic(
			source,
			'E_UNKNOWN_EDGE_NODE',
			'Unknown endpoint.',
			edgeStart,
			edgeStart + 'cliet -> client'.length
		);
		stalePosition.span.start.line += 1;
		expect(diagnosticQuickFixOptions(source, [], stalePosition)).toEqual([]);

		const characterByteStart = new TextEncoder().encode(source.slice(0, 3)).length;
		const splitSpan = {
			start: { offset: characterByteStart + 1, line: 1, column: 5 },
			end: { offset: characterByteStart + 2, line: 1, column: 6 }
		};
		const splitIssue: RefactorDiagnostic = {
			code: 'E_UNKNOWN_PROPERTY', severity: 'error', message: 'bad property',
			hint: 'Supported properties: direction.', span: splitSpan
		};
		expect(diagnosticQuickFixOptions(source, [], splitIssue)).toEqual([]);

		const actionSource = 'scenario flow for visual app {\n  highlight cliet 1s trace\n}';
		const actionStart = actionSource.indexOf('highlight');
		const actionRef = actionSource.indexOf('cliet');
		const wrongSpan = nodeSymbol(actionSource, 'cliet', 'reference', 'app', actionRef + 1, 'scenario.schemra');
		const actionIssue = diagnostic(
			actionSource,
			'E_UNKNOWN_HIGHLIGHT_NODE',
			'Unknown target.',
			actionStart,
			actionSource.indexOf('\n}', actionStart),
			null,
			'scenario.schemra'
		);
		expect(endpointQuickFixCandidates(actionSource, [wrongSpan], actionIssue)).toEqual([]);
	});
});

describe('quick-fix safety checks', () => {
	const issue: RefactorDiagnostic = {
		code: 'E_UNKNOWN_EDGE_NODE', severity: 'error', message: 'Unknown edge endpoint.',
		span: span('visual app { a -> b }', 14, 20)
	};
	const before: RefactorFile[] = [{ path: 'visual.schemra', diagnostics: [issue] }];

	it('requires the selected error to disappear and prevents new errors', () => {
		const counts = diagnosticCounts(before);
		const clean: RefactorFile[] = [{ path: 'visual.schemra', diagnostics: [] }];
		expect(isSafeQuickFix('visual.schemra', issue, counts, clean)).toBe(true);
		expect(projectIntroducesNoErrors(before, clean)).toBe(true);

		const stillBroken: RefactorFile[] = [{ path: 'visual.schemra', diagnostics: [issue] }];
		expect(isSafeQuickFix('visual.schemra', issue, counts, stillBroken)).toBe(false);

		const newError: RefactorDiagnostic = {
			...issue,
			code: 'E_PROPERTY_TYPE',
			message: 'A new diagnostic appeared.'
		};
		const broken: RefactorFile[] = [{ path: 'visual.schemra', diagnostics: [newError] }];
		expect(isSafeQuickFix('visual.schemra', issue, counts, broken)).toBe(false);
		expect(projectIntroducesNoErrors(before, broken)).toBe(false);
	});

	it('ignores warnings and errors in unrelated paths when comparing counts', () => {
		const warning: RefactorDiagnostic = { ...issue, severity: 'warning' };
		const unrelated: RefactorDiagnostic = { ...issue, message: 'Existing unrelated problem.' };
		const beforeFiles: RefactorFile[] = [
			{ path: 'visual.schemra', diagnostics: [issue] },
			{ path: 'other.schemra', diagnostics: [unrelated, warning] }
		];
		const afterFiles: RefactorFile[] = [
			{ path: 'visual.schemra', diagnostics: [] },
			{ path: 'other.schemra', diagnostics: [unrelated, warning] }
		];
		expect(isSafeQuickFix('visual.schemra', issue, diagnosticCounts(beforeFiles), afterFiles)).toBe(true);
	});
});
