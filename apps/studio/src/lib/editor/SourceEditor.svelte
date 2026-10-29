<script lang="ts">
	import { onDestroy, onMount, tick } from 'svelte';
	import {
		completeAt,
		completeAtWithNodes,
		compileSource,
		formatSource,
		type EditorSymbol
	} from '../schemra.js';
	import { createBracketMatcher } from './bracket-matching.js';
	import EditorCompletions from './EditorCompletions.svelte';
	import { getDelimiterEdit } from './delimiters.js';
	import { formatSourceForSave } from './format-on-save.js';
	import { getIndentationEdit } from './indentation.js';
	import {
		diagnosticQuickFixOptions,
		type RefactorDiagnostic,
		type RefactorSymbol
	} from './editor-refactors.js';
	import {
		buildEditorLines,
		getEditorDiagnosticAtColumn,
		getEditorPositionAtPoint,
		mapEditorSelection,
		readEditorDiagnostics,
		readEditorSyntaxTokens,
		type EditorSyntaxToken,
		type EditorDiagnostic
	} from './source-model.js';
	import { getActiveScopeGuideId, getScopeRanges } from './scope-guides.js';
	import { getScopeGuideGeometry, measureSourceGlyphInkStartX } from './scope-guide-geometry.js';
	import { findEditorSymbolAtOffset } from './symbol-navigation.js';
	import {
		editorDocumentKey,
		rememberEditorViewState,
		restoreEditorViewState,
		type EditorViewState
	} from './view-state.js';
	import {
		normalizeEditorPreferences,
		type EditorPreferences
	} from '../settings/editor.js';
	import { studioMessage } from '../i18n/messages.js';

	interface Props {
		projectId?: string | null;
		documentId?: string | null;
		disabled?: boolean;
		navigation?: { fileId: string; offset: number; requestId: number } | null;
		source: string;
		onChange: (source: string) => void;
		tokens?: readonly EditorSyntaxToken[];
		diagnostics?: readonly EditorDiagnostic[];
		nodeIds?: string[];
		label?: string;
		preferences?: Partial<EditorPreferences>;
		onSave?: (source: string, context?: { projectId: string | null; fileId: string | null }) => void;
		editorSymbols?: readonly EditorSymbol[];
		refactorDiagnostics?: readonly RefactorDiagnostic[];
		refactorSymbols?: readonly RefactorSymbol[];
		quickFixDisabled?: boolean;
		onNavigate?: (symbol: EditorSymbol) => void;
		onRenameSymbol?: (offset: number) => void;
		onQuickFix?: (diagnostic: RefactorDiagnostic, replacement: string) => void;
		onNavigationHandled?: (requestId: number) => void;
	}

	interface CompletionState {
		start: number;
		end: number;
		items: Array<{ label: string; insert_text: string; detail: string; kind: string }>;
	}

	interface TooltipState {
		key: string;
		diagnostic: EditorDiagnostic;
		left: number;
		top: number;
		maxWidth: number;
		maxHeight: number;
	}

	const props: Props = $props();
	const instanceId = $props.id();
	const completionListId = `${instanceId}-completions`;
	const diagnosticListId = `${instanceId}-diagnostics`;
	const baseLineHeight = 24;
	const verticalPadding = 8;
	const baseFontSize = 14;
	const diagnosticHoverDelayMs = 400;
	const matcher = createBracketMatcher();

	let root = $state<HTMLDivElement>();
	let viewport = $state<HTMLDivElement>();
	let codeSurface = $state<HTMLDivElement>();
	let highlightedLines = $state<HTMLElement>();
	let gutterLines = $state<HTMLDivElement>();
	let input = $state<HTMLTextAreaElement>();
	let tooltipElement = $state<HTMLDivElement>();
	let completion = $state<CompletionState | null>(null);
	let selectedCompletion = $state(0);
	let completionPosition = $state({ left: 8, top: 24, width: 288 });
	let selection = $state({ start: 0, end: 0, direction: 'none' as 'forward' | 'backward' | 'none' });
	const editorViewStates = new Map<string, EditorViewState>();
	let lastViewStateKey: string | null | undefined;
	let syntaxTokens = $state(readEditorSyntaxTokens([]));
	let diagnostics = $state(readEditorDiagnostics([]));
	let compiledSource = $state<string | null>(null);
	let matchingOffsets = $state<ReadonlySet<number>>(new Set());
	let scopeGuidePositions = $state<Record<number, { left: number; top: number; height: number }>>({});
	let tooltip = $state<TooltipState | null>(null);
	let gutterScrollTop = $state(0);
	let compileTimer: ReturnType<typeof setTimeout> | undefined;
	let completionTimer: ReturnType<typeof setTimeout> | undefined;
	let tooltipTimer: ReturnType<typeof setTimeout> | undefined;
	let pendingTooltip: TooltipState | null = null;
	let compileRequestId = 0;
	let completionRequestId = 0;
	let lastNavigationRequestId = 0;
	let measureFrame = 0;
	let sourceMeasureCanvas: HTMLCanvasElement | undefined;

	const preferences = $derived(normalizeEditorPreferences(props.preferences));
	const zoomScale = $derived(preferences.zoom / 100);
	const lineHeight = $derived(baseLineHeight * zoomScale);
	const fontSize = $derived(baseFontSize * zoomScale);
	const scopeRanges = $derived(getScopeRanges(props.source));
	const activeScopeId = $derived(getActiveScopeGuideId(scopeRanges, caretOffset(selection)));
	const currentCompile = $derived(compiledSource === props.source);
	const visibleTokens = $derived(props.tokens?.length ? props.tokens : syntaxTokens);
	const visibleDiagnostics = $derived(props.diagnostics?.length ? props.diagnostics : (currentCompile ? diagnostics : []));
	const lines = $derived(buildEditorLines(props.source, visibleTokens, visibleDiagnostics, matchingOffsets));
	const activeLineIndex = $derived(props.source.slice(0, caretOffset(selection)).split('\n').length - 1);
	const completionActiveId = $derived(completion ? `${completionListId}-${selectedCompletion}` : undefined);
	const tooltipQuickFixes = $derived.by(() => {
		if (!tooltip) return { diagnostic: null, options: [] };
		const diagnostic = refactorDiagnosticFor(tooltip.diagnostic);
		return {
			diagnostic,
			options: diagnostic
				? diagnosticQuickFixOptions(props.source, props.refactorSymbols ?? [], diagnostic)
				: []
		};
	});

	function describeDiagnostic(diagnostic: EditorDiagnostic) {
		const severity = diagnostic.severity === 'error'
			? studioMessage('editor.severity.error')
			: diagnostic.severity === 'warning'
				? studioMessage('editor.severity.warning')
				: studioMessage('editor.severity.information');
		const location = studioMessage('editor.diagnosticLocation', {
			severity,
			line: diagnostic.span.start.line,
			column: diagnostic.span.start.column,
			message: diagnostic.message
		});
		return diagnostic.hint ? `${location} ${diagnostic.hint}` : location;
	}

	$effect(() => {
		const sourceSnapshot = props.source;
		const externalTokens = props.tokens;
		const externalDiagnostics = props.diagnostics;
		const requestId = ++compileRequestId;
		clearTimeout(compileTimer);
		if (externalTokens?.length) {
			syntaxTokens = readEditorSyntaxTokens(externalTokens);
			if (externalDiagnostics !== undefined) diagnostics = readEditorDiagnostics(externalDiagnostics);
			compiledSource = sourceSnapshot;
			return;
		}
		compiledSource = null;
		compileTimer = setTimeout(() => {
			compileTimer = undefined;
			void compileSource(sourceSnapshot).then((result) => {
				if (requestId !== compileRequestId || props.source !== sourceSnapshot) return;
				syntaxTokens = readEditorSyntaxTokens(result.tokens);
				diagnostics = readEditorDiagnostics(result.diagnostics);
				compiledSource = sourceSnapshot;
			}, () => {
				if (requestId !== compileRequestId || props.source !== sourceSnapshot) return;
				compiledSource = sourceSnapshot;
			});
		}, 90);
		return () => clearTimeout(compileTimer);
	});

	$effect.pre(() => {
		const nextKey = editorDocumentKey(props.projectId, props.documentId);
		const nextSource = props.source;
		const currentInput = input;
		if (!currentInput) return;

		if (lastViewStateKey === undefined) {
			lastViewStateKey = nextKey;
			return;
		}

		if (lastViewStateKey !== nextKey) {
			rememberEditorViewState(editorViewStates, lastViewStateKey, {
				selection: {
					start: currentInput.selectionStart,
					end: currentInput.selectionEnd,
					direction: currentInput.selectionDirection
				},
				scrollLeft: viewport?.scrollLeft ?? 0,
				scrollTop: viewport?.scrollTop ?? 0
			});
			lastViewStateKey = nextKey;
			const restored = restoreEditorViewState(editorViewStates, nextKey, nextSource.length);
			dismissCompletions();
			clearTooltip();
			void tick().then(() => {
				if (input !== currentInput || editorDocumentKey(props.projectId, props.documentId) !== nextKey || currentInput.value !== nextSource) return;
				currentInput.setSelectionRange(restored.selection.start, restored.selection.end, restored.selection.direction);
				if (viewport) {
					viewport.scrollLeft = restored.scrollLeft;
					viewport.scrollTop = restored.scrollTop;
				}
				syncSelection();
				scheduleScopeGuideMeasurement();
			});
			return;
		}

		if (currentInput.value === nextSource) return;
		const mapped = mapEditorSelection(currentInput.value, nextSource, {
			start: currentInput.selectionStart,
			end: currentInput.selectionEnd,
			direction: currentInput.selectionDirection
		});
		void tick().then(() => {
			if (input !== currentInput || editorDocumentKey(props.projectId, props.documentId) !== nextKey || currentInput.value !== nextSource) return;
			currentInput.setSelectionRange(mapped.start, mapped.end, mapped.direction);
			syncSelection();
		});
	});

	$effect(() => {
		const navigation = props.navigation;
		if (
			!navigation ||
			!props.documentId ||
			navigation.fileId !== props.documentId ||
			navigation.requestId <= lastNavigationRequestId ||
			!input ||
			!viewport
		) return;

		lastNavigationRequestId = navigation.requestId;
		const source = props.source;
		const offset = Math.max(0, Math.min(source.length, Math.trunc(navigation.offset)));
		dismissCompletions();
		clearTooltip();
		input.focus({ preventScroll: true });
		input.setSelectionRange(offset, offset);

		const lineIndex = source.slice(0, offset).split('\n').length - 1;
		const lineTop = verticalPadding + lineIndex * lineHeight;
		if (lineTop < viewport.scrollTop) viewport.scrollTop = lineTop;
		else if (lineTop + lineHeight > viewport.scrollTop + viewport.clientHeight) {
			viewport.scrollTop = lineTop + lineHeight - viewport.clientHeight;
		}
		syncSelection();
		props.onNavigationHandled?.(navigation.requestId);
	});

	$effect(() => {
		const sourceSnapshot = props.source;
		const startSnapshot = selection.start;
		const endSnapshot = selection.end;
		const highlightBrackets = preferences.matchingBracketHighlight;
		const next = highlightBrackets ? matcher(sourceSnapshot, startSnapshot, endSnapshot) : new Set<number>();
		if (!sameOffsetSet(matchingOffsets, next)) matchingOffsets = next;
	});

	$effect(() => {
		props.source;
		scopeRanges;
		lines;
		preferences.indentGuides;
		if (!codeSurface) return;
		if (!preferences.indentGuides) {
			scopeGuidePositions = {};
			return;
		}
		if (measureFrame) cancelAnimationFrame(measureFrame);
		measureFrame = requestAnimationFrame(() => {
			measureFrame = 0;
			measureScopeGuides();
		});
		return () => {
			if (measureFrame) cancelAnimationFrame(measureFrame);
		};
	});

	$effect(() => {
		if (!completion) return;
		const dismissOutside = (event: PointerEvent) => {
			if (event.target instanceof Node && root?.contains(event.target)) return;
			dismissCompletions();
		};
		document.addEventListener('pointerdown', dismissOutside, true);
		return () => document.removeEventListener('pointerdown', dismissOutside, true);
	});

	onMount(() => {
		if (!root || typeof ResizeObserver === 'undefined') return;
		const observer = new ResizeObserver(() => {
			scheduleScopeGuideMeasurement();
			if (tooltip) scheduleTooltipPlacement(tooltip.key);
		});
		observer.observe(root);
		return () => observer.disconnect();
	});

	onDestroy(() => {
		clearTimeout(compileTimer);
		clearTimeout(completionTimer);
		clearTimeout(tooltipTimer);
		if (measureFrame) cancelAnimationFrame(measureFrame);
		compileRequestId += 1;
		completionRequestId += 1;
	});

	function handleInput(event: Event) {
		const target = event.currentTarget as HTMLTextAreaElement;
		syncSelection();
		dismissCompletions();
		clearTooltip();
		props.onChange(target.value);
		scheduleCompletions(false, target.value, target.selectionStart);
	}

	function syncSelection() {
		if (!input) return;
		selection = {
			start: input.selectionStart,
			end: input.selectionEnd,
			direction: input.selectionDirection
		};
	}

	function handleScroll() {
		if (viewport) gutterScrollTop = viewport.scrollTop;
		clearTooltip();
		if (completion && input) positionCompletions(input.selectionStart);
	}

	function replaceInputRange(text: string, start: number, end: number): boolean {
		if (!input) return false;
		let inputEventObserved = false;
		const observeInput = () => { inputEventObserved = true; };
		input.addEventListener('input', observeInput);
		try {
			input.focus();
			input.setSelectionRange(start, end);
			// insertText keeps these programmatic edits in the browser's undo history.
			if (input.ownerDocument.execCommand('insertText', false, text)) return inputEventObserved;
		} catch {
			// Use setRangeText where insertText is unavailable.
		} finally {
			input.removeEventListener('input', observeInput);
		}
		input.setRangeText(text, start, end, 'end');
		return false;
	}

	function applyInputEdit(text: string, start: number, end: number, selectionStart: number, selectionEnd: number) {
		if (!input) return;
		const inputEventObserved = replaceInputRange(text, start, end);
		input.setSelectionRange(selectionStart, selectionEnd);
		syncSelection();
		if (!inputEventObserved) props.onChange(input.value);
	}

	function changeIndentation(outdent: boolean) {
		if (!input) return;
		const edit = getIndentationEdit(input.value, input.selectionStart, input.selectionEnd, outdent, preferences.indentSize);
		if (!edit) return;
		dismissCompletions();
		applyInputEdit(edit.text, edit.rangeStart, edit.rangeEnd, edit.selectionStart, edit.selectionEnd);
	}

	function insertIndentedNewline() {
		if (!input) return;
		const text = input.value;
		const start = input.selectionStart;
		const end = input.selectionEnd;
		const before = text.slice(0, start);
		const after = text.slice(end);
		const lineStart = before.lastIndexOf('\n') + 1;
		const prefix = before.slice(lineStart);
		const indent = prefix.match(/^[\t ]*/)?.[0] ?? '';
		const nextLineBreak = after.search(/[\r\n]/);
		const nextText = after.slice(0, nextLineBreak < 0 ? after.length : nextLineBreak);
		const opensBlock = prefix.trimEnd().endsWith('{');
		const beforeClosingBrace = prefix.trim().length === 0 && /^\s*}/.test(nextText);
		let inserted: string;
		let caret: number | null = null;

		if (opensBlock && /^\s*}/.test(nextText)) {
			const indentationUnit = ' '.repeat(preferences.indentSize);
			inserted = `\n${indent}${indentationUnit}\n${indent}`;
			caret = start + 1 + indent.length + preferences.indentSize;
		} else {
			const indentationUnit = ' '.repeat(preferences.indentSize);
			const nextIndent = beforeClosingBrace
				? indent.slice(0, Math.max(0, indent.length - preferences.indentSize))
				: `${indent}${opensBlock ? indentationUnit : ''}`;
			inserted = `\n${nextIndent}`;
		}
		dismissCompletions();
		if (caret === null) {
			applyInputEdit(inserted, start, end, start + inserted.length, start + inserted.length);
		} else {
			applyInputEdit(inserted, start, end, caret, caret);
		}
	}

	function handleKeydown(event: KeyboardEvent) {
		if (event.key === 'F2' && props.onRenameSymbol && input) {
			event.preventDefault();
			dismissCompletions();
			props.onRenameSymbol(input.selectionStart);
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's' && props.onSave) {
			event.preventDefault();
			void saveSource();
			return;
		}
		if ((event.ctrlKey || event.metaKey) && event.code === 'Space') {
			event.preventDefault();
			scheduleCompletions(true, input?.value, input?.selectionStart);
			return;
		}
		if (completion) {
			if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
				event.preventDefault();
				const direction = event.key === 'ArrowDown' ? 1 : -1;
				selectedCompletion = (selectedCompletion + direction + completion.items.length) % completion.items.length;
				return;
			}
			if (event.key === 'Enter' || event.key === 'Tab') {
				if (acceptCompletion()) event.preventDefault();
				return;
			}
			if (event.key === 'Escape') {
				event.preventDefault();
				dismissCompletions();
				return;
			}
		}
		if (!input || event.isComposing || event.ctrlKey || event.metaKey || event.altKey) return;
		const delimiterEdit = preferences.autoClosePairs
			? getDelimiterEdit(input.value, input.selectionStart, input.selectionEnd, event.key)
			: null;
		if (delimiterEdit) {
			event.preventDefault();
			if (delimiterEdit.kind === 'move-cursor') {
				input.setSelectionRange(delimiterEdit.cursor, delimiterEdit.cursor);
				syncSelection();
				return;
			}
			applyInputEdit(
				delimiterEdit.text,
				delimiterEdit.rangeStart,
				delimiterEdit.rangeEnd,
				delimiterEdit.selectionStart,
				delimiterEdit.selectionEnd
			);
			if (delimiterEdit.refreshCompletions) scheduleCompletions(false, input.value, input.selectionStart);
			return;
		}
		if (event.key === 'Enter' && preferences.autoIndent) {
			event.preventDefault();
			insertIndentedNewline();
		} else if (event.key === 'Tab') {
			event.preventDefault();
			changeIndentation(event.shiftKey);
		} else if (['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) {
			dismissCompletions();
		}
	}

	function handlePointerUp() {
		syncSelection();
		dismissCompletions();
	}

	function handleSymbolClick(event: MouseEvent) {
		if (!(event.ctrlKey || event.metaKey) || !props.onNavigate || !input) return;
		const symbol = findEditorSymbolAtOffset(props.editorSymbols ?? [], input.selectionStart, 'reference');
		if (!symbol) return;
		event.preventDefault();
		dismissCompletions();
		props.onNavigate(symbol);
	}

	async function saveSource() {
		const projectId = props.projectId ?? null;
		const fileId = props.documentId ?? null;
		const documentKey = editorDocumentKey(projectId, fileId);
		const sourceSnapshot = input?.value ?? props.source;
		const readCurrentSource = () => editorDocumentKey(props.projectId, props.documentId) === documentKey
			? input?.value ?? props.source
			: sourceSnapshot;
		const formatted = await formatSourceForSave(readCurrentSource, preferences.formatOnSave, formatSource);
		const context = { projectId, fileId };

		if (editorDocumentKey(props.projectId, props.documentId) !== documentKey) {
			props.onSave?.(formatted, context);
			return;
		}

		let sourceToSave = formatted;
		const sourceBeforeFormat = input?.value;
		if (sourceBeforeFormat !== undefined && formatted !== sourceBeforeFormat) {
			if (!applyFormattedSource(sourceBeforeFormat, formatted)) sourceToSave = input?.value ?? props.source;
		}
		props.onSave?.(sourceToSave, context);
	}

	export async function formatDocument(): Promise<'formatted' | 'unchanged' | 'stale'> {
		const currentInput = input;
		if (!currentInput) return 'stale';
		const documentKey = editorDocumentKey(props.projectId, props.documentId);
		const sourceBeforeFormat = currentInput.value;
		const formatted = await formatSource(sourceBeforeFormat);
		if (
			input !== currentInput ||
			editorDocumentKey(props.projectId, props.documentId) !== documentKey ||
			currentInput.value !== sourceBeforeFormat
		) return 'stale';
		if (formatted === sourceBeforeFormat) return 'unchanged';
		return applyFormattedSource(sourceBeforeFormat, formatted) ? 'formatted' : 'stale';
	}

	function applyFormattedSource(sourceBeforeFormat: string, formatted: string) {
		if (!input || input.value !== sourceBeforeFormat) return false;
		const previousSelection = {
			start: input.selectionStart,
			end: input.selectionEnd,
			direction: input.selectionDirection
		};
		input.setRangeText(formatted, 0, sourceBeforeFormat.length, 'end');
		const mapped = mapEditorSelection(sourceBeforeFormat, formatted, previousSelection);
		input.setSelectionRange(mapped.start, mapped.end, mapped.direction);
		syncSelection();
		props.onChange(formatted);
		return true;
	}

	function handleEditorPointerMove(event: PointerEvent) {
		if (preferences.diagnosticPresentation !== 'hover' || event.pointerType === 'touch' || !viewport || !highlightedLines) {
			clearTooltip();
			return;
		}
		const firstLine = highlightedLines.querySelector<HTMLElement>('[data-line-index="0"]');
		if (!firstLine) {
			clearTooltip();
			return;
		}
		const firstLineTop = firstLine.getBoundingClientRect().top;
		const lineOffset = event.clientY - firstLineTop;
		if (lineOffset < 0 || lineOffset >= lines.length * lineHeight) {
			clearTooltip();
			return;
		}
		const lineIndex = Math.floor(lineOffset / lineHeight);
		const line = lines[lineIndex];
		if (!line?.diagnostics.length) {
			clearTooltip();
			return;
		}
		const renderedLine = highlightedLines.querySelector<HTMLElement>(`[data-line-index="${lineIndex}"]`);
		const context = getSourceMeasureContext();
		if (!renderedLine || !context) {
			clearTooltip();
			return;
		}
		const renderedRect = renderedLine.getBoundingClientRect();
		const localX = event.clientX - renderedRect.left;
		const position = getEditorPositionAtPoint(
			lines,
			localX,
			lineIndex * lineHeight + lineHeight / 2,
			lineHeight,
			(character) => context.measureText(character).width,
			context.measureText(' ').width * preferences.indentSize
		);
		if (!position) {
			clearTooltip();
			return;
		}
		const diagnostic = getEditorDiagnosticAtColumn(line, position.column - 1);
		handleDiagnosticEnter(event, diagnostic);
	}

	function scheduleCompletions(explicit: boolean, text = input?.value, cursor = input?.selectionStart) {
		clearTimeout(completionTimer);
		const requestId = ++completionRequestId;
		if (text === undefined || cursor === undefined) return;
		const update = async () => {
			try {
				const result = props.nodeIds?.length
					? await completeAtWithNodes(text, cursor, props.nodeIds, explicit)
					: await completeAt(text, cursor, explicit);
				if (
					requestId !== completionRequestId ||
					input?.value !== text ||
					input.selectionStart !== cursor
				) return;
				completion = result && result.items.length > 0 ? result : null;
				selectedCompletion = 0;
				if (result) positionCompletions(cursor);
			} catch {
				if (requestId === completionRequestId) completion = null;
			}
		};
		if (explicit) void update();
		else completionTimer = setTimeout(() => void update(), 45);
	}

	function positionCompletions(cursor: number) {
		if (!root || !input) return;
		const before = input.value.slice(0, cursor);
		const linesBefore = before.split('\n');
		const lineIndex = linesBefore.length - 1;
		const column = Array.from(linesBefore.at(-1) ?? '').length;
		const width = Math.min(340, Math.max(190, root.clientWidth - 16));
		const left = Math.max(8, Math.min(8 + column * fontSize * 0.61, root.clientWidth - width - 8));
		const popupHeight = Math.min(280, Math.max(48, (completion?.items.length ?? 1) * 52 + 8));
		const lineTop = verticalPadding + lineIndex * lineHeight - (viewport?.scrollTop ?? 0);
		const top = lineTop + lineHeight + popupHeight > (viewport?.clientHeight ?? root.clientHeight)
			? Math.max(8, lineTop - popupHeight)
			: lineTop + lineHeight;
		completionPosition = { left, top, width };
	}

	function acceptCompletion() {
		if (!completion || !input) return false;
		const item = completion.items[selectedCompletion];
		if (!item) return false;
		applyInputEdit(item.insert_text, completion.start, completion.end, completion.start + item.insert_text.length, completion.start + item.insert_text.length);
		dismissCompletions();
		input.focus();
		return true;
	}

	function selectCompletion(index: number) {
		selectedCompletion = index;
		acceptCompletion();
	}

	function dismissCompletions() {
		clearTimeout(completionTimer);
		completionTimer = undefined;
		completionRequestId += 1;
		completion = null;
	}

	function handleDiagnosticEnter(event: PointerEvent, diagnostic: EditorDiagnostic | null) {
		if (preferences.diagnosticPresentation !== 'hover') {
			clearTooltip();
			return;
		}
		if (!diagnostic) {
			clearTooltip();
			return;
		}
		const key = diagnosticKey(diagnostic);
		const bounds = root?.getBoundingClientRect();
		if (!bounds) return;
		const target: TooltipState = {
			key,
			diagnostic,
			left: event.clientX - bounds.left + 12,
			top: event.clientY - bounds.top + 14,
			maxWidth: Math.max(120, root!.clientWidth - 16),
			maxHeight: Math.max(80, root!.clientHeight - 16)
		};
		if (tooltip?.key === key) {
			tooltip = target;
			scheduleTooltipPlacement(key);
			return;
		}
		if (pendingTooltip?.key === key) {
			pendingTooltip = target;
			return;
		}
		clearTimeout(tooltipTimer);
		tooltip = null;
		pendingTooltip = target;
		tooltipTimer = setTimeout(() => {
			tooltip = pendingTooltip;
			pendingTooltip = null;
			tooltipTimer = undefined;
			if (tooltip?.key === key) scheduleTooltipPlacement(key);
		}, diagnosticHoverDelayMs);
	}

	function diagnosticKey(diagnostic: EditorDiagnostic) {
		return `${diagnostic.code}:${diagnostic.span.start.line}:${diagnostic.span.start.column}:${diagnostic.span.end.line}:${diagnostic.span.end.column}`;
	}

	function refactorDiagnosticFor(diagnostic: EditorDiagnostic) {
		return props.refactorDiagnostics?.find((candidate) =>
			candidate.code === diagnostic.code &&
			candidate.message === diagnostic.message &&
			candidate.span.start.line === diagnostic.span.start.line &&
			candidate.span.start.column === diagnostic.span.start.column &&
			candidate.span.end.line === diagnostic.span.end.line &&
			candidate.span.end.column === diagnostic.span.end.column
		) ?? null;
	}

	function applyQuickFix(diagnostic: RefactorDiagnostic, replacement: string) {
		clearTooltip();
		props.onQuickFix?.(diagnostic, replacement);
	}

	function firstLineDiagnostic(items: readonly EditorDiagnostic[], lineNumber: number) {
		const lineItems = items.filter((item) => item.span.start.line === lineNumber);
		return lineItems.find((item) => item.severity === 'error') ??
			lineItems.find((item) => item.severity === 'warning') ??
			lineItems[0] ?? null;
	}

	function handleDiagnosticLeave(event: PointerEvent) {
		if ((event.relatedTarget as Element | null)?.closest('.editor-diagnostic-tooltip')) return;
		clearTooltip();
	}

	function scheduleTooltipPlacement(key: string) {
		void tick().then(() => {
			if (!tooltip || tooltip.key !== key || !tooltipElement || !root) return;
			const elementRect = tooltipElement.getBoundingClientRect();
			const left = Math.max(8, Math.min(tooltip.left, root.clientWidth - elementRect.width - 8));
			const top = Math.max(8, Math.min(tooltip.top, root.clientHeight - elementRect.height - 8));
			if (left !== tooltip.left || top !== tooltip.top) tooltip = { ...tooltip, left, top };
		});
	}

	function clearTooltip() {
		clearTimeout(tooltipTimer);
		tooltipTimer = undefined;
		pendingTooltip = null;
		tooltip = null;
	}

	function scheduleScopeGuideMeasurement() {
		if (!preferences.indentGuides) {
			scopeGuidePositions = {};
			return;
		}
		if (measureFrame) cancelAnimationFrame(measureFrame);
		measureFrame = requestAnimationFrame(() => {
			measureFrame = 0;
			measureScopeGuides();
		});
	}

	function measureScopeGuides() {
		if (!preferences.indentGuides || !codeSurface || !highlightedLines) return;
		const context = getSourceMeasureContext();
		if (!context) return;
		const surfaceRect = highlightedLines.getBoundingClientRect();
		const next: Record<number, { left: number; top: number; height: number }> = {};
		for (const guide of scopeRanges) {
			const openingLine = highlightedLines.querySelector<HTMLElement>(`[data-line-index="${guide.openLine}"]`);
			const bodyLineIndex = guide.closed ? guide.endLine - 1 : guide.endLine;
			const lastBodyLine = highlightedLines.querySelector<HTMLElement>(`[data-line-index="${bodyLineIndex}"]`);
			if (!openingLine || !lastBodyLine) continue;
			const lineStart = props.source.lastIndexOf('\n', guide.anchorOffset - 1) + 1;
			const sourceColumn = guide.anchorOffset - lineStart;
			const glyphX = measureSourceGlyphInkStartX(openingLine, sourceColumn, context);
			if (glyphX === null) continue;
			next[guide.id] = getScopeGuideGeometry(
				surfaceRect,
				openingLine.getBoundingClientRect(),
				lastBodyLine.getBoundingClientRect(),
				glyphX
			);
		}
		scopeGuidePositions = next;
	}

	function getSourceMeasureContext() {
		if (!codeSurface) return null;
		sourceMeasureCanvas ??= document.createElement('canvas');
		const context = sourceMeasureCanvas.getContext('2d');
		if (context) context.font = getComputedStyle(codeSurface).font;
		return context;
	}

	function sameOffsetSet(left: ReadonlySet<number>, right: ReadonlySet<number>) {
		if (left.size !== right.size) return false;
		for (const offset of left) if (!right.has(offset)) return false;
		return true;
	}

	function caretOffset(value: { start: number; end: number; direction: 'forward' | 'backward' | 'none' }) {
		return value.direction === 'backward' ? value.start : value.end;
	}

