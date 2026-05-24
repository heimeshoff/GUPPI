// Pure layout / visibility-policy helpers behind the kanban-accordion frame
// interior (canvas-020, ADR-017).
//
// The canvas pivot (ADR-017) makes each project frame a hybrid: a PixiJS
// world-space **shell** (border + header + drag hit-area) plus a DOM **interior**
// (the BC accordion → kanban board → task cards) positioned to world coordinates
// via `camera.worldToScreen` and scaled with `transform: scale(z)`.
//
// The interior is a Svelte/DOM overlay and is not headless-unit-testable, so the
// *decisions* it leans on live here as pure functions:
//   - bucketing a BC's tasks[] into the four kanban columns (`project-registry-005`)
//   - the deterministic default frame shell size (bc-layout.ts autofit retired)
//   - the ADR-017 cost-governor policy (viewport culling + zoom-floor LOD) that
//     decides whether a given frame mounts its DOM interior at all
//   - the screen-space AABB of a frame under the camera transform
//
// No Svelte, no PixiJS, no IPC — keeps the policy reviewable and testable.

import type { Task, TaskColumn } from './types';
import { shape } from './design/tokens';

/** The four kanban columns in task-file lifecycle order — the fixed column
 *  layout of every BC's board (styleguide §3.10). */
export const COLUMN_ORDER: readonly TaskColumn[] = [
	'backlog',
	'todo',
	'doing',
	'done'
] as const;

/** Tasks grouped by their kanban column. */
export type ColumnBuckets = Record<TaskColumn, Task[]>;

/** Bucket a BC's `tasks[]` into the four kanban columns by `task.column`
 *  (`project-registry-005`). Input order is preserved within each column —
 *  the snapshot owns sort order (`snapshot-patch.ts` keeps `tasks[]` sorted by
 *  column-rank then id), and bucketing must not reshuffle it. */
export function bucketTasksByColumn(tasks: Task[]): ColumnBuckets {
	const buckets: ColumnBuckets = {
		backlog: [],
		todo: [],
		doing: [],
		done: []
	};
	for (const t of tasks) buckets[t.column].push(t);
	return buckets;
}

/**
 * Format the live-agent indicator's elapsed duration (canvas-021) from
 * agent-awareness-002's `since` Unix-ms transition timestamp. The card derives
 * "waiting 2m 14s" locally and a local timer re-invokes this so the visible
 * value advances without a per-second event (`agent-awareness-002`, ADR-018).
 *
 * Scale-adaptive, compact:
 *   - under a minute → `"Ns"` (e.g. `"5s"`, `"59s"`)
 *   - under an hour  → `"Mm Ss"` (e.g. `"2m 14s"`, `"1m 0s"`)
 *   - an hour or more → `"Hh Mm"` (seconds dropped — at the hour scale the
 *     ticking second is visual noise)
 *
 * Sub-second remainders floor (no fractional seconds). A future / equal / skewed
 * `since` clamps to `"0s"` (the runner's clock and GUPPI's may differ by a few
 * ms; never render a negative duration).
 */
export function formatElapsed(sinceMs: number, nowMs: number): string {
	const totalSeconds = Math.max(0, Math.floor((nowMs - sinceMs) / 1000));
	if (totalSeconds < 60) return `${totalSeconds}s`;
	const totalMinutes = Math.floor(totalSeconds / 60);
	if (totalMinutes < 60) {
		const seconds = totalSeconds % 60;
		return `${totalMinutes}m ${seconds}s`;
	}
	const hours = Math.floor(totalMinutes / 60);
	const minutes = totalMinutes % 60;
	return `${hours}h ${minutes}m`;
}

/** A world-space size (zoom-1 px). The DOM interior is laid out once at this
 *  size and zoom is a compositor `transform: scale(z)` (ADR-017), never a
 *  reflow. */
export interface FrameSize {
	width: number;
	height: number;
}

