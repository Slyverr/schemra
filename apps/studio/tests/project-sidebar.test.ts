import { describe, expect, test } from 'bun:test';
import { studioMessage } from '../src/lib/i18n/messages.js';
import { buildProjectSidebarModel } from '../src/lib/studio/project-sidebar.js';
import { buildWorkspaceTabIds, nextWorkspaceTabIndex } from '../src/lib/studio/workspace-tabs.js';

describe('project resource sidebar model', () => {
	test('groups linked scenarios and places non-visual files under unlinked sources', () => {
		const documents = [
			{ id: 'visual-file', path: 'visuals/gateway.schemra', source: 'visual source', savedSource: 'visual source' },
			{ id: 'focused-visual', path: 'test-suite/visuals/focused/recovery.schemra', source: 'example visual', savedSource: 'example visual' },
			{ id: 'scenario-file', path: 'scenarios/traffic.schemra', source: 'edited scenario', savedSource: 'scenario' },
			{ id: 'component-file', path: 'components/service.schemra', source: 'component', savedSource: 'component' },
			{ id: 'orphan-file', path: 'scenarios/orphan.schemra', source: 'orphan', savedSource: 'orphan' }
		];
		const compilation = [
			{
				id: 'visual-file',
				path: 'visuals/gateway.schemra',
				kind: 'visual',
				visual_id: 'gateway',
				diagnostics: [],
				declarations: [{ kind: 'visual', id: 'gateway', title: 'Gateway' }]
			},
			{
				id: 'focused-visual',
				path: 'test-suite/visuals/focused/recovery.schemra',
				kind: 'visual',
				visual_id: 'recovery',
				diagnostics: [],
				declarations: [{ kind: 'visual', id: 'recovery', title: 'Recovery' }]
			},
			{
				id: 'scenario-file',
				path: 'scenarios/traffic.schemra',
				kind: 'scenario',
				visual_id: 'gateway',
				diagnostics: [{ severity: 'error' }],
				declarations: [
					{ kind: 'scenario', id: 'load', title: 'Load' },
					{ kind: 'scenario', id: 'recovery', title: 'Recovery' }
				]
			},
			{
				id: 'component-file',
				path: 'components/service.schemra',
				kind: 'component',
				visual_id: null,
				diagnostics: [],
				declarations: []
			}
		];

		const model = buildProjectSidebarModel(documents, compilation);

		expect(model.visuals).toEqual([{
			id: 'gateway',
			fileId: 'visual-file',
			path: 'visuals/gateway.schemra',
			title: 'Gateway',
			pathHint: null,
			dirty: false,
			scenarios: [
				{ id: 'load', fileId: 'scenario-file', title: 'Load', dirty: true, hasErrors: true },
				{ id: 'recovery', fileId: 'scenario-file', title: 'Recovery', dirty: true, hasErrors: true }
			]
		}, {
			id: 'recovery',
			fileId: 'focused-visual',
			path: 'test-suite/visuals/focused/recovery.schemra',
			title: 'Recovery',
			pathHint: null,
			dirty: false,
			scenarios: []
		}]);
		expect(model.primaryVisuals.map(({ id }) => id)).toEqual(['gateway']);
		expect(model.exampleVisuals.map(({ id }) => id)).toEqual(['recovery']);
		expect(model.otherSources.map(({ fileId, kind }) => [fileId, kind])).toEqual([
			['component-file', 'component'],
			['orphan-file', 'scenario']
		]);
	});

	test('disambiguates repeated visual titles with the shortest unique path suffix', () => {
		const documents = [
			{ id: 'north', path: 'visuals/north/gateway.schemra', source: '', savedSource: '' },
			{ id: 'south', path: 'visuals/south/gateway.schemra', source: '', savedSource: '' }
		];
		const compilation = documents.map(({ id, path }) => ({
			id,
			path,
			kind: 'visual',
			visual_id: id,
			diagnostics: [],
			declarations: [{ kind: 'visual', id, title: 'Gateway' }]
		}));

		const { visuals } = buildProjectSidebarModel(documents, compilation);
		expect(visuals.map(({ pathHint }) => pathHint)).toEqual(['north/gateway', 'south/gateway']);
	});
});

test('catalog messages interpolate labels for user resources', () => {
	expect(studioMessage('sidebar.scenariosFor', { visual: 'Gateway' })).toBe('Scenarios for Gateway');
});

test('workspace tabs keep the visual before its scenarios and retain a selected component', () => {
	const documents = [
		{ id: 'visual', path: 'visuals/gateway.schemra' },
		{ id: 'scenario-b', path: 'scenarios/b.schemra' },
		{ id: 'component', path: 'components/service.schemra' },
		{ id: 'scenario-a', path: 'scenarios/a.schemra' },
		{ id: 'other-visual', path: 'visuals/other.schemra' }
	];
	const compilation = [
		{ id: 'visual', kind: 'visual', visual_id: 'gateway' },
		{ id: 'scenario-b', kind: 'scenario', visual_id: 'gateway' },
		{ id: 'component', kind: 'component', visual_id: null },
		{ id: 'scenario-a', kind: 'scenario', visual_id: 'gateway' },
		{ id: 'other-visual', kind: 'visual', visual_id: 'other' }
	];

	expect(buildWorkspaceTabIds(documents, compilation, 'gateway', 'component')).toEqual([
		'visual', 'scenario-b', 'scenario-a', 'component'
	]);
});

test('workspace tabs wrap arrow navigation and support Home and End', () => {
	expect(nextWorkspaceTabIndex(0, 'ArrowLeft', 4)).toBe(3);
	expect(nextWorkspaceTabIndex(3, 'ArrowRight', 4)).toBe(0);
	expect(nextWorkspaceTabIndex(2, 'Home', 4)).toBe(0);
	expect(nextWorkspaceTabIndex(1, 'End', 4)).toBe(3);
	expect(nextWorkspaceTabIndex(0, 'ArrowRight', 0)).toBeNull();
});
