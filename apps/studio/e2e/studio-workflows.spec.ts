import { expect, test } from '@playwright/test';
import { studioMessage } from '../src/lib/i18n/messages.js';
import { getScopeRanges } from '../src/lib/editor/scope-guides.js';

test('loads the editor, preview, and sidebar brand assets', async ({ page }) => {
	await page.goto('/');

	await expect(page).toHaveTitle('Schemra Studio');
	await expect(page.getByRole('textbox', { name: / editor$/ })).toBeVisible();
	const canvas = page.getByRole('application', { name: studioMessage('preview.canvas') });
	await expect(canvas).toBeVisible();
	await expect.poll(async () => (await canvas.boundingBox())?.width ?? 0).toBeGreaterThan(0);
	await expect.poll(async () => (await canvas.locator('svg.schemra-preview-svg').boundingBox())?.width ?? 0).toBeGreaterThan(0);

	const brand = page.getByRole('img', { name: 'Schemra', exact: true });
	await expect.poll(() => brand.evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth > 0)).toBe(true);

	await page.getByRole('separator', { name: studioMessage('sidebar.resize') }).dblclick();
	const mark = page.getByRole('button', { name: studioMessage('sidebar.expand') }).locator('img');
	await expect(mark).toBeVisible();
	await expect.poll(() => mark.evaluate((image: HTMLImageElement) => image.complete && image.naturalWidth > 0)).toBe(true);
});

test('shows tab save state through icon and text colors instead of status dots', async ({ page }) => {
	await page.goto('/');

	const selectedTab = page.getByRole('tab', { selected: true });
	const inactiveTab = page.getByRole('tab', { selected: false }).first();
	const selectedLabel = selectedTab.locator('span');
	const inactiveLabel = inactiveTab.locator('span');
	const selectedIcon = selectedTab.locator('svg').first();
	const inactiveIcon = inactiveTab.locator('svg').first();
	const matchesThemeColor = (label: typeof selectedLabel, token: string) => label.evaluate((element, name) => {
		const probe = document.createElement('span');
		probe.style.color = `var(--${name})`;
		document.body.append(probe);
		const expected = getComputedStyle(probe).color;
		probe.remove();
		return getComputedStyle(element).color === expected;
	}, token);

	await expect.poll(() => matchesThemeColor(selectedLabel, 'primary')).toBe(true);
	await expect.poll(() => matchesThemeColor(selectedIcon, 'primary')).toBe(true);
	await expect.poll(() => matchesThemeColor(inactiveLabel, 'muted-foreground')).toBe(true);
	await expect.poll(() => matchesThemeColor(inactiveIcon, 'muted-foreground')).toBe(true);

	const editor = page.getByRole('textbox', { name: / editor$/ });
	await editor.fill(`${await editor.inputValue()}\n`);
	const title = (await selectedTab.locator('span').textContent())?.trim() ?? '';
	await expect(selectedTab).toHaveAttribute('aria-label', `${studioMessage('sidebar.resourceKindVisual')}: ${title}, ${studioMessage('tabs.unsavedState')}`);
	await expect.poll(() => matchesThemeColor(selectedLabel, 'warning')).toBe(true);
	await expect.poll(() => matchesThemeColor(selectedIcon, 'warning')).toBe(true);
	await expect(selectedTab.locator('span')).toHaveCount(1);
});

test('exposes compile errors in the source tab accessibility state', async ({ page }) => {
	await page.goto('/');
	const editor = page.getByRole('textbox', { name: / editor$/ });
	const source = await editor.inputValue();
	await editor.fill(source.replace(/\}\s*$/, ''));

	const selectedTab = page.getByRole('tab', { selected: true });
	const tabTitle = (await selectedTab.locator('span').textContent())?.trim() ?? '';
	await expect(selectedTab).toHaveAccessibleName(studioMessage('tabs.hasErrors', { name: tabTitle }));
	const hasDestructiveColor = (element: typeof selectedTab) => element.evaluate((node) => {
		const probe = document.createElement('span');
		probe.style.color = 'var(--destructive)';
		document.body.append(probe);
		const expected = getComputedStyle(probe).color;
		probe.remove();
		return getComputedStyle(node).color === expected;
	});
	await expect.poll(() => hasDestructiveColor(selectedTab.locator('span'))).toBe(true);
	await expect.poll(() => hasDestructiveColor(selectedTab.locator('svg').first())).toBe(true);
});

