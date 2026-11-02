import { expect, test } from '@playwright/test';
import { studioMessage } from '../src/lib/i18n/messages.js';

test('keeps preview controls over the canvas and supports zoom, pan, and fit', async ({ page }) => {
	await page.goto('/');

	const preview = page.getByRole('region', { name: studioMessage('pane.previewPanel') }).last();
	const canvas = preview.getByRole('application', { name: studioMessage('preview.canvas') });
	const toolbar = preview.locator('.preview-toolbar');
	const timeline = preview.locator('.timeline');
	await expect(toolbar).toBeVisible();
	await expect(timeline).toBeHidden();
	expect(await toolbar.evaluate((element) => getComputedStyle(element).position)).toBe('absolute');

	const scenarioKind = studioMessage('sidebar.resourceKindScenario');
	const scenarioTab = page.getByRole('tab', { name: new RegExp(`^${scenarioKind}:`) }).first();
	await scenarioTab.click();
	await expect(timeline).toBeVisible();
	expect(await timeline.evaluate((element) => getComputedStyle(element).position)).toBe('absolute');
	const previewBounds = await preview.boundingBox();
	const canvasBounds = await canvas.boundingBox();
	expect(canvasBounds?.x).toBe(previewBounds?.x);
	expect(canvasBounds?.y).toBe(previewBounds?.y);
	expect(canvasBounds?.width).toBe(previewBounds?.width);
	expect(canvasBounds?.height).toBe(previewBounds?.height);
	await expect(canvas).toHaveCSS('user-select', 'none');
	await canvas.focus();
	await expect(canvas).toHaveCSS('outline-style', 'none');
	await expect.poll(() => preview.locator('svg.schemra-preview-svg > rect').evaluateAll((rects) =>
		rects.filter((rect) => {
			const { width, height } = rect.ownerSVGElement?.viewBox.baseVal ?? { width: 0, height: 0 };
			return Number(rect.getAttribute('width')) === width && Number(rect.getAttribute('height')) === height;
		}).length
	)).toBe(0);

	const zoom = preview.getByRole('group', { name: studioMessage('preview.zoomControls') }).locator('output');
	await expect(zoom).toHaveText('100%');
	expect(canvasBounds).not.toBeNull();
	if (!canvasBounds) return;

	await page.mouse.move(canvasBounds.x + canvasBounds.width / 2, canvasBounds.y + canvasBounds.height / 2);
	await page.mouse.wheel(0, -500);
	await expect(zoom).not.toHaveText('100%');

	await canvas.focus();
	const svg = canvas.locator('svg.schemra-preview-svg');
	const beforeKeyboardPan = await svg.boundingBox();
	expect(beforeKeyboardPan).not.toBeNull();
	await page.keyboard.press('ArrowRight');
	await expect.poll(async () => (await svg.boundingBox())?.x ?? 0).toBeGreaterThan(beforeKeyboardPan?.x ?? 0);

	const beforeDragPan = await svg.boundingBox();
	expect(beforeDragPan).not.toBeNull();
	await page.mouse.move(canvasBounds.x + canvasBounds.width / 2, canvasBounds.y + canvasBounds.height / 2);
	await page.mouse.down();
	await page.mouse.move(canvasBounds.x + canvasBounds.width / 2 + 50, canvasBounds.y + canvasBounds.height / 2 + 30, { steps: 3 });
	await page.mouse.up();
	await expect.poll(async () => (await svg.boundingBox())?.x ?? 0).toBeGreaterThan(beforeDragPan?.x ?? 0);

	await canvas.focus();
	await page.keyboard.press('Home');
	await expect(zoom).toHaveText('100%');
});

test('stacks preview controls on narrow screens and identifies the static visual', async ({ page }) => {
	await page.setViewportSize({ width: 560, height: 800 });
	await page.goto('/');

	const selector = page.getByRole('combobox', { name: studioMessage('preview.scenarioLabel') });
	await expect(selector).toContainText(studioMessage('preview.staticVisual'));
	await expect(page.getByRole('slider', { name: studioMessage('preview.scenarioTimeline') })).toBeHidden();

	const controls = page.getByRole('group', { name: studioMessage('preview.controls') });
	const zoomControls = page.getByRole('group', { name: studioMessage('preview.zoomControls') });
	const controlsBounds = await controls.boundingBox();
	const zoomBounds = await zoomControls.boundingBox();
	expect(controlsBounds).not.toBeNull();
	expect(zoomBounds).not.toBeNull();
	expect(zoomBounds?.y).toBeGreaterThan(controlsBounds?.y ?? Number.POSITIVE_INFINITY);
});

