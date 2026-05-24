// Tests for `frame-interior.ts` — the pure layout / visibility-policy helpers
// behind the kanban-accordion frame interior (canvas-020, ADR-017).
//
// The interior itself is a Svelte/DOM overlay (not unit-testable headless), so
// the *decisions* it leans on are extracted here as pure functions and pinned:
//   - kanban column bucketing of a BC's tasks by `task.column`
//   - the deterministic default frame size (bc-layout.ts retired)
//   - the ADR-017 cost-governor policy: viewport culling (CULL_MARGIN_PX) +
//     zoom-threshold LOD (LOD_ZOOM_FLOOR) deciding whether a frame mounts its
//     DOM interior at all
//   - screen-space AABB of a frame under the camera transform

import { describe, it, expect } from 'vitest';
import type { BoundedContext, Task, TaskColumn } from './types';
import { shape } from './design/tokens';
import {
	COLUMN_ORDER,
	CULL_MARGIN_PX,
	FRAME_ASPECT_RATIO,
	LOD_ZOOM_FLOOR,
	bucketTasksByColumn,
	formatElapsed,
	frameSize,
	frameScreenAabb,
	shouldMountInterior
} from './frame-interior';

function task(id: string, column: TaskColumn): Task {
	return { id, title: id, column, type_: 'feature', tags: [] };
}

function bc(name: string, tasks: Task[]): BoundedContext {
	const counts = { backlog: 0, todo: 0, doing: 0, done: 0 };
	for (const t of tasks) counts[t.column] += 1;
	return { name, task_counts: counts, tasks, relationships: [] };
}

describe('COLUMN_ORDER', () => {
	it('is the four kanban columns in lifecycle order', () => {
		expect(COLUMN_ORDER).toEqual(['backlog', 'todo', 'doing', 'done']);
	});
});

describe('bucketTasksByColumn', () => {
	it('groups a BC tasks[] into the four columns by task.column', () => {
		const b = bc('canvas', [
			task('c-1', 'backlog'),
			task('c-2', 'todo'),
			task('c-3', 'doing'),
			task('c-4', 'done'),
			task('c-5', 'backlog')
		]);
		const buckets = bucketTasksByColumn(b.tasks);
		expect(buckets.backlog.map((t) => t.id)).toEqual(['c-1', 'c-5']);
		expect(buckets.todo.map((t) => t.id)).toEqual(['c-2']);
		expect(buckets.doing.map((t) => t.id)).toEqual(['c-3']);
		expect(buckets.done.map((t) => t.id)).toEqual(['c-4']);
	});

	it('returns four empty arrays for a BC with no tasks', () => {
		const buckets = bucketTasksByColumn([]);
		expect(buckets.backlog).toEqual([]);
		expect(buckets.todo).toEqual([]);
		expect(buckets.doing).toEqual([]);
		expect(buckets.done).toEqual([]);
	});

	it('preserves the input order within a column', () => {
		const buckets = bucketTasksByColumn([
			task('b-2', 'backlog'),
			task('b-1', 'backlog')
		]);
		// the caller (snapshot) owns sort order; bucketing must not reorder.
		expect(buckets.backlog.map((t) => t.id)).toEqual(['b-2', 'b-1']);
	});
});

describe('frameSize', () => {
	it('returns a fixed sheet size independent of bc count', () => {
		const a = frameSize(0);
		const b = frameSize(5);
		expect(a.width).toBeGreaterThan(0);
		expect(a.height).toBeGreaterThan(0);
		// `bcCount` is unused — the A4 sheet is the same size for every frame.
		expect(b.width).toBe(a.width);
		expect(b.height).toBe(a.height);
	});

	it('has a DIN-A4 portrait aspect ratio (height = width × √2)', () => {
		const { width, height } = frameSize(3);
		expect(height).toBe(Math.round(width * FRAME_ASPECT_RATIO));
		// Portrait: taller than wide.
		expect(height).toBeGreaterThan(width);
	});

	it('is wide enough to show all four kanban columns without horizontal scroll', () => {
		// body padding (both sides) + board padding (both sides) + 4 columns + 3 gaps.
		const expected =
			shape.framePadding * 2 +
			shape.accordionRowPadding * 2 +
			shape.kanbanColumnMinWidth * 4 +
			shape.kanbanColumnGap * 3;
		expect(frameSize(3).width).toBe(expected);
		// Sanity: it really does fit four columns + their gaps inside the padding.
		const innerForBoard =
			frameSize(3).width - shape.framePadding * 2 - shape.accordionRowPadding * 2;
		expect(innerForBoard).toBeGreaterThanOrEqual(
			shape.kanbanColumnMinWidth * 4 + shape.kanbanColumnGap * 3
		);
	});
});

