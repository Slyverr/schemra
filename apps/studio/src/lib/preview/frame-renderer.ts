import type { ScenarioPlayback } from '../schemra';
import { studioMessage } from '../i18n/messages.js';
import {
	previewTimeAtElapsed,
	readPreviewFrame,
	routePathData,
	isSafeSvgColor,
	type PreviewFrame,
	type PreviewScenario,
	type PreviewScene
} from './model';
import { bidirectionalTrace, mergeNearbyTracePulses, type TracePulseState } from './trace-pulses';

export type AnimationFrameRenderer = ((timeMs: number, background: string) => string) & {
	dispose: () => void;
};

interface FrameRenderCache {
	progressPaths: Map<number, SVGPathElement>;
	markerGroups: Map<number, SVGGElement>;
	highlightGroups: Map<string, SVGGraphicsElement[]>;
}

interface TraceHighlightSource {
	highlight: PreviewFrame['highlights'][number];
	shape: SVGGraphicsElement;
}

const frameRenderCaches = new WeakMap<SVGSVGElement, FrameRenderCache>();
const SVG_NS = 'http://www.w3.org/2000/svg';
const TOKEN_ICONS: Record<string, string[]> = {
	mail: ['M-8 -5H8V6H-8Z', 'M-8 -4L0 2L8 -4'],
	message: ['M-7 -7H7A3 3 0 0 1 10 -4V2A3 3 0 0 1 7 5H-2L-7 9V5A3 3 0 0 1 -10 2V-4A3 3 0 0 1 -7 -7Z'],
	user: ['M0 -8A3.5 3.5 0 1 0 0 -1A3.5 3.5 0 0 0 0 -8Z', 'M-8 8A8 5 0 0 1 8 8'],
	package: ['M-8 -5L0 -9L8 -5V5L0 9L-8 5Z', 'M0 -9V0M-8 -5L0 0L8 -5M0 0V9'],
	file: ['M-6 -9H2L7 -4V9H-6Z', 'M2 -9V-4H7M-3 1H4M-3 5H4'],
	server: ['M-8 -7H8V-1H-8ZM-8 2H8V8H-8Z'],
	database: ['M-7 -6C-7 -9 7 -9 7 -6V6C7 9 -7 9 -7 6Z', 'M-7 -6C-7 -3 7 -3 7 -6M-7 0C-7 3 7 3 7 0']
};

