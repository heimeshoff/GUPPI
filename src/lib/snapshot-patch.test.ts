// Characterisation tests for `snapshot-patch.ts` — the in-place patcher
// that applies fine-grained filesystem-observation domain events to a
// client-side `ProjectSnapshot` (canvas-001, ADR-008 / ADR-009).
//
// These tests codify the load-bearing invariants from `canvas-001`'s
// Outcome section:
//   - lazy zero-count BC creation when `task_*` arrives before `bc_appeared`
//   - count clamping at zero on underflow (warn, never go negative)
//   - idempotent `bc_appeared` after lazy create
//   - BC ordering after lazy create matches the Rust `get_project` sort
//   - `bc_relationships_changed` returns false and ensures the node exists

import { describe, it, expect } from 'vitest';
import type { DomainEvent, ProjectSnapshot } from './types';
import { applyDomainEvent, type WarnFn } from './snapshot-patch';

/** Build a fresh snapshot. The patcher mutates in place, so each test
 *  needs its own object. */
function snapshot(bcs: ProjectSnapshot['bcs'] = []): ProjectSnapshot {
	return {
		id: 1,
		name: 'demo',
		path: '/tmp/demo',
		bcs,
		missing: false
	};
}

/** A spy warn sink — captures messages so tests can assert on count-drift. */
function spyWarn(): WarnFn & { calls: string[] } {
	const calls: string[] = [];
	const fn = ((msg: string) => {
		calls.push(msg);
	}) as WarnFn & { calls: string[] };
	fn.calls = calls;
	return fn;
}

describe('applyDomainEvent — lazy BC creation', () => {
	it('creates a zero-count BC when `task_added` arrives before `bc_appeared`', () => {
		const snap = snapshot();
		const warn = spyWarn();
		const event: DomainEvent = {
			kind: 'task_added',
			project_id: 1,
			bc: 'canvas',
			state: 'backlog',
			task_id: 'canvas-001'
		};

		const handled = applyDomainEvent(snap, event, warn);

		expect(handled).toBe(true);
		expect(snap.bcs.length).toBe(1);
		expect(snap.bcs[0].name).toBe('canvas');
		expect(snap.bcs[0].task_counts).toEqual({
			backlog: 1, // the lazy node + the +1 from the event itself
			todo: 0,
			doing: 0,
			done: 0
		});
		expect(snap.bcs[0].relationships).toEqual([]);
		expect(warn.calls).toEqual([]);
	});

	it('makes `bc_appeared` idempotent after a lazy create (no double node)', () => {
		const snap = snapshot();
		const warn = spyWarn();

		applyDomainEvent(
			snap,
			{
				kind: 'task_added',
				project_id: 1,
				bc: 'canvas',
				state: 'todo',
				task_id: 't1'
			},
			warn
		);
		applyDomainEvent(
			snap,
			{ kind: 'bc_appeared', project_id: 1, bc: 'canvas' },
			warn
		);

		// Still exactly one `canvas` node; its existing task_counts survive.
		expect(snap.bcs.filter((b) => b.name === 'canvas').length).toBe(1);
		expect(snap.bcs[0].task_counts.todo).toBe(1);
	});

	it('sorts the snapshot.bcs alphabetically after a lazy create (matches the Rust `get_project` order)', () => {
		const snap = snapshot([
			{
				name: 'design-system',
				task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
				relationships: []
			},
			{
				name: 'project-registry',
				task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
				relationships: []
			}
		]);
		const warn = spyWarn();

		applyDomainEvent(
			snap,
			{
				kind: 'task_added',
				project_id: 1,
				bc: 'canvas',
				state: 'backlog',
				task_id: 'canvas-x'
			},
			warn
		);

		const names = snap.bcs.map((b) => b.name);
		expect(names).toEqual(['canvas', 'design-system', 'project-registry']);
	});
});