test('keeps the new-scenario action centered and opens scenario creation', async ({ page }) => {
	await page.setViewportSize({ width: 780, height: 600 });
	await page.goto('/');
	const action = page.getByRole('button', { name: studioMessage('resource.newScenario'), exact: true });
	await expect(action).toBeEnabled();
	const verticalCenterDelta = await action.evaluate((button) => {
		const nav = button.parentElement!.getBoundingClientRect();
		const bounds = button.getBoundingClientRect();
		return Math.abs((bounds.top + bounds.bottom) / 2 - (nav.top + nav.bottom) / 2);
	});
	const horizontalCenterDelta = await action.evaluate((button) => {
		const icon = button.querySelector('svg')!.getBoundingClientRect();
		const bounds = button.getBoundingClientRect();
		return Math.abs((icon.left + icon.right) / 2 - (bounds.left + bounds.right) / 2);
	});
	expect(verticalCenterDelta).toBeLessThan(1);
	expect(horizontalCenterDelta).toBeLessThan(1);
	await action.click();
	const dialog = page.getByRole('dialog', { name: studioMessage('resource.dialog.createScenarioTitle') });
	await expect(dialog).toBeVisible();
	await expect(dialog.getByLabel(studioMessage('resource.dialog.scenarioName'))).toBeVisible();
	await dialog.getByRole('button', { name: studioMessage('resource.dialog.cancel') }).click();
	await expect(dialog).toBeHidden();
});

test('keeps pointer selection and deletion aligned with highlighted source text', async ({ page }) => {
	await page.goto('/');
	const editor = page.locator('.editor-input');
	await expect(editor).toBeEnabled();
	const source = await editor.inputValue();
	const token = page.locator('.editor-line[data-line-index="0"] .syntax-keyword').first();
	await expect(token).toHaveText('visual');
	const bounds = await token.boundingBox();
	expect(bounds).not.toBeNull();
	if (!bounds) return;
	const y = bounds.y + bounds.height / 2;
	await page.mouse.move(bounds.x + 0.5, y);
	await page.mouse.down();
	await page.mouse.move(bounds.x + bounds.width - 0.5, y, { steps: 6 });
	await page.mouse.up();
	await expect.poll(() => editor.evaluate((element: HTMLTextAreaElement) => element.value.slice(element.selectionStart, element.selectionEnd))).toBe('visual');
	await page.keyboard.press('Backspace');
	await expect(editor).toHaveValue(source.replace(/^visual\b/, ''));
});

