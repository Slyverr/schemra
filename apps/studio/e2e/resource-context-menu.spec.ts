import { expect, test } from '@playwright/test';
import { studioMessage } from '../src/lib/i18n/messages.js';

test('resource actions support pointer and keyboard context menus', async ({ page }) => {
	await page.goto('/');

	const selectedTab = page.getByRole('tab', { selected: true });
	const menu = page.getByRole('menu', { name: studioMessage('resource.contextActions') });

	await selectedTab.click({ button: 'right' });
	await expect(menu).toBeVisible();
	await expect(menu.getByRole('menuitem', { name: studioMessage('resource.rename') })).toBeVisible();
	await expect(menu.getByRole('menuitem', { name: studioMessage('resource.removeFromProject') })).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(menu).toBeHidden();
	await expect(selectedTab).toBeFocused();

	await selectedTab.press('Shift+F10');
	await expect(menu).toBeVisible();
	await page.keyboard.press('Escape');
	await expect(menu).toBeHidden();
	await expect(selectedTab).toBeFocused();
});
