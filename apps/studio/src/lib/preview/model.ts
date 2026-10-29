export interface PreviewPoint {
	x: number;
	y: number;
}

export interface PreviewRouteSegment {
	start: PreviewPoint;
	control1: PreviewPoint;
	control2: PreviewPoint;
	end: PreviewPoint;
}

export interface PreviewEdge {
	from: string;
	to: string;
	label: string | null;
	path: PreviewRouteSegment[];
}

export interface PreviewGroup {
	name: string;
	title: string;
	bounds: { x: number; y: number; width: number; height: number };
}

export interface PreviewNode {
	id: string;
	label?: string;
	detail?: string | null;
	bounds: { x: number; y: number; width: number; height: number };
}

export interface PreviewScene {
	width: number;
	height: number;
	groups: PreviewGroup[];
	edges: PreviewEdge[];
	nodes: PreviewNode[];
}

export interface PreviewSceneSearchMatch<T> {
	item: T;
	index: number;
	score: number;
}

export interface PreviewSceneSearchResults {
	groups: PreviewSceneSearchMatch<PreviewGroup>[];
	nodes: PreviewSceneSearchMatch<PreviewNode>[];
	connections: PreviewSceneSearchMatch<PreviewEdge>[];
	totalMatches: number;
	visibleMatches: number;
}

export interface PreviewTokenStyle {
	token: string;
	icon?: string | null;
}

export interface PreviewScenario {
	id: string;
	title: string;
	duration_ms: number;
	tokens?: PreviewTokenStyle[];
}

export interface SampledToken {
	instanceId: number;
	token: string;
	x: number;
	y: number;
}

export interface SampledEdge {
	instanceId: number;
	edgeIndex: number;
	progress: number;
	forward: boolean;
}

export interface SampledHighlight {
	instanceId: number;
	nodeId: string;
	effect: 'trace' | 'pulse';
	progress: number;
	opacity: number;
	origin: PreviewPoint | null;
}

export interface PreviewFrame {
	tokens: SampledToken[];
	edges: SampledEdge[];
	highlights: SampledHighlight[];
}

export function readPreviewScene(value: unknown): PreviewScene | null {
	if (!isRecord(value) || !isFiniteNumber(value.width) || !isFiniteNumber(value.height)) return null;
	if (!Array.isArray(value.edges) || !Array.isArray(value.nodes)) return null;

	const edges = value.edges.map((edge) => {
		const segments = isRecord(edge) && Array.isArray(edge.path) ? edge.path.map(readRouteSegment) : [];
		const path = segments.every((segment): segment is PreviewRouteSegment => segment !== null) ? segments : [];
		return {
			from: isRecord(edge) && typeof edge.from === 'string' ? edge.from : '',
			to: isRecord(edge) && typeof edge.to === 'string' ? edge.to : '',
			label: isRecord(edge) && typeof edge.label === 'string' ? edge.label : null,
			path
		};
	});
	const nodes = value.nodes.flatMap((node) => {
		if (!isRecord(node) || typeof node.id !== 'string' || !isRecord(node.bounds)) return [];
		const bounds = readBounds(node.bounds);
		return bounds ? [{
			id: node.id,
			label: typeof node.label === 'string' && node.label.trim() ? node.label : node.id,
			detail: typeof node.detail === 'string' ? node.detail : null,
			bounds
		}] : [];
	});
	const groups = Array.isArray(value.groups)
		? value.groups.flatMap((group) => {
			if (!isRecord(group) || typeof group.name !== 'string' || !isRecord(group.bounds)) return [];
			const bounds = readBounds(group.bounds);
			if (!bounds) return [];
			return [{
				name: group.name,
				title: typeof group.title === 'string' && group.title.trim() ? group.title : group.name,
				bounds
			}];
		})
		: [];

	return { width: value.width, height: value.height, groups, edges, nodes };
}

/** Search groups, nodes, and connections with the same fuzzy scoring used by the Studio command UI. */
export function searchPreviewScene(
	scene: PreviewScene,
	query: string,
	maxPerKind = 30,
	scoreMatch: (text: string, query: string) => number = defaultCommandScore
): PreviewSceneSearchResults {
	const normalizedQuery = query.trim();
	const limit = Number.isFinite(maxPerKind) ? Math.max(0, Math.trunc(maxPerKind)) : 30;
	const nodeById = new Map(scene.nodes.map((node) => [node.id, node]));

	const groups = rankMatches(scene.groups, normalizedQuery, limit, (group) => `${group.name} ${group.title} group`, scoreMatch);
	const nodes = rankMatches(scene.nodes, normalizedQuery, limit, (node) => `${node.label ?? node.id} ${node.id} ${node.detail ?? ''} node`, scoreMatch);
	const connections = rankMatches(
		scene.edges,
		normalizedQuery,
		limit,
		(edge) => {
			if (!edge.from || !edge.to) return '';
			const source = nodeById.get(edge.from);
			const target = nodeById.get(edge.to);
			return `${edge.label ?? ''} ${edge.from} ${source?.label ?? ''} ${edge.to} ${target?.label ?? ''} connection`;
		},
		scoreMatch,
		(edge) => Boolean(edge.from && edge.to)
	);

	return {
		groups: groups.matches,
		nodes: nodes.matches,
		connections: connections.matches,
		totalMatches: groups.total + nodes.total + connections.total,
		visibleMatches: groups.matches.length + nodes.matches.length + connections.matches.length
	};
}

