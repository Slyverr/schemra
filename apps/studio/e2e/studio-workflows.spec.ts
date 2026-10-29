import { expect, test } from '@playwright/test';

test('loads the editor, preview, and responsive brand assets', async ({ page }) => {
	await page.goto('/');

	await expect(page).toHaveTitle('Schemra Studio');
	await expect(page.getByRole('textbox', { name: / editor$/ })).toBeVisible();
	await expect(page.getByRole('region', { name: 'Diagram canvas' })).toBeVisible();

	const brand = page.getByRole('link', { name: 'Schemra Studio home' }).locator('img');
	await expect.poll(() => brand.evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth > 0)).toBe(true);

	await page.setViewportSize({ width: 390, height: 844 });
	await expect.poll(() => brand.evaluate((image: HTMLImageElement) => image.currentSrc)).toContain('schemra-style-mark.png');

	await page.setViewportSize({ width: 1280, height: 720 });
	await expect.poll(() => brand.evaluate((image: HTMLImageElement) => image.currentSrc)).toContain('schemra-geometric-wordmark.png');
});

test('searches Studio actions and opens settings', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: 'Search files and actions' }).click();

	const palette = page.getByRole('dialog', { name: 'Search files and actions' });
	await expect(palette).toBeVisible();
	await page.getByRole('combobox', { name: 'Search open tabs, project files, projects, and actions' }).fill('Open settings');
	await page.getByRole('option').filter({ hasText: 'Open settings' }).click();

	const settings = page.getByRole('dialog', { name: 'Settings' });
	await expect(settings).toBeVisible();
	await page.getByRole('button', { name: 'Close settings' }).click();
	await expect(settings).toBeHidden();
});

test('scenario playback advances and can be paused', async ({ page }) => {
	await page.goto('/');

	const timeline = page.getByRole('slider', { name: 'Scenario timeline' });
	await expect(timeline).toBeEnabled();
	await page.getByRole('button', { name: 'Play scenario' }).click();
	await expect(page.getByRole('button', { name: 'Pause scenario' })).toBeVisible();
	await expect.poll(async () => Number(await timeline.inputValue())).toBeGreaterThan(0);

	await page.getByRole('button', { name: 'Pause scenario' }).click();
	await expect(page.getByRole('button', { name: 'Play scenario' })).toBeVisible();
});

test('restores the caret and vertical scroll when returning to a source tab', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('button', { name: 'Save workspace' })).toBeEnabled();
	const tabs = page.locator('.file-tabs button.file-tab');
	await expect(tabs).toHaveCount(6);
	const firstTab = tabs.nth(0);
	const secondTab = tabs.nth(1);
	await firstTab.click();
	await expect(firstTab).toHaveAttribute('aria-current', 'page');

	const editor = page.getByRole('textbox', { name: / editor$/ });
	const longSource = Array.from({ length: 100 }, (_, index) => `// editor view state ${index}`).join('\n');
	await editor.fill(longSource);
	await editor.press('Control+End');
	const savedView = await editor.evaluate((element: HTMLTextAreaElement) => ({
		selectionStart: element.selectionStart,
		scrollTop: element.parentElement?.parentElement?.scrollTop ?? 0
	}));
	expect(savedView.scrollTop).toBeGreaterThan(0);

	await secondTab.click();
	await expect(secondTab).toHaveAttribute('aria-current', 'page');
	await expect(editor).not.toHaveValue(longSource);
	await firstTab.click();
	await expect(firstTab).toHaveAttribute('aria-current', 'page');
	await expect(editor).toHaveValue(longSource);
	await expect.poll(() => editor.evaluate((element: HTMLTextAreaElement) => element.selectionStart)).toBe(savedView.selectionStart);
	await expect.poll(() => editor.evaluate((element: HTMLTextAreaElement) => element.parentElement?.parentElement?.scrollTop ?? 0)).toBe(savedView.scrollTop);
});

test('flushes edits when hidden and warns before leaving unsaved source changes', async ({ page }) => {
	await page.goto('/');
	await expect(page.getByRole('button', { name: 'Save workspace' })).toBeEnabled();

	const editor = page.getByRole('textbox', { name: / editor$/ });
	const updatedSource = `${await editor.inputValue()}\n// lifecycle persistence`;
	await editor.fill(updatedSource);
	await page.evaluate(() => {
		Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
		document.dispatchEvent(new Event('visibilitychange'));
	});
	await expect(page.locator('.save-state')).toContainText('Saved locally');

	const beforeUnload = page.waitForEvent('dialog');
	const reload = page.reload();
	const dialog = await beforeUnload;
	expect(dialog.type()).toBe('beforeunload');
	await dialog.accept();
	await reload;

	await expect(page.getByRole('button', { name: 'Save workspace' })).toBeEnabled();
	await expect(editor).toHaveValue(updatedSource);
});
