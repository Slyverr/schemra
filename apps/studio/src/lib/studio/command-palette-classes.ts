export const commandPaletteClasses = {
	content:
		'top-[min(14vh,7rem)] max-h-[min(78vh,46rem)] w-full max-w-[calc(100%_-_2rem)] translate-y-0 flex flex-col gap-0 overflow-hidden rounded-xl border border-border bg-popover p-0 text-popover-foreground shadow-2xl ring-0 sm:max-w-[42rem] max-[36rem]:top-2 max-[36rem]:max-h-[calc(100dvh-1rem)]',
	overlay: 'bg-foreground/45 backdrop-blur-[2px]',
	heading: 'flex-row items-center gap-3 px-4 pb-3 pt-4 max-[36rem]:px-3 max-[36rem]:pt-3',
	icon: 'grid size-9 shrink-0 place-items-center rounded-[.65rem] border border-border bg-muted text-primary',
	copy: 'min-w-0 flex-1',
	title: 'text-[.95rem] font-semibold leading-none tracking-[-.01em]',
	description: 'mt-1 text-xs leading-[1.4] text-muted-foreground max-[36rem]:text-[.69rem]',
	close: 'rounded-lg text-muted-foreground hover:bg-muted hover:text-foreground',
	search:
		'mx-3 flex min-h-12 items-center gap-2.5 rounded-[.65rem] border border-input bg-background px-3 text-muted-foreground focus-within:border-ring focus-within:ring-2 focus-within:ring-ring/20',
	input:
		'h-[2.8rem] min-w-0 flex-1 border-0 bg-transparent text-sm text-foreground outline-none placeholder:text-muted-foreground/85 focus-visible:ring-0',
	keyboardHint:
		'rounded-[.3rem] border border-border bg-muted px-1.5 py-0.5 font-mono text-[.65rem] font-medium leading-[1.35] text-muted-foreground',
	results: 'min-h-16 overflow-auto overscroll-contain px-2 pb-3 pt-2',
	empty: 'm-0 px-4 py-8 text-center text-[.82rem] text-muted-foreground',
	group: 'space-y-1',
	groupTitle:
		'flex justify-between px-2.5 pb-1 pt-[.45rem] text-[.68rem] font-semibold uppercase tracking-[.06em] text-muted-foreground',
	groupCount: 'tabular-nums font-medium normal-case tracking-normal',
	option:
		'flex min-h-12 w-full items-center gap-2.5 rounded-[.55rem] border border-transparent px-[.55rem] py-[.42rem] text-left text-foreground transition-colors hover:border-primary/15 hover:bg-accent hover:text-accent-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 disabled:cursor-not-allowed disabled:opacity-45 disabled:hover:border-transparent disabled:hover:bg-transparent disabled:hover:text-foreground max-[36rem]:gap-[.45rem] max-[36rem]:px-[.4rem]',
	optionIcon: 'grid size-[1.8rem] shrink-0 place-items-center rounded-[.45rem] bg-muted text-muted-foreground',
	optionIconActive: 'bg-primary/10 text-primary',
	optionCopy: 'flex min-w-0 flex-1 flex-col gap-0.5',
	optionTitle: 'truncate text-[.82rem] font-medium',
	optionDetail: 'truncate text-[.7rem] text-muted-foreground max-[36rem]:max-w-[min(70vw,22rem)]',
	badge: 'shrink-0 rounded-full px-2 py-0.5 text-[.64rem] leading-tight capitalize',
	resourceKind: 'border border-border text-muted-foreground',
	current: 'bg-secondary text-secondary-foreground normal-case',
	unsaved:
		'bg-[color-mix(in_srgb,var(--warning)_14%,var(--background))] text-[var(--warning)] normal-case',
	shortcut: 'rounded-[.3rem] border border-border bg-muted px-1.5 py-0.5 font-mono text-[.65rem] font-medium leading-[1.35] text-muted-foreground',
	footer:
		'flex min-h-10 items-center justify-between gap-4 border-t border-border px-3.5 py-1.5 text-[.68rem] text-muted-foreground',
	footerHint: 'inline-flex items-center gap-1',
	footerKey:
		'min-w-5 rounded-[.3rem] border border-border bg-muted px-1 py-px text-center font-mono text-[.65rem] font-medium leading-[1.35] text-muted-foreground'
} as const;
