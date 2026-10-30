import { describe, expect, test } from 'bun:test';
import { createPdfPrintDocument } from '../../src/lib/preview/pdf-export.js';

describe('diagram PDF export', () => {
	test('wraps a standalone SVG and escapes the document title', () => {
		const document = createPdfPrintDocument('<svg xmlns="http://www.w3.org/2000/svg"></svg>', 'Service <flow> & data');

		expect(document).toContain('<title>Service &lt;flow&gt; &amp; data</title>');
		expect(document).toContain('<body><svg xmlns="http://www.w3.org/2000/svg"></svg></body>');
		expect(document).not.toContain('<style');
	});

	test('rejects markup that is not an SVG document', () => {
		expect(() => createPdfPrintDocument('<div>not a diagram</div>', 'Diagram')).toThrow(TypeError);
	});
});
