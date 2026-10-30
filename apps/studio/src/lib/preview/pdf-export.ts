import { studioMessage } from '../i18n/messages.js';

const ROOT_SVG = /^\s*(?:<\?xml[^>]*>\s*)?<svg(?:\s|>)/i;

/** Create a browser document that prints the sanitized SVG without adding presentation styles. */
export function createPdfPrintDocument(svgMarkup: string, title: string): string {
	if (!ROOT_SVG.test(svgMarkup)) throw new TypeError(studioMessage('preview.pdfSvgRequired'));
	return `<!doctype html><html><head><meta charset="utf-8"><title>${escapeHtml(title)}</title></head><body>${svgMarkup}</body></html>`;
}

/** Open the browser print dialog so the user can save the diagram as a PDF. */
export async function printDiagramPdf(svgMarkup: string, title: string): Promise<void> {
	if (typeof window === 'undefined' || typeof document === 'undefined') {
		throw new Error(studioMessage('preview.pdfBrowserOnly'));
	}

	const html = createPdfPrintDocument(svgMarkup, title);
	const printWindow = window.open('', '_blank');
	if (!printWindow) throw new Error(studioMessage('preview.pdfPopupBlocked'));

	await new Promise<void>((resolve, reject) => {
		let settled = false;
		const fail = (error: unknown) => {
			if (settled) return;
			settled = true;
			reject(error);
		};
		const print = async () => {
			if (settled) return;
			try {
				await printWindow.document.fonts?.ready;
				if (printWindow.closed) throw new Error(studioMessage('preview.pdfPrintWindowClosed'));
				printWindow.focus();
				printWindow.print();
				settled = true;
				resolve();
			} catch (error) {
				fail(error);
			}
		};

		printWindow.addEventListener('pagehide', () => fail(new Error(studioMessage('preview.pdfPrintWindowClosed'))), { once: true });
		printWindow.addEventListener('load', () => void print(), { once: true });
		try {
			printWindow.document.open();
			printWindow.document.write(html);
			printWindow.document.close();
			if (printWindow.document.readyState === 'complete') void print();
		} catch (error) {
			printWindow.close();
			fail(error);
		}
	});
}

function escapeHtml(value: string): string {
	return value
		.replaceAll('&', '&amp;')
		.replaceAll('"', '&quot;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;');
}
