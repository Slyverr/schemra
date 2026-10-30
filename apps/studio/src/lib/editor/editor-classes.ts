export const editorClasses = {
	root: 'source-editor relative flex h-full min-h-48 w-full flex-col overflow-hidden bg-[var(--editor-background,var(--background))] font-mono text-[length:var(--editor-font-size)] leading-[var(--editor-line-height)] text-[var(--editor-foreground,var(--foreground))]',
	gutter: 'editor-gutter pointer-events-none absolute inset-y-0 left-0 z-[4] w-[var(--editor-gutter-width)] select-none overflow-hidden border-r border-[var(--editor-border)] bg-[var(--editor-gutter-background)] text-right text-[var(--editor-gutter-foreground)]',
	gutterLines: 'pt-2',
	gutterLine: 'editor-gutter-line box-border h-[var(--editor-line-height)] pr-3 text-[calc(var(--editor-font-size)*0.8)] leading-[var(--editor-line-height)]',
	gutterError: 'text-[var(--editor-error)] font-semibold',
	viewport: 'editor-viewport absolute inset-y-0 right-0 left-[var(--editor-gutter-width)] overflow-auto overscroll-contain [scrollbar-gutter:stable] motion-reduce:scroll-auto',
	content: 'editor-content relative box-border min-h-full min-w-full w-max px-4 py-2',
	highlight: 'editor-highlight pointer-events-none relative z-[1] min-w-full w-max whitespace-normal [tab-size:var(--editor-indent-size)] [font:inherit] [font-variant-ligatures:none]',
	line: 'editor-line relative z-[1] h-[var(--editor-line-height)] min-w-full w-max whitespace-pre leading-[var(--editor-line-height)]',
	activeLine: 'bg-[color-mix(in_srgb,var(--editor-foreground)_3%,transparent)]',
	inlineDiagnostic: 'inline-diagnostic ml-3 whitespace-nowrap font-sans text-[calc(var(--editor-font-size)*0.82)] font-normal leading-[var(--editor-line-height)]',
	inlineDiagnosticError: 'inline-diagnostic-error text-[var(--editor-error)]',
	inlineDiagnosticWarning: 'inline-diagnostic-warning text-warning',
	inlineDiagnosticInformation: 'inline-diagnostic-information text-primary',
	scopeGuide: 'scope-guide pointer-events-none absolute z-0 w-px bg-[color-mix(in_srgb,var(--editor-foreground)_16%,transparent)]',
	activeScopeGuide: 'bg-[color-mix(in_srgb,var(--primary)_36%,transparent)]',
	input: 'editor-input absolute inset-0 z-[3] box-border h-full min-h-full w-full min-w-full resize-none overflow-hidden border-0 bg-transparent p-[inherit] text-transparent caret-[var(--editor-caret)] outline-none [-webkit-text-fill-color:transparent] [font:inherit] [font-variant-ligatures:none] leading-[var(--editor-line-height)] [tab-size:var(--editor-indent-size)] whitespace-pre selection:bg-[var(--editor-selection)] disabled:cursor-wait disabled:opacity-100',
	matchingBracket: 'editor-matching-bracket rounded-[1px] bg-[color-mix(in_srgb,var(--primary)_15%,transparent)] shadow-[inset_0_-2px_0_color-mix(in_srgb,var(--primary)_54%,transparent)]',
	diagnosticError: 'editor-diagnostic-error underline decoration-wavy decoration-[var(--editor-error)] underline-offset-2',
	diagnosticWarning: 'editor-diagnostic-warning underline decoration-wavy decoration-warning underline-offset-2',
	diagnosticInformation: 'editor-diagnostic-information underline decoration-dotted decoration-primary underline-offset-2',
	tooltip: 'editor-diagnostic-tooltip pointer-events-auto absolute z-10 box-border max-w-[calc(100%_-_1rem)] overflow-auto rounded-lg border border-border bg-popover px-3 py-2 font-sans text-xs leading-5 text-popover-foreground shadow-[0_8px_24px_rgb(15_35_65_/_16%)]',
	tooltipError: 'border-[color-mix(in_srgb,var(--editor-error)_48%,var(--border))]',
	tooltipSeverity: 'capitalize',
	tooltipHint: 'mt-1 text-muted-foreground',
	quickFixes: 'mt-2 flex flex-wrap items-center gap-[.35rem] border-t border-border pt-[.45rem] text-muted-foreground',
	quickFixButton: 'h-auto rounded-[.25rem] border-border px-[.35rem] py-[.12rem] font-mono text-xs text-foreground disabled:cursor-wait disabled:opacity-[.55]'
} as const;

export const editorSyntaxClasses = {
	keyword: 'syntax-keyword text-[var(--editor-syntax-keyword)] font-bold',
	property: 'syntax-property text-[var(--editor-syntax-property)] font-semibold',
	value: 'syntax-value text-[var(--editor-syntax-value)] font-semibold',
	operator: 'syntax-operator text-[var(--editor-syntax-punctuation)]',
	punctuation: 'syntax-punctuation text-[var(--editor-syntax-punctuation)]',
	number: 'syntax-number text-[var(--editor-syntax-number)] font-semibold',
	string: 'syntax-string text-[var(--editor-syntax-string)]',
	comment: 'syntax-comment text-[var(--editor-syntax-comment)] italic',
	identifier: 'syntax-identifier text-[var(--editor-foreground,var(--foreground))]'
} as const;

export const editorDiagnosticClasses = {
	error: editorClasses.diagnosticError,
	warning: editorClasses.diagnosticWarning,
	information: editorClasses.diagnosticInformation
} as const;

export const tooltipSeverityClasses = {
	error: 'text-[var(--editor-error)]',
	warning: 'text-warning',
	information: 'text-primary'
} as const;

export const inlineDiagnosticClasses = {
	error: `${editorClasses.inlineDiagnostic} ${editorClasses.inlineDiagnosticError}`,
	warning: `${editorClasses.inlineDiagnostic} ${editorClasses.inlineDiagnosticWarning}`,
	information: `${editorClasses.inlineDiagnostic} ${editorClasses.inlineDiagnosticInformation}`
} as const;
