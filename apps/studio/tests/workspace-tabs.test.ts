import { describe, expect, test } from 'bun:test';
import { buildWorkspaceTabViews } from '../src/lib/studio/workspace-tabs.js';

describe('workspace tab view model', () => {
	test('uses the active declaration, compilation status, and source state', () => {
		const renamed: string[] = [];
		const removed: string[] = [];
		const views = buildWorkspaceTabViews({
			files: [
				{ id: 'visual', path: 'visuals/gateway.schemra', source: 'visual', savedSource: 'visual' },
				{ id: 'scenario', path: 'scenarios/traffic.schemra', source: 'edited', savedSource: 'saved' },
				{ id: 'component', path: 'components/service.schemra', source: 'component', savedSource: 'component' }
			],
			compilation: [
				{
					id: 'visual', kind: 'visual', diagnostics: [{ severity: 'error' }],
					declarations: [{ kind: 'visual', id: 'gateway', title: 'Gateway' }]
				},
				{
					id: 'scenario', kind: 'scenario', diagnostics: [],
					declarations: [
						{ kind: 'scenario', id: 'first', title: 'First' },
						{ kind: 'scenario', id: 'active', title: 'Active scenario' }
					]
				},
				{ id: 'component', kind: 'component', diagnostics: [], declarations: [] }
			],
			tabIds: ['visual', 'scenario', 'component', 'missing'],
			activeFileId: 'scenario',
			currentDeclaration: { kind: 'scenario', id: 'active', title: 'Active scenario' },
			projectFileCount: 3,
			initialized: true,
			compiling: false,
			projectValid: true,
			onRename: (_fileId, declaration) => renamed.push(declaration.id ?? ''),
			onRemove: (_fileId, _kind, declaration) => removed.push(declaration?.id ?? '')
		});

		expect(views.map(({ id, title, resourceKind, displayKind, hasErrors, unsaved }) => ({
			id, title, resourceKind, displayKind, hasErrors, unsaved
		}))).toEqual([
			{ id: 'visual', title: 'Gateway', resourceKind: 'visual', displayKind: 'visual', hasErrors: true, unsaved: false },
			{ id: 'scenario', title: 'Active scenario', resourceKind: 'scenario', displayKind: 'scenario', hasErrors: false, unsaved: true },
			{ id: 'component', title: 'service', resourceKind: 'source', displayKind: 'component', hasErrors: false, unsaved: false }
		]);
		views[1]?.rename?.();
		views[1]?.remove();
		expect(renamed).toEqual(['active']);
		expect(removed).toEqual(['active']);
	});

	test('disables file actions until the current project is valid', () => {
		const [view] = buildWorkspaceTabViews({
			files: [{ id: 'visual', path: 'visuals/gateway.schemra', source: '', savedSource: '' }],
			compilation: [{ id: 'visual', kind: 'visual', diagnostics: [], declarations: [{ kind: 'visual', id: 'gateway', title: 'Gateway' }] }],
			tabIds: ['visual'],
			activeFileId: 'visual',
			currentDeclaration: { kind: 'visual', id: 'gateway', title: 'Gateway' },
			projectFileCount: 1,
			initialized: true,
			compiling: false,
			projectValid: false,
			onRename: () => undefined,
			onRemove: () => undefined
		});

		expect(view?.renameDisabled).toBe(true);
		expect(view?.removeDisabled).toBe(true);
	});
});
