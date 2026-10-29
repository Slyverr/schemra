import { expect, test } from '@playwright/test';

test('keeps the caret visible while navigating long source lines', async ({ page }) => {
	await page.setViewportSize({ width: 800, height: 480 });
	await page.goto('/');

	const editor = page.locator('.editor-input');
	const viewport = page.locator('.editor-viewport');
	const longLine = `  // ${'wide '.repeat(80)}`;
	const source = `visual app {\n${longLine}\n${'  // filler\n'.repeat(40)}}`;
	await editor.fill(source);
	await editor.press('Control+Home');
	await editor.press('ArrowDown');
	await editor.press('End');

	await expect.poll(() => viewport.evaluate((element) => element.scrollLeft)).toBeGreaterThan(0);
	await expect.poll(() => viewport.evaluate((element) => {
		const input = element.querySelector('textarea');
		if (!input) return false;
		const offset = input.selectionDirection === 'backward' ? input.selectionStart : input.selectionEnd;
		const beforeCaret = input.value.slice(0, offset);
		const lineIndex = beforeCaret.split('\n').length - 1;
		const column = beforeCaret.length - beforeCaret.lastIndexOf('\n') - 1;
		const line = element.querySelector<HTMLElement>(`[data-line-index="${lineIndex}"]`);
		if (!line) return false;
		const walker = document.createTreeWalker(line, NodeFilter.SHOW_TEXT, {
			acceptNode(node) {
				return node.parentElement?.closest('.inline-diagnostic')
					? NodeFilter.FILTER_REJECT
					: NodeFilter.FILTER_ACCEPT;
			}
		});
		let remaining = column;
		let textNode: Text | null;
		while ((textNode = walker.nextNode() as Text | null)) {
			if (remaining <= textNode.length) {
				const range = document.createRange();
				const start = remaining === 0 ? 0 : remaining - 1;
				range.setStart(textNode, start);
				range.setEnd(textNode, remaining === 0 ? Math.min(1, textNode.length) : remaining);
				const caretX = range.getBoundingClientRect().right;
				const bounds = element.getBoundingClientRect();
				const visibleLeft = bounds.left + element.clientLeft;
				const visibleRight = visibleLeft + element.clientWidth;
				return caretX >= visibleLeft - 1 && caretX <= visibleRight + 1;
			}
			remaining -= textNode.length;
		}
		return false;
	})).toBe(true);
	await editor.press('Control+End');
	await expect.poll(() => viewport.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
	await expect.poll(() => viewport.evaluate((element) => {
		const input = element.querySelector('textarea');
		if (!input) return false;
		const offset = input.selectionDirection === 'backward' ? input.selectionStart : input.selectionEnd;
		const lineIndex = input.value.slice(0, offset).split('\n').length - 1;
		const line = element.querySelector<HTMLElement>(`[data-line-index="${lineIndex}"]`);
		if (!line) return false;
		const viewportRect = element.getBoundingClientRect();
		const lineRect = line.getBoundingClientRect();
		const top = viewportRect.top + element.clientTop;
		const bottom = top + element.clientHeight;
		return lineRect.top >= top - 1 && lineRect.bottom <= bottom + 1;
	})).toBe(true);
});
