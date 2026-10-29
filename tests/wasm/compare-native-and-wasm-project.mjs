import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const [projectPath, nativeOutputPath, wasmModulePath] = process.argv.slice(2);
if (!projectPath || !nativeOutputPath || !wasmModulePath) {
	throw new Error('usage: compare-native-and-wasm-project.mjs <project-json> <native-json> <wasm-module>');
}

const project = readFileSync(projectPath, 'utf8');
const native = JSON.parse(readFileSync(nativeOutputPath, 'utf8'));
const wasm = createRequire(import.meta.url)(wasmModulePath);
const actual = JSON.parse(wasm.compile_project_schemra(project));

assert.deepEqual(actual, native, 'WASM project compilation differs from native Rust');
assert.equal(actual.error, null);
assert.equal(actual.visuals.length, 1, 'the project should compile one visual');
assert.equal(actual.visuals[0].output.ok, true);
assert.equal(actual.visuals[0].output.stats.nodes, 2);
assert.equal(actual.files[1].kind, 'scenario');
assert.equal(actual.visuals[0].output.scenarios.length, 1);

process.stdout.write('Native Rust and wasm32 project outputs match.\n');
