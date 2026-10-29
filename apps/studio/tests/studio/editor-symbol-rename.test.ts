import { describe, expect, it } from 'bun:test';
import type { ProjectSource, RenameEdit, RenameResult, RenameSelection } from '../../src/lib/schemra.js';
import { studioMessage } from '../../src/lib/i18n/messages.js';
import { createValidatedRenameUpdates } from '../../src/lib/studio/editor-symbol-rename.js';

function span(source: string, start: number, length: number) {
	return {
		start: { offset: start, line: 1, column: start + 1 },
		end: { offset: start + length, line: 1, column: start + length + 1 }
	};
}

function edit(path: string, source: string, name: string, replacement: string): RenameEdit {
	const start = source.indexOf(name);
	return { path, span: span(source, start, name.length), replacement };
}

function renamePlan(selection: RenameSelection, edits: RenameEdit[]): RenameResult {
	return { error: null, selection, edits };
}

describe('validated editor symbol rename plans', () => {
	it('applies matching definition and reference edits across linked files', () => {
		const visual = 'visual network { node client {} }';
		const scenario = 'scenario traffic { move client -> server }';
		const sources: ProjectSource[] = [
			{ id: 'visual', path: 'visuals/network.schemra', source: visual },
			{ id: 'scenario', path: 'scenarios/traffic.schemra', source: scenario }
		];
		const selection: RenameSelection = { name: 'client', kind: 'node', span: span(visual, visual.indexOf('client'), 6) };
		const plan = renamePlan(selection, [
			edit('visuals/network.schemra', visual, 'client', 'consumer'),
			edit('scenarios/traffic.schemra', scenario, 'client', 'consumer')
		]);

		const updates = createValidatedRenameUpdates(sources, 'client', 'consumer', selection, plan);

		expect(updates.get('visual')?.source).toBe('visual network { node consumer {} }');
		expect(updates.get('scenario')?.source).toBe('scenario traffic { move consumer -> server }');
	});

	it('rejects edits that do not replace the selected symbol at its source span', () => {
		const source = 'visual network { node client {} }';
		const sources: ProjectSource[] = [{ id: 'visual', path: 'visuals/network.schemra', source }];
		const selection: RenameSelection = { name: 'client', kind: 'node', span: span(source, source.indexOf('client'), 6) };
		const plan = renamePlan(selection, [edit('visuals/network.schemra', source, 'client', 'server')]);

		expect(() => createValidatedRenameUpdates(sources, 'client', 'consumer', selection, plan))
			.toThrow(studioMessage('editor.renamePlanMismatch'));
	});

	it('rejects edits for files outside the project snapshot', () => {
		const source = 'visual network { node client {} }';
		const sources: ProjectSource[] = [{ id: 'visual', path: 'visuals/network.schemra', source }];
		const selection: RenameSelection = { name: 'client', kind: 'node', span: span(source, source.indexOf('client'), 6) };
		const plan = renamePlan(selection, [edit('scenarios/missing.schemra', 'client', 'client', 'consumer')]);

		expect(() => createValidatedRenameUpdates(sources, 'client', 'consumer', selection, plan))
			.toThrow(studioMessage('editor.renamePlanInvalid'));
	});
});
