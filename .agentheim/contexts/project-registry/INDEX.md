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
*(None.)*
<!-- todo-list:end -->

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [project-registry-003-manual-add-remove-and-missing-projects](done/project-registry-003-manual-add-remove-and-missing-projects.md) — `feature` — Completed ADR-005's IPC surface: `register_project` (canonicalise → `.agentheim/` validate → upsert with NULL `scan_root_id` → supervisor.add), `remove_project` (soft-delete via `projects.deleted_at`, watcher down, `ProjectRemoved` event), schema v2→v3 + 30-day startup GC sweep (`RETENTION_DAYS = 30`), `ProjectSnapshot.missing: bool` for registered-but-unwatched rows (no more silent skip), `ProjectRemoved { project_id }` event fired by **both** single-remove and `remove_scan_root` cascade (BEFORE supervisor.remove + db.remove_project). 89/89 cargo tests (16 new). `pnpm check` 0/0/0. **Verifier note:** the `register_project_rejects_a_non_agentheim_folder_with_exact_error_string` test is tautological (asserts hardcoded constant against itself rather than calling the IPC handler) — production code is correct, but the test does not exercise it. Commit `ebe2e48`. (2026-05-15)
- [project-registry-002b-import-and-cascade-deregister](done/project-registry-002b-import-and-cascade-deregister.md) — `feature` — mutation layer atop 002a: `Db::upsert_scanned_project` + `Db::remove_project` + `Db::delete_scan_root` + `Db::list_projects_by_scan_root`; `import_scanned_projects` IPC (re-verifies pick against candidate set, upserts + `supervisor.add` per pick); `remove_scan_root` IPC (app-driven cascade: `supervisor.remove` → `db.remove_project` per child → `delete_scan_root` last; `ON DELETE RESTRICT` as checked invariant). 73/73 cargo tests (13 new). Hard-delete; manually-added projects (NULL `scan_root_id`) immune to cascade. Commit `5ad554a`. (2026-05-15)
- [project-registry-002a-scan-roots-and-walk](done/project-registry-002a-scan-roots-and-walk.md) — `feature` — schema v1→v2 (new `scan_roots` table + nullable `projects.scan_root_id` FK with `ON DELETE RESTRICT`), new `scan.rs` walker (depth-cap 3, junk-dir pruning, UNC-stripping canonicalisation), `Db` scan-root CRUD, three new IPC commands (`add_scan_root` / `rescan_scan_root` / `list_scan_roots`) returning a `ScanCandidate` checklist. 60/60 cargo tests (19 new). Commit `bace9fd`. (2026-05-15)
- [project-registry-001-multi-project-snapshot-model](done/project-registry-001-multi-project-snapshot-model.md) — `feature` — `Db::list_projects()` + `get_project(project_id)`, new `supervisor.rs` (`Arc<Mutex>` per-project watcher map publishing `ProjectAdded`), `AppState` drops single-project fields. 41/41 cargo tests green, `pnpm check` clean. Commit `d594ad5`. (2026-05-15)
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
