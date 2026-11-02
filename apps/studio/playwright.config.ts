import { defineConfig } from '@playwright/test';

const port = Number(process.env.STUDIO_E2E_PORT ?? 5173);
if (!Number.isInteger(port) || port < 1 || port > 65535) {
	throw new Error('STUDIO_E2E_PORT must be an integer between 1 and 65535.');
}

const baseURL = `http://127.0.0.1:${port}`;
const browserExecutable = process.env.STUDIO_E2E_BROWSER_EXECUTABLE;

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
		screenshot: 'only-on-failure',
		...(browserExecutable ? { launchOptions: { executablePath: browserExecutable } } : {})
	},
	webServer: {
		command: `bun run dev -- --port ${port} --strictPort`,
		url: baseURL,
		reuseExistingServer: true,
		timeout: 120_000
	}
});
