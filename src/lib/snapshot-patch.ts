// Targeted in-place patching of a `ProjectSnapshot` from fine-grained
// filesystem-observation domain events (ADR-008 / ADR-009 — canvas-001).
//
// The canvas no longer re-fetches the whole `get_project` snapshot on every
// `.agentheim/` change. Instead the Rust core emits `task_moved` / `task_added`
// / `task_removed` / `bc_appeared` / `bc_disappeared`, and these functions
// patch the client-side model in place: a tile's task counts tick without a
// round-trip, a BC node appears/disappears without redrawing everything.
//
// This module is deliberately pure — no Svelte, no IPC, no PixiJS. It mutates
// the `ProjectSnapshot` it is handed (the caller owns reactivity: in
// `Canvas.svelte` the snapshot is Svelte 5 `$state`, so a mutation here is
// picked up on the next ticker frame). Keeping it pure also keeps it reviewable
// and unit-testable in isolation.

import type { AgentheimState, BcSnapshot, DomainEvent, ProjectSnapshot } from './types';

/** A sink for the count-clamping warnings — `logToCore` in production, a spy
 * in tests. Kept as a parameter so this module imports no IPC. */
export type WarnFn = (message: string) => void;

/** Find a BC node by name, or lazily create a zero-count one and append it.
 *
 * Lazy creation is required because `correlate()` in the Rust watcher pushes
 * BC events *after* task events in the same batch: a brand-new BC created with
 * task files in it yields `task_added` *before* `bc_appeared`. A `task_*` event
 * for a BC not yet in the model must therefore not be dropped — it creates the
 * node, then applies the delta. `bc_appeared` is consequently idempotent. */
function bcNode(snapshot: ProjectSnapshot, name: string): BcSnapshot {
	const existing = snapshot.bcs.find((b) => b.name === name);
	if (existing) return existing;
	const created: BcSnapshot = {
		name,
		task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 }
	};
	snapshot.bcs.push(created);
	// Keep BC ordering stable and matching the Rust `get_project` snapshot
	// (which sorts by name) so a later `resync_required` re-fetch does not
	// reshuffle nodes relative to what patching produced.
	snapshot.bcs.sort((a, b) => a.name.localeCompare(b.name));
	return created;
}

/** Apply a +1 to a BC's count for `state`. */
function increment(bc: BcSnapshot, state: AgentheimState): void {
	bc.task_counts[state] += 1;
}

/** Apply a -1 to a BC's count for `state`, clamped at 0.
 *
 * A delta that would drive a count below zero means the client model has
 * drifted from disk (an event was missed, or applied twice). The robustness
 * contract: clamp at 0, warn, never render a negative count, never throw. */
function decrement(
	bc: BcSnapshot,
	state: AgentheimState,
	warn: WarnFn
): void {
	if (bc.task_counts[state] <= 0) {
		bc.task_counts[state] = 0;
		warn(
			`count drift: tried to decrement ${bc.name}.${state} below zero; ` +
				`clamped at 0 (client model is out of sync with disk)`
		);
		return;
	}
	bc.task_counts[state] -= 1;
}

/**
 * Patch `snapshot` in place for one filesystem-observation `DomainEvent`.
 *
 * Returns `true` if the event was a filesystem-observation event that was
 * handled (the caller may want to re-render); `false` if the event was not one
 * this function handles (`project_added`, `project_missing`, `resync_required`
 * — the caller deals with those itself).
 *
 * `project_id` filtering is the caller's responsibility — it knows the loaded
 * project's id; this pure function patches whatever snapshot it is given.
 */
export function applyDomainEvent(
	snapshot: ProjectSnapshot,
	event: DomainEvent,
	warn: WarnFn
): boolean {
	switch (event.kind) {
		case 'task_moved': {
			const bc = bcNode(snapshot, event.bc);
			decrement(bc, event.from, warn);
			increment(bc, event.to);
			return true;
		}
		case 'task_added': {
			const bc = bcNode(snapshot, event.bc);
			increment(bc, event.state);
			return true;
		}
		case 'task_removed': {
			const bc = bcNode(snapshot, event.bc);
			decrement(bc, event.state, warn);
			return true;
		}
		case 'bc_appeared': {
			// Idempotent: the node may already exist (a `task_*` event in the
			// same batch lazily created it). `bcNode` is the no-op-or-create.
			bcNode(snapshot, event.bc);
			return true;
		}
		case 'bc_disappeared': {
			const idx = snapshot.bcs.findIndex((b) => b.name === event.bc);
			if (idx !== -1) snapshot.bcs.splice(idx, 1);
			return true;
		}
		default:
			// `project_added`, `project_missing`, `resync_required` — not a
			// filesystem-observation patch event. The caller handles these.
			return false;
	}
}