describe('frameScreenAabb', () => {
	it('maps a world-space frame rect to a screen-space AABB via the camera', () => {
		const aabb = frameScreenAabb(
			{ x: 100, y: 50 },
			{ width: 400, height: 300 },
			{ pan_x: 10, pan_y: 20, zoom: 2 }
		);
		// screen = world * zoom + pan
		expect(aabb.left).toBe(100 * 2 + 10);
		expect(aabb.top).toBe(50 * 2 + 20);
		expect(aabb.right).toBe((100 + 400) * 2 + 10);
		expect(aabb.bottom).toBe((50 + 300) * 2 + 20);
	});
});

describe('formatElapsed — live-agent indicator duration (canvas-021)', () => {
	const since = 1_000_000_000_000;

	it('formats the reference "2m 14s" from a since timestamp', () => {
		const now = since + (2 * 60 + 14) * 1000;
		expect(formatElapsed(since, now)).toBe('2m 14s');
	});

	it('shows seconds only under a minute', () => {
		expect(formatElapsed(since, since + 5 * 1000)).toBe('5s');
		expect(formatElapsed(since, since + 59 * 1000)).toBe('59s');
	});

	it('rolls into minutes at 60s', () => {
		expect(formatElapsed(since, since + 60 * 1000)).toBe('1m 0s');
	});

	it('rolls into hours past 60 minutes (drops seconds at the hour scale)', () => {
		const now = since + (1 * 3600 + 5 * 60 + 30) * 1000;
		expect(formatElapsed(since, now)).toBe('1h 5m');
	});

	it('clamps a future/equal/negative since to 0s (clock skew safety)', () => {
		expect(formatElapsed(since, since)).toBe('0s');
		expect(formatElapsed(since, since - 5000)).toBe('0s');
	});

	it('floors sub-second remainders (no fractional seconds shown)', () => {
		expect(formatElapsed(since, since + 1750)).toBe('1s');
	});
});

describe('shouldMountInterior — ADR-017 cost governors', () => {
	const viewport = { w: 1280, h: 800 };
	const size = { width: 400, height: 300 };

	it('mounts a frame whose AABB is inside the viewport at default zoom', () => {
		const aabb = frameScreenAabb({ x: 100, y: 100 }, size, {
			pan_x: 0,
			pan_y: 0,
			zoom: 1
		});
		expect(shouldMountInterior(aabb, viewport, 1)).toBe(true);
	});

	it('does NOT mount a frame fully off-screen beyond the cull margin', () => {
		// place the frame far to the left: right edge well past -CULL_MARGIN_PX
		const aabb = frameScreenAabb(
			{ x: -5000, y: 100 },
			size,
			{ pan_x: 0, pan_y: 0, zoom: 1 }
		);
		expect(aabb.right).toBeLessThan(-CULL_MARGIN_PX);
		expect(shouldMountInterior(aabb, viewport, 1)).toBe(false);
	});

	it('mounts a frame just off-screen but within the cull margin (no shell-flash)', () => {
		// right edge at -(CULL_MARGIN_PX / 2): off-screen but inside the margin
		const aabb = {
			left: -size.width - CULL_MARGIN_PX / 2,
			top: 100,
			right: -CULL_MARGIN_PX / 2,
			bottom: 400
		};
		expect(aabb.right).toBeGreaterThan(-CULL_MARGIN_PX);
		expect(shouldMountInterior(aabb, viewport, 1)).toBe(true);
	});

	it('suppresses ALL interiors below the zoom floor even if on-screen', () => {
		const aabb = frameScreenAabb({ x: 100, y: 100 }, size, {
			pan_x: 0,
			pan_y: 0,
			zoom: LOD_ZOOM_FLOOR - 0.01
		});
		expect(shouldMountInterior(aabb, viewport, LOD_ZOOM_FLOOR - 0.01)).toBe(false);
	});

	it('mounts an on-screen interior exactly at the zoom floor', () => {
		const aabb = frameScreenAabb({ x: 100, y: 100 }, size, {
			pan_x: 0,
			pan_y: 0,
			zoom: LOD_ZOOM_FLOOR
		});
		expect(shouldMountInterior(aabb, viewport, LOD_ZOOM_FLOOR)).toBe(true);
	});
});
