<script lang="ts">
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
	class="editor-completions"
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
				class="completion-option"
				class:completion-selected={index === selectedIndex}
				role="option"
				aria-selected={index === selectedIndex}
				type="button"
				tabindex="-1"
				title={item.detail}
				onmousedown={(event) => event.preventDefault()}
				onclick={() => onSelect(index)}
			>
				<span class="completion-heading"><code>{item.label}</code><small>{item.kind}</small></span>
				<span class="completion-detail">{item.detail}</span>
			</button>
		</li>
	{/each}
</ul>

<style>
	.editor-completions {
		position: absolute;
		z-index: 10;
		box-sizing: border-box;
		max-height: 17.5rem;
		margin: 0;
		overflow-y: auto;
		padding: 0.25rem;
		border: 1px solid var(--border);
		border-radius: 0.5rem;
		background: var(--popover);
		color: var(--popover-foreground);
		box-shadow: 0 8px 24px rgb(15 35 65 / 16%);
		list-style: none;
	}
	.completion-option {
		display: block;
		width: 100%;
		padding: 0.5rem;
		border: 0;
		border-radius: 0.25rem;
		background: transparent;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.completion-selected, .completion-option:hover { background: var(--accent); color: var(--accent-foreground); }
	.completion-heading { display: flex; align-items: center; gap: 0.5rem; }
	.completion-heading code { flex: 1; overflow: hidden; font: inherit; text-overflow: ellipsis; white-space: nowrap; }
	.completion-heading small { color: var(--muted-foreground); font-size: 0.625rem; text-transform: uppercase; }
	.completion-detail { display: block; margin-top: 0.125rem; color: var(--muted-foreground); font-family: var(--font-sans, sans-serif); font-size: 0.6875rem; line-height: 1rem; }
</style>
