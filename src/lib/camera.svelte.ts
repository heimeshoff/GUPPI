// Camera state — the single source of truth for pan + zoom, modelled as
// Svelte 5 runes (ADR-003). Both the PixiJS scene and any HTML overlay layer
// subscribe to this so they stay in lockstep.

import type { CameraState } from './types';

/** Zoom clamp — keeps the wheel from inverting or exploding the scene. */
const MIN_ZOOM = 0.1;
const MAX_ZOOM = 5;

/**
 * Reactive camera. `pan` is in screen pixels (the world-origin's screen
 * position); `zoom` is a scalar. World-to-screen mapping:
 *   screen = world * zoom + pan
 */
export class Camera {
	pan_x = $state(0);
	pan_y = $state(0);
	zoom = $state(1);

	/** Apply a relative pan in screen pixels (drag delta). */
	panBy(dx: number, dy: number) {
		this.pan_x += dx;
		this.pan_y += dy;
	}

	/**
	 * Zoom by `factor` about a screen-space anchor (the cursor), keeping the
	 * world point under the cursor fixed — the standard infinite-canvas feel.
	 */
	zoomAt(factor: number, screenX: number, screenY: number) {
		const next = clamp(this.zoom * factor, MIN_ZOOM, MAX_ZOOM);
		const applied = next / this.zoom;
		// Keep the world point under (screenX, screenY) stationary.
		this.pan_x = screenX - (screenX - this.pan_x) * applied;
		this.pan_y = screenY - (screenY - this.pan_y) * applied;
		this.zoom = next;
	}

	/** Map a world coordinate to a screen coordinate. */
	worldToScreen(x: number, y: number): { x: number; y: number } {
		return { x: x * this.zoom + this.pan_x, y: y * this.zoom + this.pan_y };
	}

	/** Serialise for persistence (ADR-004). */
	snapshot(): CameraState {
		return { pan_x: this.pan_x, pan_y: this.pan_y, zoom: this.zoom };
	}

	/** Restore from a persisted snapshot. */
	restore(state: CameraState) {
		this.pan_x = state.pan_x;
		this.pan_y = state.pan_y;
		this.zoom = clamp(state.zoom, MIN_ZOOM, MAX_ZOOM);
	}
}

function clamp(value: number, lo: number, hi: number): number {
	return Math.min(hi, Math.max(lo, value));
}
