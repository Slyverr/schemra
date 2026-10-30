<script lang="ts">
	import { LoaderCircle } from '@lucide/svelte';
	import { studioMessage } from '../i18n/messages.js';
	import type { ResolvedTheme } from '../settings/appearance.js';

	let { theme, reduceMotion }: { theme: ResolvedTheme; reduceMotion: boolean } = $props();
</script>

<main class={`${theme} grid min-h-dvh place-items-center bg-background px-6 py-12 text-foreground`} role="status" aria-live="polite" aria-atomic="true" aria-busy="true">
	<div class="flex max-w-sm flex-col items-center text-center">
		<div class="mb-5 grid size-11 place-items-center rounded-full border border-border bg-muted/40">
			<LoaderCircle class={`size-5 text-muted-foreground motion-reduce:animate-none ${reduceMotion ? '!animate-none' : 'animate-spin'}`} aria-hidden="true" />
		</div>
		<p class="m-0 text-sm font-[550]">{studioMessage('resource.restoringWorkspace')}</p>
		<p class="mt-1.5 mb-0 text-sm leading-[1.4rem] text-muted-foreground">{studioMessage('resource.loadingWorkspaceDescription')}</p>
	</div>
</main>
