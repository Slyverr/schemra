<script lang="ts">
	import { DropdownMenu } from 'bits-ui';
	import type { ComponentProps, Snippet } from 'svelte';

	type ContentProps = Omit<ComponentProps<typeof DropdownMenu.Content>, 'child' | 'children'>;
	interface Props extends ContentProps {
		children: Snippet;
	}

	let { children, ...contentProps }: Props = $props();
</script>

<DropdownMenu.Content {...contentProps}>
	{#snippet child({ props, wrapperProps })}
		<div {...wrapperProps} class="!z-[1000]">
			<div {...props}>
				{@render children()}
			</div>
		</div>
	{/snippet}
</DropdownMenu.Content>
