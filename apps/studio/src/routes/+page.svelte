<script lang="ts">
	import { studioTitle } from '../lib/studio-title';
</script>

<svelte:head>
	<title>{studioTitle}</title>
	<meta name="description" content="Schemra local authoring environment" />
</svelte:head>

<main>
	<h1>{studioTitle}</h1>
</main>

<style>
	:global(body) {
		margin: 0;
		min-width: 20rem;
		min-height: 100vh;
		font-family: system-ui, sans-serif;
		color: #182235;
		background: #f5f8fc;
	}

	main {
		display: grid;
		min-height: 100vh;
		place-items: center;
	}

	h1 {
		margin: 0;
		font-size: clamp(2rem, 8vw, 4rem);
		font-weight: 650;
		letter-spacing: -0.045em;
	}
</style>