export function applyPreviewFrame(
	root: SVGSVGElement,
	frame: PreviewFrame | null,
	scene: PreviewScene | null,
	scenario: PreviewScenario | null
) {
	const cache = frameRenderCacheFor(root);
	const { progressPaths, markerGroups, highlightGroups } = cache;
	const linkLayer = ensureLayer(root, 'schemra-preview-links');
	const markerLayer = ensureLayer(root, 'schemra-preview-markers');
	const portMarkerLayer = ensureLayer(root, 'schemra-preview-port-markers', true);
	const highlightLayer = ensureLayer(root, 'schemra-preview-highlights', true);
	const activeEdges = new Set<number>();
	const activeTokens = new Set<number>();
	const activeHighlights = new Set<string>();

	for (const edge of frame?.edges ?? []) {
		const path = scene?.edges[edge.edgeIndex]?.path;
		const data = path ? routePathData(path, edge.forward) : '';
		if (!data || edge.progress <= 0) continue;
		activeEdges.add(edge.instanceId);
		let progressPath = progressPaths.get(edge.instanceId);
		if (!progressPath) {
			progressPath = root.ownerDocument.createElementNS(SVG_NS, 'path');
			progressPath.setAttribute('fill', 'none');
			progressPath.setAttribute('stroke', 'var(--primary)');
			progressPath.setAttribute('stroke-width', '3');
			progressPath.setAttribute('stroke-linecap', 'round');
			progressPath.setAttribute('stroke-linejoin', 'round');
			progressPath.setAttribute('pathLength', '1');
			progressPath.setAttribute('aria-hidden', 'true');
			progressPaths.set(edge.instanceId, progressPath);
			linkLayer.append(progressPath);
		}
		progressPath.setAttribute('d', data);
		progressPath.setAttribute('stroke-dasharray', `${edge.progress} 1`);
	}

	const tracePulses: TracePulseState[] = [];
	const traceSources = new Map<string, TraceHighlightSource>();
	for (const highlight of frame?.highlights ?? []) {
		const node = [...root.querySelectorAll<SVGGElement>(':scope > g[data-node-id]')]
			.find((candidate) => candidate.getAttribute('data-node-id') === highlight.nodeId);
		const shape = node?.querySelector<SVGGraphicsElement>('[data-node-background="true"]');
		if (!shape) continue;
		if (highlight.effect === 'trace') {
			const id = String(highlight.instanceId);
			const originKey = highlight.origin
				? `${highlight.nodeId}:${highlight.origin.x.toFixed(2)},${highlight.origin.y.toFixed(2)}`
				: `${highlight.nodeId}:default`;
			tracePulses.push({ id, originKey, progress: highlight.progress, pulseOpacity: highlight.opacity });
			traceSources.set(id, { highlight, shape });
		} else {
			applyHighlightOverlay(
				highlightLayer,
				highlightGroups,
				activeHighlights,
				`pulse:${highlight.instanceId}`,
				highlight,
				shape,
				1
			);
		}
	}

	for (const pulse of mergeNearbyTracePulses(tracePulses)) {
		const sourceId = pulse.sourceIds[0];
		const source = sourceId ? traceSources.get(sourceId) : undefined;
		if (!source) continue;
		applyHighlightOverlay(
			highlightLayer,
			highlightGroups,
			activeHighlights,
			`trace:${pulse.id}`,
			{ ...source.highlight, progress: pulse.progress, opacity: pulse.pulseOpacity },
			source.shape,
			pulse.count
		);
	}

	const artworkByToken = new Map((scenario?.tokens ?? []).map(({ token, icon, char }) => [
		token,
		{ icon: icon ?? '', char: char ?? '' }
	]));
	for (const token of frame?.tokens ?? []) {
		activeTokens.add(token.instanceId);
		const activeEdge = frame?.edges.find((edge) => edge.instanceId === token.instanceId);
		const markerLayerForToken = shouldShowDepartureMarkerAboveNode(token, activeEdge, scene)
			? portMarkerLayer
			: markerLayer;
		const artwork = artworkByToken.get(token.token) ?? { icon: '', char: '' };
		const artworkKey = JSON.stringify([token.token, artwork.icon, artwork.char]);
		let marker = markerGroups.get(token.instanceId);
		if (!marker) {
			marker = root.ownerDocument.createElementNS(SVG_NS, 'g');
			marker.setAttribute('aria-label', token.token);
			marker.setAttribute('data-token-instance', String(token.instanceId));
			const circle = root.ownerDocument.createElementNS(SVG_NS, 'circle');
			circle.setAttribute('r', '11');
			circle.setAttribute('fill', 'var(--primary)');
			circle.setAttribute('stroke', 'var(--background)');
			circle.setAttribute('stroke-width', '2');
			marker.append(circle);
			appendMarkerArtwork(marker, artwork.icon, artwork.char, token.token);
			markerGroups.set(token.instanceId, marker);
			marker.setAttribute('data-token-artwork', artworkKey);
		} else {
			if (marker.getAttribute('data-token-artwork') !== artworkKey) {
				const circle = marker.firstElementChild;
				if (circle) marker.replaceChildren(circle);
				appendMarkerArtwork(marker, artwork.icon, artwork.char, token.token);
				marker.setAttribute('data-token-artwork', artworkKey);
			}
		}
		if (marker.parentNode !== markerLayerForToken) markerLayerForToken.append(marker);
		marker.setAttribute('transform', `translate(${token.x} ${token.y})`);
	}

	removeInactive(progressPaths, activeEdges);
	removeInactive(markerGroups, activeTokens);
	removeInactive(highlightGroups, activeHighlights);
}

