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

	/**
	 * Camera affordance (design-system styleguide): frame a world-space
	 * bounding box so it sits centred in the given viewport with a margin.
	 * Used by zoom-to-fit. Sets state directly — callers that want the
	 * styleguide's eased transition should `tweenTo` toward `fitTo`'s result.
	 */
	fitTo(
		box: { x: number; y: number; w: number; h: number },
		viewport: { w: number; h: number },
		marginPx = 64
	): CameraState {
		const usableW = Math.max(1, viewport.w - marginPx * 2);
		const usableH = Math.max(1, viewport.h - marginPx * 2);
		const zoom = clamp(Math.min(usableW / box.w, usableH / box.h), MIN_ZOOM, MAX_ZOOM);
		// Centre the box: screen-centre = world-centre * zoom + pan.
		const pan_x = viewport.w / 2 - (box.x + box.w / 2) * zoom;
		const pan_y = viewport.h / 2 - (box.y + box.h / 2) * zoom;
		return { pan_x, pan_y, zoom };
	}

	/**
	 * Linear interpolation toward a target camera state by factor `t` in
	 * [0,1]. The styleguide's eased transitions (zoom-to-fit, focus-on-tile)
	 * call this each animation frame with an eased `t`.
	 */
	lerpTo(target: CameraState, t: number) {
		this.pan_x += (target.pan_x - this.pan_x) * t;
		this.pan_y += (target.pan_y - this.pan_y) * t;
		this.zoom += (target.zoom - this.zoom) * t;
	}
}

function clamp(value: number, lo: number, hi: number): number {
	return Math.min(hi, Math.max(lo, value));
}
