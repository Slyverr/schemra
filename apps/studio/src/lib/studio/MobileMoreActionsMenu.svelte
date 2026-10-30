<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { Ellipsis, Plus, Settings2 } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import StudioDropdownContent from './StudioDropdownContent.svelte';

	interface Props {
		disabled: boolean;
		creationDisabled: boolean;
		onOpenSettings: () => void;
		onCreateVisual: () => void;
	}

	let { disabled, creationDisabled, onOpenSettings, onCreateVisual }: Props = $props();
</script>

<div class="hidden max-md:block">
	<DropdownMenu.Root>
		<DropdownMenu.Trigger>
			{#snippet child({ props })}
				<Button {...props} variant="outline" size="icon" class="size-[2.4rem] rounded-[.55rem] shadow-none hover:bg-accent hover:text-accent-foreground aria-expanded:bg-accent aria-expanded:text-accent-foreground" type="button" aria-label={studioMessage('header.moreActions')} title={studioMessage('header.moreActions')}>
					<Ellipsis class="size-4" aria-hidden="true" />
				</Button>
			{/snippet}
		</DropdownMenu.Trigger>
		<DropdownMenu.Portal>
		<StudioDropdownContent align="end" sideOffset={6} class="grid w-[min(14rem,calc(100vw-1rem))] gap-[.15rem] overflow-auto rounded-[.6rem] border border-border bg-popover p-[.45rem] text-popover-foreground shadow-[0_.75rem_2rem_rgb(7_21_41_/_18%)] [&>[role=group]]:grid [&>[role=group]]:gap-[.1rem]">
			<DropdownMenu.Group>
				<DropdownMenu.GroupHeading class="px-[.45rem] py-[.3rem] text-[.63rem] font-[650] tracking-[.06em] text-muted-foreground uppercase">{studioMessage('header.moreActions')}</DropdownMenu.GroupHeading>
				<DropdownMenu.Item onSelect={onOpenSettings} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:size-[.95rem] [&>svg]:shrink-0">
					<Settings2 aria-hidden="true" /><span>{studioMessage('header.settings')}</span>
				</DropdownMenu.Item>
				<DropdownMenu.Item disabled={disabled || creationDisabled} onSelect={onCreateVisual} class="flex min-h-8 items-center gap-2 rounded-[.35rem] px-[.45rem] py-[.35rem] font-[inherit] text-[.75rem] text-foreground outline-none hover:bg-accent hover:text-accent-foreground data-[highlighted]:bg-accent data-[highlighted]:text-accent-foreground aria-disabled:opacity-50 [&>svg]:size-[.95rem] [&>svg]:shrink-0">
					<Plus aria-hidden="true" /><span>{studioMessage('sidebar.newVisual')}</span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
		</StudioDropdownContent>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
</div>
