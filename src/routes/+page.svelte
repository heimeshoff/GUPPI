<script lang="ts">
	// The walking skeleton's single screen: a full-window canvas (ADR-002 +
	// ADR-003). No routing, no chrome — the styleguide is not signed off yet.
	import Canvas from '$lib/Canvas.svelte';
</script>

<main>
	<Canvas />
</main>

<style>
	:global(html, body) {
		margin: 0;
		padding: 0;
		height: 100%;
		background: #1e1e1e;
		overflow: hidden;
	}
	main {
		position: absolute;
		inset: 0;
	}
</style>
