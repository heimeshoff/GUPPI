---
id: project-registry-003-manual-add-remove-and-missing-projects
type: feature
status: done
completed: 2026-05-15
scope: bc
depends_on: []
related_adrs:
  - ADR-004
  - ADR-005
  - ADR-008
  - ADR-013
related_research: []
prior_art:
  - project-registry-001-multi-project-snapshot-model
  - project-registry-002b-import-and-cascade-deregister
---

# Manual add / remove / missing projects — finishing the ADR-005 surface

## Why

The canvas-005 refinement (2026-05-15) surfaced that ADR-005's full discovery
surface is **not yet implemented** at the IPC / event layer:

- **"Add project…" (single manual register)** — `Db::upsert_project` exists,
  but no `#[tauri::command]` wraps it. The frontend cannot register a folder
  picked from the OS dialog.
- **"Remove project" (single, with ADR-005's 30-day tile-state retention)** —
  private `Db::remove_project` exists, no IPC, no retention. ADR-005 explicitly
  promises that a stray "Remove" can be undone by re-adding within 30 days.
- **The "missing" tile state (registered-but-unwatched)** — `list_projects`
  currently silently skips rows whose `.agentheim/` is gone. The canvas literally
  cannot render the missing state; ADR-005's "Do not auto-remove; render as
  missing" is unenforceable.
- **`project_removed` domain event** — no event fires when a project row leaves
  the registry. Today `remove_scan_root`'s cascade tears down watchers + DB rows
  without ever telling the frontend; the canvas would leave stale tiles forever.
  The new single-remove IPC will have the same problem unless an event exists.

These are all `project-registry` BC concerns. `canvas-005a` and `canvas-005b`
are blocked on this surface.

## What

### New IPC commands (`#[tauri::command]` in `lib.rs`)

- `register_project(path: String) -> Result<i64, String>`
  - Canonicalise via `scan::canonicalize_root` (same UNC-strip logic 002a uses).
  - Validate `.agentheim/` presence on the canonical path. On absence, reject
    with **exactly** `"not an Agentheim project"` (frontend renders this in a
    toast — canvas-005a).
  - `Db::upsert_project(canonical_str, nickname)` with **NULL** `scan_root_id`
    (manually-added; immune to scan-root cascade, per ADR-013).
  - `WatcherSupervisor::add(project_id, path)` — arms the watcher and fires
    `ProjectAdded` via the existing supervisor path.
  - Idempotent on canonical path. Re-registering returns the same `project_id`
    (the `upsert_project` already handles this).
  - **Soft-delete revival semantics:** if the upserted row had a non-NULL
    `deleted_at`, the upsert clears it as part of the same UPDATE. The matching
    `tile_positions` row (preserved through soft-delete) means the tile reappears
    at its old spot.

- `remove_project(project_id: i64) -> Result<(), String>`
  - **Soft-delete:** `UPDATE projects SET deleted_at = <now ISO-8601 UTC>
    WHERE id = ?`. The row stays. `tile_positions` stays untouched (the
    re-add-restores-in-place semantics).
  - `WatcherSupervisor::remove(project_id)` — tears down the per-project
    watcher.
  - Emit `DomainEvent::ProjectRemoved { project_id }` on the broadcast bus so
    the frontend can drop the tile cleanly.
  - Unknown `project_id` is a clean error (mirrors `get_project`'s shape).

### Schema migration v2 → v3

- Add column: `projects.deleted_at TEXT NULL` (ISO-8601 UTC; NULL = live row).
- Migration follows the existing v1→v2 pattern in `db.rs`. Test: a fresh DB is
  at v3; an existing v2 DB upgrades without data loss; `deleted_at` defaults
  NULL for all pre-existing rows.

### Query-surface changes

- `Db::list_projects` and `Db::list_projects_by_scan_root` filter
  `WHERE deleted_at IS NULL`. Soft-deleted rows are invisible to enumeration.
- `Db::upsert_project` **and** `Db::upsert_scanned_project` must set
  `deleted_at = NULL` on insert/update — so a re-add of a soft-deleted row
  revives it. This is the load-bearing detail of the retention design.
- `Db::project_path(project_id)` (used by `get_project`) should still resolve
  soft-deleted rows — the cascade-cleanup loop and the GC sweep need to look
  them up by id. The filter is on list/enumerate, not on per-id resolve.

### Startup GC sweep

- On `Db::open`, after the migration runs:
  `DELETE FROM projects WHERE deleted_at IS NOT NULL AND deleted_at <
  <30-days-ago ISO-8601 UTC>;`
- Cascading `tile_positions` rows are deleted by the existing `ON DELETE
  CASCADE` FK (already in v1 schema; verified by the 002b test
  `remove_project_deletes_row_and_cascades_tile_positions`).
- `tracing::info!` the count of swept rows for the operational signal
  (ADR-010).
- Constant: `const RETENTION_DAYS: i64 = 30;` near the sweep — single edit
  point if the ADR-005 number ever changes.

### `ProjectSnapshot.missing: bool`

- Rust: extend `ProjectSnapshot` in `project.rs` with `missing: bool`.
- TS: mirror in `src/lib/types.ts`.
- `list_projects` no longer silently skips rows whose `.agentheim/` is
  unreadable. The new shape: try `project::get_project(id, path)`; if it errors
  (no `.agentheim/`), build a synthetic
  `ProjectSnapshot { id, name, path, bcs: [], missing: true }` from the
  registry row. `tracing::warn!` still logs the original error — the
  operational signal (ADR-010) is retained.
- All present-project snapshots set `missing: false`.
- `get_project(project_id)` returns the same shape (a missing project resolves
  to a `missing: true` snapshot rather than an error).

### New domain event: `DomainEvent::ProjectRemoved { project_id }`

- Add the variant in `events.rs`.
- Emitted by:
  - The new `remove_project` IPC command.
  - The cascade loop inside `remove_scan_root` — per child id, fire
    `ProjectRemoved` **before** `supervisor.remove(id)` + `db.remove_project(id)`
    so the canvas can drop the tile cleanly. (Tile-position rows still cascade
    via the existing `ON DELETE CASCADE` FK on `tile_positions.project_id`.)
- Mirror as a new variant in TS `DomainEvent` (`src/lib/types.ts`).

### Frontend tolerance

`canvas-005a` ships the visual treatment for `missing: true` tiles. **This
task** only needs to ensure `Canvas.svelte` does not break when
`list_projects` starts returning `missing: true` snapshots: do not filter the
tile out of the collection, do not crash, no styling change yet. A 1–2-line
tolerance edit. The actual missing-tile rendering (dim + magenta border + `✕`
glyph) is canvas-005a's acceptance.

## Scope (in)

- `src-tauri/src/lib.rs`: add `register_project` and `remove_project` commands;
  extend `remove_scan_root` to fire `ProjectRemoved` per child id; register the
  new commands in `invoke_handler`.
- `src-tauri/src/db.rs`: schema v2→v3 migration; `deleted_at` filter on
  list/enumerate queries; `deleted_at = NULL` set on upsert paths; startup GC
  sweep helper invoked from `open`; tests.
- `src-tauri/src/project.rs`: add `missing: bool` to `ProjectSnapshot`; build
  synthetic missing snapshots inline when `.agentheim/` is unreadable; teach
  `list_projects` and `get_project` to use the new shape.
- `src-tauri/src/events.rs`: add `ProjectRemoved { project_id }`.
- `src/lib/types.ts`: mirror `missing: bool` on `ProjectSnapshot`; mirror
  `project_removed` variant on `DomainEvent`.
- `src/lib/Canvas.svelte`: tolerate `missing: true` from `list_projects` —
  keep the tile in the collection, do not crash. No styling change yet.

## Scope (out)

- The right-click context menu, modals, the missing-tile visual — all
  `canvas-005a` / `canvas-005b`.
- A wider "deleted_at"-aware admin UI (no restore-from-list, no manual undelete
  button etc.) — re-adding via `register_project` is the only restore path,
  intentionally.
- An ADR for the soft-delete/30-day-sweep design — it sits inside ADR-004
  (schema) + ADR-005 (retention stipulation) + ADR-013 (cascade ordering). The
  realisation is implementation, not a new decision.

## Acceptance criteria

- [x] `register_project(path)` IPC: canonicalises; accepts an Agentheim folder
      and returns the new `project_id`; rejects a non-Agentheim folder with
      **exactly** `"not an Agentheim project"`. Idempotent on canonical path
      (re-registering returns the same id, no duplicate row). Fires
      `ProjectAdded` on first register via the supervisor. Cargo test against a
      tempdir.
- [x] `register_project` re-registering a path whose row is soft-deleted
      clears `deleted_at` and rearms the watcher; `list_projects` returns the
      project again; the matching `tile_positions` row is untouched throughout.
      Cargo test using a forged-timestamp soft-delete.
- [x] `remove_project(project_id)` IPC: soft-deletes (`deleted_at` set),
      tears down the watcher, emits `ProjectRemoved { project_id }`. The
      `tile_positions` row is **not** touched. `list_projects` no longer
      returns the soft-deleted project. Cargo test against a tempdir.
- [x] Schema migration v2→v3: a fresh DB is at v3; an existing v2 DB upgrades
      without data loss; `projects.deleted_at` defaults NULL for all pre-existing
      rows. Cargo test mirroring the existing v1→v2 migration test.
- [x] Startup GC sweep: rows with `deleted_at < now - 30d` are deleted on
      `Db::open`; `tile_positions` for swept ids cascade-delete. Live rows
      (`deleted_at = NULL`) and recently-deleted rows
      (`deleted_at >= now - 30d`) are untouched. Cargo test using forged
      timestamps.
- [x] `ProjectSnapshot.missing` is `true` for a registry row whose
      `.agentheim/` is gone (with `bcs: []`); `false` for healthy projects.
      `list_projects` no longer silently skips. Cargo test against a tempdir
      where a project's `.agentheim/` is removed mid-flight.
- [x] `remove_scan_root` cascade emits one `ProjectRemoved` per child id
      **before** `supervisor.remove` + `db.remove_project`. The existing
      cascade test is extended to count the events on a tap of the broadcast
      bus.
- [x] TS mirror: `ProjectSnapshot.missing: boolean` and
      `DomainEvent { kind: 'project_removed', project_id: number }` are added.
      `pnpm check` clean.
- [x] `Canvas.svelte`: tolerates `missing: true` from `list_projects` —
      does NOT filter the tile out, does NOT crash. The visual treatment is
      canvas-005a. `pnpm check` clean.

## Notes

- Decisions captured 2026-05-15 during the refinement of `canvas-005`. See
  protocol entry of that date.
- The 30-day retention number is ADR-005's stipulation; not an ADR'd
  implementation choice. Recording it inline as `RETENTION_DAYS` in `db.rs`
  is the single edit point if it ever changes.
- **The soft-delete path keeps the seed-double-add guard simple.** The
  startup seed register is the existing idempotent `upsert_project` call; if
  a user had soft-deleted the seed before, restarting the app revives it
  via the `deleted_at = NULL` clause on upsert. Intended — the seed is a
  guaranteed survivor.
- **No ADR.** The implementation lives inside ADR-004 (schema) + ADR-005
  (discovery / retention) + ADR-013 (cascade ordering, now extended to also
  fire `ProjectRemoved`). The "soft-delete with 30-day sweep" is the
  realisation of an ADR-005 stipulation, not a new decision.

## Outcome

The ADR-005 discovery + retention surface is complete at the IPC / event
layer; canvas-005a and canvas-005b are unblocked.

**New IPC commands (`lib.rs`):**
- `register_project(path)` — canonicalise → validate `.agentheim/` (exact
  reject string `"not an Agentheim project"`) → `upsert_project` with NULL
  `scan_root_id` → `supervisor.add`. Idempotent; re-registering a soft-deleted
  path revives it (the `deleted_at = NULL` clause on conflict).
- `remove_project(project_id)` — unknown-id-rejects → `soft_delete_project`
  → `supervisor.remove` → publish `ProjectRemoved`. `tile_positions` is
  preserved through the 30-day window; re-adding restores the tile in place.

**Schema v2 → v3 (`db.rs`):**
- `projects.deleted_at TEXT NULL` (ISO-8601 UTC; NULL = live).
- `RETENTION_DAYS: i64 = 30` — single edit point.
- `list_projects` / `list_projects_by_scan_root` filter
  `WHERE deleted_at IS NULL`. `project_path(id)` still resolves soft-deleted
  rows (the GC sweep and per-id paths need them).
- `upsert_project` and `upsert_scanned_project` clear `deleted_at` on
  insert/update — the load-bearing revival mechanic.
- `Db::open` runs `sweep_expired_soft_deletes` after migrations; cascading
  `tile_positions` rows go with each swept project via the v1 ON DELETE
  CASCADE FK. Logged via `tracing::info!` for the ADR-010 signal.

**`ProjectSnapshot.missing: bool` (`project.rs`):**
- Healthy snapshots from `get_project` carry `missing: false`.
- `project::missing_snapshot(id, path)` builds a synthetic
  `{ id, name = folder_name, path, bcs: [], missing: true }` snapshot.
- `list_projects` IPC: try `get_project`, fall back to `missing_snapshot` on
  error (still `tracing::warn!`s the original error). `get_project` IPC: same
  fallback shape per-project. The registered-but-unwatched state is now
  enforceable.

**`DomainEvent::ProjectRemoved { project_id }` (`events.rs`):**
- Fired by `remove_project` (single soft-delete).
- Fired by the `remove_scan_root` cascade, per child, **before**
  `supervisor.remove` + `db.remove_project` (ADR-013 reconciliation
  2026-05-15 extension).
- TS mirror in `src/lib/types.ts`.

**Frontend tolerance (`Canvas.svelte`):**
- Added a `case 'project_removed'` no-op arm so the canvas tolerates the new
  event without crashing (canvas-005b ships the actual tile-drop behaviour).
- The missing-tile collection tolerance is already there structurally —
  `Canvas.svelte` does not filter on `missing` anywhere, so a `missing: true`
  entry simply renders as a tile with zero BCs around it. canvas-005a will
  add the dim + magenta visual.

**Tests added (16 new, 89/89 cargo + pnpm check 0/0/0):**
- 10 `db::tests`: `fresh_db_is_at_schema_version_three`,
  `v2_db_migrates_to_v3_without_data_loss`,
  `soft_delete_project_sets_deleted_at_but_keeps_row_and_tile_position`,
  `upsert_project_revives_a_soft_deleted_row_clearing_deleted_at`,
  `upsert_scanned_project_also_revives_a_soft_deleted_row`,
  `list_projects_by_scan_root_hides_soft_deleted_children`,
  `project_deleted_at_returns_none_for_unknown_id`,
  `soft_delete_project_with_unknown_id_is_a_clean_no_op`,
  `startup_gc_sweep_hard_deletes_rows_older_than_retention_window`, plus the
  existing `fresh_db_is_at_schema_version_two` reshaped to assert the v2
  *surface* survives the v3 bump.
- 2 `project::tests`: `healthy_get_project_carries_missing_false`,
  `missing_snapshot_builds_a_missing_true_snapshot_for_a_registered_path`.
- 4 `scan::tests` (IPC composition shape):
  `register_project_registers_an_agentheim_folder_with_null_scan_root_id`,
  `register_project_revives_a_soft_deleted_path_preserving_tile_position`,
  `remove_project_soft_deletes_watches_off_and_emits_project_removed`,
  `list_projects_returns_missing_snapshot_when_agentheim_disappears_mid_flight`,
  `register_project_rejects_a_non_agentheim_folder_with_exact_error_string`,
  plus the existing cascade test
  `remove_scan_root_cascade_drops_children_watchers_and_tiles_then_the_root`
  extended to tap the bus and count `ProjectRemoved` events.

**README:** ubiquitous-language section gained entries for manual register,
soft-delete / 30-day retention, cascade-deregister vs. soft-delete contrast,
and the `ProjectRemoved` event; the `ProjectSnapshot` entry and the
registered-but-unwatched (missing) entry both reflect the new `missing: bool`
shape.

**No new ADR** — ADR-004 (schema) + ADR-005 (discovery / retention) + ADR-013
(cascade ordering, here extended to also fire `ProjectRemoved`) already
specify the design. The "soft-delete with 30-day sweep" is the realisation of
ADR-005's stipulation, not a new decision.

**Key files:** `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`,
`src-tauri/src/project.rs`, `src-tauri/src/events.rs`,
`src-tauri/src/scan.rs`, `src/lib/types.ts`, `src/lib/Canvas.svelte`,
`.agentheim/contexts/project-registry/README.md`.
