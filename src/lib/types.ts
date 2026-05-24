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

/** A DDD relationship classification between two bounded contexts inside
 * the same project — surfaced via each BC README's YAML frontmatter and
 * consumed by the canvas to render intra-project edges (`canvas-007`,
 * `project-registry-004`). Mirrors `project::RelationshipType` in Rust.
 *
 * Directional types (`customer-supplier`, `anticorruption-layer`,
 * `conformist`) pair with a `Direction`; non-directional types
 * (`shared-kernel`, `partnership`) carry `direction: null` (or absent). */
export type RelationshipType =
	| 'customer-supplier'
	| 'shared-kernel'
	| 'partnership'
	| 'anticorruption-layer'
	| 'conformist';

/** Direction of a directional relationship — `upstream` (the **target** BC is
 * upstream of the **source** BC) or `downstream`. Mirrors `project::Direction`
 * in Rust. */
export type Direction = 'upstream' | 'downstream';

/** One BC↔BC relationship as declared in a BC's README frontmatter. `to` is
 * the bare directory name of a sibling BC inside the same project (e.g.
 * `project-registry`). Cross-project references are dropped by the Rust
 * parser at v1, so the canvas never sees them. Mirrors `project::Relationship`
 * in Rust. */
export interface Relationship {
	to: string;
	type: RelationshipType;
	/** `null` (or absent) for non-directional relationship types. */
	direction?: Direction | null;
}

/** Which kanban column a task currently lives in — derived from its
 * subdirectory (`backlog`/`todo`/`doing`/`done`). Mirrors `project::TaskColumn`
 * in Rust (`project-registry-005`). Same vocabulary as `AgentheimState`, kept
 * as its own alias for the per-task record where it reads as a column. */
export type TaskColumn = 'backlog' | 'todo' | 'doing' | 'done';

/** One individual task record the kanban-accordion canvas draws as a card
 * (`project-registry-005`). Read from a task file's YAML frontmatter; a
 * malformed/missing frontmatter degrades on the Rust side to a filename-stem
 * `id` with empty metadata. Mirrors `project::Task` in Rust.
 *
 * Coordination with `agent-awareness-002`: this is the **static** on-disk
 * record. The **live** per-task agent state and the "AGENT NEEDS AN ANSWER"
 * callout are agent-awareness's surface; `blocked_question` here is only the
 * disk-artifact fallback when the task file carries one. */
export interface Task {
	id: string;
	title: string;
	column: TaskColumn;
	/** The task's `type:` frontmatter (`feature`/`bug`/`spike`/`decision`).
	 * Named `type_` (not `type`) to match the Rust serde field and dodge no
	 * JS keyword clash; empty string if absent. */
	type_: string;
	tags: string[];
	/** A `blocked_question:` from the task's frontmatter, if present on disk.
	 * Absent for the common (not-blocked) case. */
	blocked_question?: string | null;
}

/** One bounded context as the canvas draws it. `relationships` is parsed from
 * the BC's README YAML frontmatter at enumeration time (`project-registry-004`);
 * malformed frontmatter degrades to an empty array without breaking BC
 * enumeration. Mirrors the Rust `BoundedContext` type.
 *
 * `tasks` (`project-registry-005`) carries the individual task records drawn as
 * kanban cards, ordered by column (backlog→done) then id. `task_counts` is
 * **derived** from `tasks` on the Rust side — kept so the counts pill /
 * accordion "N tasks" row needs no second source. */
export interface BoundedContext {
	name: string;
	task_counts: TaskCounts;
	/** Individual task records, ordered by column then id. */
	tasks: Task[];
	/** Intra-project BC↔BC relationships. Empty for BCs whose README has no
	 * frontmatter, whose frontmatter has no `relationships:` block, or whose
	 * frontmatter fails to parse. */
	relationships: Relationship[];
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
	bcs: BoundedContext[];
	missing: boolean;
}

/** A task's live agent activity (`agent-awareness-002`, ADR-018). Mirrors the
 * Rust `agent_state::AgentActivity` (`snake_case`). The unified read model that
 * survives agent-awareness's two signal sources (rich `claude-runner` events;
 * best-effort filesystem signals) — the canvas sees one shape. */
export type AgentActivity = 'running' | 'idle' | 'blocked_on_question';

/** One task's live agent state (`agent-awareness-002`, ADR-018). Returned by the
 * `get_task_agent_state` IPC command and mirrored from the
 * `task_agent_state_changed` bus event. Mirrors Rust `agent_state::TaskAgentState`.
 *
 * `agent_label` / `since` are present for `running` / `blocked_on_question` and
 * absent for `idle`. `since` is a Unix-millisecond transition timestamp; the
 * card derives the "waiting 2m 14s" elapsed string from it locally (no
 * per-second event spam). `question` is the live "AGENT NEEDS AN ANSWER" callout
 * body, present only when blocked (runner-sourced live, on-disk
 * `blocked_question` as fallback). */
