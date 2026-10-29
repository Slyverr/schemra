import { defineConfig } from '@playwright/test';

const baseURL = 'http://127.0.0.1:5174';

export default defineConfig({
	testDir: './e2e',
	testMatch: '**/*.spec.ts',
	fullyParallel: true,
	forbidOnly: Boolean(process.env.CI),
	workers: 1,
	reporter: 'list',
	use: {
		baseURL,
		headless: true,
		trace: 'retain-on-failure',
		screenshot: 'only-on-failure'
	},
	webServer: {
		command: 'bun run dev -- --port 5174 --strictPort',
		url: baseURL,
		reuseExistingServer: false,
		timeout: 120_000
	}
});
