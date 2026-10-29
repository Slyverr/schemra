<script lang="ts">
	import { tick, type Snippet } from 'svelte';
	import { Pencil, Trash2 } from '@lucide/svelte';

	interface Props {
		children: Snippet;
		onRename?: () => void;
		onRemove: () => void;
		renameDisabled?: boolean;
		removeDisabled?: boolean;
	}

	const props: Props = $props();
	let root = $state<HTMLDivElement>();
	let menu = $state<HTMLDivElement>();
	let trigger = $state<HTMLElement | null>(null);
	let open = $state(false);
	let position = $state({ left: 0, top: 0 });

	function handleContextMenu(event: MouseEvent) {
		event.preventDefault();
		event.stopPropagation();
		const target = event.target instanceof Element ? event.target.closest<HTMLElement>('button') : null;
		showMenu(event.clientX, event.clientY, target);
	}

	function handleTriggerKeydown(event: KeyboardEvent) {
		if (event.key !== 'ContextMenu' && !(event.key === 'F10' && event.shiftKey)) return;
		event.preventDefault();
		event.stopPropagation();
		const target = event.target instanceof Element ? event.target.closest<HTMLElement>('button') : null;
		const bounds = target?.getBoundingClientRect();
		showMenu(bounds?.left ?? 0, bounds?.bottom ?? 0, target);
	}

	function showMenu(left: number, top: number, activeTrigger: HTMLElement | null) {
		if (!props.onRemove) return;
		trigger = activeTrigger;
		position = {
			left: Math.max(4, Math.min(left, window.innerWidth - 208)),
			top: Math.max(4, Math.min(top, window.innerHeight - 88))
		};
		open = true;
		void tick().then(() => {
			if (!menu) return;
			const bounds = menu.getBoundingClientRect();
			position = {
				left: Math.max(4, Math.min(position.left, window.innerWidth - bounds.width - 4)),
				top: Math.max(4, Math.min(position.top, window.innerHeight - bounds.height - 4))
			};
			void tick().then(() => firstMenuItem()?.focus());
		});
	}

	function closeMenu(restoreFocus = false) {
		open = false;
		if (restoreFocus) void tick().then(() => trigger?.focus());
	}

	function handleDocumentPointerDown(event: PointerEvent) {
		if (open && !root?.contains(event.target as Node)) closeMenu();
	}

	function handleDocumentKeydown(event: KeyboardEvent) {
		if (!open || event.key !== 'Escape') return;
		event.preventDefault();
		event.stopPropagation();
		closeMenu(true);
	}

	function handleMenuKeydown(event: KeyboardEvent) {
		if (!menu) return;
		const items = [...menu.querySelectorAll<HTMLButtonElement>('[role="menuitem"]:not(:disabled)')];
		if (items.length === 0) return;
		const current = items.indexOf(document.activeElement as HTMLButtonElement);
		let next = -1;
		if (event.key === 'ArrowDown') next = current < 0 ? 0 : (current + 1) % items.length;
		else if (event.key === 'ArrowUp') next = current < 0 ? items.length - 1 : (current - 1 + items.length) % items.length;
		else if (event.key === 'Home') next = 0;
		else if (event.key === 'End') next = items.length - 1;
		if (next < 0) return;
		event.preventDefault();
		items[next]?.focus();
	}

	function firstMenuItem() {
		return menu?.querySelector<HTMLButtonElement>('[role="menuitem"]:not(:disabled)');
	}

	function selectRename() {
		closeMenu();
		props.onRename?.();
	}

	function selectRemove() {
		closeMenu();
		props.onRemove();
	}
</script>

<svelte:document onpointerdown={handleDocumentPointerDown} onkeydown={handleDocumentKeydown} />

<!-- Keyboard and context-menu events are delegated from the contained tab button. -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div bind:this={root} class="resource-context-root" role="presentation" oncontextmenu={handleContextMenu} onkeydown={handleTriggerKeydown}>
	{@render props.children()}
	{#if open}
		<div
			bind:this={menu}
			class="resource-context-menu"
			role="menu"
			tabindex="-1"
			aria-label="Resource actions"
			style:left={`${position.left}px`}
			style:top={`${position.top}px`}
			onkeydown={handleMenuKeydown}
		>
			{#if props.onRename}
				<button type="button" role="menuitem" disabled={props.renameDisabled} onclick={selectRename}>
					<Pencil size={15} aria-hidden="true" />
					<span>Rename</span>
				</button>
				<div class="menu-separator" role="separator"></div>
			{/if}
			<button class="remove-action" type="button" role="menuitem" disabled={props.removeDisabled} onclick={selectRemove}>
				<Trash2 size={15} aria-hidden="true" />
				<span>Remove from project</span>
			</button>
		</div>
	{/if}
</div>

<style>
	.resource-context-root { position: relative; display: inline-flex; min-width: max-content; flex: 0 1 auto; }
	.resource-context-menu {
		position: fixed;
		z-index: 60;
		width: 12.5rem;
		padding: .25rem;
		border: 1px solid var(--border);
		border-radius: .55rem;
		background: var(--popover, var(--background));
		color: var(--popover-foreground, var(--foreground));
		box-shadow: 0 10px 28px rgb(15 35 65 / 18%);
	}
	.resource-context-menu button {
		display: flex;
		width: 100%;
		min-height: 2.15rem;
		align-items: center;
		gap: .55rem;
		padding: .35rem .55rem;
		border: 0;
		border-radius: .35rem;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: .8rem;
		text-align: left;
		cursor: pointer;
	}
	.resource-context-menu button:hover:not(:disabled), .resource-context-menu button:focus-visible { outline: none; background: var(--accent); color: var(--accent-foreground, var(--foreground)); }
	.resource-context-menu button:disabled { opacity: .5; cursor: not-allowed; }
	.resource-context-menu .remove-action { color: var(--destructive); }
	.menu-separator { height: 1px; margin: .25rem .35rem; background: var(--border); }
</style>
