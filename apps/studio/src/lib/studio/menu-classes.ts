const popoverPanelClass = 'border border-border bg-popover text-popover-foreground';

export const dropdownPanelClass =
	`grid gap-[.15rem] p-[.45rem] shadow-[0_.75rem_2rem_rgb(7_21_41_/_18%)] ${popoverPanelClass}`;

export const contextMenuPanelClass =
	`w-[12.5rem] rounded-[.55rem] p-1 shadow-[0_10px_28px_rgb(15_35_65_/_18%)] ${popoverPanelClass}`;

export const dropdownGroupHeadingClass =
	'px-[.45rem] py-[.3rem] text-[.63rem] font-[650] tracking-[.06em] text-muted-foreground uppercase';

export const dropdownItemClass =
	'flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50';

export const contextMenuItemClass =
	'flex min-h-[2.15rem] w-full cursor-pointer items-center gap-[.55rem] rounded-[.35rem] px-[.55rem] py-[.35rem] text-left font-[inherit] text-[.8rem] text-popover-foreground outline-none data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:cursor-not-allowed aria-disabled:opacity-50';
