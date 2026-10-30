import { expect, test } from '@playwright/test';

test('opens completion suggestions at the caret with their start aligned', async ({ page }) => {
	await page.goto('/');
	const editor = page.locator('.editor-input');
	await expect(editor).toBeEnabled();
	await editor.fill('vis');

	const suggestions = page.locator('.editor-completions');
	await expect(suggestions).toBeVisible();
	const alignment = await page.evaluate(() => {
		const input = document.querySelector<HTMLTextAreaElement>('.editor-input');
		const line = document.querySelector<HTMLElement>('.editor-line[data-line-index="0"]');
		const popup = document.querySelector<HTMLElement>('.editor-completions');
		if (!input || !line || !popup) return null;

		let remaining = input.selectionStart;
		const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT);
		let node: Text | null;
		while ((node = walker.nextNode() as Text | null)) {
			if (remaining <= node.length) {
				const range = document.createRange();
				range.setStart(node, 0);
				range.setEnd(node, remaining);
				return { caret: range.getBoundingClientRect().right, popup: popup.getBoundingClientRect().left };
			}
			remaining -= node.length;
		}
		return null;
	});

	expect(alignment).not.toBeNull();
	expect(Math.abs(alignment!.popup - alignment!.caret)).toBeLessThan(1.5);
});

test('Shift+Enter inserts a newline and closes suggestions instead of accepting one', async ({ page }) => {
	await page.goto('/');
	const editor = page.locator('.editor-input');
	await expect(editor).toBeEnabled();
	await editor.fill('vis');

	const suggestions = page.locator('.editor-completions');
	await expect(suggestions).toBeVisible();
	await editor.press('Shift+Enter');

	await expect(editor).toHaveValue('vis\n');
	await expect(suggestions).toHaveCount(0);
});
