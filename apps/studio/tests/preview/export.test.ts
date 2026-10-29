import { describe, expect, test } from 'bun:test';
import {
	calculatePngDimensions,
	createDiagramPngBlob,
	createDiagramSvgBlob,
	diagramExportFileName,
	getDiagramDimensions,
	serializeDiagramSvg,
	type PngExportPlatform,
	type SvgMarkupAdapter
} from '../../src/lib/preview/export.js';

class FakeSvgElement {
	readonly attributes = new Map<string, string>();
	readonly layers: Set<string>;

	constructor(layers: Iterable<string> = []) {
		this.layers = new Set(layers);
	}

	cloneNode() {
		const clone = new FakeSvgElement(this.layers);
		for (const [name, value] of this.attributes) clone.attributes.set(name, value);
		return clone;
	}

	querySelector(selector: string) {
		const match = selector.match(/data-schemra-preview-(links|markers|highlights)/);
		const layer = match?.[1];
		if (!layer || !this.layers.has(layer)) return null;
		return { remove: () => this.layers.delete(layer) };
	}

	getAttribute(name: string) {
		return this.attributes.get(name) ?? null;
	}

	setAttribute(name: string, value: string) {
		this.attributes.set(name, value);
	}
}

function fakeSvg(layers = ['links', 'markers', 'highlights']) {
	const root = new FakeSvgElement(layers);
	root.setAttribute('viewBox', '0 0 640 360');
	root.setAttribute('width', '640');
	root.setAttribute('height', '360');
	return root as unknown as SVGSVGElement;
}

function fakeSvgAdapter(serialized: string[] = []): SvgMarkupAdapter {
	return {
		serialize: (svg) => {
			serialized.push([...((svg as unknown as FakeSvgElement).layers)].join(','));
			return '<svg></svg>';
		},
		sanitize: (markup) => markup
	};
}

describe('diagram exports', () => {
	test('removes preview playback layers from an exported SVG clone', () => {
		const root = fakeSvg();
		const serializedLayers: string[] = [];
		const markup = serializeDiagramSvg(root, fakeSvgAdapter(serializedLayers));

		expect(markup).toBe('<svg></svg>');
		expect(serializedLayers).toEqual(['']);
		expect([...((root as unknown as FakeSvgElement).layers)]).toEqual(['links', 'markers', 'highlights']);
	});

	test('creates a self-contained SVG blob from sanitized diagram markup', async () => {
		const adapter: SvgMarkupAdapter = {
			serialize: () => '<svg xmlns="http://www.w3.org/2000/svg"></svg>',
			sanitize: (markup) => markup.replace('<svg ', '<svg class="schemra-preview-svg" ')
		};
		const blob = createDiagramSvgBlob(fakeSvg(), adapter);

		expect(blob.type).toBe('image/svg+xml;charset=utf-8');
		expect(await blob.text()).toContain('class="schemra-preview-svg"');
	});

	test('reads intrinsic size from the viewBox and falls back to pixel dimensions', () => {
		expect(getDiagramDimensions(fakeSvg())).toEqual({ width: 640, height: 360 });
		const root = new FakeSvgElement();
		root.setAttribute('width', '320px');
		root.setAttribute('height', '180');
		expect(getDiagramDimensions(root as unknown as SVGSVGElement)).toEqual({ width: 320, height: 180 });
		expect(() => getDiagramDimensions(new FakeSvgElement() as unknown as SVGSVGElement)).toThrow(RangeError);
	});

	test('keeps raster dimensions within side and pixel limits', () => {
		expect(calculatePngDimensions(640, 360)).toEqual({ width: 1280, height: 720 });
		expect(calculatePngDimensions(10_000, 10_000)).toEqual({ width: 5656, height: 5656 });
		expect(calculatePngDimensions(12, 8, 3)).toEqual({ width: 36, height: 24 });
		expect(() => calculatePngDimensions(0, 10)).toThrow(RangeError);
	});

	test('rasterizes a safe SVG to PNG and releases its temporary object URL', async () => {
		const calls: unknown[][] = [];
		const png = new Blob(['png bytes'], { type: 'image/png' });
		const platform: PngExportPlatform = {
			serialize: () => '<svg></svg>',
			sanitize: (markup) => markup,
			createObjectUrl: (blob) => {
				calls.push(['createObjectUrl', blob.type]);
				return 'blob:schemra-diagram';
			},
			revokeObjectUrl: (url) => calls.push(['revokeObjectUrl', url]),
			loadImage: async (url) => {
				calls.push(['loadImage', url]);
				return {} as CanvasImageSource;
			},
			createCanvas: (width, height) => ({
				getContext: () => ({ drawImage: (...args: unknown[]) => calls.push(['drawImage', ...args]) }) as unknown as CanvasRenderingContext2D,
				toBlob: (callback) => callback(png),
				width,
				height
			}) as unknown as HTMLCanvasElement
		};
		const blob = await createDiagramPngBlob(fakeSvg(), { scale: 2 }, platform);

		expect(blob).toBe(png);
		expect(calls[0]).toEqual(['createObjectUrl', 'image/svg+xml;charset=utf-8']);
		expect(calls[1]).toEqual(['loadImage', 'blob:schemra-diagram']);
		expect(calls[2]?.slice(0, 1)).toEqual(['drawImage']);
		expect(calls.at(-1)).toEqual(['revokeObjectUrl', 'blob:schemra-diagram']);
	});

	test('releases the SVG URL if the browser cannot load it for rasterization', async () => {
		const revoked: string[] = [];
		const platform: PngExportPlatform = {
			serialize: () => '<svg></svg>',
			sanitize: (markup) => markup,
			createObjectUrl: () => 'blob:failed-diagram',
			revokeObjectUrl: (url) => revoked.push(url),
			loadImage: async () => { throw new Error('decode failed'); },
			createCanvas: () => { throw new Error('canvas should not be created'); }
		};

		await expect(createDiagramPngBlob(fakeSvg(), {}, platform)).rejects.toThrow('decode failed');
		expect(revoked).toEqual(['blob:failed-diagram']);
	});

	test('normalizes unsafe filenames and appends the requested extension', () => {
		expect(diagramExportFileName('  Architecture / v1.svg  ', 'png')).toBe('Architecture - v1.png');
		expect(diagramExportFileName('   ', 'svg')).toBe('schemra-diagram.svg');
	});
});
