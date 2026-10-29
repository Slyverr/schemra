/** Convert viewport rectangles into the content coordinates used by scope guides. */
export function getScopeGuideGeometry(
	surfaceRect: Pick<DOMRect, 'left' | 'top'>,
	openLineRect: Pick<DOMRect, 'left' | 'bottom'>,
	lastBodyLineRect: Pick<DOMRect, 'bottom'>,
	firstGlyphX: number
) {
	const top = openLineRect.bottom - surfaceRect.top;
	const bottom = lastBodyLineRect.bottom - surfaceRect.top;
	return {
		left: openLineRect.left - surfaceRect.left + firstGlyphX,
		top,
		height: Math.max(0, bottom - top)
	};
}

/** Measure the ink edge of a source glyph within its rendered line. */
export function measureSourceGlyphInkStartX(
	line: HTMLElement,
	sourceColumn: number,
	context: CanvasRenderingContext2D
): number | null {
	const position = locateSourceTextPosition(line, sourceColumn);
	if (!position) return null;
	const offsets = getUnicodeCharacterRange(position.node.data, position.offset);
	if (!offsets) return null;

	const range = line.ownerDocument.createRange();
	range.setStart(position.node, offsets.start);
	range.setEnd(position.node, offsets.end);
	const glyphRect = range.getBoundingClientRect();
	if (glyphRect.width <= 0) return null;

	const view = line.ownerDocument.defaultView;
	const parent = position.node.parentElement;
	if (!view || !parent) return glyphRect.left - line.getBoundingClientRect().left;

	const style = view.getComputedStyle(parent);
	const previousFont = context.font;
	const previousTextAlign = context.textAlign;
	const previousDirection = context.direction;
	context.font = `${style.fontStyle || 'normal'} ${style.fontWeight || 'normal'} ${style.fontSize} ${style.fontFamily}`;

	context.textAlign = 'left';
	context.direction = style.direction === 'rtl' ? 'rtl' : 'ltr';
	const glyph = position.node.data.slice(offsets.start, offsets.end);
	const inkLeft = context.measureText(glyph).actualBoundingBoxLeft;
	context.font = previousFont;
	context.textAlign = previousTextAlign;
	context.direction = previousDirection;
	return Number.isFinite(inkLeft)
		? glyphRect.left - line.getBoundingClientRect().left - inkLeft
		: glyphRect.left - line.getBoundingClientRect().left;
}

function locateSourceTextPosition(line: HTMLElement, sourceColumn: number): { node: Text; offset: number } | null {
	const view = line.ownerDocument.defaultView;
	const showText = view?.NodeFilter.SHOW_TEXT ?? 4;
	const walker = line.ownerDocument.createTreeWalker(line, showText, {
		acceptNode(node) {
			return node.parentElement?.closest('.inline-diagnostic') ? 2 : 1;
		}
	});
	let offset = Math.max(0, sourceColumn);
	let node: Text | null;
	while ((node = walker.nextNode() as Text | null)) {
		if (offset < node.length) return { node, offset };
		offset -= node.length;
	}
	return null;
}

function getUnicodeCharacterRange(text: string, offset: number): { start: number; end: number } | null {
	if (!text.length) return null;
	const index = Math.max(0, Math.min(text.length - 1, offset));
	const current = text.charCodeAt(index);
	if (current >= 0xdc00 && current <= 0xdfff && index > 0) {
		const previous = text.charCodeAt(index - 1);
		if (previous >= 0xd800 && previous <= 0xdbff) return { start: index - 1, end: index + 1 };
	}
	if (current >= 0xd800 && current <= 0xdbff && index + 1 < text.length) {
		const next = text.charCodeAt(index + 1);
		if (next >= 0xdc00 && next <= 0xdfff) return { start: index, end: index + 2 };
	}
	return { start: index, end: index + 1 };
}
