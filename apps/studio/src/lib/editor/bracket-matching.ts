const closingFor: Readonly<Record<string, string>> = { '(': ')', '[': ']', '{': '}' };
const openingFor: Readonly<Record<string, string>> = { ')': '(', ']': '[', '}': '{' };

/** Build one source index, then reuse it as the caret moves through unchanged text. */
export function createBracketMatcher() {
	let indexedSource: string | undefined;
	let pairs = new Map<number, number>();

	return (source: string, selectionStart: number, selectionEnd: number): ReadonlySet<number> => {
		if (selectionStart !== selectionEnd) return new Set();

		if (source !== indexedSource) {
			indexedSource = source;
			pairs = indexBrackets(source);
		}

		for (const offset of [selectionStart - 1, selectionStart]) {
			const match = pairs.get(offset);
			if (match !== undefined) return new Set([offset, match]);
		}
		return new Set();
	};
}

function indexBrackets(source: string) {
	const pairs = new Map<number, number>();
	const stack: Array<{ character: string; offset: number }> = [];
	let inString = false;
	let escaped = false;
	let inComment = false;

	for (let offset = 0; offset < source.length; offset += 1) {
		const character = source[offset] ?? '';
		if (inComment) {
			if (character === '\n') inComment = false;
			continue;
		}
		if (inString) {
			if (escaped) escaped = false;
			else if (character === '\\') escaped = true;
			else if (character === '"') inString = false;
			continue;
		}
		if (character === '/' && source[offset + 1] === '/') {
			inComment = true;
			offset += 1;
			continue;
		}
		if (character === '"') {
			inString = true;
			continue;
		}
		if (closingFor[character]) {
			stack.push({ character, offset });
			continue;
		}
		const expectedOpening = openingFor[character];
		if (!expectedOpening) continue;
		const opening = stack.at(-1);
		if (opening?.character === expectedOpening) {
			stack.pop();
			pairs.set(opening.offset, offset);
			pairs.set(offset, opening.offset);
		} else {
			// A mismatched closer breaks this nesting chain; retaining its openers
			// could incorrectly highlight pairs that cross the malformed delimiter.
			stack.length = 0;
		}
	}

	return pairs;
}
