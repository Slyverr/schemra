export type ThemePickerNavigationKey = 'ArrowDown' | 'ArrowUp' | 'Home' | 'End';

export function nextThemeOptionIndex(
	currentIndex: number,
	optionCount: number,
	key: ThemePickerNavigationKey
): number {
	if (optionCount <= 0) return -1;
	const index = Number.isFinite(currentIndex)
		? Math.min(optionCount - 1, Math.max(0, Math.floor(currentIndex)))
		: 0;

	switch (key) {
		case 'ArrowDown': return (index + 1) % optionCount;
		case 'ArrowUp': return (index - 1 + optionCount) % optionCount;
		case 'Home': return 0;
		case 'End': return optionCount - 1;
	}
}