test('renders scope guides inside the body and beneath the first opener glyph', async ({ page }) => {
	await page.goto('/');
	const editor = page.locator('.editor-input');
	await expect(editor).toBeEnabled();
	const source = await editor.inputValue();
	const scopes = getScopeRanges(source);
	const guides = page.locator('.scope-guide');
	await expect(guides).toHaveCount(scopes.length);

	for (const scope of scopes) {
		const guide = page.locator(`[data-scope-guide="${scope.id}"]`);
		const geometry = await guide.evaluate((element, { openLineIndex, bodyLineIndex, sourceColumn }) => {
			const container = element.closest('.editor-highlight');
			const opener = container?.querySelector<HTMLElement>(`[data-line-index="${openLineIndex}"]`);
			const body = container?.querySelector<HTMLElement>(`[data-line-index="${bodyLineIndex}"]`);
			if (!opener || !body) return null;

			const walker = document.createTreeWalker(opener, NodeFilter.SHOW_TEXT, {
				acceptNode(node) {
					return node.parentElement?.closest('.inline-diagnostic')
						? NodeFilter.FILTER_REJECT
						: NodeFilter.FILTER_ACCEPT;
				}
			});
			let remaining = sourceColumn;
			let sourceNode: Text | null = null;
			while ((sourceNode = walker.nextNode() as Text | null)) {
				if (remaining < sourceNode.length) break;
				remaining -= sourceNode.length;
			}
			if (!sourceNode) return null;

			const glyphRange = document.createRange();
			glyphRange.setStart(sourceNode, remaining);
			glyphRange.setEnd(sourceNode, remaining + 1);
			const glyph = glyphRange.toString();
			const context = document.createElement('canvas').getContext('2d');
			if (!context || !sourceNode.parentElement) return null;
			const style = getComputedStyle(sourceNode.parentElement);
			context.font = `${style.fontStyle || 'normal'} ${style.fontWeight || 'normal'} ${style.fontSize} ${style.fontFamily}`;
			context.textAlign = 'left';
			context.direction = 'ltr';
			const guideRect = element.getBoundingClientRect();
			const openerRect = opener.getBoundingClientRect();
			const bodyRect = body.getBoundingClientRect();
			const glyphRect = glyphRange.getBoundingClientRect();
			const glyphInkLeft = glyphRect.left - context.measureText(glyph).actualBoundingBoxLeft;
			return {
				guideTop: guideRect.top,
				guideBottom: guideRect.bottom,
				openerBottom: openerRect.bottom,
				bodyBottom: bodyRect.bottom,
				guideLeft: guideRect.left,
				glyphInkLeft
			};
		}, {
			openLineIndex: scope.openLine,
			bodyLineIndex: scope.closed ? scope.endLine - 1 : scope.endLine,
			sourceColumn: scope.anchorOffset - (source.lastIndexOf('\n', scope.anchorOffset - 1) + 1)
		});

		expect(geometry).not.toBeNull();
		if (!geometry) continue;
		expect(Math.abs(geometry.guideTop - geometry.openerBottom)).toBeLessThan(1);
		expect(Math.abs(geometry.guideBottom - geometry.bodyBottom)).toBeLessThan(1);
		expect(Math.abs(geometry.guideLeft - geometry.glyphInkLeft)).toBeLessThan(1);
	}
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

test('navigates the searchable Studio themes from the keyboard', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: studioMessage('header.search') }).click();
	const openSettingsLabel = studioMessage('command.action.openSettings');
	await page.getByRole('combobox', { name: studioMessage('command.inputLabel') }).fill(openSettingsLabel);
	await page.getByRole('option').filter({ hasText: openSettingsLabel }).click();

	const settings = page.getByRole('dialog', { name: studioMessage('settings.heading') });
	const trigger = settings.locator('button[aria-controls="studio-theme-options"]');
	await trigger.click();
	const popover = settings.locator('#studio-theme-options');
	await expect(popover).toBeVisible();
	const triggerSwatch = trigger.locator('svg').first();
	const optionSwatch = popover.getByRole('option').first().locator('svg');
	await expect.poll(async () => (await triggerSwatch.boundingBox())?.width ?? 0).toBe(48);
	await expect.poll(async () => (await optionSwatch.boundingBox())?.width ?? 0).toBe(48);
	await expect(optionSwatch.locator('rect[stroke]')).toHaveCount(0);
	expect(await settings.evaluate((element) => element.contains(element.querySelector('#studio-theme-options')))).toBe(true);
	expect(await popover.evaluate((element) => getComputedStyle(element.parentElement!).zIndex)).toBe('1000');
	const popoverBounds = await popover.boundingBox();
	const settingsBounds = await settings.boundingBox();
	expect(popoverBounds).not.toBeNull();
	expect(settingsBounds).not.toBeNull();
	expect(popoverBounds!.x).toBeGreaterThanOrEqual(settingsBounds!.x);
	expect(popoverBounds!.x + popoverBounds!.width).toBeLessThanOrEqual(settingsBounds!.x + settingsBounds!.width);
	expect(popoverBounds!.y).toBeGreaterThanOrEqual(settingsBounds!.y);
	expect(popoverBounds!.y + popoverBounds!.height).toBeLessThanOrEqual(settingsBounds!.y + settingsBounds!.height);
	expect(await popover.evaluate((element) => {
		const bounds = element.getBoundingClientRect();
		return element.contains(document.elementFromPoint(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2));
	})).toBe(true);

	const search = popover.getByRole('combobox', { name: studioMessage('settings.searchThemes') });
	await search.fill('schemra');
	await expect(search).toHaveAttribute('aria-activedescendant', 'studio-theme-option-schemra-light');
	await search.press('ArrowDown');
	await expect(search).toHaveAttribute('aria-activedescendant', 'studio-theme-option-schemra-dark');
	await search.press('Home');
	await expect(search).toHaveAttribute('aria-activedescendant', 'studio-theme-option-schemra-light');
	await search.press('End');
	await search.press('Enter');
	await expect(trigger).toContainText(studioMessage('theme.schemraDark'));
	await expect(trigger).toHaveAttribute('aria-expanded', 'false');

	const paletteStylesMatchRenderedEditor = () => page.locator('.syntax-keyword').first().evaluate((keyword) => {
		const editor = keyword.closest('.source-editor');
		if (!editor) return false;
		const expected = document.createElement('span');
		expected.style.color = 'var(--editor-syntax-keyword)';
		editor.append(expected);
		const expectedColor = getComputedStyle(expected).color;
		expected.remove();
		return getComputedStyle(keyword).color === expectedColor;
	});
	await expect.poll(paletteStylesMatchRenderedEditor).toBe(true);
	const warningUtilityMatchesTheme = () => page.locator('.source-editor').evaluate((editor) => {
		const utility = document.createElement('span');
		utility.className = 'text-warning';
		utility.textContent = '.';
		const token = document.createElement('span');
		token.style.color = 'var(--warning)';
		token.textContent = '.';
		editor.append(utility, token);
		const utilityColor = getComputedStyle(utility).color;
		const tokenColor = getComputedStyle(token).color;
		utility.remove();
		token.remove();
		return utilityColor === tokenColor;
	});
	await expect.poll(warningUtilityMatchesTheme).toBe(true);
});

