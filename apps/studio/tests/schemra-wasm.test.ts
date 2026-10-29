import { beforeAll, describe, expect, test } from 'bun:test';
import { readFileSync } from 'node:fs';
import { initSync } from '../src/lib/generated/schemra_wasm.js';
import {
	compileProject,
	compileSource,
	completeAt,
	completeAtWithNodes,
	createScenarioPlayback,
	formatSource,
	renameProject,
	type ProjectSource
} from '../src/lib/schemra';

beforeAll(() => {
	initSync({
		module: readFileSync(new URL('../src/lib/generated/schemra_wasm_bg.wasm', import.meta.url))
	});
});

describe('Studio compiler bridge', () => {
	test('compiles a visual and a linked project through the browser binding', async () => {
		const visual = `visual network "Network" {
  node client "Client" {}
  node server "Server" {}
  client -> server
}`;
		const scenario = `scenario requests for visual network {
  token packet { icon: "mail" }
  move packet from client -> server { duration: 100ms }
}`;
		const single = await compileSource(visual);
		const project = await compileProject([
			{ id: 'visual', path: 'visuals/network.schemra', source: visual },
			{ id: 'scenario', path: 'scenarios/requests.schemra', source: scenario }
		]);

		expect(single.ok).toBe(true);
		expect(single.svg).toContain('<svg');
		expect(project.error).toBeNull();
		expect(project.visuals[0]?.output.scenarios).toHaveLength(1);
	});

	test('formats source with the shared formatter', async () => {
		const formatted = await formatSource('visual app {\nnode api {}\n}');
		expect(formatted).toBe('visual app {\n  node api {}\n}\n');
		expect(await formatSource(formatted)).toBe(formatted);
	});

	test('maps completion ranges between Rust UTF-8 and editor UTF-16 offsets', async () => {
		const source = '// café\nvisual app {\n  client -> se';
		const completion = await completeAtWithNodes(source, source.length, ['server'], true);
		expect(completion?.start).toBe(source.length - 2);
		expect(completion?.end).toBe(source.length);
		expect(completion?.items.map((item) => item.label)).toContain('server');

		const rootSource = '// café\nvis';
		const rootCompletion = await completeAt(rootSource, rootSource.length, true);
		expect(rootCompletion?.items.map((item) => item.label)).toContain('visual');
		expect(rootCompletion?.start).toBe(rootSource.length - 3);
	});

	test('prepares project rename edits with UTF-16 editor spans', async () => {
		const sources: ProjectSource[] = [
			{
				id: 'visual',
				path: 'visual.schemra',
				source: `// 😀\nvisual network {\n  node client {}\n  node server {}\n  client -> server\n}`
			},
			{
				id: 'scenario',
				path: 'scenario.schemra',
				source: `// café\nscenario request for visual network {\n  token packet {}\n  move packet from client -> server {}\n}`
			}
		];
		const source = sources[0]!.source;
		const cursorOffset = source.indexOf('client');
		const result = await renameProject(sources, 'visual.schemra', cursorOffset, 'frontend');

		expect(result.error).toBeNull();
		expect(result.selection?.name).toBe('client');
		expect(result.edits).toHaveLength(3);
		for (const edit of result.edits) {
			const editedSource = sources.find((item) => item.path === edit.path)?.source;
			expect(editedSource?.slice(edit.span.start.offset, edit.span.end.offset)).toBe('client');
		}
	});

	test('samples a scenario repeatedly and releases its WASM sampler', async () => {
		const sources: ProjectSource[] = [
			{
				id: 'visual',
				path: 'visual.schemra',
				source: 'visual network {\n  node client {}\n  node server {}\n  client -> server\n}'
			},
			{
				id: 'scenario',
				path: 'scenario.schemra',
				source: `scenario requests for visual network {\n  token packet {}\n  move packet from client -> server { duration: 100ms }\n  wait 25ms\n}`
			}
		];
		const playback = await createScenarioPlayback(sources, 'network', 'requests');

		expect(playback.durationMs).toBe(125);
		expect(playback.sample(50)['tokens']).toHaveLength(1);
		expect(playback.sample(100)['tokens']).toHaveLength(0);
		playback.dispose();
		playback.dispose();
		expect(() => playback.sample(0)).toThrow('disposed');
	});
});
