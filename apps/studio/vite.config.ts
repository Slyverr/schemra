import { realpathSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import adapter from '@sveltejs/adapter-auto';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig, searchForWorkspaceRoot } from 'vite';
import tailwindcss from '@tailwindcss/vite';

const studioRoot = fileURLToPath(new URL('.', import.meta.url));
const interFontPath = realpathSync(resolve(studioRoot, 'node_modules/@fontsource-variable/inter'));

export default defineConfig({
	server: {
		fs: {
			allow: [searchForWorkspaceRoot(studioRoot), interFontPath]
		}
	},
	plugins: [tailwindcss(), sveltekit({ adapter: adapter() })]
});
