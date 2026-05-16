// Pure deterministic layout for BC bubbles inside a project frame
// (`canvas-007-project-as-frame`).
//
// The project frame (design-system-002 §3.6) contains a project's bounded
// contexts as bubbles (§3.7) connected by intra-project edges drawn from
// the parsed BC↔BC relationship graph (§3.8). This module computes the
// world-space layout: each BC's local (frame-relative) position, plus the
// frame's auto-fit inner width/height.
//
// Strategy: a one-shot, fixed-seed force-directed (Fruchterman-Reingold-
// style spring-electrical) layout, run for a bounded iteration count with
// a deterministic cooling schedule. Same input -> same output: a
// never-dragged BC lands in the same spot across restarts.
//
// Manual drag overrides win. When a BC has a saved position in
// `savedPositions`, the layout pins that BC at its saved spot and lays the
// rest out around the pinned set. Re-layout fires only on BC add / remove
// / relationship-change (the canvas observes this via
// `bc_relationships_changed` and BC appearance/disappearance events).
//
// Deliberately a stand-alone module with **no** imports from
// `./Canvas.svelte`, `./camera.svelte`, `pixi.js`, or Svelte: those bring
// runtime dependencies that block plain unit testing. Verification mirrors
// the `tile-layout.ts` / `snapshot-patch.ts` strategy.

import { shape } from './design/tokens';
import type { BoundedContext, Point } from './types';

/** Per-BC layout output — local-to-frame coordinates (origin = frame's
 *  inner top-left, *inside* `framePadding`). */
export type BcPositionMap = Map<string, Point>;

/** The complete layout for one project frame. `width` / `height` are the
 *  frame's outer extent in world space (already including header + padding);
 *  `positions` are the per-BC top-left corners in world-space relative to
 *  the frame's outer top-left. The canvas adds the frame's own world-space
 *  origin to draw. */
export interface BcFrameLayout {
	width: number;
	height: number;
	positions: BcPositionMap;
}

/* ----------------------------------------------------------------- */
/* Deterministic seeded RNG — Mulberry32. Pure, stateless callers     */
/* take an integer seed and read sequential values.                   */
/* ----------------------------------------------------------------- */