test('switches between a scenario and the static visual preview', async ({ page }) => {
	await page.goto('/');
	const selector = page.getByRole('combobox', { name: studioMessage('preview.scenarioLabel') });
	await expect(selector).toBeVisible();
	const timeline = page.getByRole('slider', { name: studioMessage('preview.scenarioTimeline') });
	await expect(selector).toContainText(studioMessage('preview.staticVisual'));
	await expect(timeline).toBeHidden();

	const scenarioKind = studioMessage('sidebar.resourceKindScenario');
	const scenarioTab = page.getByRole('tab', { name: new RegExp(`^${scenarioKind}:`) }).first();
	await expect(scenarioTab).toBeVisible();
	await scenarioTab.click();
	await expect(selector).not.toContainText(studioMessage('preview.staticVisual'));
	await expect(timeline).toBeVisible();

	await selector.click();
	await page.getByRole('option').nth(1).click();
	await expect(timeline).toBeVisible();

	await selector.click();
	await page.getByRole('option').first().click();
	await expect(timeline).toBeHidden();
	await expect(page.getByRole('application', { name: studioMessage('preview.canvas') }).locator('svg.schemra-preview-svg')).toBeVisible();
});

test('searches diagram targets from the preview finder', async ({ page }) => {
	await page.goto('/');
	const trigger = page.getByRole('button', { name: studioMessage('preview.findDiagram') });
	await trigger.click();

	const finder = page.getByRole('dialog', { name: studioMessage('preview.findInDiagram') });
	const search = finder.getByRole('combobox', { name: studioMessage('preview.searchScene') });
	await expect(finder).toBeVisible();
	await search.fill('Database');
	const database = finder.getByRole('option', { name: 'Database database' });
	await expect(database).toBeVisible();
	await database.click();
	await expect(finder).toBeHidden();
	await expect(trigger).toBeFocused();
});

test('navigates diagram finder results with arrow keys and Enter, and closes with Escape', async ({ page }) => {
	await page.goto('/');
	const trigger = page.getByRole('button', { name: studioMessage('preview.findDiagram') });
	await trigger.click();

	const finder = page.getByRole('dialog', { name: studioMessage('preview.findInDiagram') });
	const search = finder.getByRole('combobox', { name: studioMessage('preview.searchScene') });
	await search.fill('Database');

	const database = finder.getByRole('option', { name: 'Database database' });
	await expect(database).toHaveAttribute('aria-selected', 'true');
	const initialActiveId = await search.getAttribute('aria-activedescendant');
	await search.press('ArrowDown');
	await expect(database).toHaveAttribute('aria-selected', 'false');
	await expect(search).not.toHaveAttribute('aria-activedescendant', initialActiveId ?? '');

	await search.press('ArrowUp');
	await expect(database).toHaveAttribute('aria-selected', 'true');
	const zoom = page.getByRole('group', { name: studioMessage('preview.zoomControls') }).locator('output');
	await expect(zoom).toHaveText('100%');
	await search.press('Enter');
	await expect(finder).toBeHidden();
	await expect(trigger).toBeFocused();
	await expect(zoom).not.toHaveText('100%');

	await trigger.click();
	const reopenedFinder = page.getByRole('dialog', { name: studioMessage('preview.findInDiagram') });
	await reopenedFinder.getByRole('combobox', { name: studioMessage('preview.searchScene') }).press('Escape');
	await expect(reopenedFinder).toBeHidden();
	await expect(trigger).toBeFocused();
});

test('organizes export options into tabs and keeps select values readable', async ({ page }) => {
	await page.goto('/');
	const selector = page.getByRole('combobox', { name: studioMessage('preview.scenarioLabel') });
	await selector.click();
	await page.getByRole('option').nth(1).click();

	await page.getByRole('button', { name: studioMessage('preview.exportScenarioAnimation') }).click();
	const dialog = page.getByRole('dialog');
	await expect(dialog.getByRole('tab', { name: studioMessage('export.tab.video') })).toBeVisible();
	await expect(dialog.getByRole('tab', { name: studioMessage('export.tab.background') })).toBeVisible();
	await expect(dialog.locator('small')).toHaveCount(0);

	const bitrate = dialog.locator('#animation-export-bitrate');
	await expect(bitrate).toContainText(studioMessage('export.bitrate.balanced'));
	await expect(bitrate).not.toContainText('12000000');

	const format = dialog.locator('#animation-export-format');
	await format.click();
	await page.getByRole('option', { name: studioMessage('export.format.webm') }).click();
	await expect(format).toContainText(studioMessage('export.format.webm'));

	await dialog.getByRole('tab', { name: studioMessage('export.tab.background') }).click();
	await expect(dialog.locator(`[role="group"][aria-label="${studioMessage('export.background')}"]`)).toBeVisible();
});

test('disables animation export when showing the last valid preview', async ({ page }) => {
	await page.goto('/');
	const visualKind = studioMessage('sidebar.resourceKindVisual');
	const visualTab = page.getByRole('tab', { name: new RegExp(`^${visualKind}:`) }).first();
	await expect(visualTab).toBeVisible();
	await visualTab.click();

	const selector = page.getByRole('combobox', { name: studioMessage('preview.scenarioLabel') });
	await expect(selector).toBeVisible();
	await selector.click();
	await page.getByRole('option').nth(1).click();

	const editor = page.getByRole('textbox', { name: / editor$/ });
	await editor.fill('???');
	await expect(page.getByRole('button', { name: studioMessage('resource.lastValidPreview') })).toBeVisible();
	await expect(page.getByRole('button', { name: studioMessage('preview.exportScenarioAnimation') })).toBeDisabled();
});
