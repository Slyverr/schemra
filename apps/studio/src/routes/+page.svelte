<script lang="ts">
	import { studioTitle } from '../lib/studio-title';
</script>

<svelte:head>
	<title>{studioTitle}</title>
	<meta name="description" content="Schemra local authoring environment" />
</svelte:head>

<main class="grid min-h-screen place-items-center bg-background text-foreground">
	<h1 class="m-0 text-[clamp(2rem,8vw,4rem)] font-semibold tracking-[-0.045em]">{studioTitle}</h1>
</main>