test('removes shell viewport minimums and keeps the tab strip sized', async ({ page }) => {
	await page.setViewportSize({ width: 320, height: 240 });
	await page.goto('/');
	await expect(page.getByRole('textbox', { name: / editor$/ })).toBeVisible();
	const shell = page.getByRole('main');
	const shellBox = await shell.boundingBox();
	expect(shellBox).not.toBeNull();
	expect(shellBox?.width).toBe(320);
	expect(shellBox?.height).toBe(240);
	expect(await shell.evaluate((element) => {
		const style = getComputedStyle(element);
		return [style.minWidth, style.minHeight];
	})).toEqual(['0px', '0px']);
	const tabList = page.getByRole('tablist', { name: studioMessage('tabs.projectSourceFiles') });
	const tabsHeader = page.getByRole('navigation', { name: studioMessage('tabs.projectSourceFiles') });
	await expect.poll(() => tabsHeader.evaluate((element) => element.getBoundingClientRect().height)).toBe(44);
	await expect.poll(() => tabList.evaluate((element) => element.getBoundingClientRect().height)).toBe(43);
});

test('cancels visual creation from the ResourceDialog without adding a resource', async ({ page }) => {
	await page.goto('/');
	const visuals = page.locator('#project-resource-sidebar .resource-section').first().locator('.resource-button');
	const visualCount = await visuals.count();
	await page.getByRole('button', { name: studioMessage('sidebar.newVisual'), exact: true }).click();
	const dialog = page.getByRole('dialog', { name: studioMessage('resource.dialog.createVisualTitle') });
	await expect(dialog).toBeVisible();
	await expect(dialog.getByText(studioMessage('resource.dialog.createVisualDescription'), { exact: true })).toBeVisible();
	await expect(dialog.getByLabel(studioMessage('resource.dialog.visualName'))).toBeVisible();

	const primary = dialog.getByRole('button', { name: studioMessage('resource.dialog.createVisual') });
	const cancel = dialog.getByRole('button', { name: studioMessage('resource.dialog.cancel') });
	await expect(primary).toHaveClass(/bg-primary/);
	await expect(primary).toHaveClass(/text-primary-foreground/);
	await expect(cancel).toHaveClass(/border-border/);
	await expect(cancel).toHaveClass(/bg-background/);
	await cancel.click();
	await expect(dialog).toBeHidden();
	await expect(visuals).toHaveCount(visualCount);
});

