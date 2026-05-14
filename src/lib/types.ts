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

/**
 * The `DomainEvent` payload forwarded by the Rust core's frontend bridge
 * under the single `guppi://event` Tauri event name (ADR-009). Only the
 * variants the skeleton emits are modelled; `kind` is the serde tag.
 */
export type DomainEvent =
	| { kind: 'project_added'; project_id: number; path: string }
	| { kind: 'project_missing'; project_id: number }
	| { kind: 'agentheim_changed'; project_id: number };
