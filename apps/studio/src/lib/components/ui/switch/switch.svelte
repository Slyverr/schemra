<script lang="ts">
	import { Switch as SwitchPrimitive } from 'bits-ui';
	import { cn, type WithoutChild } from '#lib/utils.js';

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	}: WithoutChild<SwitchPrimitive.RootProps> = $props();
</script>

<SwitchPrimitive.Root
	bind:ref
	data-slot="switch"
	class={cn(
		'inline-flex h-[1.125rem] w-8 shrink-0 cursor-pointer items-center rounded-full border border-input bg-muted transition-colors data-[state=checked]:border-primary data-[state=checked]:bg-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50',
		className
	)}
	{...restProps}
>
	<SwitchPrimitive.Thumb class="pointer-events-none block size-[.82rem] translate-x-[.12rem] rounded-full bg-background shadow-sm transition-transform data-[state=checked]:translate-x-[.92rem] data-[state=checked]:bg-primary-foreground" />
</SwitchPrimitive.Root>
