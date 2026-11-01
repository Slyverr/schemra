import { tv } from 'tailwind-variants';

export const settingsCategoryVariants = tv({
	base: 'flex min-h-9 w-full min-w-0 justify-start gap-2 rounded-none px-3 py-[.3rem] text-left text-[.8125rem] font-normal text-muted-foreground hover:bg-muted hover:text-foreground max-[35rem]:gap-1.5 max-[35rem]:px-[.4rem] max-[35rem]:text-xs [&_svg]:shrink-0 max-[35rem]:[&_svg]:size-5',
	variants: {
		active: {
			true: 'bg-muted font-semibold text-foreground',
			false: ''
		}
	}
});

export const preferenceRowVariants = tv({
	base: 'flex min-w-0 items-center justify-between gap-2',
	variants: {
		variant: {
			card: 'min-h-[4.5rem] cursor-pointer rounded-lg border border-[color-mix(in_srgb,var(--border)_80%,transparent)] bg-[color-mix(in_srgb,var(--muted)_22%,var(--background))] p-2',
			inline: 'min-h-[2.75rem] cursor-pointer px-2 py-[.4rem]'
		}
	}
});

export const settingsClasses = {
	panel: 'grid min-h-full grid-cols-[9rem_minmax(0,1fr)] text-foreground max-[35rem]:grid-cols-[7rem_minmax(0,1fr)]',
	navigation: 'grid content-start border-r border-border bg-[color-mix(in_srgb,var(--muted)_25%,transparent)]',
	navigationLabel: 'mb-1.5 mt-0 px-3 pt-2.5 text-[.625rem] font-semibold uppercase tracking-[.06em] text-muted-foreground',
	content: '@container/settings-panel min-w-0 px-4 py-3 max-[35rem]:px-2',
	section: 'grid min-w-0 gap-3',
	sectionHeading: 'mb-0.5 grid gap-0.5',
	sectionTitle: 'm-0 text-sm font-[650]',
	settingGrid: 'grid grid-cols-2 gap-2',
	preferenceGrid: 'grid grid-cols-2 gap-2 @max-[34rem]/settings-panel:grid-cols-1',
	settingLabel: 'text-xs font-semibold',
	selectTrigger: 'h-8 w-full px-2 text-xs',
	themeOptions: 'grid grid-cols-3 gap-1.5 @max-[30rem]/settings-panel:grid-cols-2 @max-[22rem]/settings-panel:grid-cols-1',
	themeOption: 'grid min-w-0 cursor-pointer gap-1.5 rounded-md border border-[color-mix(in_srgb,var(--border)_75%,transparent)] bg-[color-mix(in_srgb,var(--muted)_22%,var(--background))] p-2 has-[:checked]:border-ring has-[:checked]:bg-[color-mix(in_srgb,var(--accent)_42%,var(--background))] has-[:focus-visible]:outline has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-ring has-[:focus-visible]:outline-offset-2',
	themeOptionLabel: 'flex min-w-0 items-center gap-1.5 text-xs font-[550] leading-tight',
	syntaxPreview: 'grid min-w-0 gap-0.5 overflow-hidden rounded border border-[var(--editor-border)] bg-[var(--editor-background)] px-1.5 py-1 text-[.625rem] leading-[1.35] text-[var(--editor-foreground)]',
	syntaxCode: 'whitespace-nowrap',
	preferenceCopy: 'grid min-w-0 gap-0.5',
	preferenceTitle: 'text-xs font-[550] leading-tight',
	preferenceDescription: 'text-[.6875rem] leading-[1.3] text-muted-foreground',
	appearanceOptions: 'border-y border-[color-mix(in_srgb,var(--border)_70%,transparent)]'
} as const;

export const themePickerClasses = {
	trigger: 'flex h-auto min-h-12 w-full items-center justify-start gap-3 rounded-[.45rem] px-2 py-1 text-left text-xs font-normal whitespace-normal text-foreground shadow-none hover:border-ring hover:bg-accent hover:text-foreground aria-expanded:border-ring aria-expanded:bg-accent',
	triggerCopy: 'grid min-w-0 flex-1 gap-px text-left',
	popover: 'z-[1] grid max-h-[min(22rem,60vh,var(--bits-floating-available-height,60vh))] w-[min(24rem,var(--bits-floating-available-width,24rem))] gap-1.5 overflow-hidden rounded-[.55rem] border border-border bg-popover p-2 text-popover-foreground shadow-[0_.65rem_1.8rem_rgb(7_21_41_/_16%)]',
	popoverContent: 'grid min-w-0 gap-1.5',
	search: 'h-[2.1rem] text-xs',
	list: 'grid min-h-0 gap-[.15rem] overflow-auto p-0',
	option: 'flex min-h-12 w-full items-center justify-start gap-3 rounded-[.35rem] px-[.4rem] py-1 text-left text-xs font-normal text-foreground hover:bg-accent hover:text-foreground',
	activeOption: 'bg-accent',
	empty: 'm-0 p-2.5 text-xs text-muted-foreground'
} as const;
