<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { Check, FileCode2, LayoutGrid, Plus, Workflow } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import StudioDropdownContent from './StudioDropdownContent.svelte';
	import {
		dropdownGroupHeadingClass,
		dropdownItemClass,
		dropdownPanelClass
	} from './menu-classes.js';
	import type { SidebarResource, SidebarVisual } from './project-sidebar.js';

	interface Props {
		visuals: SidebarVisual[];
		otherSources: SidebarResource[];
		activeFileId: string | null;
		activeVisualId: string | null;
		resourcesDisabled: boolean;
		creationDisabled: boolean;
		onSelectFile: (fileId: string) => void;
		onCreateVisual: () => void;
	}

	let { visuals, otherSources, activeFileId, activeVisualId, resourcesDisabled, creationDisabled, onSelectFile, onCreateVisual }: Props = $props();
</script>

<div class="hidden max-md:block">
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button {...props} variant="outline" size="icon" class="size-[2.4rem] rounded-[.55rem] shadow-none hover:bg-accent hover:text-accent-foreground aria-expanded:bg-accent aria-expanded:text-accent-foreground" type="button" aria-label={studioMessage('sidebar.mobileBrowse')} title={studioMessage('sidebar.mobileBrowse')}>
					<LayoutGrid class="size-4" aria-hidden="true" />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Portal>
		<StudioDropdownContent align="start" sideOffset={6} class={`max-h-[min(70vh,30rem)] w-[min(16rem,calc(100vw-1rem))] overflow-auto rounded-[.6rem] ${dropdownPanelClass} [&>[role=group]]:grid [&>[role=group]]:gap-[.1rem]`}>
			<DropdownMenu.Group>
				<DropdownMenu.GroupHeading class={dropdownGroupHeadingClass}>{studioMessage('sidebar.visuals')}</DropdownMenu.GroupHeading>
				{#each visuals as visual (visual.id)}
					<DropdownMenu.Item
						disabled={resourcesDisabled}
						onSelect={() => onSelectFile(visual.fileId)}
						aria-label={visual.pathHint ? `${visual.title}, ${visual.path}` : undefined}
						title={visual.pathHint ? `${visual.title} — ${visual.path}` : visual.title}
						class={`${dropdownItemClass} [&>svg:first-child]:size-[.95rem] [&>svg:first-child]:shrink-0 [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate [&>small]:max-w-24 [&>small]:truncate [&>small]:text-[.62rem] [&>small]:text-muted-foreground`}
					>
						<Workflow class="size-[.95rem] shrink-0" aria-hidden="true" />
						<span>{visual.title}</span>
						{#if visual.pathHint}<small class="max-w-24 truncate text-[.62rem] text-muted-foreground">{visual.pathHint}</small>{/if}
						{#if visual.dirty}<i class="block size-[.38rem] shrink-0 rounded-full bg-primary" aria-label={studioMessage('sidebar.unsavedChanges', { name: visual.title })}></i>{/if}
						{#if activeVisualId === visual.id}<Check aria-hidden="true" class="size-[.85rem] shrink-0" />{/if}
					</DropdownMenu.Item>
				{:else}
					<div class="px-[.55rem] py-[.4rem] text-[.72rem] text-muted-foreground" role="status">{studioMessage('sidebar.noVisuals')}</div>
				{/each}
				<DropdownMenu.Item disabled={resourcesDisabled || creationDisabled} onSelect={onCreateVisual} class={`${dropdownItemClass} [&>svg:first-child]:size-[.95rem] [&>svg:first-child]:shrink-0 [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate`}>
					<Plus class="size-[.95rem] shrink-0" aria-hidden="true" /><span>{studioMessage('sidebar.newVisual')}</span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
			{#if otherSources.length > 0}
				<DropdownMenu.Separator class="my-1 h-px bg-border" />
				<DropdownMenu.Group>
					<DropdownMenu.GroupHeading class={dropdownGroupHeadingClass}>{studioMessage('sidebar.otherSourceFiles')}</DropdownMenu.GroupHeading>
					{#each otherSources as source (source.fileId)}
						<DropdownMenu.Item disabled={resourcesDisabled} onSelect={() => onSelectFile(source.fileId)} title={source.path} class={`${dropdownItemClass} [&>svg:first-child]:size-[.95rem] [&>svg:first-child]:shrink-0 [&>span]:min-w-0 [&>span]:flex-1 [&>span]:truncate`}>
							<FileCode2 class="size-[.95rem] shrink-0" aria-hidden="true" />
							<span>{source.path.split('/').at(-1) ?? source.path}</span>
							{#if source.dirty}<i class="block size-[.38rem] shrink-0 rounded-full bg-primary" aria-label={studioMessage('sidebar.unsavedChanges', { name: source.title })}></i>{/if}
							{#if source.hasErrors}<span class="size-[.35rem] shrink-0 rounded-full bg-destructive" aria-label={studioMessage('sidebar.hasErrors', { name: source.title })}></span>{/if}
							{#if activeFileId === source.fileId}<Check aria-hidden="true" class="size-[.85rem] shrink-0" />{/if}
						</DropdownMenu.Item>
					{/each}
				</DropdownMenu.Group>
			{/if}
		</StudioDropdownContent>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
</div>
