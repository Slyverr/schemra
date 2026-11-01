<script lang="ts">
	import { X } from '@lucide/svelte';
	import type { EditorPreferences } from '../settings/editor.js';
	import SettingsPanel from '../settings/SettingsPanel.svelte';
	import type { ThemePreferences } from '../settings/appearance.js';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { Button } from '../components/ui/button/index.js';
	import { studioMessage } from '../i18n/messages.js';

	let {
		open = $bindable(false),
		themePreferences,
		editorPreferences,
		onThemePreferencesChange,
		onEditorPreferencesChange
	}: {
		open?: boolean;
		themePreferences: ThemePreferences;
		editorPreferences: EditorPreferences;
		onThemePreferencesChange: (value: ThemePreferences) => void;
		onEditorPreferencesChange: (value: EditorPreferences) => void;
	} = $props();

	let contentElement = $state<HTMLElement | null>(null);
</script>

<Dialog.Root bind:open>
	{#if open}
		<Dialog.Content
			bind:ref={contentElement}
			class="h-[min(34rem,calc(100dvh_-_2rem))] max-h-[calc(100dvh_-_2rem)] w-[min(54rem,calc(100vw_-_2rem))] max-w-none sm:max-w-none grid-rows-[auto_minmax(0,1fr)] gap-0 overflow-visible p-0"
		>
			<Dialog.Header class="min-h-12 flex-row items-center justify-between border-b border-border px-5 py-2.5 sm:px-6">
				<Dialog.Title id="settings-title" class="text-base font-semibold">{studioMessage('settings.heading')}</Dialog.Title>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button variant="ghost" size="icon-sm" class="shrink-0 text-muted-foreground" aria-label={studioMessage('settings.close')} title={studioMessage('settings.close')} {...props}>
							<X size={17} class="size-[17px]" aria-hidden="true" />
						</Button>
					{/snippet}
				</Dialog.Close>
			</Dialog.Header>

			<div class="min-h-0 min-w-0 flex-1 overflow-x-hidden overflow-y-auto">
				<SettingsPanel
					themePreferences={themePreferences}
					editorPreferences={editorPreferences}
					popoverBoundary={contentElement}
					onThemePreferencesChange={onThemePreferencesChange}
					onEditorPreferencesChange={onEditorPreferencesChange}
				/>
			</div>
		</Dialog.Content>
	{/if}
</Dialog.Root>
