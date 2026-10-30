<script lang="ts">
	import { X } from '@lucide/svelte';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { Button } from '../components/ui/button/index.js';
	import { studioMessage } from '../i18n/messages.js';
	import { languageReferenceSections } from './language-reference.js';

	let { open = $bindable(false) }: { open?: boolean } = $props();
	const instanceId = $props.id();
</script>

<Dialog.Root bind:open>
	{#if open}
		<Dialog.Content class="max-h-[calc(100dvh-2rem)] w-full gap-5 overflow-y-auto p-5 sm:max-w-2xl sm:p-6" showCloseButton={false}>
			<Dialog.Header class="flex-row items-start justify-between gap-4 pr-9">
				<div class="min-w-0">
					<Dialog.Title id={`${instanceId}-title`} class="text-base font-semibold">{studioMessage('reference.title')}</Dialog.Title>
					<Dialog.Description class="mt-1 text-sm leading-[1.5]">{studioMessage('reference.description')}</Dialog.Description>
				</div>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button variant="ghost" size="icon" class="absolute top-4 right-4 size-9 rounded-[.55rem]" aria-label={studioMessage('reference.close')} title={studioMessage('reference.close')} {...props}>
							<X size={16} aria-hidden="true" />
						</Button>
					{/snippet}
				</Dialog.Close>
			</Dialog.Header>

			<div class="grid gap-5 text-sm">
				{#each languageReferenceSections as section}
					{@const titleId = `${instanceId}-${section.title}`}
					<section aria-labelledby={titleId}>
						<h3 id={titleId} class="text-base font-semibold">{studioMessage(section.title)}</h3>
						<p class="mt-1 text-sm leading-[1.5] text-muted-foreground">{studioMessage(section.description)}</p>
						<pre class="mt-3 overflow-x-auto whitespace-pre-wrap break-words rounded-md border border-border bg-muted p-3 text-xs leading-[1.55]"><code class="font-mono">{section.source}</code></pre>
					</section>
				{/each}
			</div>
		</Dialog.Content>
	{/if}
</Dialog.Root>
