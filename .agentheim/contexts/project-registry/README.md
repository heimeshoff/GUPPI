# project-registry

## Purpose

Discovers, lists, and creates Agentheim projects on disk. Watches the filesystem for folders containing an `.agentheim/` directory, enumerates each project's bounded contexts (`contexts/<name>/`), counts tasks per state (backlog / todo / doing / done) per BC, and exposes this model upstream to the canvas. Also owns the new-project flow: create folder, `git init`, invoke the `brainstorm` skill inside the new folder (which is itself a `claude-runner` operation).

In v1 this BC is read-only-plus-create: it observes existing projects and creates new ones, but does not mutate the internal state of existing projects (that's `claude-runner`'s job, via spawned `claude` sessions).

## Classification

**Supporting.** Necessary scaffolding — without it the canvas has nothing to draw — but discovery itself is not GUPPI's differentiator. The Agentheim-on-disk shape is specific enough that this isn't pure generic plumbing, but the value-add is in what the canvas and `agent-awareness` do with the discovered projects.

## Ubiquitous language (seed)

- **Project** — a folder on disk containing an `.agentheim/` directory.
- **Agentheim project** — synonym for project, used when disambiguating from arbitrary folders.
- **Bounded context (BC)** — a `contexts/<name>/` directory inside a project.
- **Task** — a markdown file under `contexts/<name>/{backlog,todo,doing,done}/`.
- **Task state** — backlog / todo / doing / done, derived from which subdirectory the task file lives in.
- **Discovery** — the act of scanning the filesystem for Agentheim projects.
- **New-project flow** — the sequence: create folder → `git init` → invoke `brainstorm`.
- **Vision file** — `.agentheim/vision.md`, the canonical "this project exists" marker (alongside the `.agentheim/` directory itself).
- **Registry** — GUPPI's own SQLite `projects` table (ADR-004 / ADR-005) — the canonical list of known projects, keyed by canonical absolute path.
- **Project id** — `projects.id` in the registry. Stamped on every `ProjectSnapshot` and on every fine-grained domain event so the canvas can route updates to the right tile.
- **Project snapshot** — `{ id, name, path, bcs[] }`, the read-model the registry hands the canvas. Produced by `get_project(project_id)` for one project and by `list_projects()` for the full set.
- **Registered-but-unwatched** — a project whose row exists in the registry but whose `.agentheim/` directory is missing on disk. The supervisor leaves no watcher; the tile is shown in the ADR-005 "missing" state.
- **WatcherSupervisor** — the central per-project watcher orchestrator (ADR-008). Owns a `project_id → AgentheimWatcher` map. `add` starts a watcher and publishes `ProjectAdded`; `remove` tears one down. Idempotent on `project_id`.
- **Seed project** — the one hard-coded project (`HARDCODED_PROJECT_PATH` in `lib.rs`) registered at startup so the canvas is not stranded at zero projects before the user adds their first scan root (`project-registry-002a`/`002b`).
- **Scan root** — a folder the user has registered as a rescannable parent for project discovery (ADR-013). One row in the `scan_roots` table, with a per-root `depth_cap` (default 3). Adding or rescanning a root walks the subtree and returns a candidate checklist; the root itself is persisted FIRST so an empty subtree still leaves a rescannable root behind.
- **Scan candidate** — one row in the checklist `add_scan_root` / `rescan_scan_root` returns: `{ path, nickname_suggestion, already_imported }`. The walk reports every `.agentheim/`-bearing directory under the root (depth-capped, junk-pruned, never descending into an identified project). `already_imported = true` for candidates whose canonical path is already in `projects`.
- **Origin tracking** — every project row carries a nullable `scan_root_id` FK to its discovering scan root (ADR-013). NULL = manually added (ADR-005 "Add project…"); non-NULL = discovered under that root. `ON DELETE RESTRICT` makes the app-driven cascade-deregister (`project-registry-002b`) a checked invariant rather than a convention.

## Upstream / downstream

- **Downstream of:** the filesystem (conformist to the Agentheim-on-disk shape — if the shape changes upstream, the fix lives here).
- **Upstream of:** `canvas` (supplies the project model), `agent-awareness` (may share the filesystem watcher — foundation decision).

## Open questions

- Is the filesystem watcher shared with `agent-awareness` (one watcher, two consumers via the infrastructure event bus) or independent? Foundation pass.

## Resolved

- *Where does GUPPI look for projects?* — Settled by ADR-013 (`project-registry-002a`): the user registers **scan roots** once; GUPPI walks each on demand and presents a candidate checklist. Manually-added projects (ADR-005 "Add project…") coexist with discovered ones via the nullable `projects.scan_root_id` FK.