/**
 * The deterministic default frame **shell** size. bc-layout.ts's per-BC
 * force-directed autofit is retired (ADR-017): the frame is a fixed-size region
 * and its kanban-accordion interior scrolls within it. The size is the same for
 * every frame regardless of BC count (the `bcCount` parameter is accepted so a
 * future per-content sizing pass can land without changing call sites, but v1
 * is intentionally constant — the interior, not the shell, absorbs content).
 *
 * Width: enough for the four kanban columns (so a default-zoom expanded board
 * shows ~2 columns before horizontal scroll) plus the frame padding. Height:
 * the `frameMin*` floor plus the header — tall enough to read a couple of
 * accordion rows before the interior scrolls.
 */
export function frameSize(_bcCount: number): FrameSize {
	const width = Math.max(
		shape.frameMinInnerWidth,
		shape.kanbanColumnMinWidth * 2 + shape.kanbanColumnGap + shape.accordionRowPadding * 2
	);
	const height = shape.frameHeaderHeight + shape.frameMinInnerHeight + shape.framePadding * 2;
	return { width, height };
}

/** A screen-space axis-aligned bounding box. */
export interface ScreenAabb {
	left: number;
	top: number;
	right: number;
	bottom: number;
}

/** The minimal camera shape these helpers read (a `Camera` satisfies it). */
export interface CameraLike {
	pan_x: number;
	pan_y: number;
	zoom: number;
}

/** A viewport size in CSS px. */
export interface Viewport {
	w: number;
	h: number;
}

/** Map a frame's world-space rect to its screen-space AABB under the camera
 *  transform (`screen = world * zoom + pan`). The same mapping `worldToScreen`
 *  performs, computed on the rect's two corners. */
export function frameScreenAabb(
	worldPos: { x: number; y: number },
	size: FrameSize,
	camera: CameraLike
): ScreenAabb {
	const left = worldPos.x * camera.zoom + camera.pan_x;
	const top = worldPos.y * camera.zoom + camera.pan_y;
	const right = (worldPos.x + size.width) * camera.zoom + camera.pan_x;
	const bottom = (worldPos.y + size.height) * camera.zoom + camera.pan_y;
	return { left, top, right, bottom };
}

/**
 * ADR-017 viewport-culling margin (px). Only frames whose screen-space AABB
 * intersects the viewport *plus this margin* mount a DOM interior; off-screen
 * frames render the cheap persistent Pixi shell only. The margin mounts
 * interiors slightly off-screen so a fast pan does not flash an empty shell as
 * a frame scrolls in. Tunable on-machine (canvas-019a spike recommendation).
 */
export const CULL_MARGIN_PX = 120;

/**
 * ADR-017 zoom-threshold level-of-detail floor. Below this zoom ALL DOM
 * interiors are suppressed (Pixi shells only) and re-mount on zoom-in. This is
 * rendering-layer LOD, NOT a model-level summary — the same frame renders at a
 * different fidelity depending on zoom (decision #1 holds intact). At the floor
 * an 11px card font renders sub-5px (illegible anyway), so suppressing it is
 * visually free.
 */
export const LOD_ZOOM_FLOOR = 0.45;

/**
 * The ADR-017 cost-governor policy: should this frame mount its DOM interior?
 *
 * `false` (cheap Pixi shell only) when:
 *   - the zoom is below `LOD_ZOOM_FLOOR` (global LOD suppression), OR
 *   - the frame's screen-space AABB does not intersect the viewport expanded by
 *     `CULL_MARGIN_PX` (off-screen culling).
 *
 * Otherwise `true` (mount the full DOM accordion-kanban interior).
 */
export function shouldMountInterior(
	aabb: ScreenAabb,
	viewport: Viewport,
	zoom: number,
	opts: { cullMarginPx?: number; lodZoomFloor?: number } = {}
): boolean {
	const margin = opts.cullMarginPx ?? CULL_MARGIN_PX;
	const floor = opts.lodZoomFloor ?? LOD_ZOOM_FLOOR;
	if (zoom < floor) return false;
	// AABB-vs-expanded-viewport intersection test.
	if (aabb.right < -margin) return false;
	if (aabb.left > viewport.w + margin) return false;
	if (aabb.bottom < -margin) return false;
	if (aabb.top > viewport.h + margin) return false;
	return true;
}
