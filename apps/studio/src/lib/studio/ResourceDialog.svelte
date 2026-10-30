<script lang="ts">
	import { X } from '@lucide/svelte';
	import * as Dialog from '../components/ui/dialog/index.js';
	import { Button } from '../components/ui/button/index.js';
	import { Input } from '../components/ui/input/index.js';
	import { Label } from '../components/ui/label/index.js';
	import { studioMessage } from '../i18n/messages.js';

	type DialogKind = 'create-project' | 'create-visual' | 'create-scenario' | 'rename-project' | 'rename-resource' | 'remove-resource' | null;
	type ResourceKind = 'visual' | 'scenario' | 'source' | null;

	let {
		kind,
		targetKind,
		scenarioVisualTitle = '',
		nameDraft = $bindable(''),
		error,
		removeLabel,
		onClose,
		onSave,
		onRemove
	}: {
		kind: DialogKind;
		targetKind: ResourceKind;
		scenarioVisualTitle?: string;
		nameDraft?: string;
		error: string;
		removeLabel: string;
		onClose: () => void;
		onSave: () => void;
		onRemove: () => void;
	} = $props();

	function handleOpenChange(open: boolean) {
		if (!open && kind) onClose();
	}

	function resourceKindLabel() {
		if (targetKind === 'visual') return studioMessage('resource.dialog.kindVisual');
		if (targetKind === 'scenario') return studioMessage('resource.dialog.kindScenario');
		return studioMessage('resource.dialog.kindSource');
	}

	function title() {
		if (kind === 'create-project') return studioMessage('resource.dialog.createProjectTitle');
		if (kind === 'create-visual') return studioMessage('resource.dialog.createVisualTitle');
		if (kind === 'create-scenario') return studioMessage('resource.dialog.createScenarioTitle');
		if (kind === 'rename-project') return studioMessage('resource.dialog.renameProjectTitle');
		if (kind === 'rename-resource') return studioMessage('resource.dialog.renameTitle', { kind: resourceKindLabel() });
		return studioMessage('resource.dialog.removeTitle', { kind: resourceKindLabel() });
	}

	function description() {
		if (kind === 'create-project' || kind === 'rename-project') return studioMessage('resource.dialog.projectDescription');
		if (kind === 'create-visual') return studioMessage('resource.dialog.createVisualDescription');
		if (kind === 'create-scenario') {
			return studioMessage('resource.dialog.createScenarioDescription', {
				visual: scenarioVisualTitle || studioMessage('resource.dialog.thisVisual')
			});
		}
		if (kind === 'rename-resource' && targetKind === 'visual') return studioMessage('resource.dialog.renameVisualDescription');
		if (kind === 'rename-resource' && targetKind === 'scenario') return studioMessage('resource.dialog.renameScenarioDescription');
		if (kind === 'rename-resource' && targetKind === 'source') return studioMessage('resource.dialog.renameSourceDescription');
		if (targetKind === 'visual') return studioMessage('resource.dialog.removeVisualDescription');
		if (targetKind === 'scenario') return studioMessage('resource.dialog.removeScenarioDescription');
		if (targetKind === 'source') return studioMessage('resource.dialog.removeSourceDescription');
		return '';
	}

	function fieldLabel() {
		if (kind === 'create-project' || kind === 'rename-project') return studioMessage('resource.dialog.projectName');
		if (kind === 'create-scenario' || targetKind === 'scenario') return studioMessage('resource.dialog.scenarioName');
		return studioMessage('resource.dialog.visualName');
	}

	function submitLabel() {
		if (kind === 'rename-resource' || kind === 'rename-project') return studioMessage('resource.dialog.saveName');
		if (kind === 'create-project') return studioMessage('resource.dialog.createProject');
		if (kind === 'create-visual') return studioMessage('resource.dialog.createVisual');
		if (kind === 'create-scenario') return studioMessage('resource.dialog.createScenario');
		return studioMessage('resource.dialog.create');
	}
</script>

<Dialog.Root open={Boolean(kind)} onOpenChange={handleOpenChange}>
	{#if kind}
		<Dialog.Content
			class={kind === 'remove-resource'
				? 'max-h-[calc(100dvh-2rem)] w-full gap-5 overflow-y-auto sm:max-w-sm'
				: 'max-h-[calc(100dvh-2rem)] w-full gap-5 overflow-y-auto sm:max-w-md'}
			role={kind === 'remove-resource' ? 'alertdialog' : 'dialog'}
			showCloseButton={false}
		>
			<Dialog.Header class="gap-1.5 pr-8">
				<Dialog.Title class="text-base font-semibold leading-6">{title()}</Dialog.Title>
				{#if description()}<Dialog.Description class="text-sm leading-5">{description()}</Dialog.Description>{/if}
			</Dialog.Header>

			<Dialog.Close>
				{#snippet child({ props })}
					<Button variant="ghost" size="icon-sm" class="absolute top-4 right-4" aria-label={studioMessage('resource.dialog.close')} {...props}>
						<X size={16} aria-hidden="true" />
					</Button>
				{/snippet}
			</Dialog.Close>

			{#if kind !== 'remove-resource'}
				<form id="resource-name-form" class="grid gap-2.5" onsubmit={(event) => { event.preventDefault(); onSave(); }}>
					<Label for="resource-name" class="text-xs text-muted-foreground">{fieldLabel()}</Label>
					<Input id="resource-name" bind:value={nameDraft} maxlength={80} autocomplete="off" spellcheck={false} aria-invalid={Boolean(error)} aria-describedby={error ? 'resource-dialog-error' : undefined} />
				</form>
			{/if}

			{#if error}<p id="resource-dialog-error" class="text-sm text-destructive" role="alert">{error}</p>{/if}

			<Dialog.Footer>
				<Dialog.Close>
					{#snippet child({ props })}
						<Button variant="outline" {...props}>{studioMessage('resource.dialog.cancel')}</Button>
					{/snippet}
				</Dialog.Close>
				{#if kind === 'remove-resource'}
					<Button variant="destructive" type="button" onclick={onRemove}>{removeLabel}</Button>
				{:else}
					<Button type="submit" form="resource-name-form">{submitLabel()}</Button>
				{/if}
			</Dialog.Footer>
		</Dialog.Content>
	{/if}
</Dialog.Root>