type RankedMatches<T> = { matches: PreviewSceneSearchMatch<T>[]; total: number };

function rankMatches<T>(
	items: readonly T[],
	query: string,
	limit: number,
	searchText: (item: T) => string,
	scoreMatch: (text: string, query: string) => number,
	include: (item: T) => boolean = () => true
): RankedMatches<T> {
	const matches = items.flatMap((item, index) => {
		if (!include(item)) return [];
		const score = query ? scoreMatch(searchText(item), query) : 1;
		return Number.isFinite(score) && score > 0 ? [{ item, index, score }] : [];
	});
	if (query) matches.sort((left, right) => right.score - left.score || left.index - right.index);
	return { matches: matches.slice(0, limit), total: matches.length };
}

function defaultCommandScore(text: string, query: string): number {
	const normalizedText = text.toLocaleLowerCase();
	const normalizedQuery = query.toLocaleLowerCase();
	const position = normalizedText.indexOf(normalizedQuery);
	if (position >= 0) return 1000 - position - (normalizedText.length - normalizedQuery.length) / 1000;
	let queryIndex = 0;
	for (const character of normalizedText) {
		if (character === normalizedQuery[queryIndex]) queryIndex += 1;
		if (queryIndex === normalizedQuery.length) return 500 - normalizedText.length / 1000;
	}
	return 0;
}


export function readPreviewScenario(value: unknown): PreviewScenario | null {
	if (
		!isRecord(value) ||
		typeof value.id !== 'string' ||
		typeof value.title !== 'string' ||
		!isFiniteNumber(value.duration_ms) ||
		value.duration_ms < 0
	) return null;

	const tokens = Array.isArray(value.tokens)
		? value.tokens.flatMap((item) => {
			if (!isRecord(item) || typeof item.token !== 'string') return [];
			return [{ token: item.token, icon: typeof item.icon === 'string' ? item.icon : null }];
		})
		: [];
	return { id: value.id, title: value.title, duration_ms: value.duration_ms, tokens };
}

export function readPreviewFrame(value: unknown): PreviewFrame {
	if (!isRecord(value)) return { tokens: [], edges: [], highlights: [] };
	const tokens = readArray(value.tokens, (item) => {
		if (!isRecord(item) || !isInteger(item.instance_id) || typeof item.token !== 'string') return null;
		if (!isFiniteNumber(item.x) || !isFiniteNumber(item.y) || item.traveling !== true) return null;
		return { instanceId: item.instance_id, token: item.token, x: item.x, y: item.y };
	});
	const edges = readArray(value.active_edges, (item) => {
		if (!isRecord(item) || !isInteger(item.instance_id) || !isInteger(item.edge_index) || item.edge_index < 0) return null;
		if (!isFiniteNumber(item.progress_micros) || item.traversal_direction !== 1 && item.traversal_direction !== -1) return null;
		return {
			instanceId: item.instance_id,
			edgeIndex: item.edge_index,
			progress: clamp(item.progress_micros / 1_000_000),
			forward: item.traversal_direction === 1
		};
	});
	const highlights = readArray(value.highlights, (item) => {
		if (!isRecord(item) || !isInteger(item.instance_id) || typeof item.node_id !== 'string') return null;
		if (item.effect !== 'Trace' && item.effect !== 'Pulse' && item.effect !== 'trace' && item.effect !== 'pulse') return null;
		const effect: SampledHighlight['effect'] = item.effect === 'Trace' || item.effect === 'trace' ? 'trace' : 'pulse';
		const origin = Array.isArray(item.origin) && item.origin.length === 2 && item.origin.every(isFiniteNumber)
			? { x: item.origin[0], y: item.origin[1] }
			: null;
		return {
			instanceId: item.instance_id,
			nodeId: item.node_id,
			effect,
			progress: isFiniteNumber(item.progress_micros) ? clamp(item.progress_micros / 1_000_000) : 0,
			opacity: isFiniteNumber(item.pulse_opacity) ? clamp(item.pulse_opacity) : 1,
			origin
		};
	});
	return { tokens, edges, highlights };
}

export function previewTimeAtElapsed(elapsedMs: number, durationMs: number): number {
	if (!Number.isFinite(elapsedMs) || !Number.isFinite(durationMs) || durationMs <= 0) return 0;
	return Math.max(0, Math.min(durationMs, elapsedMs));
}

