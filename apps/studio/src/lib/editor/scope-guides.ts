/** One brace-scope guide. Offsets and columns use JavaScript UTF-16 source units. */
export interface ScopeRange {
	id: number;
	anchorOffset: number;
	startOffset: number;
	endOffset: number;
	depth: number;
	openLine: number;
	endLine: number;
	closed: boolean;
}

/** Find brace scopes and anchor each guide at the first code character on its opening line. */
export function getScopeRanges(source: string): ScopeRange[] {
	interface OpenScope {
		openOffset: number;
		openLine: number;
		anchorOffset: number;
		depth: number;
	}
	interface CompletedScope extends OpenScope {
		endLine: number;
		endOffset: number;
		closed: boolean;
	}

	const lines = source.split('\n');
	const lineStarts: number[] = [];
	let lineStart = 0;
	for (const line of lines) {
		lineStarts.push(lineStart);
		lineStart += line.length + 1;
	}

	const stack: OpenScope[] = [];
	const scopes: CompletedScope[] = [];
	let line = 0;
	let inString = false;
	let escaped = false;
	let inComment = false;

	for (let offset = 0; offset < source.length; offset += 1) {
		const character = source[offset] ?? '';
		if (character === '\n') {
			line += 1;
			if (inComment) inComment = false;
			continue;
		}
		if (inComment) continue;
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
		if (character === '{') {
			const indentation = lines[line]?.match(/^[ \t]*/)?.[0] ?? '';
			stack.push({
				openOffset: offset,
				openLine: line,
				anchorOffset: (lineStarts[line] ?? 0) + indentation.length,
				depth: stack.length
			});
			continue;
		}
		if (character === '}') {
			const open = stack.pop();
			if (open) scopes.push({ ...open, endLine: line, endOffset: offset, closed: true });
		}
	}

	for (const open of stack) {
		scopes.push({ ...open, endLine: lines.length - 1, endOffset: source.length, closed: false });
	}

	return scopes
		.filter((scope) => scope.closed ? scope.endLine > scope.openLine + 1 : scope.endLine > scope.openLine)
		.map((scope) => ({
			id: scope.openOffset,
			anchorOffset: scope.anchorOffset,
			startOffset: lineStarts[scope.openLine] ?? scope.openOffset,
			endOffset: scope.endOffset,
			depth: scope.depth,
			openLine: scope.openLine,
			endLine: scope.endLine,
			closed: scope.closed
		}))
		.sort((left, right) => left.openLine - right.openLine || left.anchorOffset - right.anchorOffset);
}

/** Return the deepest scope containing the cursor, or null when none does. */
export function getActiveScopeGuideId(scopes: readonly ScopeRange[], cursorOffset: number): number | null {
	let activeScope: ScopeRange | undefined;
	for (const scope of scopes) {
		if (cursorOffset < scope.startOffset || cursorOffset > scope.endOffset) continue;
		if (!activeScope || scope.depth > activeScope.depth) activeScope = scope;
	}
	return activeScope?.id ?? null;
}
