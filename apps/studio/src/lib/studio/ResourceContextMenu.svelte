<script lang="ts">
	import { ContextMenu } from 'bits-ui';
	import type { Snippet } from 'svelte';
	import { Pencil, Trash2 } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { contextMenuItemClass, contextMenuPanelClass } from './menu-classes.js';

	interface Props {
		children: Snippet;
		wrapperClass?: string;
		onRename?: () => void;
		onRemove: () => void;
		renameDisabled?: boolean;
		removeDisabled?: boolean;
	}

	const props: Props = $props();
	let trigger: HTMLElement | null = null;
	let restoreFocusAfterEscape = false;

	function rememberTrigger(event: MouseEvent) {
		trigger = event.target instanceof Element ? event.target.closest<HTMLElement>('button') : null;
	}

	function handleTriggerKeydown(event: KeyboardEvent) {
		if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) return;
		const target = event.target instanceof Element ? event.target.closest<HTMLElement>('button') : null;
		if (!target) return;
		event.preventDefault();
		event.stopPropagation();
		const bounds = target.getBoundingClientRect();
		target.dispatchEvent(new MouseEvent('contextmenu', {
			bubbles: true,
			cancelable: true,
			clientX: bounds.left,
			clientY: bounds.bottom
		}));
	}

	function handleEscapeKeydown() {
		restoreFocusAfterEscape = true;
	}

	function handleCloseAutoFocus(event: Event) {
		event.preventDefault();
		if (restoreFocusAfterEscape) trigger?.focus();
		restoreFocusAfterEscape = false;
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<ContextMenu.Root>
	<ContextMenu.Trigger role="presentation" class={`inline-flex min-w-max flex-[0_1_auto] ${props.wrapperClass ?? ''}`}>
		{#snippet child({ props: triggerProps })}
			<div
				{...triggerProps}
				oncontextmenu={(event) => {
					rememberTrigger(event);
					(
						triggerProps as unknown as { oncontextmenu?: (event: MouseEvent) => void }
					).oncontextmenu?.(event);
				}}
				onkeydown={handleTriggerKeydown}
			>
				{@render props.children()}
			</div>
		{/snippet}
	</ContextMenu.Trigger>
	<ContextMenu.Portal>
		<ContextMenu.Content
			align="start"
			side="right"
			sideOffset={0}
			class={`${contextMenuPanelClass} !z-[1000]`}
			aria-label={studioMessage('resource.contextActions')}
			onEscapeKeydown={handleEscapeKeydown}
			onCloseAutoFocus={handleCloseAutoFocus}
		>
			{#if props.onRename}
				<ContextMenu.Item disabled={props.renameDisabled} onSelect={props.onRename} class={contextMenuItemClass}>
					<Pencil size={15} aria-hidden="true" />
					<span>{studioMessage('resource.rename')}</span>
				</ContextMenu.Item>
				<ContextMenu.Separator class="my-1 mx-[.35rem] h-px bg-border" />
			{/if}
			<ContextMenu.Item disabled={props.removeDisabled} onSelect={props.onRemove} class={`${contextMenuItemClass} text-destructive`}>
				<Trash2 size={15} aria-hidden="true" />
				<span>{studioMessage('resource.removeFromProject')}</span>
			</ContextMenu.Item>
		</ContextMenu.Content>
	</ContextMenu.Portal>
</ContextMenu.Root>
