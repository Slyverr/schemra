<script lang="ts">
	import StudioShell from '../lib/studio/StudioShell.svelte';
	import { studioTitle } from '../lib/studio-title';
</script>

<svelte:head>
	<title>{studioTitle}</title>
	<meta name="description" content="Schemra local authoring environment" />
</svelte:head>

<StudioShell />
