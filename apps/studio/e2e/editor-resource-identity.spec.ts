import { expect, test } from '@playwright/test';
import { studioMessage } from '../src/lib/i18n/messages.js';

test('keeps resource grouping and syntax highlighting stable during invalid edits', async ({ page }) => {
	await page.goto('/');
	const editor = page.getByRole('textbox', { name: /editor$/ });
	const source = await editor.inputValue();
	const invalidSource = source.replace(/^visual\b/m, 'unknown');
	const tabList = page.getByRole('tablist', { name: studioMessage('tabs.projectSourceFiles') });
	const visualTab = tabList.getByRole('tab').first();
	const visualTabId = await visualTab.getAttribute('id');

	await expect(tabList.getByRole('tab')).toHaveCount(6);
	await expect(visualTab).toHaveAttribute('title', /visuals\/traffic-overview\.schemra/);
	await editor.fill(invalidSource);

	await expect(tabList.getByRole('tab')).toHaveCount(6);
	await expect(tabList.getByRole('tab').first()).toHaveAttribute('id', visualTabId!);
	await expect(page.getByRole('heading', { name: studioMessage('sidebar.unlinkedSources') })).toHaveCount(0);
	await expect(page.locator('.editor-line[data-line-index="3"] .syntax-keyword').first()).toHaveText('group');
});