function mulberry32(seed: number): () => number {
	let a = seed >>> 0;
	return function () {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Stable hash of a string into a 32-bit integer — used to derive a
 *  per-BC initial position so layouts of identical inputs match across
 *  runs without depending on insertion order. djb2. */
function hashString(s: string): number {
	let h = 5381;
	for (let i = 0; i < s.length; i++) {
		h = (h * 33) ^ s.charCodeAt(i);
	}
	return h >>> 0;
}

/* ----------------------------------------------------------------- */
/* Layout core — fixed iterations, deterministic cooling.             */
/* ----------------------------------------------------------------- */

/** Iterations for the spring layout — a small constant since the graphs
 *  are tiny (single-project BC counts in v1 are at most ~10–20). */
const ITERATIONS = 120;

/** Ideal spring length between two connected BCs, in world-space px.
 *  Sized to leave clear gaps between adjacent bubble rectangles. */
const SPRING_LENGTH = shape.bcInsideWidth + 40;

/** Repulsion strength constant (k). Larger -> nodes push apart harder. */
const REPULSION_K = SPRING_LENGTH * SPRING_LENGTH;

/** Attraction strength: how strongly an edge contracts. Tuned for
 *  visible clustering of related BCs without piling them up. */
const ATTRACTION_K = 1 / SPRING_LENGTH;

/** Initial temperature for the cooling schedule — caps the per-iteration
 *  displacement so a high-energy first step does not fling nodes off-canvas. */
const INITIAL_TEMPERATURE = SPRING_LENGTH;

/** Compute the BC layout inside a project frame. Deterministic: same
 *  `bcs` (by name, sorted) + same `savedPositions` -> same output.
 *
 *  `bcs` is the project's `BoundedContext[]`; the function reads only
 *  `name` and `relationships`. `savedPositions` are persisted manual-drag
 *  positions keyed by BC name (frame-local coords, matching what
 *  `Canvas.svelte` persists via `saveBcPosition`); BCs present here are
 *  pinned and not moved by the simulation.
 *
 *  The frame auto-fits to its content: the returned `width` / `height`
 *  span the union of the BC rectangles plus `framePadding` on every side
 *  and a `frameHeaderHeight` band at the top. Width and height are
 *  floored at `frameMinInnerWidth + framePadding * 2` and
 *  `frameMinInnerHeight + frameHeaderHeight + framePadding * 2` so a
 *  sparse / empty frame still reads as a region.
 *
 *  Pure: no globals, no IO, no async. Safe to call on every layout-
 *  triggering event (BC add/remove/relationship-change). */
export function computeBcLayout(
	bcs: BoundedContext[],
	savedPositions: BcPositionMap
): BcFrameLayout {
	// Empty frame: return the minimum-size frame with no positions.
	if (bcs.length === 0) {
		return {
			width: shape.frameMinInnerWidth + shape.framePadding * 2,
			height:
				shape.frameMinInnerHeight +
				shape.frameHeaderHeight +
				shape.framePadding * 2,
			positions: new Map()
		};
	}

	// Sort BCs by name so the simulation's iteration order is stable
	// regardless of how the snapshot's `bcs` array got built.
	const ordered = [...bcs].sort((a, b) => a.name.localeCompare(b.name));

	// Initial positions: BCs with saved positions sit at their saved
	// (local) spot; the rest get a deterministic seeded layout on a
	// small jittered grid, so identical input always yields identical
	// initial state.
	const N = ordered.length;
	const cols = Math.max(1, Math.ceil(Math.sqrt(N)));
	const cellW = shape.bcInsideWidth + 40;
	const cellH = shape.bcInsideHeight + 40;

	const positions: { x: number; y: number; pinned: boolean }[] = [];
	const indexByName = new Map<string, number>();
	for (let i = 0; i < N; i++) {
		const bc = ordered[i];
		indexByName.set(bc.name, i);
		const saved = savedPositions.get(bc.name);
		if (saved) {
			positions.push({ x: saved.x, y: saved.y, pinned: true });
			continue;
		}
		const rng = mulberry32(hashString(bc.name));
		const col = i % cols;
		const row = Math.floor(i / cols);
		// Center the unsaved nodes around (0, 0) initially; the layout's
		// global recenter at the end shifts everything into the frame's
		// inner box.
		const jx = (rng() - 0.5) * 20;
		const jy = (rng() - 0.5) * 20;
		positions.push({
			x: (col - (cols - 1) / 2) * cellW + jx,
			y: (row - (Math.ceil(N / cols) - 1) / 2) * cellH + jy,
			pinned: false
		});
	}

	// Build the undirected edge set. The relationship graph is the union
	// of every BC's `relationships[]`, dropping any `to` that does not
	// resolve to a sibling in this project (cross-project references the
	// registry parser already drops — but we are defensive in case the
	// payload ever changes). A relationship between A and B contracts the
	// spring whether it is declared from A->B, B->A, or both.
	const edgePairs = new Set<string>();
	const edges: Array<[number, number]> = [];
	for (const bc of ordered) {
		const from = indexByName.get(bc.name);
		if (from === undefined) continue;
		for (const rel of bc.relationships) {
			const to = indexByName.get(rel.to);
			if (to === undefined) continue; // cross-project / unresolved
			if (from === to) continue;
			const lo = Math.min(from, to);
			const hi = Math.max(from, to);
			const key = `${lo}-${hi}`;
			if (edgePairs.has(key)) continue;
			edgePairs.add(key);
			edges.push([lo, hi]);
		}
	}

	// Spring-electrical simulation. Bounded iterations + cooling
	// schedule = deterministic, terminates in O(iterations * N^2).
	let temperature = INITIAL_TEMPERATURE;
	const cooling = INITIAL_TEMPERATURE / ITERATIONS;

	const disp: { x: number; y: number }[] = positions.map(() => ({ x: 0, y: 0 }));

	for (let iter = 0; iter < ITERATIONS; iter++) {
		// Reset displacements.
		for (let i = 0; i < N; i++) {
			disp[i].x = 0;
			disp[i].y = 0;
		}

		// Repulsive forces (every pair).
		for (let i = 0; i < N; i++) {
			for (let j = i + 1; j < N; j++) {
				let dx = positions[i].x - positions[j].x;
				let dy = positions[i].y - positions[j].y;
				let dist2 = dx * dx + dy * dy;
				if (dist2 < 0.0001) {
					// Coincident nodes -> nudge them apart deterministically.
					dx = (i - j) * 0.01;
					dy = (i + j) * 0.01;
					dist2 = dx * dx + dy * dy;
				}
				const dist = Math.sqrt(dist2);
				const force = REPULSION_K / dist2;
				const fx = (dx / dist) * force;
				const fy = (dy / dist) * force;
				disp[i].x += fx;
				disp[i].y += fy;
				disp[j].x -= fx;
				disp[j].y -= fy;
			}
		}

		// Attractive forces (edges).
		for (const [a, b] of edges) {
			let dx = positions[a].x - positions[b].x;
			let dy = positions[a].y - positions[b].y;
			let dist2 = dx * dx + dy * dy;
			if (dist2 < 0.0001) continue;
			const dist = Math.sqrt(dist2);
			const force = ATTRACTION_K * dist2;
			const fx = (dx / dist) * force;
			const fy = (dy / dist) * force;
			disp[a].x -= fx;
			disp[a].y -= fy;
			disp[b].x += fx;
			disp[b].y += fy;
		}

		// Apply displacements (capped by temperature). Pinned nodes do
		// not move.
		for (let i = 0; i < N; i++) {
			if (positions[i].pinned) continue;
			const dx = disp[i].x;
			const dy = disp[i].y;
			const mag = Math.sqrt(dx * dx + dy * dy);
			if (mag < 0.0001) continue;
			const capped = Math.min(mag, temperature);
			positions[i].x += (dx / mag) * capped;
			positions[i].y += (dy / mag) * capped;
		}

		temperature = Math.max(0, temperature - cooling);
	}

	// --- Compute the bounding box and translate everything into the
	//     frame's inner content rectangle.
	//
	// Manual-drag (pinned) positions are sticky: a BC the user dragged to
	// frame-local `(x, y)` must render at exactly `(x, y)` after re-
	// layout. Two cases:
	//
	//   - no pins   : translate so the simulation's content min-corner
	//                 sits at (framePadding, frameHeaderHeight + framePadding)
	//                 with the union centered in the inner box if it is
	//                 smaller than the minimum frame size.
	//   - any pins  : the pinned BCs carry their authoritative frame-local
	//                 coords; the simulation has moved non-pins around
	//                 them. We do NOT translate (offset = 0) so pinned
	//                 positions render exactly where the user dragged
	//                 them. The frame auto-fits its outer rectangle to
	//                 cover the union, floored at the inner minimums.
	//
	// If a pinned BC has been dragged to a position whose rectangle
	// extends beyond the frame's right/bottom edge, the frame grows. If
	// a pinned BC was dragged into negative frame-local coords, that BC
	// will visually sit OUTSIDE the frame body (to the left/above
	// `entry.pos`); this is accepted — the simpler invariant "frame
	// top-left = entry.pos" beats automatic re-anchoring that would
	// silently rewrite the user's saved position.
	const bw = shape.bcInsideWidth;
	const bh = shape.bcInsideHeight;
	let minX = Number.POSITIVE_INFINITY;
	let minY = Number.POSITIVE_INFINITY;
	let maxX = Number.NEGATIVE_INFINITY;
	let maxY = Number.NEGATIVE_INFINITY;
	for (const p of positions) {
		if (p.x < minX) minX = p.x;
		if (p.y < minY) minY = p.y;
		if (p.x + bw > maxX) maxX = p.x + bw;
		if (p.y + bh > maxY) maxY = p.y + bh;
	}

	const hasPins = positions.some((p) => p.pinned);

	let offsetX: number;
	let offsetY: number;
	let width: number;
	let height: number;

	if (hasPins) {
		offsetX = 0;
		offsetY = 0;
		// Outer frame size = max-corner of any BC + framePadding,
		// floored at the inner minimums + header + padding.
		width = Math.max(
			maxX + shape.framePadding,
			shape.frameMinInnerWidth + shape.framePadding * 2
		);
		height = Math.max(
			maxY + shape.framePadding,
			shape.frameMinInnerHeight + shape.frameHeaderHeight + shape.framePadding * 2
		);
	} else {
		const contentW = maxX - minX;
		const contentH = maxY - minY;
		const innerW = Math.max(contentW, shape.frameMinInnerWidth);
		const innerH = Math.max(contentH, shape.frameMinInnerHeight);
		offsetX = shape.framePadding + (innerW - contentW) / 2 - minX;
		offsetY =
			shape.frameHeaderHeight + shape.framePadding + (innerH - contentH) / 2 - minY;
		width = innerW + shape.framePadding * 2;
		height = innerH + shape.frameHeaderHeight + shape.framePadding * 2;
	}

	const out: BcPositionMap = new Map();
	for (let i = 0; i < N; i++) {
		const name = ordered[i].name;
		out.set(name, {
			x: positions[i].x + offsetX,
			y: positions[i].y + offsetY
		});
	}

	return {
		width,
		height,
		positions: out
	};
}
