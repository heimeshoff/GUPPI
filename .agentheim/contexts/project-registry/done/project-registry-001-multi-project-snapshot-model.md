---
id: project-registry-001-multi-project-snapshot-model
type: feature
status: done
completed: 2026-05-15
commit: d594ad5
scope: bc
depends_on: []
related_adrs:
  - ADR-004
  - ADR-005
  - ADR-008
related_research: []
prior_art: []
---

# Multi-project snapshot model

## Why

The walking skeleton (`infrastructure-012`) renders exactly **one hardcoded
project**: `AppState` carries a single `project_id`/`project_path`, `get_project`
reads that one path, and one `AgentheimWatcher` observes that one `.agentheim/`.
The vision's v1 is "a tile for **every** Agentheim project" — the irreducible
core. Nothing about the canvas overview is real until the registry can hand it
more than one project.

ADR-005 fixed the storage shape (the `projects` table is the registry); ADR-008
fixed the watching shape (one debounced watcher per project under a central
`WatcherSupervisor`). The skeleton wired neither for N. ADR-008's "Downstream"
note — "the `WatcherSupervisor` lands with the project registry" — is cashed in
here. This is the data-layer foundation for the whole v1 push: `canvas-002`
(render N tiles) and `project-registry-002` (scan + register) both build on it.

## What

Generalize the single-project core into a multi-project model.

**`list_projects()` IPC command + DB methods**
- New `Db::list_projects() -> Vec<ProjectRow>` returning every row (`id`, `path`,
  `nickname`) from the `projects` table.
- New `Db::project_path(project_id)` (or `project_row`) for single-project lookup.
- New `list_projects(state) -> Result<Vec<ProjectSnapshot>, String>` command:
  reads all rows, maps each through the existing pure `project::get_project(&path)`.
  A project whose `.agentheim/` is missing is skipped-and-logged — it must not
  abort the whole call.