export function routePathData(segments: PreviewRouteSegment[], forward = true): string {
	const route = forward ? segments : [...segments].reverse().map(({ start, control1, control2, end }) => ({
		start: end,
		control1: control2,
		control2: control1,
		end: start
	}));
	const first = route[0];
	if (!first) return '';
	return `M ${pointData(first.start)} ${route.map(({ control1, control2, end }) =>
		`C ${pointData(control1)} ${pointData(control2)} ${pointData(end)}`
	).join(' ')}`;
}

export function sanitizeSvgMarkup(source: string, parser = new DOMParser(), serializer = new XMLSerializer()): string {
	if (!source.trim()) return '';
	const document = parser.parseFromString(source, 'image/svg+xml');
	const root = document.documentElement;
	if (root.localName !== 'svg' || root.namespaceURI !== 'http://www.w3.org/2000/svg' || document.querySelector('parsererror')) {
		return '';
	}

	const allowedElements = new Set([
		'svg', 'title', 'desc', 'defs', 'filter', 'feDropShadow', 'style', 'rect', 'path', 'g', 'text',
		'tspan', 'circle', 'ellipse', 'image'
	]);
	const allowedAttributes = new Set([
		'xmlns', 'role', 'aria-labelledby', 'width', 'height', 'viewBox', 'fill', 'fill-opacity', 'stroke',
		'stroke-width', 'stroke-linecap', 'stroke-linejoin', 'stroke-dasharray', 'stroke-dashoffset', 'opacity',
		'id', 'x', 'y', 'd', 'transform', 'font-family', 'font-size', 'font-weight', 'text-anchor', 'rx', 'ry',
		'filter', 'dx', 'dy', 'stdDeviation', 'flood-color', 'flood-opacity', 'preserveAspectRatio', 'href',
		'pathLength', 'cx', 'cy', 'r'
	]);
	for (const element of [...root.querySelectorAll('*')].reverse()) {
		if (element.namespaceURI !== 'http://www.w3.org/2000/svg' || !allowedElements.has(element.localName)) {
			element.remove();
			continue;
		}
		for (const attribute of [...element.attributes]) {
			const name = attribute.name;
			const allowedData = /^data-[a-z0-9-]+$/i.test(name);
			if (!allowedAttributes.has(name) && !allowedData) {
				element.removeAttribute(name);
				continue;
			}
			if (name === 'href' && !/^data:image\/(?:png|jpeg|gif|webp);base64,[a-z0-9+/]+=*$/i.test(attribute.value)) {
				element.removeAttribute(name);
			}
			if (['fill', 'stroke', 'flood-color'].includes(name) && !isSafeColor(attribute.value)) {
				element.removeAttribute(name);
			}
			if (name === 'filter' && attribute.value !== 'url(#schemra-shadow)') {
				element.removeAttribute(name);
			}
		}
		if (element.localName === 'style' && !isRendererFontCss(element.textContent ?? '')) element.remove();
	}
	root.setAttribute('class', 'schemra-preview-svg');
	root.setAttribute('preserveAspectRatio', 'xMidYMid meet');
	return serializer.serializeToString(root);
}

function isSafeColor(value: string): boolean {
	return /^#[a-f0-9]{3,8}$/i.test(value) || /^[a-z]{1,32}$/i.test(value);
}

function isRendererFontCss(css: string): boolean {
	return /^(?:@font-face\{font-family:'[A-Za-z0-9 _-]+';font-style:normal;font-weight:100 900;font-stretch:normal;src:url\(data:font\/woff2;base64,[A-Za-z0-9+/=]+\) format\('woff2'\)\})+$/.test(css);
}

function readRouteSegment(value: unknown): PreviewRouteSegment | null {
	if (!isRecord(value)) return null;
	const start = readPoint(value.start);
	const control1 = readPoint(value.control1);
	const control2 = readPoint(value.control2);
	const end = readPoint(value.end);
	return start && control1 && control2 && end ? { start, control1, control2, end } : null;
}

function readPoint(value: unknown): PreviewPoint | null {
	if (!isRecord(value) || !isFiniteNumber(value.x) || !isFiniteNumber(value.y)) return null;
	return { x: value.x, y: value.y };
}

function readBounds(value: Record<string, unknown>): PreviewNode['bounds'] | null {
	const { x, y, width, height } = value;
	if (!isFiniteNumber(x) || !isFiniteNumber(y) || !isFiniteNumber(width) || !isFiniteNumber(height)) return null;
	return { x, y, width, height };
}

function readArray<T>(value: unknown, map: (item: unknown) => T | null): T[] {
	return Array.isArray(value) ? value.map(map).filter((item): item is T => item !== null) : [];
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isFiniteNumber(value: unknown): value is number {
	return typeof value === 'number' && Number.isFinite(value);
}

function isInteger(value: unknown): value is number {
	return typeof value === 'number' && Number.isSafeInteger(value);
}

function clamp(value: number): number {
	return Math.max(0, Math.min(1, value));
}

function pointData(point: PreviewPoint): string {
	return `${point.x} ${point.y}`;
}
