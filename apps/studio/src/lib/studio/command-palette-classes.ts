export const commandPaletteClasses = {
	content:
		'top-1/3 max-h-[min(78vh,36rem)] w-full max-w-[calc(100%_-_2rem)] translate-y-0 flex flex-col gap-0 overflow-hidden rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-2xl ring-0 sm:max-w-lg max-[36rem]:top-2 max-[36rem]:max-h-[calc(100dvh-1rem)]',
	overlay: 'bg-foreground/45 backdrop-blur-[2px]',
	root: 'flex min-h-0 w-full flex-col overflow-hidden rounded-xl bg-popover p-1',
	search:
		'mx-1 mt-1 flex h-8 min-h-8 items-center gap-2 rounded-lg border border-input/50 bg-background px-2.5 text-muted-foreground focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20',
	input:
		'h-7 min-w-0 flex-1 border-0 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground/85 focus-visible:ring-0',
	results: 'min-h-16 max-h-[min(24rem,60vh)] overflow-auto overscroll-contain px-1 pb-2 pt-1 outline-none',
	empty: 'm-0 px-4 py-8 text-center text-[.82rem] text-muted-foreground',
	group: 'overflow-hidden p-1',
	groupTitle:
		'flex justify-between px-2 py-1.5 text-xs font-medium text-muted-foreground',
	groupCount: 'tabular-nums font-medium normal-case tracking-normal',
	option:
		'relative flex min-h-10 w-full cursor-default items-center gap-3 rounded-lg px-2 py-1.5 text-left text-foreground outline-none transition-colors hover:bg-muted focus-visible:bg-muted disabled:pointer-events-none disabled:opacity-50',
	optionIcon: 'grid size-[1.8rem] shrink-0 place-items-center rounded-[.45rem] bg-muted text-muted-foreground',
	optionIconActive: 'bg-primary/10 text-primary',
	optionCopy: 'flex min-w-0 flex-1 flex-col gap-0.5',
	optionTitle: 'truncate text-[.82rem] font-medium',
	optionDetail: 'truncate text-[.7rem] text-muted-foreground max-[36rem]:max-w-[min(70vw,22rem)]',
	badge: 'shrink-0 rounded-full px-2 py-0.5 text-[.64rem] leading-tight capitalize',
	current: 'bg-secondary text-secondary-foreground normal-case',
	unsaved:
		'bg-[color-mix(in_srgb,var(--warning)_14%,var(--background))] text-warning normal-case',
	shortcut: 'rounded-[.3rem] border border-border bg-muted px-1.5 py-0.5 font-mono text-[.65rem] font-medium leading-[1.35] text-muted-foreground',
	footer:
		'flex min-h-10 items-center justify-between gap-4 border-t border-border px-3.5 py-1.5 text-[.68rem] text-muted-foreground',
	footerHint: 'inline-flex items-center gap-1',
	footerKey:
		'min-w-5 rounded-[.3rem] border border-border bg-muted px-1 py-px text-center font-mono text-[.65rem] font-medium leading-[1.35] text-muted-foreground'
} as const;
