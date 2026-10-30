import { describe, expect, test } from 'bun:test';
import { applyPreviewFrame } from '../../src/lib/preview/frame-renderer';
import type { PreviewFrame, PreviewScene } from '../../src/lib/preview/model';

class FakeSvgDocument {
	createElementNS() {
		return new FakeSvgElement(this);
	}
}

class FakeSvgElement {
	readonly ownerDocument: FakeSvgDocument;
	readonly attributes = new Map<string, string>();
	readonly children: FakeSvgElement[] = [];
	parentElement: FakeSvgElement | null = null;

	constructor(ownerDocument: FakeSvgDocument) {
		this.ownerDocument = ownerDocument;
	}

	get firstElementChild() {
		return this.children[0] ?? null;
	}

	setAttribute(name: string, value: string) {
		this.attributes.set(name, value);
	}

	getAttribute(name: string) {
		return this.attributes.get(name) ?? null;
	}

	querySelector(selector: string) {
		const name = selector.match(/\[data-([\w-]+)(?:=[^\]]*)?\]/)?.[1];
		return name ? this.children.find((child) => child.attributes.has(`data-${name}`)) ?? null : null;
	}

	querySelectorAll(selector: string) {
		const name = selector.match(/\[data-([\w-]+)(?:=[^\]]*)?\]/)?.[1];
		return name ? this.children.filter((child) => child.attributes.has(`data-${name}`)) : [];
	}

	removeAttribute(name: string) {
		this.attributes.delete(name);
	}

	cloneNode() {
		const clone = new FakeSvgElement(this.ownerDocument);
		for (const [name, value] of this.attributes) clone.setAttribute(name, value);
		return clone;
	}

	getTotalLength() {
		return 400;
	}

	getPointAtLength(distance: number) {
		return { x: distance, y: 0 };
	}

	append(...children: FakeSvgElement[]) {
		for (const child of children) {
			child.parentElement?.removeChild(child);
			child.parentElement = this;
			this.children.push(child);
		}
	}

	insertBefore(child: FakeSvgElement, reference: FakeSvgElement) {
		child.parentElement?.removeChild(child);
		const index = this.children.indexOf(reference);
		child.parentElement = this;
		this.children.splice(index, 0, child);
	}

	remove() {
		this.parentElement?.removeChild(this);
	}

	private removeChild(child: FakeSvgElement) {
		const index = this.children.indexOf(child);
		if (index !== -1) this.children.splice(index, 1);
		child.parentElement = null;
	}
}

function createTraceRoot() {
	const document = new FakeSvgDocument();
	const root = new FakeSvgElement(document);
	const node = new FakeSvgElement(document);
	node.setAttribute('data-node-id', 'worker');
	const background = new FakeSvgElement(document);
	background.setAttribute('data-node-background', 'true');
	node.append(background);
	root.append(node);
	const scene: PreviewScene = { width: 400, height: 100, groups: [], edges: [], nodes: [] };
	return { root, scene };
}