test('keeps the pane splitter centered on the divider while resizing both layouts', async ({ page }) => {
	await page.setViewportSize({ width: 1280, height: 720 });
	await page.goto('/');
	await expect(page.getByRole('textbox', { name: / editor$/ })).toBeVisible();
	const sidebar = page.getByRole('complementary', { name: studioMessage('sidebar.resources') });
	const sidebarSeparator = sidebar.getByRole('separator', { name: studioMessage('sidebar.resize') });
	const sidebarGeometry = await sidebarSeparator.evaluate((handle) => {
		const sidebarBounds = handle.parentElement!.getBoundingClientRect();
		const bounds = handle.getBoundingClientRect();
		const grip = handle.querySelector('svg')!.getBoundingClientRect();
		return {
			handleWidth: bounds.width,
			centerDelta: Math.abs((bounds.left + bounds.right) / 2 - sidebarBounds.right),
			gripSize: grip.width,
			lineWidth: handle.querySelector('span')!.getBoundingClientRect().width,
			panelBorderWidth: getComputedStyle(handle.parentElement!).borderRightWidth
		};
	});
	expect(sidebarGeometry.handleWidth).toBe(12);
	expect(sidebarGeometry.centerDelta).toBeLessThan(1);
	expect(sidebarGeometry.gripSize).toBe(16);
	expect(sidebarGeometry.lineWidth).toBe(1);
	expect(sidebarGeometry.panelBorderWidth).toBe('0px');

	const sidebarHandle = await sidebarSeparator.boundingBox();
	expect(sidebarHandle).not.toBeNull();
	if (sidebarHandle) {
		const centerX = sidebarHandle.x + sidebarHandle.width / 2;
		const centerY = sidebarHandle.y + sidebarHandle.height / 2;
		await page.mouse.move(centerX, centerY);
		await page.mouse.down();
		await page.mouse.move(centerX - 200, centerY, { steps: 4 });
		await page.mouse.up();
	}
	await expect(sidebarSeparator).toHaveAttribute('aria-valuenow', '56');

	const collapsedHandle = await sidebarSeparator.boundingBox();
	expect(collapsedHandle).not.toBeNull();
	if (collapsedHandle) {
		const centerX = collapsedHandle.x + collapsedHandle.width / 2;
		const centerY = collapsedHandle.y + collapsedHandle.height / 2;
		await page.mouse.move(centerX, centerY);
		await page.mouse.down();
		await page.mouse.move(240, centerY, { steps: 4 });
		await page.mouse.up();
	}
	await expect(sidebarSeparator).toHaveAttribute('aria-valuenow', '240');

	const separator = page.getByRole('separator', { name: studioMessage('pane.resize') });
	const measure = () => separator.evaluate((grip) => {
		const editor = grip.previousElementSibling!.getBoundingClientRect();
		const preview = grip.nextElementSibling!.getBoundingClientRect();
		const bounds = grip.getBoundingClientRect();
		const wide = grip.getAttribute('aria-orientation') === 'vertical';
		return {
			position: getComputedStyle(grip).position,
			background: getComputedStyle(grip).backgroundColor,
			editorSize: wide ? editor.width : editor.height,
			boundaryGap: wide ? preview.left - editor.right : preview.top - editor.bottom,
			centerDelta: wide
				? Math.abs((bounds.left + bounds.right) / 2 - editor.right)
				: Math.abs((bounds.top + bounds.bottom) / 2 - editor.bottom),
			gripSize: wide ? bounds.width : bounds.height
		};
	});

	await expect(separator).toHaveAttribute('aria-orientation', 'vertical');
	let geometry = await measure();
	expect(geometry.position).toBe('relative');
	expect(geometry.background).toBe('rgba(0, 0, 0, 0)');
	expect(Math.abs(geometry.boundaryGap)).toBeLessThan(1);
	expect(geometry.centerDelta).toBeLessThan(1);
	expect(geometry.gripSize).toBe(12);

	const beforeWideDrag = geometry.editorSize;
	const wideBounds = await separator.boundingBox();
	expect(wideBounds).not.toBeNull();
	if (wideBounds) {
		await page.mouse.move(wideBounds.x + wideBounds.width / 2, wideBounds.y + wideBounds.height / 2);
		await page.mouse.down();
		await page.mouse.move(wideBounds.x + wideBounds.width / 2 + 80, wideBounds.y + wideBounds.height / 2, { steps: 4 });
		await page.mouse.up();
	}
	await expect.poll(async () => (await measure()).editorSize).toBeGreaterThan(beforeWideDrag);

	await page.setViewportSize({ width: 780, height: 499 });
	await expect(separator).toHaveAttribute('aria-orientation', 'horizontal');
	geometry = await measure();
	expect(geometry.position).toBe('relative');
	expect(geometry.background).toBe('rgba(0, 0, 0, 0)');
	expect(Math.abs(geometry.boundaryGap)).toBeLessThan(1);
	expect(geometry.centerDelta).toBeLessThan(1);
	expect(geometry.gripSize).toBe(12);

	const beforeStackedDrag = geometry.editorSize;
	const stackedBounds = await separator.boundingBox();
	expect(stackedBounds).not.toBeNull();
	if (stackedBounds) {
		await page.mouse.move(stackedBounds.x + stackedBounds.width / 2, stackedBounds.y + stackedBounds.height / 2);
		await page.mouse.down();
		await page.mouse.move(stackedBounds.x + stackedBounds.width / 2, stackedBounds.y + stackedBounds.height / 2 + 40, { steps: 4 });
		await page.mouse.up();
	}
	await expect.poll(async () => (await measure()).editorSize).toBeGreaterThan(beforeStackedDrag);
});

