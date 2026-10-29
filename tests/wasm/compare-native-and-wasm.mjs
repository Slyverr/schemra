import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const [sourcePath, nativeOutputPath, wasmModulePath] = process.argv.slice(2);
if (!sourcePath || !nativeOutputPath || !wasmModulePath) {
	throw new Error('usage: compare-native-and-wasm.mjs <source> <native-json> <wasm-module>');
}

const source = readFileSync(sourcePath, 'utf8');
const native = JSON.parse(readFileSync(nativeOutputPath, 'utf8'));
const wasm = createRequire(import.meta.url)(wasmModulePath);
const project = JSON.stringify({
	sources: [{ id: 'main.schemra', path: 'main.schemra', source }],
});
const sampler = new wasm.ScenarioSampler(project, 'network', 'request');
const completionSource = 'visual network {\n client -> ';
const renameRequest = {
	sources: [{ id: 'main.schemra', path: 'main.schemra', source }],
	path: 'main.schemra',
	offset: source.indexOf('client'),
};
const actual = {
	single: JSON.parse(wasm.compile_schemra(source)),
	project: JSON.parse(wasm.compile_project_schemra(project)),
	invalid: JSON.parse(wasm.compile_schemra('visual network { node client {')),
	oversized: JSON.parse(wasm.compile_schemra(' '.repeat(2 * 1024 * 1024 + 1))),
	invalid_project: JSON.parse(wasm.compile_project_schemra('{')),
	formatted: wasm.format_schemra(source),
	completion: JSON.parse(wasm.complete_schemra(completionSource, completionSource.length, true)),
	completion_with_nodes: JSON.parse(wasm.complete_schemra_with_nodes(
		completionSource,
		completionSource.length,
		true,
		JSON.stringify({ node_ids: ['client', 'server'] }),
	)),
	rename_prepared: JSON.parse(wasm.rename_project_schemra(JSON.stringify(renameRequest))),
	rename_applied: JSON.parse(wasm.rename_project_schemra(JSON.stringify({
		...renameRequest,
		new_name: 'frontend',
	}))),
	playback_duration_ms: sampler.durationMs(),
	playback_mid: JSON.parse(sampler.sample(50)),
	playback_end: JSON.parse(sampler.sample(sampler.durationMs())),
};

assert.deepEqual(actual, native, 'WASM responses differ from native Rust responses');
assert.equal(actual.single.ok, true);
assert.match(actual.single.svg, /data-from="client" data-to="server"/);
assert.equal(actual.single.scenarios[0].duration_ms, 175);
assert.equal(actual.project.visuals[0].output.scenarios[0].id, 'request');
assert.equal(actual.oversized.diagnostics[0].code, 'E_INPUT_LIMIT');

process.stdout.write('Native Rust and wasm32 compiler API responses match.\n');
