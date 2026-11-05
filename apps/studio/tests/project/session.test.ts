import { describe, expect, it } from 'bun:test';
import {
	assertCanAddDocument,
	createInitialProject,
	createProjectFromBundle,
	MAX_PROJECT_FILES,
	MAX_PROJECT_SOURCE_BYTES,
	MAX_SOURCE_FILE_BYTES,
	normalizeProjectFilePath,
	parseProjectBundle,
	restoreStudioProject,
	serializeProjectBundle,
	uniqueScenarioId,
	uniqueProjectPath,
	uniqueProjectName,
	type StudioProject
} from '../../src/lib/project/session.ts';

describe('project bundle serialization', () => {
	it('rehydrates persisted source files and ignores obsolete metadata', () => {
		const restored = restoreStudioProject({
			id: 'project-local',
			name: 'Local project',
			packLock: 'format = 1\n',
			packCacheFiles: [{
				path: '.schemra/packs/acme/status/1.0.0/schemra-pack.toml',
				contents_base64: 'W3BhY2td'
			}],
			projectConfigToml: '[visual]\ntheme = "paper"\n',
			files: [{ id: 'visual-main', path: 'visuals/main.schemra', source: 'visual main "Main" {}' }]
		});

		expect(restored).not.toHaveProperty('packLock');
		expect(restored).not.toHaveProperty('packCacheFiles');
		expect(restored).not.toHaveProperty('projectConfigToml');
		expect(restored.files[0]).toMatchObject({
			savedSource: 'visual main "Main" {}',
			handle: null,
			mode: 'memory'
		});
	});

	it('round trips source files only', () => {
		const project: StudioProject = {
			id: 'project-local',
			name: 'Local project',
			files: [{
				id: 'visual-main',
				path: 'visuals/main.schemra',
				source: 'visual main "Main" {}',
				savedSource: 'visual main "Main" {}',
				handle: null,
				mode: 'memory'
			}]
		};
		const bundle = parseProjectBundle(serializeProjectBundle(project));
		const restored = createProjectFromBundle(bundle);

		expect(bundle.version).toBe(2);
		expect(Object.keys(bundle).sort()).toEqual(['files', 'format', 'name', 'version']);
		expect(restored.files).toHaveLength(1);
	});

	it('upgrades version 1 bundles while preserving source text', () => {
		const bundle = parseProjectBundle(JSON.stringify({
			format: 'schemra-project',
			version: 1,
			name: 'Legacy project',
			files: [{ path: 'visuals/main.schemra', source: 'legacy opaque source' }]
		}));

		expect(bundle.version).toBe(2);
		expect(bundle.files[0]?.source).toBe('legacy opaque source');
	});

	it('rejects an otherwise valid project when JSON escaping exceeds the import limit', () => {
		const expandedSource = String.fromCharCode(0).repeat(1_950_000);
		const project: StudioProject = {
			id: 'project-large',
			name: 'Large project',
			files: Array.from({ length: 6 }, (_, index) => ({
				id: `file-${index + 1}`,
				path: `visuals/large-${index + 1}.schemra`,
				source: expandedSource,
				savedSource: expandedSource,
				handle: null,
				mode: 'memory'
			}))
		};

		expect(() => serializeProjectBundle(project)).toThrow('64 MiB maximum');
	});
});

describe('unique project names', () => {
	it('adds a collision suffix without exceeding the project name limit', () => {
		const maximumLengthName = 'a'.repeat(80);

		expect(uniqueProjectName([maximumLengthName], maximumLengthName)).toBe(`${'a'.repeat(78)} 2`);
		expect(Array.from(uniqueProjectName([maximumLengthName], maximumLengthName))).toHaveLength(80);
	});

	it('uses the next free suffix and normalizes both names', () => {
		expect(uniqueProjectName(['Café', 'Café 2'], ' Café ')).toBe('Café 3');
	});
});

describe('scenario identifiers', () => {
	it('normalizes titles and resolves identifier collisions deterministically', () => {
		expect(uniqueScenarioId('Retry Request', [])).toBe('retry_request');
		expect(uniqueScenarioId('Retry Request', ['retry_request'])).toBe('retry_request_2');
		expect(uniqueScenarioId('Éxample', [])).toBe('example');
		expect(uniqueScenarioId('42nd try', [])).toBe('scenario_42nd_try');
		expect(uniqueScenarioId('x'.repeat(40), ['x'.repeat(32)])).toBe(`${'x'.repeat(30)}_2`);
	});
});

