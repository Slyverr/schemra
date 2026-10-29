import { describe, expect, test } from 'bun:test';
import type { ProjectCompileResult, ProjectSource } from '../../src/lib/schemra.js';
import { compareSourceFiles, isErrorDiagnostic, isValidProjectCompilation, makeSourcesKey, stabilizeFileIdentity } from '../../src/lib/studio/project-view.js';

describe('project source view helpers', () => {
	test('builds a source key from the ordered project source snapshot', () => {
		const sources: ProjectSource[] = [
			{ id: 'visual', path: 'visuals/main.schemra', source: 'visual main {}' },
			{ id: 'scenario', path: 'scenarios/main.schemra', source: 'scenario main {}' }
		];

		expect(makeSourcesKey(sources)).toBe(JSON.stringify([
			['visual', 'visuals/main.schemra', 'visual main {}'],
			['scenario', 'scenarios/main.schemra', 'scenario main {}']
		]));
		expect(makeSourcesKey(sources)).not.toBe(makeSourcesKey([...sources].reverse()));
	});

	test('sorts visual sources before scenarios and other files deterministically', () => {
		const files = [
			{ path: 'components/service.schemra' },
			{ path: 'scenarios/traffic.schemra' },
			{ path: 'visuals/zeta.schemra' },
			{ path: 'visuals/alpha.schemra' }
		].sort(compareSourceFiles);

		expect(files.map(({ path }) => path)).toEqual([
			'visuals/alpha.schemra',
			'visuals/zeta.schemra',
			'scenarios/traffic.schemra',
			'components/service.schemra'
		]);
	});

	test('recognizes only error diagnostics', () => {
		expect(isErrorDiagnostic({ severity: 'error' })).toBe(true);
		expect(isErrorDiagnostic({ severity: 'warning' })).toBe(false);
		expect(isErrorDiagnostic(null)).toBe(false);
	});

	test('retains prior resource identity while an edited file has diagnostics', () => {
		const lastValid = [
			{ id: 'visual', kind: 'visual', visual_id: 'traffic', declarations: [{ id: 'traffic' }], diagnostics: [] },
			{ id: 'scenario', kind: 'scenario', visual_id: 'traffic', declarations: [{ id: 'request' }], diagnostics: [] }
		];
		const current = [
			{ id: 'visual', kind: 'source', visual_id: null, declarations: [], diagnostics: [{ severity: 'error' }], tokens: [] },
			{ id: 'scenario', kind: 'scenario', visual_id: 'traffic', declarations: [{ id: 'request' }], diagnostics: [] }
		];

		const files = stabilizeFileIdentity(current, lastValid, ['visual', 'scenario']);

		expect(files.map(({ id, kind, visual_id }) => [id, kind, visual_id])).toEqual([
			['visual', 'visual', 'traffic'],
			['scenario', 'scenario', 'traffic']
		]);
		expect(files[0]?.diagnostics).toEqual([{ severity: 'error' }]);
		expect(files[0]?.tokens).toEqual([]);
	});

	test('uses the previous file identity when a failed project compile omits that file', () => {
		const lastValid = [{ id: 'scenario', kind: 'scenario', visual_id: 'traffic', declarations: [], diagnostics: [] }];
		expect(stabilizeFileIdentity([], lastValid, ['scenario'])).toEqual(lastValid);
		expect(stabilizeFileIdentity([], lastValid, ['removed'])).toEqual([]);
	});

	test('requires an error-free project and at least one renderable visual', () => {
		const valid: ProjectCompileResult = {
			error: null,
			files: [{ id: 'visual', path: 'visuals/main.schemra', kind: 'visual', visual_id: 'main', declarations: [], editor_symbols: [], tokens: [], diagnostics: [], output: null }],
			visuals: [{ id: 'main', path: 'visuals/main.schemra', title: 'Main', output: { ok: true, svg: '<svg />', title: 'Main', stats: null, diagnostics: [], tokens: [], declarations: [], scenarios: [], scene: {} } }]
		};

		expect(isValidProjectCompilation(valid)).toBe(true);
		expect(isValidProjectCompilation({ ...valid, files: valid.files.map((file) => ({ ...file, diagnostics: [{ severity: 'error' }] })) })).toBe(false);
		expect(isValidProjectCompilation({ ...valid, visuals: [] })).toBe(false);
		expect(isValidProjectCompilation({ ...valid, visuals: [{ ...valid.visuals[0]!, output: { ...valid.visuals[0]!.output, scene: null } }] })).toBe(false);
	});
});
