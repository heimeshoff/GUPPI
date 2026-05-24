---
id: ADR-021
title: Retire bc_positions; add bc_view_state for accordion collapse + reorder
status: Accepted
scope: bc
bc: canvas
date: 2026-05-24
related_tasks: [canvas-023]
related_adrs: [ADR-004, ADR-017, ADR-005, ADR-014]
---

# ADR-021: Retire `bc_positions`; add `bc_view_state` for accordion collapse + reorder

**Status:** Accepted
**Scope:** bc (canvas)

## Context

canvas-023 persists two new per-BC pieces of GUPPI view-state (ADR-004) for the
kanban-accordion frame interior (ADR-017): whether each BC's accordion row is
collapsed, and the user's drag-reordered row order.

There was already a per-BC SQLite table at exactly the right grain:
`bc_positions (project_id, bc_name, x, y)` (schema v4, `project-registry-004`,
ADR-014), which stored where each draggable BC *bubble* sat inside its project
frame. But canvas-020 (ADR-017) retired the draggable BC bubbles entirely —
BCs are now DOM accordion rows, not Pixi bubbles — so `bc_positions` lost its
only reader. It was dead surface: the `save_bc_position` / `load_bc_position` /
`load_bc_positions` IPC commands and their TS wrappers were referenced from
nowhere but comments.

So canvas-023 faced a choice: **repurpose** `bc_positions` into
`bc_view_state (project_id, bc_name, collapsed, sort_order)`, OR **retire**
`bc_positions` and add a fresh `bc_view_state` table alongside.

## Decision

**Retire `bc_positions` and create a fresh `bc_view_state` table** in the same
v5→v6 migration step.

```sql
DROP TABLE IF EXISTS bc_positions;
CREATE TABLE bc_view_state (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    bc_name    TEXT    NOT NULL,
    collapsed  INTEGER NOT NULL DEFAULT 0,   -- 0 = expanded (canvas-020 default)
    sort_order INTEGER NULL,                  -- NULL = unset; fall back to BC-name order
    PRIMARY KEY (project_id, bc_name)
);
```

- `CURRENT_SCHEMA_VERSION` bumped 5 → 6.
- The dead `save_bc_position` / `bc_position` / `bc_positions` DB methods, the
  three IPC commands, and their TS wrappers are deleted; replaced by
  `save_bc_view_state` / `bc_view_state` / `bc_view_states` and
  `saveBcViewState` / `loadBcViewState` / `loadBcViewStates`, following the
  `project-registry-004` `save_bc_position` shape.

### Why retire-and-recreate rather than `ALTER TABLE`

1. **No data to preserve.** An (x, y) bubble coordinate carries zero
   information about collapse state or row order, and `bc_positions` has had no
   live writer since canvas-020. Migrating rows would migrate noise. A clean
   `DROP` + `CREATE` is the honest representation: the rows are genuinely
   discarded because they mean nothing to the new feature.
2. **Cleaner column types.** A repurpose-in-place via `ALTER TABLE` would leave
   the old `x REAL NOT NULL, y REAL NOT NULL` columns (or require an
   `ALTER ... DROP COLUMN` dance) sitting next to `collapsed` / `sort_order`,
   muddying the schema with vestigial bubble coordinates. The fresh table has
   exactly the columns the feature needs.
3. **Same grain, same FK semantics — no behaviour regression.**
   `bc_view_state` keeps the identical `(project_id, bc_name)` primary key and
   `ON DELETE CASCADE` on `project_id`, so it inherits, unchanged, the
   ADR-005 retention behaviour `bc_positions` had: a soft-delete (single
   "Remove project") never touches the table (view-state survives the 30-day
   window; a re-register revives the arrangement in place), while a hard-delete
   — the startup GC sweep or the scan-root cascade-deregister — cascades through
   the FK and clears it.

### Order semantics

`sort_order` is NULL-able. A NULL means "this BC has no explicit order"; the
canvas comparator (`orderBcs`) sends NULL-ordered rows to the band after every
explicitly-ordered row and breaks ties by the snapshot's BC-name order (the
registry's stable default). On a drag-reorder, the canvas densely renumbers the
*whole* frame's BCs (0..n-1) and writes each through, keeping the stored order
total and gap-free so a subsequent insert is unambiguous.

### Persistence mechanism (ADR-009 note)

Collapse/reorder are user-driven local mutations the canvas already knows
about, with no second consumer. Per ADR-009's guidance, they are persisted via
plain `save_*` / `load_*` IPC command round-trips (mirroring
`project-registry-004`'s positions), **not** a new `DomainEvent`.

## Consequences

- **Cross-BC coordination (project-registry):** `bc_positions` originated in
  `project-registry-004` (the DB-layer owner). canvas-023 removed it and its
  CRUD/IPC/TS surface. project-registry's `README` may still mention
  `bc_positions` as a documented surface; the canvas-023 worker flagged this
  in its Outcome for project-registry's owner to reconcile, per the held-in-
  canvas scoping rule (the worker did not edit project-registry's README).
- A v5 DB upgrades cleanly: the v5→v6 step drops the dead table and creates the
  new one; `projects` / `tile_positions` / `preferences` survive untouched
  (verified by `v5_db_migrates_to_v6_dropping_bc_positions_and_preserving_other_tables`).
- Future per-BC interior view-state (e.g. a remembered scroll position or a
  per-column filter) gets a natural home: add a column to `bc_view_state`
  rather than a new table.