test('announces editor diagnostics through its description list', async ({ page }) => {
	await page.goto('/');

	const editor = page.getByRole('textbox', { name: / editor$/ });
	const source = await editor.inputValue();
	await editor.fill(`${source}\n???`);

	await expect.poll(() => editor.getAttribute('aria-describedby')).toMatch(/-diagnostics$/);
	const diagnosticListId = await editor.getAttribute('aria-describedby');
	expect(diagnosticListId).toBeTruthy();
	const diagnosticList = page.locator(`[id="${diagnosticListId}"]`);
	await expect(diagnosticList).toHaveAttribute('aria-label', studioMessage('editor.diagnosticList'));
	await expect(diagnosticList.locator('li').first()).toHaveText(/\S/);

	await page.getByRole('button', { name: /diagnostic/i }).click();
	const diagnosticsPanel = page.locator('#studio-diagnostics-panel');
	await expect(diagnosticsPanel).toBeVisible();
	expect(await diagnosticsPanel.evaluate((element) => {
		const bounds = element.getBoundingClientRect();
		const hit = document.elementFromPoint(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2);
		return Boolean(hit && element.contains(hit));
	})).toBe(true);
	await page.keyboard.press('Escape');
	await expect(diagnosticsPanel).toBeHidden();
	await page.getByRole('button', { name: /diagnostic/i }).click();
	await expect(diagnosticsPanel).toBeVisible();
	await page.getByRole('img', { name: 'Schemra', exact: true }).click();
	await expect(diagnosticsPanel).toBeHidden();
});

test('keeps project and file operations in their matching menus', async ({ page }) => {
	await page.goto('/');

	await page.getByRole('button', { name: /Select project, current project/ }).click();
	const projectMenu = page.getByRole('menu');
	await expect(projectMenu).toContainText(studioMessage('project.open'));
	await expect(projectMenu.getByRole('menuitem', { name: studioMessage('project.create') })).toBeVisible();
	await expect(projectMenu.getByRole('menuitem', { name: studioMessage('project.import') })).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(projectMenu).toBeHidden();

	await page.getByRole('button', { name: studioMessage('file.actions') }).click();
	const fileMenu = page.getByRole('menu');
	await expect(fileMenu.getByRole('menuitem', { name: studioMessage('file.openSource') })).toBeVisible();
	await expect(fileMenu.getByRole('menuitem', { name: studioMessage('file.saveSource') })).toBeEnabled();
	await expect(fileMenu.getByRole('menuitem', { name: studioMessage('file.exportSvg') })).toBeEnabled();
	await expect(fileMenu.getByRole('menuitem', { name: studioMessage('file.exportPng') })).toBeEnabled();
});