describe('animated diagram layers', () => {
	test('keeps a departure marker above its node port and under other nodes in transit', () => {
		const document = new FakeSvgDocument();
		const root = new FakeSvgElement(document);
		const node = new FakeSvgElement(document);
		node.setAttribute('data-node-id', 'client');
		root.append(node);
		const scene: PreviewScene = {
			width: 300,
			height: 100,
			groups: [],
			edges: [{
				from: 'client',
				to: 'server',
				label: null,
				path: []
			}],
			nodes: [
				{ id: 'client', label: 'Client', detail: null, bounds: { x: 0, y: 10, width: 100, height: 40 } },
				{ id: 'server', label: 'Server', detail: null, bounds: { x: 200, y: 10, width: 80, height: 40 } }
			]
		};
		const frame: PreviewFrame = {
			tokens: [{ instanceId: 1, token: 'mail', x: 100, y: 30 }],
			edges: [{ instanceId: 1, edgeIndex: 0, progress: 0, forward: true }],
			highlights: []
		};

		applyPreviewFrame(
			root as unknown as SVGSVGElement,
			frame,
			scene,
			null
		);

		const markersAboveNodes = root.children.find((child) => child.attributes.has('data-schemra-preview-port-markers'));
		const marker = markersAboveNodes?.children[0];
		expect(markersAboveNodes).toBeDefined();
		expect(root.children.indexOf(markersAboveNodes!)).toBeGreaterThan(root.children.indexOf(node));
		expect(marker).toBeDefined();

		applyPreviewFrame(
			root as unknown as SVGSVGElement,
			{
				...frame,
				tokens: [{ ...frame.tokens[0]!, x: 125 }],
				edges: [{ ...frame.edges[0]!, progress: 0.1 }]
			},
			scene,
			null
		);

		const markersBelowNodes = root.children.find((child) => child.attributes.has('data-schemra-preview-markers'));
		expect(markersBelowNodes?.children[0]).toBe(marker);
		expect(root.children.indexOf(markersBelowNodes!)).toBeLessThan(root.children.indexOf(node));
		expect(markersAboveNodes?.children).toHaveLength(0);
	});

	test('coalesces only close traces that share the same node port', () => {
		const { root, scene } = createTraceRoot();
		const frame: PreviewFrame = {
			tokens: [],
			edges: [],
			highlights: [
				{ instanceId: 1, nodeId: 'worker', effect: 'trace', progress: 0.4, opacity: 0.7, origin: { x: 20, y: 0 } },
				{ instanceId: 2, nodeId: 'worker', effect: 'trace', progress: 0.42, opacity: 0.8, origin: { x: 20.001, y: 0 } },
				{ instanceId: 3, nodeId: 'worker', effect: 'trace', progress: 0.41, opacity: 1, origin: { x: 40, y: 0 } },
				{ instanceId: 4, nodeId: 'worker', effect: 'trace', progress: 0.8, opacity: 1, origin: { x: 20, y: 0 } }
			]
		};

		applyPreviewFrame(root as unknown as SVGSVGElement, frame, scene, null);

		const highlights = root.children.find((child) => child.attributes.has('data-schemra-preview-highlights'));
		expect(highlights?.children).toHaveLength(6);
		const strokeWidths = highlights?.children.map((child) => child.getAttribute('stroke-width'));
		expect(strokeWidths?.filter((width) => width === '3.25')).toHaveLength(2);
		expect(strokeWidths?.filter((width) => width === '2.5')).toHaveLength(4);
		const forwardOffset = highlights?.children[0]?.getAttribute('stroke-dashoffset');
		const backwardOffset = highlights?.children[1]?.getAttribute('stroke-dashoffset');
		expect(forwardOffset).not.toBe(backwardOffset);
	});

	test('renders no more than four trace pulses for one crowded port', () => {
		const { root, scene } = createTraceRoot();
		const frame: PreviewFrame = {
			tokens: [],
			edges: [],
			highlights: Array.from({ length: 7 }, (_, index) => ({
				instanceId: index + 1,
				nodeId: 'worker',
				effect: 'trace' as const,
				progress: index / 6,
				opacity: 0.5,
				origin: { x: 20, y: 0 }
			}))
		};

		applyPreviewFrame(root as unknown as SVGSVGElement, frame, scene, null);

		const highlights = root.children.find((child) => child.attributes.has('data-schemra-preview-highlights'));
		expect(highlights?.children).toHaveLength(8);
	});

	test('leaves pulse highlights independent', () => {
		const { root, scene } = createTraceRoot();
		const frame: PreviewFrame = {
			tokens: [],
			edges: [],
			highlights: [
				{ instanceId: 1, nodeId: 'worker', effect: 'pulse', progress: 0.4, opacity: 0.7, origin: { x: 20, y: 0 } },
				{ instanceId: 2, nodeId: 'worker', effect: 'pulse', progress: 0.42, opacity: 0.8, origin: { x: 20, y: 0 } }
			]
		};

		applyPreviewFrame(root as unknown as SVGSVGElement, frame, scene, null);

		const highlights = root.children.find((child) => child.attributes.has('data-schemra-preview-highlights'));
		expect(highlights?.children).toHaveLength(2);
	});
});
