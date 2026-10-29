import type { PreviewScene } from './model';

export interface PreviewViewportSize {
	width: number;
	height: number;
}

export interface PreviewNodeBounds {
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface PreviewScrollTarget {
	left: number;
	top: number;
}

/** Choose a zoom percentage that enlarges a node while keeping it inside the viewport. */
export function calculateNodeFocusZoom(
	scene: PreviewViewportSize,
	node: PreviewNodeBounds,
	viewport: PreviewViewportSize,
	margin = 24,
	minZoom = 50,
	maxZoom = 500
): number | null {
	if (!validSize(scene) || !validSize(viewport) || !validBounds(node) || !Number.isFinite(margin) || margin < 0) {
		return null;
	}
	if (!Number.isFinite(minZoom) || !Number.isFinite(maxZoom) || minZoom <= 0 || maxZoom < minZoom) return null;

	const fitScale = Math.min(viewport.width / scene.width, viewport.height / scene.height);
	const focusWidth = Math.max(1, viewport.width - margin * 2);
	const focusHeight = Math.max(1, viewport.height - margin * 2);
	const focusScale = Math.min(focusWidth / node.width, focusHeight / node.height);
	return Math.round(clamp((focusScale / fitScale) * 100, minZoom, maxZoom));
}

/** Center a node in the scrollable canvas and clamp to its available scroll range. */
export function calculateNodeScrollTarget(
	scene: PreviewViewportSize,
	node: PreviewNodeBounds,
	sheet: PreviewViewportSize,
	sheetOrigin: { x: number; y: number },
	viewport: PreviewViewportSize,
	maxScroll: { x: number; y: number }
): PreviewScrollTarget | null {
	if (
		!validSize(scene) || !validSize(sheet) || !validSize(viewport) || !validBounds(node) ||
		![sheetOrigin.x, sheetOrigin.y, maxScroll.x, maxScroll.y].every(Number.isFinite) ||
		maxScroll.x < 0 || maxScroll.y < 0
	) return null;

	const centerX = sheetOrigin.x + ((node.x + node.width / 2) / scene.width) * sheet.width;
	const centerY = sheetOrigin.y + ((node.y + node.height / 2) / scene.height) * sheet.height;
	return {
		left: clamp(centerX - viewport.width / 2, 0, maxScroll.x),
		top: clamp(centerY - viewport.height / 2, 0, maxScroll.y)
	};
}

/** Return a viewport rectangle containing a connection route and its endpoint nodes. */
export function getPreviewConnectionBounds(
	scene: Pick<PreviewScene, 'nodes' | 'edges'>,
	edgeIndex: number
): PreviewNodeBounds | null {
	if (!Number.isInteger(edgeIndex) || edgeIndex < 0 || edgeIndex >= scene.edges.length) return null;
	const edge = scene.edges[edgeIndex];
	if (!edge) return null;
	const source = scene.nodes.find(({ id }) => id === edge.from)?.bounds;
	const target = scene.nodes.find(({ id }) => id === edge.to)?.bounds;
	const points = edge.path.flatMap(({ start, control1, control2, end }) => [start, control1, control2, end]);
	const xValues = points.flatMap(({ x }) => Number.isFinite(x) ? [x] : []);
	const yValues = points.flatMap(({ y }) => Number.isFinite(y) ? [y] : []);
	for (const bounds of [source, target]) {
		if (!bounds || !validBounds(bounds)) continue;
		xValues.push(bounds.x, bounds.x + bounds.width);
		yValues.push(bounds.y, bounds.y + bounds.height);
	}
	if (xValues.length === 0 || yValues.length === 0) return null;
	const minX = Math.min(...xValues);
	const minY = Math.min(...yValues);
	const maxX = Math.max(...xValues);
	const maxY = Math.max(...yValues);
	if (![minX, minY, maxX, maxY].every(Number.isFinite)) return null;
	return { x: minX, y: minY, width: Math.max(1, maxX - minX), height: Math.max(1, maxY - minY) };
}

function validSize(value: PreviewViewportSize): boolean {
	return Number.isFinite(value.width) && Number.isFinite(value.height) && value.width > 0 && value.height > 0;
}

function validBounds(value: PreviewNodeBounds): boolean {
	return [value.x, value.y, value.width, value.height].every(Number.isFinite) && value.width > 0 && value.height > 0;
}

function clamp(value: number, minimum: number, maximum: number): number {
	return Math.max(minimum, Math.min(maximum, value));
}