</script>

<div
	bind:this={root}
	class="source-editor"
	role="group"
	aria-label={props.label ?? studioMessage('editor.sourceLabel')}
	title={props.editorSymbols?.length ? 'Ctrl/Cmd-click a reference to navigate; F2 renames the symbol at the caret' : undefined}
	style={`--editor-font-size:${fontSize}px;--editor-line-height:${lineHeight}px;--editor-indent-size:${preferences.indentSize};--editor-gutter-width:${preferences.lineNumbers ? '3.5rem' : '0px'}`}
	data-diagnostic-presentation={preferences.diagnosticPresentation}
	onpointerleave={clearTooltip}
>
	<div class="editor-gutter" class:editor-gutter-hidden={!preferences.lineNumbers} aria-hidden="true">
		<div bind:this={gutterLines} style:transform={`translateY(-${gutterScrollTop}px)`}>
			{#each lines as line (line.index)}
				<div class="editor-gutter-line" class:editor-gutter-error={line.hasErrors}>
					{#if preferences.lineNumbers}{line.index + 1}{/if}
				</div>
			{/each}
		</div>
	</div>
	<div bind:this={viewport} class="editor-viewport" onscroll={handleScroll}>
		<div bind:this={codeSurface} class="editor-content">
			<div bind:this={highlightedLines} class="editor-highlight" aria-hidden="true">
				{#if preferences.indentGuides}
					{#each scopeRanges as guide (guide.id)}
						{@const position = scopeGuidePositions[guide.id]}
						<span
							class="scope-guide"
							data-scope-guide={guide.id}
							class:scope-guide-active={guide.id === activeScopeId}
							style:left={`${position?.left ?? 0}px`}
							style:top={`${position?.top ?? 0}px`}
							style:height={`${position?.height ?? 0}px`}
						></span>
					{/each}
				{/if}
				{#each lines as line (line.index)}
					<div
						data-line-index={line.index}
						class="editor-line"
						class:editor-line-active={preferences.activeLineHighlight && activeLineIndex === line.index}
						role="presentation"
					>
						{#each line.segments as segment, segmentIndex (`${line.index}-${segmentIndex}`)}
							<span class={segment.className}>{segment.text}</span>
						{/each}
						{#if preferences.diagnosticPresentation === 'inline'}
							{@const inlineDiagnostic = firstLineDiagnostic(line.diagnostics, line.index + 1)}
							{#if inlineDiagnostic}
								<span class={`inline-diagnostic inline-diagnostic-${inlineDiagnostic.severity}`} aria-hidden="true">
									{inlineDiagnostic.message}
								</span>
							{/if}
						{/if}
					</div>
				{/each}
			</div>
			<textarea
				bind:this={input}
				class="editor-input"
				value={props.source}
				disabled={props.disabled ?? false}
				aria-label={props.label ?? studioMessage('editor.sourceLabel')}
				onclick={handleSymbolClick}
				aria-invalid={visibleDiagnostics.some((item) => item.severity === 'error')}
				aria-describedby={visibleDiagnostics.length ? diagnosticListId : undefined}
				aria-autocomplete="list"
				aria-controls={completion ? completionListId : undefined}
				aria-activedescendant={completionActiveId}
				spellcheck="false"
				autocapitalize="off"
				autocomplete="off"
				onpointermove={handleEditorPointerMove}
				onpointerleave={handleDiagnosticLeave}
				oninput={handleInput}
				onkeydown={handleKeydown}
				onkeyup={syncSelection}
				onselect={syncSelection}
				onpointerup={handlePointerUp}
				onblur={dismissCompletions}
			></textarea>
		</div>
	</div>
	{#if tooltip}
		<div
			bind:this={tooltipElement}
			class="editor-diagnostic-tooltip"
			class:diagnostic-error={tooltip.diagnostic.severity === 'error'}
			class:diagnostic-warning={tooltip.diagnostic.severity === 'warning'}
			class:diagnostic-information={tooltip.diagnostic.severity === 'information'}
			role="status"
			style:left={`${tooltip.left}px`}
			style:top={`${tooltip.top}px`}
			style:max-width={`${tooltip.maxWidth}px`}
			style:max-height={`${tooltip.maxHeight}px`}
			onpointerleave={handleDiagnosticLeave}
		>
			<strong>{tooltip.diagnostic.severity}</strong>: {tooltip.diagnostic.message}
			{#if tooltip.diagnostic.hint}<div class="diagnostic-hint">{tooltip.diagnostic.hint}</div>{/if}
			{#if tooltipQuickFixes.options.length > 0 && props.onQuickFix && tooltipQuickFixes.diagnostic}
				<div class="diagnostic-quick-fixes">
						<span>{studioMessage('editor.quickFixReplace')}</span>
					{#each tooltipQuickFixes.options as quickFix (`${tooltipQuickFixes.diagnostic.code}-${quickFix.replacement}`)}
						<button
							type="button"
							disabled={props.quickFixDisabled}
							onpointerdown={(event) => event.preventDefault()}
							onclick={(event) => {
								event.stopPropagation();
								if (tooltipQuickFixes.diagnostic) applyQuickFix(tooltipQuickFixes.diagnostic, quickFix.replacement);
							}}
						>{quickFix.replacement}</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}
	{#if visibleDiagnostics.length}
		<ul id={diagnosticListId} class="sr-only" aria-label={studioMessage('editor.diagnosticList')}>
			{#each visibleDiagnostics as diagnostic, index (`${diagnostic.code}-${index}`)}
				<li>{describeDiagnostic(diagnostic)}</li>
			{/each}
		</ul>
	{/if}
	{#if completion}
		<EditorCompletions
			id={completionListId}
			items={completion.items}
			selectedIndex={selectedCompletion}
			left={completionPosition.left}
			top={completionPosition.top}
			width={completionPosition.width}
			onSelect={selectCompletion}
		/>
	{/if}
</div>

<style>
	.source-editor {
		position: relative;
		display: flex;
		width: 100%;
		height: 100%;
		min-height: 12rem;
		overflow: hidden;
		background: var(--background);
		color: var(--foreground);
		font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace;
		font-size: var(--editor-font-size);
		line-height: var(--editor-line-height);
	}
	.editor-gutter {
		position: absolute;
		z-index: 4;
		top: 0;
		bottom: 0;
		left: 0;
		width: var(--editor-gutter-width);
		overflow: hidden;
		border-right: 1px solid var(--border);
		background: var(--muted);
		color: var(--muted-foreground);
		text-align: right;
		user-select: none;
		pointer-events: none;
	}
	.editor-gutter-hidden { border-right: 0; }
	.editor-gutter > div { padding-top: 0.5rem; }
	.editor-gutter-line {
		box-sizing: border-box;
		height: var(--editor-line-height);
		padding-right: 0.75rem;
		font-size: calc(var(--editor-font-size) * 0.8);
		line-height: var(--editor-line-height);
	}
	.editor-gutter-error { color: var(--destructive); font-weight: 600; }
	.editor-viewport {
		position: absolute;
		inset: 0 0 0 var(--editor-gutter-width);
		overflow: auto;
		overscroll-behavior: contain;
		scrollbar-gutter: stable;
	}
	.editor-content {
		position: relative;
		width: max-content;
		min-width: 100%;
		min-height: 100%;
		padding: 0.5rem 1rem;
		box-sizing: border-box;
	}
	.editor-highlight {
		position: relative;
		z-index: 1;
		width: max-content;
		min-width: 100%;
		white-space: normal;
		tab-size: var(--editor-indent-size);
		font: inherit;
		font-variant-ligatures: none;
		pointer-events: none;
	}
	.editor-line {
		position: relative;
		z-index: 1;
		width: max-content;
		min-width: 100%;
		height: var(--editor-line-height);
		white-space: pre;
		line-height: var(--editor-line-height);
	}
	.editor-line-active { background: color-mix(in srgb, var(--foreground) 3%, transparent); }
	.inline-diagnostic {
		margin-left: 0.75rem;
		font-family: var(--font-sans, sans-serif);
		font-size: calc(var(--editor-font-size) * 0.82);
		font-weight: 400;
		line-height: var(--editor-line-height);
		white-space: nowrap;
	}
	.inline-diagnostic-error { color: var(--destructive); }
	.inline-diagnostic-warning { color: #9a6500; }
	.inline-diagnostic-information { color: var(--primary); }
	.scope-guide {
		position: absolute;
		z-index: 0;
		width: 1px;
		background: color-mix(in srgb, var(--foreground) 16%, transparent);
		pointer-events: none;
	}
	.scope-guide-active { background: color-mix(in srgb, var(--primary) 62%, transparent); }
	.editor-input {
		position: absolute;
		z-index: 3;
		inset: 0;
		box-sizing: border-box;
		width: 100%;
		min-width: 100%;
		height: 100%;
		min-height: 100%;
		resize: none;
		overflow: hidden;
		padding: inherit;
		border: 0;
		outline: none;
		background: transparent;
		color: transparent;
		caret-color: var(--foreground);
		-webkit-text-fill-color: transparent;
		font: inherit;
		font-variant-ligatures: none;
		line-height: var(--editor-line-height);
		tab-size: var(--editor-indent-size);
		white-space: pre;
	}
	.editor-input::selection { background: color-mix(in srgb, var(--primary) 30%, transparent); }
	.editor-input:disabled { cursor: wait; opacity: 1; }
	.syntax-keyword { color: var(--primary); font-weight: 700; }
	.syntax-property { color: #9251b2; font-weight: 600; }
	.syntax-value { color: #23784b; font-weight: 600; }
	.syntax-operator, .syntax-punctuation { color: var(--muted-foreground); }
	.syntax-number { color: #9a6500; font-weight: 600; }
	.syntax-string { color: #23784b; }
	.syntax-comment { color: var(--muted-foreground); font-style: italic; }
	.syntax-identifier { color: var(--foreground); }
	.editor-matching-bracket {
		border-radius: 1px;
		background: color-mix(in srgb, var(--primary) 15%, transparent);
		box-shadow: inset 0 -2px 0 color-mix(in srgb, var(--primary) 54%, transparent);
	}
	.editor-diagnostic-error, .editor-diagnostic-warning, .editor-diagnostic-information {
		text-decoration-line: underline;
		text-underline-offset: 2px;
	}
	.editor-diagnostic-error { text-decoration-style: wavy; text-decoration-color: var(--destructive); }
	.editor-diagnostic-warning { text-decoration-style: wavy; text-decoration-color: #b77900; }
	.editor-diagnostic-information { text-decoration-style: dotted; text-decoration-color: var(--primary); }
	.editor-diagnostic-tooltip {
		position: absolute;
		z-index: 10;
		box-sizing: border-box;
		border: 1px solid var(--border);
		border-radius: 0.5rem;
		background: var(--popover);
		color: var(--popover-foreground);
		box-shadow: 0 8px 24px rgb(15 35 65 / 16%);
	}
	.editor-diagnostic-tooltip {
		max-width: calc(100% - 1rem);
		overflow: auto;
		padding: 0.5rem 0.75rem;
		font-family: var(--font-sans, sans-serif);
		font-size: 0.75rem;
		line-height: 1.25rem;
		pointer-events: auto;
	}
	.editor-diagnostic-tooltip strong { text-transform: capitalize; }
	.editor-diagnostic-tooltip.diagnostic-error { border-color: color-mix(in srgb, var(--destructive) 48%, var(--border)); }
	.editor-diagnostic-tooltip.diagnostic-error strong { color: var(--destructive); }
	.editor-diagnostic-tooltip.diagnostic-warning strong { color: #9a6500; }
	.editor-diagnostic-tooltip.diagnostic-information strong { color: var(--primary); }
	.diagnostic-hint { margin-top: 0.25rem; color: var(--muted-foreground); }
	.diagnostic-quick-fixes { display: flex; flex-wrap: wrap; align-items: center; gap: 0.35rem; margin-top: 0.5rem; padding-top: 0.45rem; border-top: 1px solid var(--border); color: var(--muted-foreground); }
	.diagnostic-quick-fixes button { padding: 0.12rem 0.35rem; border: 1px solid var(--border); border-radius: 0.25rem; color: var(--foreground); background: var(--background); font: inherit; font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', monospace; cursor: pointer; }
	.diagnostic-quick-fixes button:hover:not(:disabled) { background: var(--accent); }
	.diagnostic-quick-fixes button:disabled { cursor: wait; opacity: 0.55; }
	@media (prefers-reduced-motion: reduce) {
		.editor-viewport { scroll-behavior: auto; }
	}
</style>