test('keeps header selectors and menus above shell overflow boundaries', async ({ page }) => {
	const menuItemIsPainted = (locator: import('@playwright/test').Locator) => locator.evaluate((element) => {
		const bounds = element.getBoundingClientRect();
		const hit = document.elementFromPoint(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2);
		return Boolean(hit && element.contains(hit));
	});

	await page.setViewportSize({ width: 780, height: 499 });
	await page.goto('/');
	await page.getByRole('button', { name: /Select project, current project/ }).click();
	const projectMenu = page.getByRole('menu').last();
	const createProject = projectMenu.getByRole('menuitem', { name: studioMessage('project.create') });
	await expect(createProject).toBeVisible();
	expect(await projectMenu.evaluate((element) => getComputedStyle(element.parentElement!).zIndex)).toBe('1000');
	await expect.poll(() => menuItemIsPainted(createProject)).toBe(true);
	await page.keyboard.press('Escape');

	const sidebar = page.getByRole('complementary', { name: studioMessage('sidebar.resources') });
	await sidebar.getByRole('separator', { name: studioMessage('sidebar.resize') }).dblclick();
	await sidebar.getByRole('button', { name: studioMessage('sidebar.visuals') }).click();
	const sidebarVisualMenu = page.getByRole('menu').last();
	const sidebarVisual = sidebarVisualMenu.getByRole('menuitem').first();
	await expect(sidebarVisual).toBeVisible();
	expect(await sidebarVisualMenu.evaluate((element) => getComputedStyle(element.parentElement!).zIndex)).toBe('1000');
	await expect.poll(() => menuItemIsPainted(sidebarVisual)).toBe(true);
	await page.keyboard.press('Escape');

	await page.setViewportSize({ width: 390, height: 360 });
	const browse = page.getByRole('button', { name: studioMessage('sidebar.mobileBrowse') });
	await browse.click();
	const visualMenu = page.getByRole('menu').last();
	const visualChoice = visualMenu.getByRole('menuitem').first();
	await expect(visualChoice).toBeVisible();
	expect(await visualMenu.evaluate((element) => getComputedStyle(element.parentElement!).zIndex)).toBe('1000');
	await expect.poll(() => menuItemIsPainted(visualChoice)).toBe(true);
	await page.keyboard.press('Escape');

	await page.getByRole('button', { name: studioMessage('file.actions') }).click();
	const fileMenu = page.getByRole('menu').last();
	const openSource = fileMenu.getByRole('menuitem', { name: studioMessage('file.openSource') });
	await expect(openSource).toBeVisible();
	await expect.poll(() => menuItemIsPainted(openSource)).toBe(true);
	await page.keyboard.press('Escape');

	await page.getByRole('button', { name: studioMessage('header.moreActions') }).click();
	const moreMenu = page.getByRole('menu').last();
	const settings = moreMenu.getByRole('menuitem', { name: studioMessage('header.settings') });
	await expect(settings).toBeVisible();
	await expect.poll(() => menuItemIsPainted(settings)).toBe(true);
});

test('navigates visuals and linked scenarios from the project sidebar', async ({ page }) => {
	await page.goto('/');

	const sidebar = page.getByRole('complementary', { name: studioMessage('sidebar.resources') });
	await expect(sidebar).toBeVisible();
	await expect(sidebar.getByRole('heading', { name: studioMessage('sidebar.visuals') })).toBeVisible();
	const visual = sidebar.getByRole('button', { name: 'Service traffic' });
	await visual.click({ button: 'right' });
	const contextMenu = page.getByRole('menu', { name: studioMessage('resource.contextActions') });
	const rename = contextMenu.getByRole('menuitem', { name: studioMessage('resource.rename') });
	await expect(rename).toBeVisible();
	await expect.poll(() => rename.evaluate((element) => {
		const bounds = element.getBoundingClientRect();
		const hit = document.elementFromPoint(bounds.left + bounds.width / 2, bounds.top + bounds.height / 2);
		return Boolean(hit && element.contains(hit));
	})).toBe(true);
	await page.keyboard.press('Escape');
	await expect(contextMenu).toBeHidden();
	await visual.click({ button: 'right' });
	await expect(contextMenu).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(contextMenu).toBeHidden();

	await visual.click();
	await expect(visual).toHaveAttribute('aria-expanded', 'true');
	const scenarioList = sidebar.locator('ul[id^="sidebar-scenarios-"]').first();
	await expect(scenarioList).toBeVisible();

	const scenario = scenarioList.getByRole('button').first();
	await scenario.click();
	await expect(scenario).toHaveAttribute('aria-current', 'page');

	await sidebar.getByRole('separator', { name: studioMessage('sidebar.resize') }).dblclick();
	await expect(sidebar.getByRole('separator', { name: studioMessage('sidebar.resize') })).toHaveAttribute('aria-valuenow', '56');
	await sidebar.getByRole('button', { name: studioMessage('sidebar.expand') }).click();
	await expect(sidebar.getByRole('separator', { name: studioMessage('sidebar.resize') })).toHaveAttribute('aria-valuenow', '240');
});

