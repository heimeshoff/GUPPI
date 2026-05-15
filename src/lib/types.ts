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

/** Everything needed to render a project tile and its BC children.
 *
 * `id` is the registry's project id (`projects.id` in GUPPI's SQLite DB —
 * ADR-005). Carrying it on the snapshot is the load-bearing change for
 * `canvas-002`: the canvas keys per-project state on it, and uses it to
 * route fine-grained domain events back to the right tile
 * (`project-registry-001`).
 *
 * `missing` is `true` for a registry row whose `.agentheim/` directory is
 * gone on disk — the ADR-005 **registered-but-unwatched** state
 * (`project-registry-003`). Such snapshots always carry `bcs: []`, with the
 * `name` falling back to the folder name. canvas-005a is the visual
 * treatment (dim + magenta border + glyph); for the project-registry-003
 * scope the canvas only needs to *tolerate* the new field without dropping
 * the tile. */
export interface ProjectSnapshot {
	id: number;
	name: string;
	path: string;
	bcs: BcSnapshot[];
	missing: boolean;
}

/** A 2D position in world coordinates. */
export interface Point {
	x: number;
	y: number;
}

/** One row of the `scan_roots` table — a folder the user has registered as a
 * rescannable parent for project discovery (ADR-013, `project-registry-002a`).
 * Mirrors `db::ScanRootRow`. */
export interface ScanRootRow {
	id: number;
	path: string;
	depth_cap: number;
	added_at: string;
}

/** One row of a scan-root walk's checklist (ADR-013, `project-registry-002a`).
 * Mirrors `scan::ScanCandidate`. The `already_imported` flag drives the
 * disabled-pre-checked rendering in the discovery checklist modal
 * (`canvas-005b`). */
export interface ScanCandidate {
	path: string;
	nickname_suggestion: string;
	already_imported: boolean;
}

/** Return shape of `add_scan_root` — the persisted root's id plus the
 * candidate checklist from walking its subtree. Mirrors
 * `AddScanRootResult` in `src-tauri/src/lib.rs`. */
export interface AddScanRootResult {
	scan_root_id: number;
	candidates: ScanCandidate[];
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
	| { kind: 'project_removed'; project_id: number }
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
