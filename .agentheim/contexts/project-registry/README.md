---
name: project-registry
classification: supporting
relationships:
  - to: infrastructure
    type: shared-kernel
---

# project-registry

## Purpose

Discovers, lists, and creates Agentheim projects on disk. Watches the filesystem for folders containing an `.agentheim/` directory, enumerates each project's bounded contexts (`contexts/<name>/`), reads each task file's frontmatter into individual **task records** per BC (with derived per-state counts: backlog / todo / doing / done — `project-registry-005`), and exposes this model upstream to the canvas. Also owns the new-project flow: create folder, `git init`, invoke the `brainstorm` skill inside the new folder (which is itself a `claude-runner` operation).

In v1 this BC is read-only-plus-create: it observes existing projects and creates new ones, but does not mutate the internal state of existing projects (that's `claude-runner`'s job, via spawned `claude` sessions).

## Classification

**Supporting.** Necessary scaffolding — without it the canvas has nothing to draw — but discovery itself is not GUPPI's differentiator. The Agentheim-on-disk shape is specific enough that this isn't pure generic plumbing, but the value-add is in what the canvas and `agent-awareness` do with the discovered projects.

## Ubiquitous language (seed)

- **Project** — a folder on disk containing an `.agentheim/` directory.
- **Agentheim project** — synonym for project, used when disambiguating from arbitrary folders.
- **Bounded context (BC)** — a `contexts/<name>/` directory inside a project.
- **Task** — a markdown file under `contexts/<name>/{backlog,todo,doing,done}/`. Since `project-registry-005` the registry reads each task file's YAML frontmatter into a **task record** (see below), not just a per-state count.
- **Task state** — backlog / todo / doing / done, derived from which subdirectory the task file lives in.
- **Task record (snapshot shape)** — `{ id, title, column, type_, tags, blocked_question }`, the Rust struct `Task` in `project.rs` mirrored as `Task` in `src/lib/types.ts` (`project-registry-005`). The kanban-accordion canvas draws one card per record. `id` is the task file's `id:` frontmatter with a filename-stem fallback; `column` ∈ `backlog|todo|doing|done` (a `TaskColumn`, derived from the subdirectory — same vocabulary as **task state**); `title`/`type_`/`tags` parsed from frontmatter; `blocked_question` is the on-disk `blocked_question:` frontmatter field if present (`None` otherwise). A malformed/missing frontmatter degrades to a stem `id` + empty metadata (logged, never aborts the snapshot). `get_project` reads each `{backlog,todo,doing,done}/*.md` file's frontmatter — O(tasks) parses per snapshot, on mount + resync only (the canvas patches cards in place from events on the hot path, `canvas-001`).
- **`blocked_question` ownership (vs. `agent-awareness`)** — the registry owns the **static** on-disk `blocked_question` (a disk artifact the Agentheim plugin may write into task frontmatter). The **live** per-task agent state and the "AGENT NEEDS AN ANSWER" callout text are `agent-awareness-002`'s surface, not a registry field. Registry counts stay column-based; the per-BC "N active · M blocked · K idling" roll-up is live agent state, not a registry count.
- **Discovery** — the act of scanning the filesystem for Agentheim projects.
- **New-project flow** — the sequence: create folder → `git init` → invoke `brainstorm`.
- **Vision file** — `.agentheim/vision.md`, the canonical "this project exists" marker (alongside the `.agentheim/` directory itself).
- **Registry** — GUPPI's own SQLite `projects` table (ADR-004 / ADR-005) — the canonical list of known projects, keyed by canonical absolute path.
- **Project id** — `projects.id` in the registry. Stamped on every `ProjectSnapshot` and on every fine-grained domain event so the canvas can route updates to the right tile.
- **Project snapshot** — `{ id, name, path, bcs[], missing }`, the read-model the registry hands the canvas. Produced by `get_project(project_id)` for one project and by `list_projects()` for the full set. `missing: true` snapshots carry `bcs: []` and are synthesised by the IPC layer for **registered-but-unwatched** rows so the canvas can render the missing-tile visual rather than dropping the tile (`project-registry-003`).
- **Registered-but-unwatched (a.k.a. missing state)** — a project whose row exists in the registry but whose `.agentheim/` directory is missing on disk. The supervisor leaves no watcher; `list_projects` / `get_project` return the row as a synthetic `missing: true` snapshot. ADR-005 "Do not auto-remove; render as missing" is enforced by this synthesis — the row is never silently dropped from enumeration.
- **Manual register (ADR-005 "Add project…")** — `register_project(path)` IPC: canonicalises the path, validates `.agentheim/` is present (rejects with the exact string `"not an Agentheim project"`), upserts with NULL `scan_root_id`, and arms the watcher via `WatcherSupervisor::add`. Manually-added projects are NEVER touched by the scan-root cascade (ADR-013 origin-tracking invariant).
- **Soft-delete / 30-day retention** — `remove_project(project_id)` IPC: stamps `projects.deleted_at = now`, tears the watcher down, and emits `ProjectRemoved { project_id }`. The row stays in the table for `RETENTION_DAYS` (= 30, ADR-005); `tile_positions` is **preserved** through the window. Re-registering via `register_project` clears `deleted_at` and revives the tile in place — the one and only restore path. The startup GC sweep (`Db::open`) hard-deletes anything older than the window; cascading `tile_positions` rows go with it.
- **Cascade-deregister (ADR-013) vs. soft-delete (ADR-005)** — the two removal paths intentionally differ. `remove_scan_root` is a bulk discard: hard-delete, no retention, fires one `ProjectRemoved` per child BEFORE the watcher/db tear-down so the canvas drops tiles cleanly. `remove_project` is a user-initiated single remove: soft-delete with 30-day retention. Manually-added projects (NULL `scan_root_id`) are immune to the cascade; soft-deleted rows are invisible to enumeration (`list_projects`, `list_projects_by_scan_root`) but still resolvable by id (`project_path`, `project_deleted_at`).
- **`ProjectRemoved` domain event** — the ADR-009 event variant that fires when a project leaves the live registry, by either path (cascade hard-delete or single soft-delete). Carries only `project_id`; the frontend drops the matching tile on receipt.
- **WatcherSupervisor** — the central per-project watcher orchestrator (ADR-008). Owns a `project_id → AgentheimWatcher` map. `add` starts a watcher and publishes `ProjectAdded`; `remove` tears one down. Idempotent on `project_id`.
- **Seed project** — the one hard-coded project (`HARDCODED_PROJECT_PATH` in `lib.rs`) registered at startup so the canvas is not stranded at zero projects before the user adds their first scan root (`project-registry-002a`/`002b`).
- **Scan root** — a folder the user has registered as a rescannable parent for project discovery (ADR-013). One row in the `scan_roots` table, with a per-root `depth_cap` (default 3). Adding or rescanning a root walks the subtree and returns a candidate checklist; the root itself is persisted FIRST so an empty subtree still leaves a rescannable root behind.
- **Scan candidate** — one row in the checklist `add_scan_root` / `rescan_scan_root` returns: `{ path, nickname_suggestion, already_imported }`. The walk reports every `.agentheim/`-bearing directory under the root (depth-capped, junk-pruned, never descending into an identified project). `already_imported = true` for candidates whose canonical path is already in `projects`.
- **Origin tracking** — every project row carries a nullable `scan_root_id` FK to its discovering scan root (ADR-013). NULL = manually added (ADR-005 "Add project…"); non-NULL = discovered under that root. `ON DELETE RESTRICT` makes the app-driven cascade-deregister (`project-registry-002b`) a checked invariant rather than a convention.
- **Import (scanned-projects import)** — the mutation that turns the user's checklist picks from `add_scan_root`/`rescan_scan_root` into registered projects. `import_scanned_projects(scan_root_id, paths)` re-walks the root to verify each pick is still in the candidate set (out-of-set paths are skipped, not silently registered), stamps each survivor with `scan_root_id`, and arms its `.agentheim/` watcher via `WatcherSupervisor::add`. Idempotent on canonical path — re-import is a no-op.
- **Cascade-deregister** — the mutation that tears down a scan root and every project discovered under it (`remove_scan_root(scan_root_id)`). App-driven (the `WatcherSupervisor` cannot be torn down by SQLite), so the IPC drives the ordering: enumerate children → `supervisor.remove` + `db.remove_project` each → `delete_scan_root` last. The schema's `ON DELETE RESTRICT` makes that ordering a checked invariant. **Hard-deletes** child projects and their tile state — ADR-005's 30-day tile-state retention applies ONLY to the user-initiated single "Remove project" affordance (`canvas-005`), never to this cascade. Manually-added projects (NULL `scan_root_id`) are NEVER touched.
- **Bounded context (snapshot shape)** — `{ name, task_counts, tasks, relationships }`, the Rust struct named `BoundedContext` in `project.rs` and mirrored in `src/lib/types.ts`. Stable order by `name`. `tasks` is the `Vec<Task>` of individual **task records** (`project-registry-005`), ordered by column (backlog→done) then id. `task_counts` is **derived** from `tasks` (kept for count-only consumers — the counts pill / accordion "N tasks" row — so they need no second source). `relationships` is the parsed `relationships:` block from the BC's README YAML frontmatter (`project-registry-004`).
- **`TaskAdded` / `TaskChanged` payloads (ADR-009)** — `project-registry-005` grew the watcher-emitted fine-grained events so the canvas patches cards in place without a resync. `TaskAdded` carries the just-created file's frontmatter metadata (`{ …, title, type_, tags }`); the new `TaskChanged { project_id, bc, task_id, title, type_, tags, blocked_question }` fires on an **in-place** frontmatter edit (no column move — a move is still `TaskMoved`, a create still `TaskAdded`). The single-project `AgentheimWatcher` re-reads the file's frontmatter (`enrich_task_added` for adds; `changed_task_files` + `parse_task_file` for in-place edits); `correlate` stays pure.
- **BC↔BC relationship** — a DDD-typed edge between two bounded contexts inside the **same** project (cross-project edges are out of scope for v1). Declared in each BC's README YAML frontmatter as `relationships: [ { to: <sibling-bc>, type: <type>, direction?: <dir> }, … ]`. The five types `customer-supplier`, `shared-kernel`, `partnership`, `anticorruption-layer`, `conformist` mirror the DDD strategic palette; directional types pair with `direction: upstream | downstream`, non-directional types omit `direction`. Mismatched shape entries are dropped at parse time with a warning rather than silently accepted.
- **`bc_positions` table (schema v4)** — `(project_id, bc_name) -> (x, y)`, the per-BC bubble position inside its project frame on the canvas (`canvas-007` consumer). `ON DELETE CASCADE` on `project_id` mirrors `tile_positions`: soft-delete preserves through the 30-day retention window, hard-delete (cascade-deregister or GC sweep) clears the rows. CRUD: `save_bc_position` / `bc_position` / `bc_positions` on `Db`; `save_bc_position` / `load_bc_position` / `load_bc_positions` over IPC.
- **`BcRelationshipsChanged` domain event** — fires when a BC's `README.md` is written and the parsed `relationships:` set actually differs from the previously cached value (deep-equal compare). Prose-only README edits do **not** fire it (`project-registry-004`). The single-project `AgentheimWatcher` owns the per-BC cache (`RelationshipsCache`), keyed by BC name. Carried over the ADR-009 bus to the canvas, which patches the BC's `relationships` in place and re-runs intra-project edge layout (`canvas-007`).

## Upstream / downstream

- **Downstream of:** the filesystem (conformist to the Agentheim-on-disk shape — if the shape changes upstream, the fix lives here).
- **Upstream of:** `canvas` (supplies the project model), `agent-awareness` (may share the filesystem watcher — foundation decision).

## Open questions

- Is the filesystem watcher shared with `agent-awareness` (one watcher, two consumers via the infrastructure event bus) or independent? Foundation pass.

## Resolved

- *Where does GUPPI look for projects?* — Settled by ADR-013 (`project-registry-002a`): the user registers **scan roots** once; GUPPI walks each on demand and presents a candidate checklist. Manually-added projects (ADR-005 "Add project…") coexist with discovered ones via the nullable `projects.scan_root_id` FK.
