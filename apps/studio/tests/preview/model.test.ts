import { describe, expect, test } from 'bun:test';
import {
	previewTimeAtElapsed,
	routePathData,
	readPreviewFrame,
	readPreviewScenario,
	readPreviewScene,
	searchPreviewScene,
	sanitizeSvgMarkup
} from '../../src/lib/preview/model';

class FakeSvgElement {
	readonly localName: string;
	readonly namespaceURI = 'http://www.w3.org/2000/svg';
	readonly attributes: { name: string; value: string }[];

	constructor(localName: string, attributes: Record<string, string>) {
		this.localName = localName;
		this.attributes = Object.entries(attributes).map(([name, value]) => ({ name, value }));
	}

	removeAttribute(name: string) {
		const index = this.attributes.findIndex((attribute) => attribute.name === name);
		if (index !== -1) this.attributes.splice(index, 1);
	}

	remove() {}
}

describe('diagram preview model', () => {
	test('bounds sampled playback values and preserves movement direction', () => {
		const frame = readPreviewFrame({
			tokens: [
				{ instance_id: 4, token: 'packet', x: 12.5, y: 42, traveling: true },
				{ instance_id: 5, token: 'ignored', x: Number.NaN, y: 0, traveling: true }
			],
			active_edges: [
				{ instance_id: 4, edge_index: 1, progress_micros: 500_000, traversal_direction: -1 },
				{ instance_id: 6, edge_index: 0, progress_micros: 2_000_000, traversal_direction: 0 }
			],
				highlights: [
				{ instance_id: 7, node_id: 'server', effect: 'Trace', progress_micros: 400_000, pulse_opacity: 0.8, origin: [10, 20] },
				{ instance_id: 8, node_id: 'bad', effect: 'other', progress_micros: 0, pulse_opacity: 1, origin: null }
			]
		});

		expect(frame.tokens).toEqual([{ instanceId: 4, token: 'packet', x: 12.5, y: 42 }]);
		expect(frame.edges).toEqual([{ instanceId: 4, edgeIndex: 1, progress: 0.5, forward: false }]);
		expect(frame.highlights).toEqual([{
			instanceId: 7,
			nodeId: 'server',
			effect: 'trace',
			progress: 0.4,
			opacity: 0.8,
			origin: { x: 10, y: 20 }
		}]);
	});

	test('reads only usable scene and scenario data from the bridge payloads', () => {
		const scene = readPreviewScene({
			width: 300,
			height: 180,
			groups: [{ name: 'application', title: 'Application', bounds: { x: 180, y: 20, width: 100, height: 100 } }],
			edges: [{ from: 'cache', to: 'server', label: 'requests', path: [{ start: { x: 0, y: 0 }, control1: { x: 10, y: 0 }, control2: { x: 20, y: 20 }, end: { x: 30, y: 20 } }] }],
			nodes: [
				{ id: 'server', label: 'Backend service', detail: 'Handles requests', bounds: { x: 200, y: 60, width: 80, height: 40 } },
				{ id: 'cache', bounds: { x: 120, y: 60, width: 60, height: 40 } }
			]
		});
		const scenario = readPreviewScenario({
			id: 'requests',
			title: 'Requests',
			duration_ms: 1000,
			tokens: [{ token: 'packet', icon: 'mail' }]
		});

		expect(scene?.edges[0]?.path).toHaveLength(1);
		expect(scene?.edges[0]).toMatchObject({ from: 'cache', to: 'server', label: 'requests' });
		expect(scene?.groups[0]).toMatchObject({ name: 'application', title: 'Application' });
		expect(scene?.nodes[0]?.id).toBe('server');
		expect(scene?.nodes[0]?.label).toBe('Backend service');
		expect(scene?.nodes[0]?.detail).toBe('Handles requests');
		expect(scene?.nodes[1]?.label).toBe('cache');
		expect(readPreviewScene({ width: 'wide', height: 10, edges: [], nodes: [] })).toBeNull();
		expect(scenario?.tokens).toEqual([{ token: 'packet', icon: 'mail' }]);
		expect(readPreviewScenario({ id: 'x', title: 'bad', duration_ms: -1 })).toBeNull();
	});

	test('searches group titles, node IDs and details, and connection labels and endpoints', () => {
		const scene = readPreviewScene({
			width: 300,
			height: 180,
			groups: [{ name: 'application', title: 'Application', bounds: { x: 180, y: 20, width: 100, height: 100 } }],
			edges: [{ from: 'client', to: 'server', label: 'request flow', path: [] }],
			nodes: [
				{ id: 'server', label: 'Reverse proxy', detail: 'Routes requests', bounds: { x: 200, y: 60, width: 80, height: 40 } },
				{ id: 'client', label: 'Client', bounds: { x: 10, y: 60, width: 60, height: 40 } }
			]
		});
		if (!scene) throw new Error('Expected valid scene.');
		const includesQuery = (value: string, query: string) => {
			const normalized = value.toLowerCase();
			return query.toLowerCase().split(/\s+/).every((word) => normalized.includes(word)) ? 1 : 0;
		};

		expect(searchPreviewScene(scene, 'application', 30, includesQuery).groups.map(({ item }) => item.name)).toEqual(['application']);
		expect(searchPreviewScene(scene, 'routes', 30, includesQuery).nodes.map(({ item }) => item.id)).toEqual(['server']);
		expect(searchPreviewScene(scene, 'reverse client', 30, includesQuery).connections.map(({ index }) => index)).toEqual([0]);
		expect(searchPreviewScene(scene, 'request flow', 30, includesQuery).connections[0]?.item.label).toBe('request flow');
	});

	test('keeps unfiltered results in scene order and caps each category without losing totals', () => {
		const scene = readPreviewScene({
			width: 300,
			height: 180,
			groups: [
				{ name: 'first', bounds: { x: 0, y: 0, width: 20, height: 20 } },
				{ name: 'second', bounds: { x: 30, y: 0, width: 20, height: 20 } }
			],
			edges: [],
			nodes: [
				{ id: 'one', bounds: { x: 0, y: 30, width: 20, height: 20 } },
				{ id: 'two', bounds: { x: 30, y: 30, width: 20, height: 20 } }
			]
		});
		if (!scene) throw new Error('Expected valid scene.');
		const results = searchPreviewScene(scene, '', 1);

		expect(results.groups.map(({ item }) => item.name)).toEqual(['first']);
		expect(results.nodes.map(({ item }) => item.id)).toEqual(['one']);
		expect(results.totalMatches).toBe(4);
		expect(results.visibleMatches).toBe(2);
	});

	test('reverses cubic segments without changing their geometry', () => {
		const segments = [{
			start: { x: 1, y: 2 },
			control1: { x: 3, y: 4 },
			control2: { x: 5, y: 6 },
			end: { x: 7, y: 8 }
		}];

		expect(routePathData(segments, true)).toBe('M 1 2 C 3 4 5 6 7 8');
		expect(routePathData(segments, false)).toBe('M 7 8 C 5 6 3 4 1 2');
		expect(routePathData([], true)).toBe('');
	});

	test('keeps group-label paint order so canvas-colored outlines cannot cover the text', () => {
		const label = new FakeSvgElement('text', {
			'data-group-id': 'clients',
			fill: '#52525b',
			stroke: '#ffffff',
			'stroke-width': '4',
			'paint-order': 'stroke',
			'letter-spacing': '1.1'
		});
		const root = {
			localName: 'svg',
			namespaceURI: 'http://www.w3.org/2000/svg',
			querySelectorAll: () => [label],
			setAttribute: () => {}
		};
		const parser = {
			parseFromString: () => ({ documentElement: root, querySelector: () => null })
		};
		const serializer = {
			serializeToString: () => label.attributes.map(({ name, value }) => `${name}="${value}"`).join(' ')
		};

		const markup = sanitizeSvgMarkup(
			'<svg></svg>',
			parser as unknown as DOMParser,
			serializer as unknown as XMLSerializer
		);

		expect(markup).toContain('paint-order="stroke"');
		expect(markup).toContain('letter-spacing="1.1"');
	});

	test('clamps playback time and handles invalid durations', () => {
		expect(previewTimeAtElapsed(-10, 100)).toBe(0);
		expect(previewTimeAtElapsed(125, 100)).toBe(100);
		expect(previewTimeAtElapsed(Number.NaN, 100)).toBe(0);
		expect(previewTimeAtElapsed(10, 0)).toBe(0);
	});
});
