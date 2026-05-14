<script lang="ts">
	// GUPPI's single screen: a full-window canvas (ADR-002 + ADR-003), rendered
	// at the design-system styleguide baseline (design-system-001-styleguide).
	// The global document chrome reads the design tokens.
	import Canvas from '$lib/Canvas.svelte';
	import '$lib/design/tokens.css';
</script>

<main>
	<Canvas />
</main>

<style>
	:global(html, body) {
		margin: 0;
		padding: 0;
		height: 100%;
		background: var(--guppi-canvas-bg);
		color: var(--guppi-bc-text);
		font-family: var(--guppi-font-family);
		overflow: hidden;
	}
	main {
		position: absolute;
		inset: 0;
	}
</style>