- `get_project` is reshaped to `get_project(state, project_id)` — a single-project
  fetch keyed by id (resolves the path via the DB, then the pure reader). It is
  no longer path-implicit from `AppState`. Both commands exist: `list_projects()`
  for the full set, `get_project(project_id)` for per-project resync (the
  frontend bridge's `ResyncRequired { project_id }` re-fetches exactly one).

**`WatcherSupervisor` (ADR-008)**
- New module `src-tauri/src/supervisor.rs`. `watcher.rs` stays the single-project
  `AgentheimWatcher` primitive; the supervisor composes it.
- Owns a `project_id -> AgentheimWatcher` map behind `Arc<Mutex<…>>` — one
  instance, one owner of the map (ADR-008's "single owner" intent; the
  command-channel Tokio task in ADR-008's sketch is simplified to a Mutex map
  because add/remove are infrequent and want to be synchronous for IPC).
- Public surface:
  - `WatcherSupervisor::new(bus)`.
  - `add(project_id, project_path) -> Result<(), SupervisorError>` — synchronous;
    starts a debounced watcher; **publishes `ProjectAdded { project_id, path }`**;
    idempotent on `project_id`. If `.agentheim/` is missing, returns
    `SupervisorError` (project registered-but-unwatched — the ADR-005 "missing"
    state) without panicking or unwinding the caller.
  - `remove(project_id)` — synchronous; drops the project's watcher; no-op if not
    watched.

**`AppState` restructuring**
- `AppState` drops `project_id` and `project_path`. New shape:
  `{ db: Arc<Db>, supervisor: <shared WatcherSupervisor handle>, bus: EventBus,
  claude_session: Mutex<Option<ClaudeSession>> }`.
- `save_tile_position` / `load_tile_position` take `project_id` as an explicit
  IPC parameter (the `Db` methods already key on it).
- `pty_spawn_claude` takes `project_id` and resolves the spawn cwd via the DB
  (minimal change — PTY spike stays `infrastructure-013` scope; other PTY
  commands unchanged).
- `save_camera` / `load_camera` unchanged.

**`setup()` wiring**
- The hardcoded `HARDCODED_PROJECT_PATH` seed **stays** (so the app is not
  stranded at zero projects before `project-registry-002` lands) but is routed
  through `WatcherSupervisor::add` instead of `AgentheimWatcher::start` directly.
  `setup()` no longer publishes `ProjectAdded` itself — `add` does.

## Acceptance criteria

- [ ] `Db::list_projects()` returns one row per project in the `projects` table;
      covered by a unit test inserting ≥2 projects.
- [ ] `list_projects()` command returns a `ProjectSnapshot` for every registered
      project; a row whose `.agentheim/` is missing is skipped (logged), not an
      error that aborts the call.
- [ ] `get_project(project_id)` returns the snapshot for exactly that project;
      an unknown `project_id` is a clean error, not a panic.
- [ ] `WatcherSupervisor::add` starts a watcher and publishes `ProjectAdded`;
      adding the same `project_id` twice is a no-op (idempotent) — unit tested.
- [ ] `WatcherSupervisor::add` on a path with no `.agentheim/` returns
      `SupervisorError` without unwinding the caller; the supervisor map gains
      no entry — unit tested.
- [ ] `WatcherSupervisor::remove` tears down the project's watcher; the map no
      longer contains it — unit tested (add then remove).
- [ ] With ≥2 projects added to the supervisor, a task-file move in either
      project produces a fine-grained domain event carrying the correct
      `project_id` on the single `EventBus` — integration tested.
- [ ] `AppState` no longer has `project_id` / `project_path` fields; the app
      builds and the skeleton's single seeded project still renders end-to-end
      via `setup()` → `supervisor.add` → `list_projects()`.

## Notes

This task holds as ONE task (not split): the `AppState` restructure is forced by
introducing the supervisor and touches every command; `list_projects()` is a
thin command over one DB method. Splitting would create child tasks all editing
`lib.rs`'s `AppState`/`setup()` at once with no independently shippable piece.

Concurrency shape of the supervisor (`Arc<Mutex<map>>` rather than ADR-008's
sketched command-channel Tokio task) is a deliberate simplification — add/remove
are infrequent and want synchronous IPC calls. Recorded in the ADR-008
reconciliation note (project-registry-001, 2026-05-14).

`remove_project` — the actual deletion of a `projects` row — is **deferred to
`project-registry-002`** (it is the registration *mutation*). This task delivers
the supervisor's `remove` watcher-lifecycle surface and unit-tests it; nothing
in this task's scope calls it with a real removal yet.

Coordination:
- `project-registry-002` calls `WatcherSupervisor::add` after inserting a scanned
  project, and will add `Db::remove_project` + wire it to `supervisor.remove`.
- `canvas-002`'s open question is answered: the registry emits a live
  `ProjectAdded` domain event (from `supervisor.add`); the canvas reacts to it
  rather than polling.
- `canvas-002` requires `ProjectSnapshot` to carry an `id: i64` field (added in
  `src-tauri/src/project.rs`, mirrored in `src/lib/types.ts`). Both
  `list_projects()` and `get_project(project_id)` must populate it so the id
  flows with the snapshot to the frontend. (Decision with Marco, 2026-05-15.)

Decisions made during refinement (2026-05-14, with Marco):
- `get_project` reshaped to `get_project(project_id)` + `list_projects()` added
  (per-project resync wants a precise re-fetch).
- `WatcherSupervisor` uses incremental add/remove, not wholesale rebuild.
- `list_projects()` is a cold disk read each call — no cached model (canvas-001
  patches the frontend in place from fine-grained events; `list_projects()` runs
  only on mount + `ResyncRequired`).

## Outcome

Multi-project data-layer foundation landed. The skeleton's path-implicit
single-project core is gone; every per-project IPC command takes `project_id`
explicitly and resolves the path through the registry.

**Key files**
- `src-tauri/src/db.rs` — new `ProjectRow`, `Db::list_projects()`,
  `Db::project_path(project_id)`. Four new unit tests cover the multi-row
  listing, empty registry, id→path resolution, and unknown-id-as-`None`.
- `src-tauri/src/project.rs` — `ProjectSnapshot` gains `id: i64`;
  `get_project(project_id, &path)` stamps it on the snapshot. One new test
  (`snapshot_carries_the_project_id_supplied_by_the_caller`); existing tests
  updated to pass an id.
- `src-tauri/src/supervisor.rs` — **new module.** `WatcherSupervisor` owns
  the `project_id → AgentheimWatcher` map behind `Arc<Mutex<…>>`. `add` is
  idempotent on `project_id`, publishes `ProjectAdded` on first add, returns
  `SupervisorError::AgentheimMissing` on missing `.agentheim/` without
  unwinding the caller. `remove` tears down the watcher. Five new tests,
  including a real-FS integration test seating two projects and asserting
  fine-grained events carry the correct `project_id` on the shared bus.
- `src-tauri/src/lib.rs` — `AppState` drops `project_id`/`project_path`,
  gains `supervisor: WatcherSupervisor`. New `list_projects` IPC command;
  `get_project`/`save_tile_position`/`load_tile_position`/`pty_spawn_claude`
  now take `project_id` explicitly. `setup()` seeds the hard-coded project
  through `supervisor.add(...)` (which also publishes `ProjectAdded` —
  `setup()` no longer publishes it itself).
- `src/lib/types.ts` — `ProjectSnapshot` gains `id: number`.
- `src/lib/ipc.ts` — new `listProjects()`; `getProject(projectId)`,
  `saveTilePosition(projectId, pos)`, `loadTilePosition(projectId)`.
- `src/lib/Canvas.svelte` — on mount calls `listProjects()`, renders the
  first project, persists per-id tile position. `resync_required` re-fetches
  via `getProject(id)`. Single-tile rendering preserved; `canvas-002` will
  generalise to a per-id tile map.
- `.agentheim/contexts/project-registry/README.md` — extended ubiquitous
  language: registry, project id, project snapshot, registered-but-unwatched,
  WatcherSupervisor, seed project.

**Tests**: 41/41 Rust unit + integration tests pass (`cargo test --lib`),
10 of which are new for this task. Frontend `pnpm check` clean.

**Acceptance criteria — all met**:
1. `Db::list_projects()` covered by `list_projects_returns_every_registered_row`.
2. `list_projects` command skip-and-logs unreadable rows (lib.rs, inline).
3. `get_project(project_id)` returns a clean `unknown project_id: N` error
   for unknown ids (lib.rs).
4. `supervisor.add` publishes `ProjectAdded` + is idempotent — two tests.
5. `supervisor.add` on missing `.agentheim/` returns error, seats no entry —
   one test.
6. `supervisor.remove` drops the entry — one test.
7. With two projects, fine-grained events carry the correct `project_id` —
   `two_projects_emit_events_carrying_their_own_project_id`.
8. `AppState` has no `project_id`/`project_path`; build is clean; skeleton
   single-project path is preserved (`setup()` → `supervisor.add` →
   `listProjects()` on the frontend).
