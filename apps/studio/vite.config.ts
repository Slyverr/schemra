import { realpathSync } from 'node:fs';
import { resolve } from 'node:path';
import { env } from 'node:process';
import { fileURLToPath } from 'node:url';
import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import tailwindcss from '@tailwindcss/vite';
import { wasmRefreshPlugin } from './vite-plugins/wasm-refresh.ts';

const studioRoot = fileURLToPath(new URL('.', import.meta.url));
const workspaceRoot = resolve(studioRoot, '../..');
const configuredBasePath = env.BASE_PATH ?? '';
if (configuredBasePath && (!configuredBasePath.startsWith('/') || configuredBasePath.endsWith('/'))) {
	throw new Error('BASE_PATH must start with / and must not end with /.');
}
const basePath = configuredBasePath as '' | `/${string}`;
const interFontPath = realpathSync(resolve(studioRoot, 'node_modules/@fontsource-variable/inter'));
const examplesPath = realpathSync(resolve(studioRoot, '../../examples'));

export default defineConfig({
	server: {
		fs: {
			allow: [searchForWorkspaceRoot(studioRoot), interFontPath, examplesPath]
		}
	},
	plugins: [
		tailwindcss(),
		wasmRefreshPlugin(studioRoot, workspaceRoot),
		sveltekit({ adapter: adapter(), paths: { base: basePath } })
	]
});
