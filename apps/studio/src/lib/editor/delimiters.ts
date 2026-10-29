export type DelimiterEdit =
	| { kind: 'move-cursor'; cursor: number }
	| {
			kind: 'replace-text';
			rangeStart: number;
			rangeEnd: number;
			text: string;
			selectionStart: number;
			selectionEnd: number;
			refreshCompletions: boolean;
	  };

const closingFor: Readonly<Record<string, string>> = {
	'(': ')',
	'[': ']',
	'{': '}',
	'"': '"'
};

function lexicalStateAt(source: string, offset: number) {
	let inString = false;
	let escaped = false;
	let inComment = false;

	for (let index = 0; index < Math.min(offset, source.length); index += 1) {
		const character = source[index] ?? '';
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

		if (character === '/' && source[index + 1] === '/') {
			inComment = true;
			index += 1;
		} else if (character === '"') {
			inString = true;
		}
	}

	return { inString, inComment };
}

/** Calculate a delimiter pairing, skip-over, or paired Backspace edit. */
export function getDelimiterEdit(
	source: string,
	selectionStart: number,
	selectionEnd: number,
	key: string
): DelimiterEdit | null {
	const hasSelection = selectionStart !== selectionEnd;
	const lexicalState = lexicalStateAt(source, selectionStart);
	const insideStringOrComment = lexicalState.inString || lexicalState.inComment;
	const nextCharacter = source[selectionStart];
	const previousCharacter = source[selectionStart - 1];

	const existingClosingOffset = !hasSelection && ['}', ')', ']', '"'].includes(key)
		? nextCharacter === key
			? selectionStart
			: key === '}'
				? findNextLineClosingBrace(source, selectionStart)
				: null
		: null;

	if (existingClosingOffset !== null) {
		if (key === '"' ? !lexicalState.inComment : !insideStringOrComment) {
			return { kind: 'move-cursor', cursor: existingClosingOffset + 1 };
		}
	}

	if (!hasSelection && key === 'Backspace' && selectionStart > 0 && nextCharacter) {
		const isEmptyPair =
			(previousCharacter === '(' && nextCharacter === ')') ||
			(previousCharacter === '[' && nextCharacter === ']') ||
			(previousCharacter === '{' && nextCharacter === '}') ||
			(previousCharacter === '"' && nextCharacter === '"');

		if (isEmptyPair && (previousCharacter === '"' || !insideStringOrComment)) {
			return {
				kind: 'replace-text',
				rangeStart: selectionStart - 1,
				rangeEnd: selectionStart + 1,
				text: '',
				selectionStart: selectionStart - 1,
				selectionEnd: selectionStart - 1,
				refreshCompletions: false
			};
		}
	}

	const closing = closingFor[key];
	if (!closing || key.length !== 1 || (!hasSelection && insideStringOrComment)) return null;

	const selectedText = source.slice(selectionStart, selectionEnd);
	const caret = selectionStart + 1;
	return {
		kind: 'replace-text',
		rangeStart: selectionStart,
		rangeEnd: selectionEnd,
		text: `${key}${selectedText}${closing}`,
		selectionStart: caret,
		selectionEnd: caret + selectedText.length,
		refreshCompletions: true
	};
}

function findNextLineClosingBrace(source: string, selectionStart: number): number | null {
	const trailingWhitespace = source.slice(selectionStart);
	const firstNonWhitespace = trailingWhitespace.search(/[^\t \r\n]/);
	if (firstNonWhitespace <= 0 || trailingWhitespace[firstNonWhitespace] !== '}') return null;
	if (!/[\r\n]/.test(trailingWhitespace.slice(0, firstNonWhitespace))) return null;
	return selectionStart + firstNonWhitespace;
}
