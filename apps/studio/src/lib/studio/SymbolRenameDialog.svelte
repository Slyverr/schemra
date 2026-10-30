<script lang="ts">
	import { X } from '@lucide/svelte';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { Label } from '../components/ui/label/index.js';
	import { studioMessage } from '../i18n/messages.js';

	let {
		open,
		kind,
		draft = $bindable(''),
		error,
		pending,
		inputElement = $bindable<HTMLInputElement | null>(null),
		onClose,
		onSave
	}: {
		open: boolean;
		kind: string;
		draft?: string;
		error: string;
		pending: boolean;
		inputElement?: HTMLInputElement | null;
		onClose: () => void;
		onSave: () => void;
	} = $props();

	function handleOpenChange(nextOpen: boolean) {
		if (!nextOpen && !pending) onClose();
	}
</script>

<Dialog.Root open={open} onOpenChange={handleOpenChange}>
	{#if open}
		<Dialog.Content class="max-h-[calc(100dvh-2rem)] w-full gap-4 overflow-y-auto p-[1.1rem] sm:max-w-md" showCloseButton={false}>
			<Dialog.Header class="gap-1.5 pr-9">
				<Dialog.Title id="symbol-rename-title" class="text-base font-semibold leading-6">
					{studioMessage('symbolRename.title', { kind })}
				</Dialog.Title>
				<Dialog.Description class="text-sm leading-5">{studioMessage('symbolRename.description')}</Dialog.Description>
			</Dialog.Header>

			<Dialog.Close>
				{#snippet child({ props })}
					<Button variant="ghost" size="icon" class="absolute top-4 right-4 size-9 rounded-[.55rem]" aria-label={studioMessage('symbolRename.close')} disabled={pending} {...props}>
						<X size={16} aria-hidden="true" />
					</Button>
				{/snippet}
			</Dialog.Close>

			<form id="symbol-rename-form" class="grid gap-[.45rem]" onsubmit={(event) => { event.preventDefault(); onSave(); }}>
				<Label for="symbol-rename-input" class="text-xs text-muted-foreground">{studioMessage('symbolRename.newIdentifier')}</Label>
				<Input
					id="symbol-rename-input"
					bind:ref={inputElement}
					bind:value={draft}
					autocomplete="off"
					autocapitalize="off"
					spellcheck="false"
					aria-invalid={Boolean(error)}
					aria-describedby={error ? 'symbol-rename-error' : undefined}
					disabled={pending}
				/>
			</form>

			{#if error}<p id="symbol-rename-error" class="-mt-1 text-sm leading-[1.4] text-destructive" role="alert">{error}</p>{/if}

			<Dialog.Footer class="flex-row justify-end pt-1">
				<Dialog.Close>
					{#snippet child({ props })}
						<Button variant="outline" disabled={pending} {...props}>{studioMessage('resource.dialog.cancel')}</Button>
					{/snippet}
				</Dialog.Close>
				<Button type="submit" form="symbol-rename-form" disabled={pending || !draft.trim()}>
					{pending ? studioMessage('symbolRename.checking') : studioMessage('symbolRename.rename')}
				</Button>
			</Dialog.Footer>
		</Dialog.Content>
	{/if}
</Dialog.Root>
