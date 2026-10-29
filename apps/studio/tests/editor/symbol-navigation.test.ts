import { describe, expect, test } from 'bun:test';
import {
	findEditorSymbolAtOffset,
	findEditorSymbolDefinition,
	findEditorSymbolDefinitionAtOffset
} from '../../src/lib/editor/symbol-navigation.js';
import type { EditorSymbol } from '../../src/lib/schemra.js';

function symbol(
	name: string,
	role: EditorSymbol['role'],
	start: number,
	end: number,
	key = 'node:network:client'
): EditorSymbol {
	return {
		name,
		kind: 'node',
		role,
		target_key: key,
		span: {
			start: { offset: start, line: 1, column: start + 1 },
			end: { offset: end, line: 1, column: end + 1 }
		}
	};
}

describe('editor symbol navigation', () => {
	test('selects only semantic references at the caret and prefers the narrowest span', () => {
		const definition = symbol('client', 'definition', 3, 9);
		const reference = symbol('client', 'reference', 14, 20);
		const overlapping = symbol('client', 'reference', 15, 19);

		expect(findEditorSymbolAtOffset([definition, reference, overlapping], 16, 'reference')).toBe(overlapping);
		expect(findEditorSymbolAtOffset([definition, reference], 6, 'reference')).toBeNull();
		expect(findEditorSymbolAtOffset([reference], 21, 'reference')).toBeNull();
		expect(findEditorSymbolAtOffset([reference], Number.NaN, 'reference')).toBeNull();
	});

	test('finds the unique matching definition in another linked file', () => {
		const reference = symbol('client', 'reference', 14, 20);
		const definition = symbol('client', 'definition', 3, 9);
		const result = findEditorSymbolDefinition([
			{ id: 'visual-file', path: 'visuals/network.schemra', editor_symbols: [definition] },
			{ id: 'scenario-file', path: 'scenarios/traffic.schemra', editor_symbols: [reference] }
		], reference);

		expect(result).toEqual({
			fileId: 'visual-file',
			path: 'visuals/network.schemra',
			symbol: definition
		});
	});

	test('resolves either a reference or definition caret to its editable definition', () => {
		const reference = symbol('client', 'reference', 14, 20);
		const definition = symbol('client', 'definition', 3, 9);
		const files = [
			{ id: 'visual-file', path: 'visuals/network.schemra', editor_symbols: [definition] },
			{ id: 'scenario-file', path: 'scenarios/traffic.schemra', editor_symbols: [reference] }
		];

		expect(findEditorSymbolDefinitionAtOffset(files, 'scenario-file', 17)).toEqual({
			fileId: 'visual-file',
			path: 'visuals/network.schemra',
			symbol: definition
		});
		expect(findEditorSymbolDefinitionAtOffset(files, 'visual-file', 5)).toEqual({
			fileId: 'visual-file',
			path: 'visuals/network.schemra',
			symbol: definition
		});
		expect(findEditorSymbolDefinitionAtOffset(files, 'missing-file', 17)).toBeNull();
	});

	test('rejects missing or ambiguous definitions', () => {
		const reference = symbol('client', 'reference', 14, 20);
		const definition = symbol('client', 'definition', 3, 9);
		expect(findEditorSymbolDefinition([{ id: 'scenario', path: 'scenario.schemra', editor_symbols: [reference] }], reference)).toBeNull();
		expect(findEditorSymbolDefinition([
			{ id: 'visual-a', path: 'a.schemra', editor_symbols: [definition] },
			{ id: 'visual-b', path: 'b.schemra', editor_symbols: [definition] }
		], reference)).toBeNull();
	});
});
