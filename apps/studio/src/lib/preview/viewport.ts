import type { PreviewScene } from './model';

export interface PreviewViewportSize {
	width: number;
	height: number;
	centerOffsetY?: number;
	canvasHeight?: number;
}

export interface PreviewViewportInsets {
	top: number;
	bottom: number;
}

/** Derive the portion of the canvas left clear by controls overlaid at its edges. */
export function insetPreviewViewport(
	viewport: PreviewViewportSize,
	insets: PreviewViewportInsets
): PreviewViewportSize {
	if (!validSize(viewport)) return { width: 0, height: 0, centerOffsetY: 0, canvasHeight: 0 };

	const top = clamp(finiteInset(insets.top), 0, Math.max(0, viewport.height - 1));
	const bottom = clamp(finiteInset(insets.bottom), 0, Math.max(0, viewport.height - top - 1));
	return {
		width: viewport.width,
		height: Math.max(1, viewport.height - top - bottom),
		centerOffsetY: (top - bottom) / 2,
		canvasHeight: viewport.height
	};
}

export interface PreviewNodeBounds {
	x: number;
	y: number;
	width: number;
	height: number;
}

export interface PreviewPan {
	x: number;
	y: number;
}

export interface PreviewTransform extends PreviewPan {
	zoomPercent: number;
}

export interface PreviewGridTransform {
	x: number;
	y: number;
	scale: number;
}

export const MIN_PREVIEW_ZOOM = 50;
export const MAX_PREVIEW_ZOOM = 2000;

const MAX_FOCUS_ZOOM = 300;
const FOCUS_PADDING = 48;
const VISIBLE_PAN_MARGIN = 64;

export function clampPreviewZoom(value: number): number {
	return clamp(value, MIN_PREVIEW_ZOOM, MAX_PREVIEW_ZOOM);
}

/** Keep the infinite canvas grid aligned with the same camera transform as the scene. */
export function previewGridTransform(
	viewport: PreviewViewportSize,
	pan: PreviewPan,
	zoomPercent: number
): PreviewGridTransform {
	if (!validSize(viewport) || ![pan.x, pan.y, zoomPercent].every(Number.isFinite) || zoomPercent <= 0) {
		return { x: 0, y: 0, scale: 1 };
	}

	const scale = zoomPercent / 100;
	return {
		x: viewport.width * (1 - scale) / 2 + pan.x,
		y: viewport.height * (1 - scale) / 2 + pan.y,
		scale
	};
}

/** Keep the pointer's diagram position fixed while zoom changes around it. */
export function zoomPanAtPoint(
	pan: PreviewPan,
	currentZoom: number,
	nextZoom: number,
	anchor: PreviewPan,
	viewport: PreviewViewportSize
): PreviewPan {
	if (!validSize(viewport) || ![pan.x, pan.y, currentZoom, nextZoom, anchor.x, anchor.y].every(Number.isFinite) || currentZoom <= 0) {
		return pan;
	}
	const scaleRatio = nextZoom / currentZoom;
	return {
		x: pan.x + (anchor.x - viewport.width / 2) * (1 - scaleRatio),
		y: pan.y + (anchor.y - (viewport.canvasHeight ?? viewport.height) / 2 - (viewport.centerOffsetY ?? 0)) * (1 - scaleRatio)
	};
}

/** Fit and center a scene target, matching the preview's Find behavior. */
export function calculateSceneFocusTransform(
	viewport: PreviewViewportSize,
	scene: PreviewViewportSize,
	target: PreviewNodeBounds
): PreviewTransform | null {
	if (!validSize(viewport) || !validSize(scene) || !validBounds(target)) return null;

	const fitScale = Math.min(viewport.width / scene.width, viewport.height / scene.height);
	const availableWidth = Math.max(1, viewport.width - FOCUS_PADDING * 2);
	const availableHeight = Math.max(1, viewport.height - FOCUS_PADDING * 2);
	const targetZoom = Math.min(
		availableWidth / (target.width * fitScale),
		availableHeight / (target.height * fitScale)
	) * 100;
	const zoomPercent = clamp(Math.round(targetZoom), MIN_PREVIEW_ZOOM, MAX_FOCUS_ZOOM);
	const scale = fitScale * (zoomPercent / 100);

	return {
		zoomPercent,
		x: -((target.x + target.width / 2) - scene.width / 2) * scale,
		y: -((target.y + target.height / 2) - scene.height / 2) * scale + (viewport.centerOffsetY ?? 0)
	};
}

/** Keep enough of an enlarged scene reachable while preventing it from drifting away. */
export function clampScenePan(
	pan: PreviewPan,
	viewport: PreviewViewportSize,
	scene: PreviewViewportSize,
	zoomPercent: number
): PreviewPan {
	if (!validSize(viewport) || !validSize(scene) || !Number.isFinite(zoomPercent) || zoomPercent <= 0) return { x: 0, y: 0 };

	const fitScale = Math.min(viewport.width / scene.width, viewport.height / scene.height);
	const scale = fitScale * (zoomPercent / 100);
	const maxX = Math.max(0, (viewport.width + scene.width * scale) / 2 - VISIBLE_PAN_MARGIN);
	const maxY = Math.max(0, (viewport.height + scene.height * scale) / 2 - VISIBLE_PAN_MARGIN);
	const centerOffsetY = viewport.centerOffsetY ?? 0;
	return {
		x: clamp(pan.x, -maxX, maxX),
		y: clamp(pan.y, centerOffsetY - maxY, centerOffsetY + maxY)
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
	return Number.isFinite(value.width)
		&& Number.isFinite(value.height)
		&& value.width > 0
		&& value.height > 0
		&& (value.centerOffsetY === undefined || Number.isFinite(value.centerOffsetY))
		&& (value.canvasHeight === undefined || (Number.isFinite(value.canvasHeight) && value.canvasHeight > 0));
}

function finiteInset(value: number): number {
	return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function validBounds(value: PreviewNodeBounds): boolean {
	return [value.x, value.y, value.width, value.height].every(Number.isFinite) && value.width > 0 && value.height > 0;
}

function clamp(value: number, minimum: number, maximum: number): number {
	return Math.max(minimum, Math.min(maximum, value));
}
