import { describe, expect, test } from 'bun:test';
import type { HmrContext, ViteDevServer } from 'vite';
import { wasmRefreshPlugin } from '../vite-plugins/wasm-refresh';

const plugin = wasmRefreshPlugin('/studio', '/workspace');
const hotUpdateHook = plugin.handleHotUpdate;
const handleHotUpdate = typeof hotUpdateHook === 'function' ? hotUpdateHook : hotUpdateHook?.handler;

function update(file: string): HmrContext {
	return {
		file,
		timestamp: 1,
		modules: [],
		read: async () => '',
		server: {} as ViteDevServer
	};
}

describe('WASM dev refresh', () => {
	test('defers generated binding updates until the build completes', async () => {
		expect(handleHotUpdate).toBeDefined();
		expect(handleHotUpdate?.(update('/studio/src/lib/generated/schemra_wasm.js'))).toEqual([]);
		expect(handleHotUpdate?.(update('/studio/src/lib/generated/schemra_wasm_bg.wasm'))).toEqual([]);
	});

	test('leaves ordinary source updates to Vite', () => {
		expect(handleHotUpdate?.(update('/studio/src/routes/+page.svelte'))).toBeUndefined();
	});
});
