import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const [sourcePath, nativeOutputPath, wasmModulePath] = process.argv.slice(2);
if (!sourcePath || !nativeOutputPath || !wasmModulePath) {
	throw new Error('usage: compare-native-and-wasm-compile.mjs <source> <native-json> <wasm-module>');
}

const source = readFileSync(sourcePath, 'utf8');
const native = JSON.parse(readFileSync(nativeOutputPath, 'utf8'));
const wasm = createRequire(import.meta.url)(wasmModulePath);
const actual = JSON.parse(wasm.compile_schemra(source));

assert.deepEqual(actual, native, 'WASM compilation differs from native Rust');
assert.equal(actual.ok, true);
assert.equal(actual.scene.nodes.length, 4);
assert.equal(actual.scene.edges.length, 3);
assert.equal(actual.scenarios[0].duration_ms, 1775);

const scenario = actual.scenarios[0];
assert.deepEqual(scenario.tokens.map(({ token, icon, char }) => ({ token, icon, char })), [
	{ token: 'request', icon: 'lucide/mail', char: null },
	{ token: 'response', icon: null, char: 'R' },
]);

const moves = scenario.effects.filter(({ kind }) => kind === 'move');
assert.equal(moves.length, 4);
assert.deepEqual(moves.slice(0, 2).map(({ start_ms }) => start_ms), [0, 0]);
assert.equal(
	scenario.effects.filter(({ kind, target_id }) => kind === 'highlight' && target_id === 'proxy').length,
	3,
);
assert.ok(scenario.effects.some(({ kind, target_id, effect, origin }) =>
	kind === 'highlight' &&
	target_id === 'queue' &&
	effect === 'Trace' &&
	origin?.Side?.side === 'Left'));

process.stdout.write('Cross-target native Rust and wasm32 compilation outputs match.\n');