describe('applyDomainEvent — count clamping at zero', () => {
	it('clamps a `task_removed` underflow at 0, warns, and does not go negative', () => {
		const snap = snapshot([
			{
				name: 'canvas',
				task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
				relationships: []
			}
		]);
		const warn = spyWarn();

		const handled = applyDomainEvent(
			snap,
			{
				kind: 'task_removed',
				project_id: 1,
				bc: 'canvas',
				state: 'backlog',
				task_id: 'ghost'
			},
			warn
		);

		expect(handled).toBe(true);
		expect(snap.bcs[0].task_counts.backlog).toBe(0); // not -1
		expect(warn.calls.length).toBe(1);
		expect(warn.calls[0]).toContain('count drift');
		expect(warn.calls[0]).toContain('canvas');
		expect(warn.calls[0]).toContain('backlog');
	});

	it('also clamps the `from` side of a `task_moved` when its count is zero', () => {
		const snap = snapshot([
			{
				name: 'canvas',
				task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
				relationships: []
			}
		]);
		const warn = spyWarn();

		applyDomainEvent(
			snap,
			{
				kind: 'task_moved',
				project_id: 1,
				bc: 'canvas',
				from: 'todo',
				to: 'doing',
				task_id: 'x'
			},
			warn
		);

		expect(snap.bcs[0].task_counts.todo).toBe(0);
		// The `to` side still increments — the event is partial-applied
		// deterministically rather than dropped.
		expect(snap.bcs[0].task_counts.doing).toBe(1);
		expect(warn.calls.length).toBe(1);
	});
});

describe('applyDomainEvent — task_moved and task_removed normal path', () => {
	it('decrements `from` and increments `to` on a normal `task_moved`', () => {
		const snap = snapshot([
			{
				name: 'canvas',
				task_counts: { backlog: 2, todo: 0, doing: 0, done: 0 },
				relationships: []
			}
		]);
		const warn = spyWarn();

		applyDomainEvent(
			snap,
			{
				kind: 'task_moved',
				project_id: 1,
				bc: 'canvas',
				from: 'backlog',
				to: 'todo',
				task_id: 'canvas-x'
			},
			warn
		);

		expect(snap.bcs[0].task_counts).toEqual({
			backlog: 1,
			todo: 1,
			doing: 0,
			done: 0
		});
		expect(warn.calls).toEqual([]);
	});

	it('removes the BC node on `bc_disappeared`', () => {
		const snap = snapshot([
			{
				name: 'canvas',
				task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
				relationships: []
			},
			{
				name: 'voice',
				task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
				relationships: []
			}
		]);
		const warn = spyWarn();

		applyDomainEvent(
			snap,
			{ kind: 'bc_disappeared', project_id: 1, bc: 'canvas' },
			warn
		);

		expect(snap.bcs.map((b) => b.name)).toEqual(['voice']);
	});
});

describe('applyDomainEvent — bc_relationships_changed', () => {
	it('returns `false` so the caller knows to re-fetch, but ensures the BC node exists', () => {
		const snap = snapshot();
		const warn = spyWarn();

		const handled = applyDomainEvent(
			snap,
			{ kind: 'bc_relationships_changed', project_id: 1, bc: 'canvas' },
			warn
		);

		// `false` is the signal to the caller (`Canvas.svelte`) that it must
		// `refreshOne()` — the payload doesn't carry the new relationships.
		expect(handled).toBe(false);
		// But the node is lazily ensured (same contract as `bc_appeared`).
		expect(snap.bcs.length).toBe(1);
		expect(snap.bcs[0].name).toBe('canvas');
		expect(snap.bcs[0].relationships).toEqual([]);
	});

	it('does not overwrite an existing BC node when relationships change', () => {
		const snap = snapshot([
			{
				name: 'canvas',
				task_counts: { backlog: 3, todo: 1, doing: 0, done: 7 },
				relationships: [
					{ to: 'design-system', type: 'shared-kernel', direction: null }
				]
			}
		]);
		const warn = spyWarn();

		applyDomainEvent(
			snap,
			{ kind: 'bc_relationships_changed', project_id: 1, bc: 'canvas' },
			warn
		);

		// Existing counts + relationships untouched (the caller refreshes).
		expect(snap.bcs[0].task_counts).toEqual({
			backlog: 3,
			todo: 1,
			doing: 0,
			done: 7
		});
		expect(snap.bcs[0].relationships.length).toBe(1);
	});
});

describe('applyDomainEvent — non-patch events', () => {
	it('returns `false` for `project_added` / `project_missing` / `resync_required` without touching the snapshot', () => {
		const snap = snapshot([
			{
				name: 'canvas',
				task_counts: { backlog: 1, todo: 0, doing: 0, done: 0 },
				relationships: []
			}
		]);
		const warn = spyWarn();
		const before = JSON.stringify(snap);

		for (const event of [
			{ kind: 'project_added', project_id: 2, path: '/x' } as DomainEvent,
			{ kind: 'project_missing', project_id: 1 } as DomainEvent,
			{ kind: 'resync_required', project_id: 1 } as DomainEvent
		]) {
			expect(applyDomainEvent(snap, event, warn)).toBe(false);
		}

		expect(JSON.stringify(snap)).toBe(before);
		expect(warn.calls).toEqual([]);
	});
});
