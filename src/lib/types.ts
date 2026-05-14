// Shapes that cross the IPC boundary from the Rust core. These mirror the
// `serde`-serialised structs in `src-tauri/src/project.rs` and the
// `DomainEvent` enum in `src-tauri/src/events.rs` (ADR-009).

/** Task-file counts for one bounded context, keyed by Agentheim task state. */
export interface TaskCounts {
	backlog: number;
	todo: number;
	doing: number;
	done: number;
}

/** One bounded context as the canvas draws it. */
export interface BcSnapshot {
	name: string;
	task_counts: TaskCounts;
}

/** Everything needed to render a project tile and its BC children. */
export interface ProjectSnapshot {
	name: string;
	path: string;
	bcs: BcSnapshot[];
}

/** A 2D position in world coordinates. */
export interface Point {
	x: number;
	y: number;
}

/** Camera state — pan + zoom — the single source of truth per ADR-003. */
export interface CameraState {
	pan_x: number;
	pan_y: number;
	zoom: number;
}

/** An Agentheim task-state directory name — the `from` / `to` / `state`
 * fields of the filesystem-observation events (ADR-008). */
export type AgentheimState = 'backlog' | 'todo' | 'doing' | 'done';

/**
 * The `DomainEvent` payload forwarded by the Rust core's frontend bridge
 * under the single `guppi://event` Tauri event name (ADR-009). `kind` is the
 * serde `snake_case` tag of the Rust `DomainEvent` enum.
 *
 * The filesystem-observation variants (`task_*`, `bc_*`) are the fine-grained
 * normal-path events the canvas patches its model from in place. The lag-only
 * `resync_required` is the single event that triggers a full `getProject()`
 * re-fetch (ADR-009 lag-resync strategy — `canvas-001`).
 */
export type DomainEvent =
	| { kind: 'project_added'; project_id: number; path: string }
	| { kind: 'project_missing'; project_id: number }
	| {
			kind: 'task_moved';
			project_id: number;
			bc: string;
			from: AgentheimState;
			to: AgentheimState;
			task_id: string;
	  }
	| {
			kind: 'task_added';
			project_id: number;
			bc: string;
			state: AgentheimState;
			task_id: string;
	  }
	| {
			kind: 'task_removed';
			project_id: number;
			bc: string;
			state: AgentheimState;
			task_id: string;
	  }
	| { kind: 'bc_appeared'; project_id: number; bc: string }
	| { kind: 'bc_disappeared'; project_id: number; bc: string }
	| { kind: 'resync_required'; project_id: number };
