<script lang="ts">
	// Generic modal primitive (canvas-005b). A screen-space HTML overlay
	// (ADR-003 overlay layer) centered above a dimmed backdrop, with three
	// snippet slots: `header`, `body`, `footer`. Dismissal: `Escape` key,
	// click on the backdrop, or any explicit cancel/close button rendered
	// inside the footer snippet (which calls `onclose` directly).
	//
	// Three consumers in this codebase right now (canvas-005b's checklist,
	// manage-roots, and cascade-confirm modals). A fourth consumer is the
	// stated threshold for codifying the pattern as a `STYLEGUIDE.md`
	// component entry; a follow-up backlog task tracks that.
	//
	// Stacking: the parent component opens at most one modal at a time —
	// EXCEPT the cascade-confirm dialog, which stacks ON TOP of the open
	// manage-roots modal (the management modal stays mounted behind it).
	// The backdrop is opaque enough that the lower modal reads as dimmed
	// context; the `z-index` ordering is driven by the parent's render
	// order, not by anything here.
	//
	// Styling is entirely token-driven; no hard-coded values.

	import type { Snippet } from 'svelte';

	interface Props {
		/** Called when the user dismisses the modal via the backdrop or
		 * `Escape`. The header/body/footer snippets are free to also call
		 * this from their own buttons. */
		onclose: () => void;
		/** Optional override for max-width — the checklist/manage-roots
		 * modals use the default 720px; the cascade-confirm dialog wants
		 * narrower. */
		maxWidth?: string;
		header: Snippet;
		body: Snippet;
		footer: Snippet;
	}

	let { onclose, maxWidth = '720px', header, body, footer }: Props = $props();

	function onBackdropPointerDown(e: PointerEvent) {
		// Dismiss only on a click that originated on the backdrop itself —
		// clicks bubbling up from inside the modal body must not close it.
		if (e.target === e.currentTarget) {
			e.stopPropagation();
			onclose();
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.stopPropagation();
			onclose();
		}
	}
</script>

<svelte:window onkeydown={onKeydown} />

<div
	class="modal-backdrop"
	role="presentation"
	onpointerdown={onBackdropPointerDown}
>
	<div
		class="modal"
		role="dialog"
		aria-modal="true"
		tabindex="-1"
		style="max-width: {maxWidth};"
		onpointerdown={(e) => e.stopPropagation()}
	>
		<div class="modal-header">
			{@render header()}
		</div>
		<div class="modal-body">
			{@render body()}
		</div>
		<div class="modal-footer">
			{@render footer()}
		</div>
	</div>
</div>

<style>
	@import './design/tokens.css';

	/*
	 * The backdrop is a fixed full-viewport surface at 70% of
	 * `--guppi-canvas-bg`'s colour, so the modal body itself can sit at
	 * full opacity and stay legible. Token literal `#16161c` is
	 * `--guppi-canvas-bg`; if that token shifts, this RGB must move too.
	 */
	.modal-backdrop {
		position: fixed;
		inset: 0;
		background: rgb(22 22 28 / 70%);
		display: flex;
		align-items: center;
		justify-content: center;
		padding: var(--guppi-space-lg);
		z-index: 20;
	}
	.modal {
		display: flex;
		flex-direction: column;
		width: 100%;
		max-height: 80vh;
		background: var(--guppi-tile-fill);
		border: var(--guppi-border-width) solid var(--guppi-tile-border);
		border-radius: var(--guppi-radius-tile);
		color: var(--guppi-tile-text);
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		overflow: hidden;
		user-select: none;
	}
	.modal-header {
		padding: var(--guppi-space-lg);
		border-bottom: 1px solid var(--guppi-canvas-bg-raised);
		font-weight: var(--guppi-weight-bold);
	}
	.modal-body {
		padding: var(--guppi-space-lg);
		overflow-y: auto;
		flex: 1 1 auto;
	}
	.modal-footer {
		padding: var(--guppi-space-lg);
		border-top: 1px solid var(--guppi-canvas-bg-raised);
		display: flex;
		justify-content: flex-end;
		gap: var(--guppi-space-md);
	}
</style>
