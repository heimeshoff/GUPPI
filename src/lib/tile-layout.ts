// Pure auto-placement for project tiles on the canvas (canvas-002).
//
// When a project has no saved tile position yet — first registration, or first
// time it is seen on this machine — the canvas needs a stable, non-overlapping
// world-space slot for it. This module is the deterministic source of those
// slots, isolated from Svelte and PixiJS so it can be reviewed and (when
// frontend test infra arrives) unit-tested without a browser.
//
// Strategy: an outward spiral from world origin, indexed by registration order.
// We walk a square-ring spiral (1 right, 1 down, 2 left, 2 up, 3 right, 3 down,
// …) so each new project lands one slot further out, packed tightly with no
// overlap. World-space slot size is `tileWidth + gap` by `tileHeight + gap`,
// where the gap leaves enough room for each tile's orbiting BC nodes (using
// `bcOrbitRadius` as the inter-tile clearance).
//
// Deliberately a stand-alone module with **no** imports from `./Canvas.svelte`,
// `./camera.svelte`, `pixi.js`, or Svelte: those bring runtime dependencies
// that block plain unit testing. Verification mirrors the `snapshot-patch.ts`
// strategy from canvas-001.

import { shape } from './design/tokens';
import type { Point } from './types';

/** World-space cell width: tile + clearance for the orbiting BC nodes. */
const CELL_W = shape.tileWidth + shape.bcOrbitRadius * 2;
/** World-space cell height: tile + clearance for the orbiting BC nodes. */
const CELL_H = shape.tileHeight + shape.bcOrbitRadius * 2;

/**
 * The world-space position of the `index`-th auto-placed project tile.
 *
 * `spiralPosition(0)` is the origin (0, 0). Subsequent indices walk a tight
 * square spiral around it: right, down, left×2, up×2, right×3, down×3, … so
 * consecutive indices are adjacent and the placed set forms a packed square
 * with no overlap.
 *
 * Pure: same input → same output, no globals, no IO. Safe to call from any
 * frame; the canvas calls it once per project at registration and persists the
 * result, so it is not on the hot path.
 */
export function spiralPosition(index: number): Point {
	if (index < 0 || !Number.isFinite(index)) {
		return { x: 0, y: 0 };
	}
	if (index === 0) {
		return { x: 0, y: 0 };
	}

	// Step through the square spiral: at each "leg" we move `legLength`
	// cells in a direction, and the leg length grows by 1 every two legs
	// (right, down, left, left, up, up, right, right, right, …).
	//
	// We accumulate cell-deltas; the final position is `(cx, cy) * cell`.
	let cx = 0;
	let cy = 0;
	let dirIndex = 0; // 0=right, 1=down, 2=left, 3=up
	let legLength = 1;
	let stepsInLeg = 0;
	let legsAtCurrentLength = 0;

	const dx = [1, 0, -1, 0];
	const dy = [0, 1, 0, -1];

	for (let i = 0; i < index; i++) {
		cx += dx[dirIndex];
		cy += dy[dirIndex];
		stepsInLeg += 1;
		if (stepsInLeg === legLength) {
			stepsInLeg = 0;
			dirIndex = (dirIndex + 1) % 4;
			legsAtCurrentLength += 1;
			// The leg length grows by 1 every two legs (right, down, then
			// left×2, up×2, right×3, …).
			if (legsAtCurrentLength === 2) {
				legsAtCurrentLength = 0;
				legLength += 1;
			}
		}
	}

	return { x: cx * CELL_W, y: cy * CELL_H };
}

/**
 * Convenience: positions for indices `0..count-1`. Useful for batch initial
 * placement when seeding tiles from `list_projects()`. The canvas typically
 * calls `spiralPosition` per-project as projects are encountered (some have
 * saved positions; only the unsaved ones need a slot), so this helper is
 * provided mostly for tests and review.
 */
export function spiralPositions(count: number): Point[] {
	const out: Point[] = [];
	for (let i = 0; i < count; i++) out.push(spiralPosition(i));
	return out;
}
