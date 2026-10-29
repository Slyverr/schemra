export interface CaretVisibilityBounds {
	caretLeft: number;
	lineTop: number;
	lineBottom: number;
	viewportLeft: number;
	viewportTop: number;
	viewportRight: number;
	viewportBottom: number;
}

export interface CaretScrollAdjustment {
	left: number;
	top: number;
}

/** Return the scroll delta needed to keep a caret line inside the visible viewport. */
export function getCaretScrollAdjustment(
	bounds: CaretVisibilityBounds,
	lineHeight: number,
	horizontalPadding = 8
): CaretScrollAdjustment {
	const values = [
		bounds.caretLeft,
		bounds.lineTop,
		bounds.lineBottom,
		bounds.viewportLeft,
		bounds.viewportTop,
		bounds.viewportRight,
		bounds.viewportBottom,
		lineHeight,
		horizontalPadding
	];
	if (
		!values.every(Number.isFinite) ||
		lineHeight <= 0 ||
		bounds.viewportRight < bounds.viewportLeft ||
		bounds.viewportBottom < bounds.viewportTop ||
		bounds.lineBottom < bounds.lineTop
	) return { left: 0, top: 0 };

	const viewportWidth = bounds.viewportRight - bounds.viewportLeft;
	const viewportHeight = bounds.viewportBottom - bounds.viewportTop;
	const visibleLeft = bounds.viewportLeft + Math.min(12, viewportWidth / 2);
	const visibleRight = bounds.viewportRight - Math.min(Math.max(0, horizontalPadding), viewportWidth / 2);
	let left = 0;
	if (bounds.caretLeft < visibleLeft) left = bounds.caretLeft - visibleLeft;
	else if (bounds.caretLeft > visibleRight) left = bounds.caretLeft - visibleRight;

	const verticalPadding = Math.min(lineHeight, Math.max(0, (viewportHeight - lineHeight) / 2));
	let top = 0;
	if (bounds.lineTop < bounds.viewportTop + verticalPadding) {
		top = bounds.lineTop - bounds.viewportTop - verticalPadding;
	} else if (bounds.lineBottom > bounds.viewportBottom - verticalPadding) {
		top = bounds.lineBottom - bounds.viewportBottom + verticalPadding;
	}

	return { left, top };
}
