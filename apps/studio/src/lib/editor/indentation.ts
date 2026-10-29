export interface IndentationEdit {
	rangeStart: number;
	rangeEnd: number;
	text: string;
	selectionStart: number;
	selectionEnd: number;
}

/** Calculate a tab indent or outdent edit without mutating the source. */
export function getIndentationEdit(
	source: string,
	selectionStart: number,
	selectionEnd: number,
	outdent: boolean,
	indentSize: 2 | 4 = 2
): IndentationEdit | null {
	const hasSelection = selectionStart !== selectionEnd;
	if (!outdent && !hasSelection) {
		const indentation = ' '.repeat(indentSize);
		return {
			rangeStart: selectionStart,
			rangeEnd: selectionEnd,
			text: indentation,
			selectionStart: selectionStart + indentation.length,
			selectionEnd: selectionStart + indentation.length
		};
	}

	const firstLineStart = source.lastIndexOf('\n', selectionStart - 1) + 1;
	const selectionEndsAtNextLine = hasSelection && source[selectionEnd - 1] === '\n';
	const selectedEnd = selectionEndsAtNextLine ? selectionEnd - 1 : selectionEnd;
	const nextLineEnd = source.indexOf('\n', selectedEnd);
	const blockEnd = nextLineEnd === -1 ? source.length : nextLineEnd;
	const lines = source.slice(firstLineStart, blockEnd).split('\n');
	const edits: Array<{ offset: number; inserted: number; removed: number }> = [];
	let lineOffset = firstLineStart;
	const adjustedLines = lines.map((line) => {
		let adjusted = line;
		let inserted = 0;
		let removed = 0;
		if (outdent) {
			const indentation = line.match(/^[\t ]+/)?.[0] ?? '';
			removed = indentation.startsWith('\t') ? 1 : Math.min(indentSize, indentation.length);
			adjusted = line.slice(removed);
		} else {
			inserted = indentSize;
			adjusted = `${' '.repeat(indentSize)}${line}`;
		}
		if (inserted > 0 || removed > 0) edits.push({ offset: lineOffset, inserted, removed });
		lineOffset += line.length + 1;
		return adjusted;
	});

	if (edits.length === 0) return null;

	const mapOffset = (offset: number) =>
		offset +
		edits.reduce((delta, edit) => {
			if (offset > edit.offset || (offset === edit.offset && edit.inserted > 0)) {
				return delta + edit.inserted - edit.removed;
			}
			return delta;
		}, 0);

	return {
		rangeStart: firstLineStart,
		rangeEnd: blockEnd,
		text: adjustedLines.join('\n'),
		selectionStart: mapOffset(selectionStart),
		selectionEnd: mapOffset(selectionEnd)
	};
}
