<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { Check, Download, FolderOpen, Pencil, Plus, Upload } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import StudioDropdownContent from './StudioDropdownContent.svelte';

	interface ProjectOption {
		id: string;
		name: string;
		files: readonly unknown[];
	}

	interface Props {
		projects: readonly ProjectOption[];
		activeProjectId: string | null;
		activeProjectName: string | null;
		activeProjectDirty: boolean;
		disabled: boolean;
		onSelect: (projectId: string) => void;
		onCreate: () => void;
		onRename: () => void;
		onImport: () => void;
		onExport: () => void;
	}

	let {
		projects,
		activeProjectId,
		activeProjectName,
		activeProjectDirty,
		disabled,
		onSelect,
		onCreate,
		onRename,
		onImport,
		onExport
	}: Props = $props();
</script>

<div class="w-[min(14rem,55vw)] min-w-0 flex-[0_1_14rem] max-md:w-[min(12rem,55vw)] max-md:basis-48">
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button
					{...props}
					variant="outline"
					class="h-9 w-full min-w-0 justify-start gap-2 rounded-md px-2.5 text-sm font-normal shadow-none hover:bg-accent aria-expanded:bg-accent disabled:cursor-wait disabled:opacity-[.55]"
					type="button"
					aria-label={studioMessage('project.current', { name: activeProjectName ?? '' })}
					title={activeProjectName ?? studioMessage('project.select')}
					disabled={disabled || !activeProjectId}
				>
					<FolderOpen size={16} class="shrink-0 text-foreground" aria-hidden="true" />
					<span class="min-w-0 flex-1 truncate text-left font-normal text-foreground">{activeProjectName ?? studioMessage('project.select')}</span>
					{#if activeProjectDirty}<span class="size-[.38rem] shrink-0 rounded-full bg-primary" aria-label={studioMessage('project.unsavedNamed', { name: activeProjectName ?? '' })}></span>{/if}
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Portal>
		<StudioDropdownContent align="start" sideOffset={6} class="grid max-h-[min(70vh,30rem)] w-[min(14rem,calc(100vw-1.5rem))] gap-[.15rem] overflow-auto rounded-[.6rem] border border-border bg-popover p-[.45rem] text-popover-foreground shadow-[0_.75rem_2rem_rgb(7_21_41_/_18%)] [&>[role=group]]:grid [&>[role=group]]:gap-[.15rem]">
			<DropdownMenu.Group>
				<DropdownMenu.GroupHeading class="px-[.45rem] py-[.3rem] text-[.63rem] font-[650] tracking-[.06em] text-muted-foreground uppercase">{studioMessage('project.open')}</DropdownMenu.GroupHeading>
				{#each projects as project (project.id)}
					<DropdownMenu.Item onSelect={() => onSelect(project.id)} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] text-left font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:shrink-0">
						<FolderOpen size={15} class="shrink-0" aria-hidden="true" />
						<span class="min-w-0 flex-1 truncate">{project.name}</span>
						<span class="text-[.65rem] text-muted-foreground" aria-label={studioMessage('project.fileCount', { count: project.files.length })}>{project.files.length}</span>
						{#if project.id === activeProjectId}<Check size={14} class="shrink-0" aria-hidden="true" />{/if}
					</DropdownMenu.Item>
				{/each}
			</DropdownMenu.Group>
			<DropdownMenu.Separator class="my-[.3rem] h-px bg-border" />
			<DropdownMenu.Group>
				<DropdownMenu.Item onSelect={onCreate} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] text-left font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:shrink-0">
					<Plus size={15} aria-hidden="true" /><span>{studioMessage('project.create')}</span>
				</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={onRename} disabled={!activeProjectId} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] text-left font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:shrink-0">
					<Pencil size={14} aria-hidden="true" /><span>{studioMessage('project.rename')}</span>
				</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={onImport} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] text-left font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:shrink-0">
					<Upload size={14} aria-hidden="true" /><span>{studioMessage('project.import')}</span>
				</DropdownMenu.Item>
				<DropdownMenu.Item onSelect={onExport} disabled={!activeProjectId} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] text-left font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:shrink-0">
					<Download size={14} aria-hidden="true" /><span>{studioMessage('project.export')}</span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
		</StudioDropdownContent>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
</div>