export interface TaskAgentState {
	activity: AgentActivity;
	/** Acting agent label ("orchestrator", "worker", "observed") for
	 * running/blocked; absent for idle. */
	agent_label?: string | null;
	/** Unix-millisecond transition timestamp the canvas times "waiting …" from;
	 * absent for idle. */
	since?: number | null;
	/** The live question text for `blocked_on_question`; absent otherwise. */
	question?: string | null;
}

/** A BC's agent roll-up (`active / blocked / idling`) for the accordion header
 * ("1 active · 2 blocked · 1 idling") — `agent-awareness-002`, ADR-018. Returned
 * by the `get_bc_agent_rollup` IPC command. `idling` is `total − active −
 * blocked` (tasks with no live signal). Mirrors Rust `agent_state::BcRollup`. */
export interface BcRollup {
	active: number;
	blocked: number;
	idling: number;
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
			/** Task metadata read from the just-created file's frontmatter so the
			 * canvas can draw the new card in place without a resync
			 * (`project-registry-005`). Empty strings / empty array if the file
			 * had no parseable frontmatter. */
			title: string;
			type_: string;
			tags: string[];
	  }
	| {
			kind: 'task_removed';
			project_id: number;
			bc: string;
			state: AgentheimState;
			task_id: string;
	  }
	/**
	 * A task file's frontmatter changed *in place* — a `title`/`type`/`tags`/
	 * `blocked_question` edit without the file moving columns
	 * (`project-registry-005`). The canvas patches the matching card's metadata
	 * in place; no resync. A move is `task_moved`, a create is `task_added`.
	 */
	| {
			kind: 'task_changed';
			project_id: number;
			bc: string;
			task_id: string;
			title: string;
			type_: string;
			tags: string[];
			blocked_question?: string | null;
	  }
	| { kind: 'bc_appeared'; project_id: number; bc: string }
	| { kind: 'bc_disappeared'; project_id: number; bc: string }
	/**
	 * A BC's `README.md` was written and the parsed `relationships:`
	 * frontmatter changed (deep-equal compared in the Rust watcher). Consumed
	 * by `canvas-007`: the canvas patches the BC's `relationships` array in
	 * place and re-runs intra-project edge layout. Prose-only README edits do
	 * not fire this event (`project-registry-004`).
	 */
	| { kind: 'bc_relationships_changed'; project_id: number; bc: string }
	| { kind: 'resync_required'; project_id: number }
	/**
	 * A `claude-runner`-owned session is waiting for a human answer
	 * (`agent-awareness-002`, ADR-018 / ADR-006). agent-awareness's *input* — the
	 * rich live signal it folds into its per-task projection. The runner
	 * attributes the blocked session to a `(project_id, bc, task_id)` and supplies
	 * the live `question` text. v1 has no producer wired yet (the runner does not
	 * yet attribute sessions to tasks); the canvas does not consume this directly
	 * — it consumes the derived `task_agent_state_changed`.
	 */
	| {
			kind: 'session_blocked_on_question';
			project_id: number;
			bc: string;
			task_id: string;
			agent_label: string;
			question: string;
	  }
	/**
	 * A task's live agent state changed (`agent-awareness-002`, ADR-018) — the
	 * unified read side of agent-awareness. The canvas patches the matching card's
	 * agent indicator in place (and, when `state` is `blocked_on_question`, the
	 * docked panel's "AGENT NEEDS AN ANSWER" callout). `since` is a Unix-ms
	 * transition timestamp the card derives "waiting 2m 14s" from locally (no
	 * per-second spam); `agent_label` / `since` are absent for `idle`, `question`
	 * present only when blocked. v1 is read-only — the answer/defer/edit write
	 * round-trip is post-v1.
	 */
	| {
			kind: 'task_agent_state_changed';
			project_id: number;
			bc: string;
			task_id: string;
			state: AgentActivity;
			agent_label?: string | null;
			since?: number | null;
			question?: string | null;
	  }
	/**
	 * A cross-session user preference was set (`design-system-004-light-theme`).
	 * Fired by the `set_preference` IPC. Theme is the first key; future
	 * preferences (font scale, reduced-motion override, etc.) reuse this
	 * generic shape. Consumers ignore keys they don't recognise.
	 */
	| { kind: 'preference_changed'; key: string; value: string };