function applyHighlightOverlay(
	highlightLayer: SVGGElement,
	highlightGroups: Map<string, SVGGraphicsElement[]>,
	activeHighlights: Set<string>,
	key: string,
	highlight: PreviewFrame['highlights'][number],
	shape: SVGGraphicsElement,
	pulseCount: number
) {
	activeHighlights.add(key);
	const geometry = shape as SVGGeometryElement;
	const length = typeof geometry.getTotalLength === 'function' ? geometry.getTotalLength() : 0;
	const originLength = highlight.origin && length > 0
		? nearestPathLength(geometry, length, highlight.origin.x, highlight.origin.y)
		: 0;
	let outlines = highlightGroups.get(key);
	const outlineCount = highlight.effect === 'trace' ? 2 : 1;
	if (!outlines || outlines.length !== outlineCount) {
		for (const outline of outlines ?? []) outline.remove();
		outlines = Array.from({ length: outlineCount }, () => {
			const outline = shape.cloneNode(false) as SVGGraphicsElement;
			outline.removeAttribute('data-node-background');
			outline.removeAttribute('filter');
			outline.setAttribute('fill', 'none');
			outline.setAttribute('stroke', 'var(--primary)');
			outline.setAttribute('stroke-width', '2.5');
			outline.setAttribute('stroke-linecap', 'round');
			outline.setAttribute('stroke-linejoin', 'round');
			outline.setAttribute('pointer-events', 'none');
			outline.setAttribute('aria-hidden', 'true');
			highlightLayer.append(outline);
			return outline;
		});
		highlightGroups.set(key, outlines);
	}

	for (let index = 0; index < outlines.length; index += 1) {
		const outline = outlines[index]!;
		outline.setAttribute(
			'stroke-width',
			String(highlight.effect === 'trace' ? 2.5 + Math.min(pulseCount - 1, 3) * 0.75 : 2.5)
		);
		outline.setAttribute('opacity', String(highlight.opacity));
		if (highlight.effect !== 'trace') continue;

		const trace = bidirectionalTrace(length, originLength, highlight.progress);
		if (trace.dashArray === null) {
			outline.removeAttribute('stroke-dasharray');
			outline.removeAttribute('stroke-dashoffset');
			continue;
		}
		outline.setAttribute('stroke-dasharray', trace.dashArray);
		outline.setAttribute('stroke-dashoffset', String(index === 0 ? trace.forwardOffset : trace.backwardOffset));
	}
}

function shouldShowDepartureMarkerAboveNode(
	token: PreviewFrame['tokens'][number],
	edge: PreviewFrame['edges'][number] | undefined,
	scene: PreviewScene | null
) {
	if (!edge || !scene) return false;
	const connection = scene.edges[edge.edgeIndex];
	if (!connection) return false;
	const sourceId = edge.forward ? connection.from : connection.to;
	const source = scene.nodes.find((node) => node.id === sourceId);
	if (!source) return false;

	const dx = Math.max(source.bounds.x - token.x, 0, token.x - (source.bounds.x + source.bounds.width));
	const dy = Math.max(source.bounds.y - token.y, 0, token.y - (source.bounds.y + source.bounds.height));
	return Math.hypot(dx, dy) <= 11;
}

export function createAnimationFrameRenderer(
	sourceSvg: string,
	scene: PreviewScene,
	scenario: PreviewScenario,
	playback: ScenarioPlayback,
	styleHost: Element
): AnimationFrameRenderer {
	const parsed = new DOMParser().parseFromString(sourceSvg, 'image/svg+xml');
	if (parsed.querySelector('parsererror')) throw new Error(studioMessage('preview.animationInvalidDiagram'));
	const root = parsed.documentElement as unknown as SVGSVGElement;
	let backgroundRect = [...root.children].find((element) =>
		element.localName === 'rect' &&
		Number(element.getAttribute('width')) === scene.width &&
		Number(element.getAttribute('height')) === scene.height
	) as SVGRectElement | undefined;
	if (!backgroundRect) backgroundRect = ensureCanvasBackground(root, scene);
	const colors = getComputedStyle(styleHost);
	const primary = colors.getPropertyValue('--primary').trim() || '#005bd3';
	const primaryForeground = colors.getPropertyValue('--primary-foreground').trim() || '#ffffff';
	const serializer = new XMLSerializer();
	const renderHost = document.createElement('div');
	renderHost.setAttribute('aria-hidden', 'true');
	Object.assign(renderHost.style, {
		position: 'fixed',
		left: '0',
		top: '0',
		width: `${scene.width}px`,
		height: `${scene.height}px`,
		opacity: '0',
		pointerEvents: 'none',
		zIndex: '-1'
	});
	renderHost.append(root);
	document.body.append(renderHost);

	const renderFrame = ((timeMs: number, background: string) => {
		const time = previewTimeAtElapsed(timeMs, playback.durationMs);
		applyPreviewFrame(root, readPreviewFrame(playback.sample(time)), scene, scenario);
		backgroundRect.setAttribute('fill', background);
		return serializer.serializeToString(root)
			.replaceAll('var(--primary-foreground)', primaryForeground)
			.replaceAll('var(--primary)', primary)
			.replaceAll('var(--background)', background);
	}) as AnimationFrameRenderer;
	renderFrame.dispose = () => renderHost.remove();
	return renderFrame;
}

