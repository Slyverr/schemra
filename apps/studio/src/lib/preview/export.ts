import { sanitizeSvgMarkup } from './model.js';

const PLAYBACK_LAYERS = [
	':scope > g[data-schemra-preview-links]',
	':scope > g[data-schemra-preview-markers]',
	':scope > g[data-schemra-preview-highlights]'
] as const;

export interface SvgMarkupAdapter {
	serialize(svg: SVGSVGElement): string;
	sanitize(markup: string): string;
}

export interface DiagramPngOptions {
	scale?: number;
}

export interface PngExportPlatform extends SvgMarkupAdapter {
	createObjectUrl(blob: Blob): string;
	revokeObjectUrl(url: string): void;
	loadImage(url: string): Promise<CanvasImageSource>;
	createCanvas(width: number, height: number): HTMLCanvasElement;
}

/** Clone the current static diagram and omit preview-only playback overlays. */
export function serializeDiagramSvg(svg: SVGSVGElement, adapter = browserSvgMarkupAdapter()): string {
	const clone = svg.cloneNode(true) as SVGSVGElement;
	for (const selector of PLAYBACK_LAYERS) clone.querySelector(selector)?.remove();
	const sanitized = adapter.sanitize(adapter.serialize(clone));
	if (!sanitized) throw new Error('The diagram could not be serialized as safe SVG.');
	return sanitized;
}

/** Create a standalone SVG file blob from the static diagram. */
export function createDiagramSvgBlob(svg: SVGSVGElement, adapter = browserSvgMarkupAdapter()): Blob {
	return new Blob([serializeDiagramSvg(svg, adapter)], { type: 'image/svg+xml;charset=utf-8' });
}

/** Read the diagram's intrinsic dimensions from its viewBox or width and height. */
export function getDiagramDimensions(svg: Pick<SVGSVGElement, 'getAttribute'>): { width: number; height: number } {
	const viewBox = svg.getAttribute('viewBox')?.trim().split(/[\s,]+/).map(Number);
	if (viewBox?.length === 4 && viewBox.every(Number.isFinite) && viewBox[2]! > 0 && viewBox[3]! > 0) {
		return { width: viewBox[2]!, height: viewBox[3]! };
	}
	const width = parseSvgLength(svg.getAttribute('width'));
	const height = parseSvgLength(svg.getAttribute('height'));
	if (width > 0 && height > 0) return { width, height };
	throw new RangeError('The diagram does not have valid intrinsic dimensions.');
}

/** Calculate a high-resolution raster size while keeping canvas allocation bounded. */
export function calculatePngDimensions(
	width: number,
	height: number,
	scale = 2,
	maxSide = 8192,
	maxPixels = 32_000_000
): { width: number; height: number } {
	if (![width, height, scale, maxSide, maxPixels].every(Number.isFinite) || width <= 0 || height <= 0 || scale <= 0 || maxSide < 1 || maxPixels < 1) {
		throw new RangeError('PNG dimensions and scale must be positive finite numbers.');
	}
	const boundedScale = Math.min(scale, maxSide / width, maxSide / height, Math.sqrt(maxPixels / (width * height)));
	return {
		width: Math.max(1, Math.floor(width * boundedScale)),
		height: Math.max(1, Math.floor(height * boundedScale))
	};
}

/** Rasterize the static diagram into a PNG blob using browser canvas APIs. */
export async function createDiagramPngBlob(
	svg: SVGSVGElement,
	options: DiagramPngOptions = {},
	platform = browserPngExportPlatform()
): Promise<Blob> {
	const markup = serializeDiagramSvg(svg, platform);
	const intrinsic = getDiagramDimensions(svg);
	const dimensions = calculatePngDimensions(intrinsic.width, intrinsic.height, options.scale ?? 2);
	const svgBlob = new Blob([markup], { type: 'image/svg+xml;charset=utf-8' });
	const url = platform.createObjectUrl(svgBlob);
	try {
		const image = await platform.loadImage(url);
		const canvas = platform.createCanvas(dimensions.width, dimensions.height);
		const context = canvas.getContext('2d');
		if (!context) throw new Error('A 2D canvas is unavailable for PNG export.');
		context.drawImage(image, 0, 0, dimensions.width, dimensions.height);
		return await new Promise<Blob>((resolve, reject) => {
			canvas.toBlob((blob) => blob ? resolve(blob) : reject(new Error('The browser could not encode the diagram as PNG.')), 'image/png');
		});
	} finally {
		platform.revokeObjectUrl(url);
	}
}

/** Build a safe, predictable filename for a diagram export. */
export function diagramExportFileName(name: string, format: 'svg' | 'png'): string {
	const base = name
		.trim()
		.replace(/\.(?:svg|png)$/i, '')
		.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '-')
		.replace(/\s+/g, ' ')
		.replace(/[. ]+$/g, '')
		.slice(0, 120);
	return `${base || 'schemra-diagram'}.${format}`;
}

/** Start a browser download for a prepared SVG or PNG blob. */
export function downloadDiagramBlob(blob: Blob, fileName: string): void {
	if (typeof document === 'undefined' || typeof URL.createObjectURL !== 'function') {
		throw new Error('Diagram downloads are only available in a browser.');
	}
	const url = URL.createObjectURL(blob);
	const link = document.createElement('a');
	link.href = url;
	link.download = fileName;
	link.click();
	link.remove();
	window.setTimeout(() => URL.revokeObjectURL(url), 0);
}

function browserSvgMarkupAdapter(): SvgMarkupAdapter {
	return {
		serialize: (svg) => new XMLSerializer().serializeToString(svg),
		sanitize: sanitizeSvgMarkup
	};
}

function browserPngExportPlatform(): PngExportPlatform {
	if (typeof document === 'undefined' || typeof Image === 'undefined' || typeof URL.createObjectURL !== 'function') {
		throw new Error('PNG export requires browser image and canvas support.');
	}
	return {
		...browserSvgMarkupAdapter(),
		createObjectUrl: (blob) => URL.createObjectURL(blob),
		revokeObjectUrl: (url) => URL.revokeObjectURL(url),
		loadImage: (url) => new Promise((resolve, reject) => {
			const image = new Image();
			image.onload = () => resolve(image);
			image.onerror = () => reject(new Error('The browser could not load the SVG for PNG export.'));
			image.src = url;
		}),
		createCanvas: (width, height) => {
			const canvas = document.createElement('canvas');
			canvas.width = width;
			canvas.height = height;
			return canvas;
		}
	};
}

function parseSvgLength(value: string | null): number {
	if (!value || !/^\s*\d+(?:\.\d+)?(?:px)?\s*$/i.test(value)) return 0;
	return Number.parseFloat(value);
}
