<script lang="ts">
	import { cn } from '../utils.js';
	import { studioMessage } from '../i18n/messages.js';

	interface CompletionItem {
		label: string;
		detail: string;
		kind: string;
	}

	interface Props {
		id: string;
		items: readonly CompletionItem[];
		selectedIndex: number;
		left: number;
		top: number;
		width: number;
		onSelect: (index: number) => void;
	}

	let { id, items, selectedIndex, left, top, width, onSelect }: Props = $props();
</script>

<ul
	id={id}
	class="absolute z-10 box-border m-0 max-h-[17.5rem] list-none overflow-y-auto rounded-lg border border-border bg-popover p-1 text-popover-foreground shadow-[0_8px_24px_rgb(15_35_65_/_16%)]"
	role="listbox"
	aria-label={studioMessage('editor.suggestions')}
	style:left={`${left}px`}
	style:top={`${top}px`}
	style:width={`${width}px`}
>
	{#each items as item, index (index)}
		<li role="presentation">
			<button
				id={`${id}-${index}`}
				class={cn(
					'block w-full cursor-pointer rounded border-0 p-2 text-left font-[inherit] text-inherit hover:bg-accent hover:text-accent-foreground',
					index === selectedIndex && 'bg-accent text-accent-foreground'
				)}
				role="option"
				aria-selected={index === selectedIndex}
				type="button"
				tabindex="-1"
				title={item.detail}
				onmousedown={(event) => event.preventDefault()}
				onclick={() => onSelect(index)}
			>
				<span class="flex items-center gap-2"><code class="flex-1 truncate font-[inherit]">{item.label}</code><small class="text-[.625rem] text-muted-foreground uppercase">{item.kind}</small></span>
				<span class="mt-0.5 block font-sans text-[.6875rem] leading-4 text-muted-foreground">{item.detail}</span>
			</button>
		</li>
	{/each}
</ul>