describe('canonical project structure', () => {
	it('places one visual and all six example scenarios under stable project-relative paths', () => {
		const project = createInitialProject('visual traffic_overview {}', 'scenario request_response {}', [
			{ id: 'file-ddos-test', path: 'scenarios/ddos-test.schemra', source: 'scenario ddos_test {}' },
			{ id: 'file-everything', path: 'scenarios/everything.schemra', source: 'scenario everything {}' },
			{ id: 'file-data-lifecycle', path: 'scenarios/data-lifecycle.schemra', source: 'scenario data_lifecycle {}' },
			{ id: 'file-markers', path: 'scenarios/markers.schemra', source: 'scenario markers {}' },
			{ id: 'file-service-flow', path: 'scenarios/service-flow.schemra', source: 'scenario service_flow {}' }
		]);

		expect(project.files.map(({ id, path }) => [id, path])).toEqual([
			['file-traffic-overview', 'visuals/traffic-overview.schemra'],
			['file-request-response', 'scenarios/request-response.schemra'],
			['file-ddos-test', 'scenarios/ddos-test.schemra'],
			['file-everything', 'scenarios/everything.schemra'],
			['file-data-lifecycle', 'scenarios/data-lifecycle.schemra'],
			['file-markers', 'scenarios/markers.schemra'],
			['file-service-flow', 'scenarios/service-flow.schemra']
		]);
	});
});

describe('project document bounds and paths', () => {
	const emptyProject: StudioProject = { id: 'project-local', name: 'Local project', files: [] };
	const projectWithFiles = (count: number, source: string): StudioProject => ({
		...emptyProject,
		files: Array.from({ length: count }, (_, index) => ({
			id: `file-${index}`,
			path: `visuals/${index}.schemra`,
			source,
			savedSource: source,
			handle: null,
			mode: 'memory' as const
		}))
	});

	it('allows sources at configured file and project limits and rejects values above them', () => {
		assertCanAddDocument(emptyProject, 'a'.repeat(MAX_SOURCE_FILE_BYTES));
		assertCanAddDocument(projectWithFiles(6, 'a'.repeat(MAX_SOURCE_FILE_BYTES)), '');

		expect(() => assertCanAddDocument(emptyProject, 'a'.repeat(MAX_SOURCE_FILE_BYTES + 1))).toThrow('2 MiB');
		expect(() => assertCanAddDocument(
			projectWithFiles(6, 'a'.repeat(MAX_SOURCE_FILE_BYTES)),
			'b'
		)).toThrow(`${MAX_PROJECT_SOURCE_BYTES / 1024 / 1024} MiB`);
		expect(() => assertCanAddDocument(projectWithFiles(MAX_PROJECT_FILES, ''), '')).toThrow(
			`up to ${MAX_PROJECT_FILES} source files`
		);
	});

	it('rejects unsafe source paths and allocates deterministic case-insensitive names', () => {
		expect(normalizeProjectFilePath('visuals/main.schemra')).toBe('visuals/main.schemra');
		expect(normalizeProjectFilePath('visuals/Cafe\u0301.schemra')).toBe('visuals/Café.schemra');
		for (const unsafePath of [
			'',
			'/main.schemra',
			'C:/main.schemra',
			'../main.schemra',
			'visuals/../main.schemra',
			'visuals\\main.schemra',
			'visuals//main.schemra',
			'visuals/ main.schemra'
		]) {
			expect(() => normalizeProjectFilePath(unsafePath)).toThrow('safe project-relative path');
		}
		expect(() => normalizeProjectFilePath('visuals/main.txt')).toThrow('must point to a .schemra source file');
		const project = {
			...emptyProject,
			files: [{ id: 'one', path: 'visuals/Main.schemra', source: '', savedSource: '', handle: null, mode: 'memory' as const }]
		};
		expect(uniqueProjectPath(project, 'visuals/main.schemra')).toBe('visuals/main-2.schemra');
		expect(uniqueProjectPath({
			...emptyProject,
			files: [
				{ id: 'one', path: 'visuals/Map.schemra', source: '', savedSource: '', handle: null, mode: 'memory' },
				{ id: 'two', path: 'visuals/map-2.schemra', source: '', savedSource: '', handle: null, mode: 'memory' }
			]
		}, 'visuals/map.schemra')).toBe('visuals/map-3.schemra');
		expect(uniqueProjectPath(project, 'scenarios/request.schemra')).toBe('scenarios/request.schemra');
	});
});
