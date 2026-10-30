<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { Download, FileCode2, FolderOpen, LoaderCircle, Save } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import type { DiagramExportFormat } from '../preview/export.js';
	import StudioDropdownContent from './StudioDropdownContent.svelte';
	import {
		dropdownGroupHeadingClass,
		dropdownItemClass,
		dropdownPanelClass
	} from './menu-classes.js';

	interface Props {
		disabled: boolean;
		canSaveSource: boolean;
		canExportDiagram: boolean;
		sourceOperation: 'opening' | 'saving' | null;
		onOpenSource: () => void;
		onSaveSource: () => void;
		onExportDiagram: (format: DiagramExportFormat) => void;
	}

	let {
		disabled,
		canSaveSource,
		canExportDiagram,
		sourceOperation,
		onOpenSource,
		onSaveSource,
		onExportDiagram
	}: Props = $props();
</script>

<DropdownMenu.Root>
	<DropdownMenu.Trigger>
		{#snippet child({ props })}
			<Button {...props} variant="outline" size="sm" class="h-9 w-9 shrink-0 gap-[.45rem] rounded-[.55rem] px-0 text-[.77rem] font-normal shadow-none hover:bg-accent hover:text-accent-foreground aria-expanded:bg-accent aria-expanded:text-accent-foreground lg:w-auto lg:px-3" type="button" aria-label={studioMessage('file.actions')} title={studioMessage('file.actions')}>
				<FileCode2 size={17} class="size-[17px]" aria-hidden="true" />
				<span class="hidden lg:inline">{studioMessage('header.file')}</span>
			</Button>
		{/snippet}
	</DropdownMenu.Trigger>
	<DropdownMenu.Portal>
	<StudioDropdownContent align="end" sideOffset={6} class={`w-[min(14rem,calc(100vw-1.5rem))] rounded-[.6rem] ${dropdownPanelClass}`}>
		<DropdownMenu.Group>
			<DropdownMenu.GroupHeading class={dropdownGroupHeadingClass}>{studioMessage('header.file')}</DropdownMenu.GroupHeading>
			<DropdownMenu.Item disabled={disabled || sourceOperation !== null} onSelect={onOpenSource} class={`${dropdownItemClass} [&>svg]:shrink-0`}>
				<FolderOpen size={15} aria-hidden="true" />{studioMessage('file.openSource')}
			</DropdownMenu.Item>
			<DropdownMenu.Item disabled={!canSaveSource || sourceOperation !== null} onSelect={onSaveSource} class={`${dropdownItemClass} [&>svg]:shrink-0`}>
				{#if sourceOperation === 'saving'}<LoaderCircle size={15} class="animate-spin" aria-hidden="true" />{:else}<Save size={15} aria-hidden="true" />{/if}
				{sourceOperation === 'saving' ? studioMessage('file.savingSource') : studioMessage('file.saveSource')}
			</DropdownMenu.Item>
		</DropdownMenu.Group>
		<DropdownMenu.Separator class="my-[.3rem] h-px bg-border" />
		<DropdownMenu.Group>
			<DropdownMenu.Item disabled={!canExportDiagram} onSelect={() => onExportDiagram('svg')} class={`${dropdownItemClass} [&>svg]:shrink-0`}><Download size={15} aria-hidden="true" />{studioMessage('file.exportSvg')}</DropdownMenu.Item>
			<DropdownMenu.Item disabled={!canExportDiagram} onSelect={() => onExportDiagram('png')} class={`${dropdownItemClass} [&>svg]:shrink-0`}><Download size={15} aria-hidden="true" />{studioMessage('file.exportPng')}</DropdownMenu.Item>
			<DropdownMenu.Item disabled={!canExportDiagram} onSelect={() => onExportDiagram('pdf')} class={`${dropdownItemClass} [&>svg]:shrink-0`}><Download size={15} aria-hidden="true" />{studioMessage('file.exportPdf')}</DropdownMenu.Item>
		</DropdownMenu.Group>
	</StudioDropdownContent>
	</DropdownMenu.Portal>
</DropdownMenu.Root>
