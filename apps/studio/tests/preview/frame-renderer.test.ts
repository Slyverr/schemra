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
		const name = selector.match(/\[data-([\w-]+)\]/)?.[1];
		return name ? this.children.find((child) => child.attributes.has(`data-${name}`)) ?? null : null;
	}

	querySelectorAll(selector: string) {
		const name = selector.match(/\[data-([\w-]+)\]/)?.[1];
		return name ? this.children.filter((child) => child.attributes.has(`data-${name}`)) : [];
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
});
