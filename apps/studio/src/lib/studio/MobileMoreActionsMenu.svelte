<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import { Ellipsis, Plus, Settings2 } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import { Button } from '../components/ui/button/index.js';
	import StudioDropdownContent from './StudioDropdownContent.svelte';
	import {
		dropdownGroupHeadingClass,
		dropdownItemClass,
		dropdownPanelClass
	} from './menu-classes.js';

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
		<StudioDropdownContent align="end" sideOffset={6} class={`w-[min(14rem,calc(100vw-1rem))] overflow-auto rounded-[.6rem] ${dropdownPanelClass} [&>[role=group]]:grid [&>[role=group]]:gap-[.1rem]`}>
			<DropdownMenu.Group>
				<DropdownMenu.GroupHeading class={dropdownGroupHeadingClass}>{studioMessage('header.moreActions')}</DropdownMenu.GroupHeading>
				<DropdownMenu.Item onSelect={onOpenSettings} class={`${dropdownItemClass} [&>svg]:size-[.95rem] [&>svg]:shrink-0`}>
					<Settings2 aria-hidden="true" /><span>{studioMessage('header.settings')}</span>
				</DropdownMenu.Item>
				<DropdownMenu.Item disabled={disabled || creationDisabled} onSelect={onCreateVisual} class={`${dropdownItemClass} [&>svg]:size-[.95rem] [&>svg]:shrink-0`}>
					<Plus aria-hidden="true" /><span>{studioMessage('sidebar.newVisual')}</span>
				</DropdownMenu.Item>
			</DropdownMenu.Group>
		</StudioDropdownContent>
		</DropdownMenu.Portal>
	</DropdownMenu.Root>
</div>
