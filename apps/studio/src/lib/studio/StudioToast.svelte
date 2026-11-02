<script lang="ts">
	import { AlertCircle, CircleCheck, X } from '@lucide/svelte';
	import { Button } from '../components/ui/button/index.js';
	import { studioMessage } from '../i18n/messages.js';

	type Message = { kind: 'error' | 'success'; text: string } | null;
	let { message, onDismiss }: { message: Message; onDismiss: () => void } = $props();

	$effect(() => {
		if (!message) return;
		const timeout = setTimeout(onDismiss, message.kind === 'error' ? 7_000 : 4_000);
		return () => clearTimeout(timeout);
	});
</script>

{#if message}
	<div
		class={`fixed bottom-4 right-4 z-[1000] flex max-h-[calc(100dvh-2rem)] max-w-[min(28rem,calc(100vw-2rem))] items-start gap-3 overflow-auto rounded-lg border bg-popover/90 p-3 text-sm shadow-lg backdrop-blur-sm ${message.kind === 'error' ? 'border-destructive/35 text-destructive' : 'border-primary/30 text-primary'}`}
		role={message.kind === 'error' ? 'alert' : 'status'}
		aria-live={message.kind === 'error' ? 'assertive' : 'polite'}
		aria-atomic="true"
	>
		{#if message.kind === 'error'}
			<AlertCircle size={16} class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
		{:else}
			<CircleCheck size={16} class="mt-0.5 size-4 shrink-0" aria-hidden="true" />
		{/if}
		<span class="min-w-0 flex-1 break-words">{message.text}</span>
		<Button variant="ghost" size="icon-sm" class="-mr-1 -mt-1 size-7 shrink-0 rounded-md p-0 text-current hover:bg-current/10 hover:text-current" type="button" aria-label={studioMessage('resource.dismissMessage')} onclick={onDismiss}>
			<X size={14} class="size-[14px]" aria-hidden="true" />
		</Button>
	</div>
{/if}
