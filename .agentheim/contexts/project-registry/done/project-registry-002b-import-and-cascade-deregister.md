---
id: project-registry-002b-import-and-cascade-deregister
type: feature
status: done
completed: 2026-05-15
commit: 5ad554a
scope: bc
depends_on:
  - project-registry-002a-scan-roots-and-walk
related_adrs:
  - ADR-004
  - ADR-005
  - ADR-008
  - ADR-013
related_research: []
prior_art: []
---

# Import scanned projects + cascade-deregister

## Why

`project-registry-002a` delivers persisted scan roots and the walk that turns a
root into a checklist of candidate projects — but nothing yet *registers* a
picked candidate, and a scan root cannot be removed. This task is the
**mutation layer**: importing the user's checklist picks into the registry, and
removing a scan root (cascade-deregistering everything it brought in).

This completes the v1 "real way to get projects in" alongside `002a`.

## What

**Import flow**
- `Db::upsert_scanned_project(path, nickname, scan_root_id) -> i64` — like the
  existing `upsert_project` but stamps `scan_root_id` (the discovering root).
  The existing `upsert_project` (seed path, NULL origin) stays unchanged.
- `import_scanned_projects(scan_root_id, paths) -> Vec<project_id>` IPC command
  — the checklist result. Per picked path: `upsert_scanned_project(...)` then
  `supervisor.add(project_id, path)` (from `project-registry-001`). Re-verifies
  each path belongs to the root's candidate set before importing (re-walk-and-
  verify — cheap and safer than trusting the frontend's set). Importing the
  same path twice does not duplicate the row (path-idempotent upsert).

**Cascade-deregister**
- `Db::remove_project(project_id)` — deferred here from `project-registry-001`.
  A single `DELETE FROM projects WHERE id = ?1`; `tile_positions` follows via
  the existing `ON DELETE CASCADE`. **No soft-delete / retention** — ADR-005's
  30-day tile-state retention is scoped to the user-initiated single "Remove
  project" affordance, not bulk cascade (see Notes / ADR-013).
- A `Db` method to list a scan root's child project ids (drives the cascade).
- `remove_scan_root(scan_root_id)` IPC command — the **app-driven** cascade
  (it cannot be DB-level: `supervisor.remove` must run per project in
  application code). Order: enumerate child projects → `supervisor.remove`
  each → `db.remove_project` each → delete the `scan_roots` row **last** (so
  `ON DELETE RESTRICT` never trips on a correct run). A manually-added project
  (NULL `scan_root_id`) is never touched by any root's cascade.

**Frontend** — none in this task. The user-facing affordances (folder picker,
discovery checklist modal, "Remove project") belong to the canvas BC per
ADR-005 and are captured as `canvas-005-project-discovery-affordances`. This
task ships backend-only and is integration-tested.

## Acceptance criteria

- [ ] `import_scanned_projects` registers each picked project with its
      `scan_root_id` set and starts a watcher via `supervisor.add`; importing
      the same path twice does not duplicate the row — integration tested.
- [ ] `import_scanned_projects` only imports paths that belong to the named
      root's candidate set (re-verified) — a path outside the set is rejected,
      not silently registered.
- [ ] `remove_scan_root` removes every project imported under the root
      (`supervisor.remove` called per project, `projects` rows gone,
      `tile_positions` rows gone via cascade) and then deletes the root row —
      integration tested.
- [ ] A manually-added project (NULL `scan_root_id`) located under the same
      path subtree as a scan root is untouched by `remove_scan_root` —
      integration tested.
- [ ] `Db::remove_project` deletes the row and its `tile_positions`; an unknown
      `project_id` is a clean no-op/error, not a panic — unit tested.

## Notes

Surfaced from the v1 "finish v1 first" capture pass (2026-05-14); split from
the original `project-registry-002` during refinement (2026-05-15).