export function ensureCanvasBackground(
	root: SVGSVGElement,
	scene: Pick<PreviewScene, 'width' | 'height'>
): SVGRectElement {
	const background = root.ownerDocument.createElementNS(SVG_NS, 'rect');
	background.setAttribute('width', String(scene.width));
	background.setAttribute('height', String(scene.height));
	const preservedColor = root.getAttribute('data-schemra-canvas-background');
	background.setAttribute('fill', isSafeSvgColor(preservedColor) ? preservedColor : '#ffffff');
	const firstArtwork = [...root.children].find((element) => !['title', 'desc', 'defs', 'style'].includes(element.localName));
	root.insertBefore(background, firstArtwork ?? null);
	return background;
}

function frameRenderCacheFor(root: SVGSVGElement): FrameRenderCache {
	let cache = frameRenderCaches.get(root);
	if (!cache) {
		cache = { progressPaths: new Map(), markerGroups: new Map(), highlightGroups: new Map() };
		frameRenderCaches.set(root, cache);
	}
	return cache;
}

function removeInactive<K, T extends Element | Element[]>(elements: Map<K, T>, active: Set<K>) {
	for (const [id, element] of elements) {
		if (active.has(id)) continue;
		for (const child of Array.isArray(element) ? element : [element]) child.remove();
		elements.delete(id);
	}
}

function ensureLayer(root: SVGSVGElement, name: string, aboveNodes = false) {
	let layer = root.querySelector<SVGGElement>(`:scope > g[data-${name}]`);
	if (layer) return layer;
	layer = root.ownerDocument.createElementNS(SVG_NS, 'g');
	layer.setAttribute(`data-${name}`, 'true');
	layer.setAttribute('pointer-events', 'none');
	layer.setAttribute('aria-hidden', 'true');
	const firstNode = root.querySelector(':scope > g[data-node-id]');
	if (!aboveNodes && firstNode) root.insertBefore(layer, firstNode);
	else root.append(layer);
	return layer;
}

function appendMarkerArtwork(marker: SVGGElement, icon: string, char: string, token: string) {
	const paths = TOKEN_ICONS[icon.toLowerCase().replace(/^lucide\//, '')];
	if (paths) {
		const iconGroup = marker.ownerDocument.createElementNS(SVG_NS, 'g');
		iconGroup.setAttribute('transform', 'scale(.68)');
		for (const data of paths) {
			const path = marker.ownerDocument.createElementNS(SVG_NS, 'path');
			path.setAttribute('d', data);
			path.setAttribute('fill', 'none');
			path.setAttribute('stroke', 'var(--primary-foreground)');
			path.setAttribute('stroke-width', '1.6');
			path.setAttribute('stroke-linecap', 'round');
			path.setAttribute('stroke-linejoin', 'round');
			iconGroup.append(path);
		}
		marker.append(iconGroup);
		return;
	}
	const text = marker.ownerDocument.createElementNS(SVG_NS, 'text');
	const label = char || Array.from(token.trim() || '?').slice(0, 2).join('').toUpperCase();
	text.textContent = label;
	text.setAttribute('y', '3');
	text.setAttribute('text-anchor', 'middle');
	text.setAttribute('font-size', label.length === 1 ? '9' : '6.5');
	text.setAttribute('font-weight', '700');
	text.setAttribute('fill', 'var(--primary-foreground)');
	marker.append(text);
}

function nearestPathLength(path: SVGGeometryElement, length: number, x: number, y: number) {
	let nearest = 0;
	let minDistance = Number.POSITIVE_INFINITY;
	for (let index = 0; index <= 128; index += 1) {
		const candidate = (length * index) / 128;
		const point = path.getPointAtLength(candidate);
		const distance = (point.x - x) ** 2 + (point.y - y) ** 2;
		if (distance < minDistance) {
			minDistance = distance;
			nearest = candidate;
		}
	}
	return nearest;
}
