# project-registry — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
- [ADR-013 — Scan roots: persisted, rescannable discovery folders](../../knowledge/decisions/ADR-013-scan-roots.md) — Accepted. Persisted `scan_roots` table + nullable `projects.scan_root_id` FK (`ON DELETE RESTRICT`); add/rescan walks → checklist; root removal app-drives a hard-delete cascade. Evolves ADR-005's one-shot scan.
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
*(None.)*
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
- [project-registry-001-multi-project-snapshot-model](todo/project-registry-001-multi-project-snapshot-model.md) — `type: feature`, no deps (skeleton done). Refined: `get_project(project_id)` + `list_projects()` (cold read), new `WatcherSupervisor` (`supervisor.rs`, `Arc<Mutex>` map, incremental add/remove, publishes `ProjectAdded`), `AppState` drops single-project fields. 8 acceptance criteria. v1 data-layer foundation.
- [project-registry-002a-scan-roots-and-walk](todo/project-registry-002a-scan-roots-and-walk.md) — `type: feature`, depends on `project-registry-001`. Schema v1→v2 (`scan_roots` table + `projects.scan_root_id` FK), new `scan.rs` walk (depth-cap 3, junk-dir pruning, canonicalisation), scan-root CRUD + `add_scan_root`/`rescan`/`list` IPC returning a candidate checklist. v1 core. (Split from `002`.)
- [project-registry-002b-import-and-cascade-deregister](todo/project-registry-002b-import-and-cascade-deregister.md) — `type: feature`, depends on `project-registry-002a`. `import_scanned_projects` (upsert + `supervisor.add` per pick), `Db::remove_project`, `remove_scan_root` app-driven hard-delete cascade. v1 core. (Split from `002`.)
<!-- todo-list:end -->

## Doing

<!-- doing-list:start -->
*(None yet.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
*(None yet.)*
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