Decisions made during refinement (2026-05-15, with Marco):
- **Cascade-deregister on root removal** — removing a root removes its
  discovered-and-imported projects. The cascade is **app-driven**, not
  DB-level: `supervisor.remove` must run per project (it can't run inside
  SQLite), so the command drives the ordering and `ON DELETE RESTRICT` guards
  it.
- **Cascade hard-deletes** — ADR-005's 30-day tile-state retention applies only
  to the user-initiated single "Remove project" affordance (an undo window),
  *not* to scan-root cascade-deregister, which is a deliberate bulk discard.
  The retention/GC machinery, when built, belongs with the "Remove project"
  affordance (`canvas-005`) plus a persistence GC sweep — not here. Recorded in
  ADR-005's reconciliation note and ADR-013.

Consumes `project-registry-001`'s `supervisor.add` / `supervisor.remove` and
`project-registry-002a`'s schema (`scan_root_id` column, `scan_roots` table)
and scan walk (for import re-verification).

BC seam: the *scan/import capability* is project-registry. The *UI* (folder
picker, checklist modal, "Remove project") is canvas BC per ADR-005 — captured
as `canvas-005-project-discovery-affordances` (`depends_on: this task,
design-system-001`).

## Outcome

Landed the v1 mutation layer atop `002a`'s persisted scan roots + walker.

**`Db` surface (`src-tauri/src/db.rs`):**
- `upsert_scanned_project(path, nickname, scan_root_id) -> i64` — idempotent
  on canonical path; stamps the discovering root for cascade-deregister.
- `remove_project(project_id) -> Result<(), DbError>` — single-row DELETE;
  `tile_positions` follows via the schema-v1 `ON DELETE CASCADE`; unknown id
  is a clean no-op.
- `list_projects_by_scan_root(scan_root_id) -> Vec<i64>` — drives the cascade
  enumeration; manually-added projects (NULL) never appear.
- `delete_scan_root(scan_root_id) -> Result<(), DbError>` — guarded by the
  ADR-013 `ON DELETE RESTRICT` FK: a stray child still referencing the root
  causes the delete to fail loud rather than orphan rows.

**IPC commands (`src-tauri/src/lib.rs`):**
- `import_scanned_projects(scan_root_id, paths) -> Vec<i64>` — re-walks the
  root to verify each pick, stamps `scan_root_id` on survivors, arms each
  watcher via `supervisor.add`. Out-of-set paths are skipped+logged, not
  silently registered. Missing `.agentheim/` is the registered-but-unwatched
  state (ADR-005), not a fatal error.
- `remove_scan_root(scan_root_id)` — app-driven cascade: enumerate children
  → `supervisor.remove` + `db.remove_project` each → `delete_scan_root` last.
  Hard-delete (ADR-013); manually-added projects untouched.

Tightened the dead-code allow on the `AppState.supervisor` field and
`WatcherSupervisor::remove` — both are now called by the cascade IPC.

**Tests added (13 new, 73/73 cargo tests green):**
- 8 `db::tests` — `upsert_scanned_project` stamps + idempotency,
  `remove_project` cascades + no-op on unknown id, `list_projects_by_scan_root`
  filters to that root only, `delete_scan_root` succeeds after cleanup +
  rejected by `RESTRICT` when children remain + no-op on unknown id.
- 4 `scan::tests` — composition tests that stitch `Db` + walker +
  `WatcherSupervisor` against real temp trees, mirroring the IPC handlers
  end-to-end without a Tauri test app. Cover: idempotent import + paths
  outside-candidate-set rejection + full cascade tear-down + manually-added
  survival.

**README:** added "Import (scanned-projects import)" and "Cascade-deregister"
to the BC ubiquitous-language section.

**Key files:** `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`,
`src-tauri/src/scan.rs`, `src-tauri/src/supervisor.rs`,
`.agentheim/contexts/project-registry/README.md`.

No new ADR — ADR-013 already specified the cascade semantics; the
implementation matches.