test('scenario playback advances and can be paused', async ({ page }) => {
	await page.goto('/');

	const timeline = page.getByRole('slider', { name: studioMessage('preview.scenarioTimeline') });
	await expect(timeline).toBeEnabled();
	await page.getByRole('button', { name: studioMessage('preview.play') }).click();
	await expect(page.getByRole('button', { name: studioMessage('preview.pause') })).toBeVisible();
	await expect.poll(async () => Number(await timeline.getAttribute('aria-valuenow'))).toBeGreaterThan(0);

	await page.getByRole('button', { name: studioMessage('preview.pause') }).click();
	await expect(page.getByRole('button', { name: studioMessage('preview.play') })).toBeVisible();
	const pausedTime = Number(await timeline.getAttribute('aria-valuenow'));
	const maxTime = Number(await timeline.getAttribute('aria-valuemax'));
	await timeline.focus();
	const nextTime = pausedTime >= maxTime ? pausedTime - 1 : pausedTime + 1;
	await page.keyboard.press(pausedTime >= maxTime ? 'ArrowLeft' : 'ArrowRight');
	await expect(timeline).toHaveAttribute('aria-valuenow', String(nextTime));
});

test('restores the caret and vertical scroll when returning to a source tab', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: studioMessage('file.actions') }).click();
	await expect(page.getByRole('menuitem', { name: studioMessage('file.saveSource') })).toBeEnabled();
	await page.keyboard.press('Escape');
	await expect(page.getByRole('textbox', { name: / editor$/ })).toBeVisible();
	const tabs = page.getByRole('tablist', { name: studioMessage('tabs.projectSourceFiles') }).getByRole('tab');
	await expect(tabs).toHaveCount(6);
	const firstTab = tabs.nth(0);
	const secondTab = tabs.nth(1);
	await firstTab.click();
	await expect(firstTab).toHaveAttribute('aria-selected', 'true');

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
	await expect(secondTab).toHaveAttribute('aria-selected', 'true');
	await expect(editor).not.toHaveValue(longSource);
	await firstTab.click();
	await expect(firstTab).toHaveAttribute('aria-selected', 'true');
	await expect(editor).toHaveValue(longSource);
	await expect.poll(() => editor.evaluate((element: HTMLTextAreaElement) => element.selectionStart)).toBe(savedView.selectionStart);
	await expect.poll(() => editor.evaluate((element: HTMLTextAreaElement) => element.parentElement?.parentElement?.scrollTop ?? 0)).toBe(savedView.scrollTop);
});

test('flushes edits when hidden and warns before leaving unsaved source changes', async ({ page }) => {
	await page.goto('/');
	await page.getByRole('button', { name: studioMessage('file.actions') }).click();
	await expect(page.getByRole('menuitem', { name: studioMessage('file.saveSource') })).toBeEnabled();
	await page.keyboard.press('Escape');

	const editor = page.getByRole('textbox', { name: / editor$/ });
	const updatedSource = `${await editor.inputValue()}\n// lifecycle persistence`;
	await editor.fill(updatedSource);
	await page.evaluate(() => {
		Object.defineProperty(document, 'visibilityState', { configurable: true, value: 'hidden' });
		document.dispatchEvent(new Event('visibilitychange'));
	});
	await expect.poll(() => editor.inputValue()).toBe(updatedSource);

	const beforeUnload = page.waitForEvent('dialog');
	const reload = page.reload();
	const dialog = await beforeUnload;
	expect(dialog.type()).toBe('beforeunload');
	await dialog.accept();
	await reload;

	await expect(editor).toHaveValue(updatedSource);
});
