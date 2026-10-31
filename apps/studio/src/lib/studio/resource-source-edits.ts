import { applySourceTextEdits, utf8SpanToEditorSpan } from '../editor/source-edits.js';
import type { SourceSpan } from '../schemra.js';

export type SourceToken = { kind: string; span: SourceSpan };

export function findDeclarationIdentifier(
	source: string,
	tokens: readonly SourceToken[],
	declarationSpan: SourceSpan,
	identifier: string
): SourceToken | null {
	return tokens.find((token) => {
		if (token.kind !== 'identifier' || token.span.start.line !== declarationSpan.start.line) return false;
		const span = utf8SpanToEditorSpan(source, token.span);
		return source.slice(span.start.offset, span.end.offset) === identifier;
	}) ?? null;
}

export function renameDeclarationTitle(
	source: string,
	tokens: readonly SourceToken[],
	declarationSpan: SourceSpan,
	declarationId: string | null,
	title: string
): string | null {
	const titleToken = tokens.find((token) =>
		token.kind === 'string' &&
		token.span.start.line === declarationSpan.start.line &&
		token.span.start.offset >= declarationSpan.start.offset &&
		token.span.end.offset <= declarationSpan.end.offset
	);
	if (titleToken) {
		return applySourceTextEdits(source, [{ span: utf8SpanToEditorSpan(source, titleToken.span), replacement: JSON.stringify(title) }]);
	}
	const identifier = declarationId && findDeclarationIdentifier(source, tokens, declarationSpan, declarationId);
	if (!identifier) return null;
	const point = utf8SpanToEditorSpan(source, identifier.span).end;
	return applySourceTextEdits(source, [{ span: { start: point, end: point }, replacement: ` ${JSON.stringify(title)}` }]);
}
