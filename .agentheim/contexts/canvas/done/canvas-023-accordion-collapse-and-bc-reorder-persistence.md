---
id: canvas-023
title: Persist BC accordion collapse state + drag-to-reorder BC rows
status: done
type: feature
context: canvas
created: 2026-05-24
completed: 2026-05-24
commit:
depends_on: [canvas-020]
blocks: []
tags: [persistence, accordion, reorder, sqlite, view-state]
related_adrs: [ADR-004, ADR-009, ADR-021]
related_research: []
prior_art: [canvas-020, project-registry-004]
---

## Why

The reference shows "drag BC header to reorder" and per-BC collapse state.
canvas-020 ships these interactions with in-memory defaults; this task makes
them stick across restarts so a user's arrangement of a frame's accordion
(which BCs are collapsed, what order the rows are in) survives like frame and
(formerly) BC-bubble positions do. This is GUPPI's own view-state, persisted
in GUPPI's SQLite (ADR-004), NOT in the target project's `.agentheim/`.

## What

- **Collapse state** — persist per `(project_id, bc_name)` whether the
  accordion row is collapsed or expanded. New SQLite storage (a
  `bc_collapsed` column on the existing per-BC table, or a small new table —
  the `bc_positions` table from project-registry-004 is the natural neighbour
  and may be retired/repurposed since BC bubble POSITIONS no longer exist
  post-canvas-020; decide in refinement whether to repurpose `bc_positions`
  → `bc_view_state (project_id, bc_name, collapsed, sort_order)` or add
  alongside).
- **Reorder** — drag a BC accordion header to reorder rows within the frame;
  persist a per-`(project_id, bc_name)` `sort_order`. Default order when
  unset: stable by BC name (the registry's current order).
- IPC + DB CRUD following the project-registry-004 `save_bc_position`
  pattern (`save_bc_view_state` / `load_bc_view_state` / batch load on frame
  paint). Schema migration bumps the SQLite version (ADR-004 pattern, with the
  data-preservation test the design-system-004 v4→v5 migration set as
  precedent).
- Note: BC-bubble drag-position storage (`bc_positions`, schema v4) is now
  dead surface — canvas-020 retired BC bubbles. This task is the natural place
  to repurpose or retire it; coordinate with project-registry (the DB owner).

## Acceptance criteria

- [ ] Collapsing/expanding a BC accordion row persists per
      `(project_id, bc_name)` and survives restart.
- [ ] Dragging a BC accordion header reorders the rows; the order persists
      per `(project_id, bc_name)` and survives restart; unset order defaults
      stably to BC-name order.
- [ ] SQLite migration bumps the schema version with a data-preservation
      test (existing rows survive); the dead `bc_positions` surface is
      repurposed or explicitly retired (decision recorded).
- [ ] IPC CRUD (`save_bc_view_state` / `load_bc_view_state` + batch) follows
      the project-registry-004 pattern; frame paint batch-loads view state.
- [ ] `pnpm check` 0/0/0; `cargo test --lib` green (new migration + CRUD
      tests).

## Notes

depends_on canvas-020 only (the accordion it persists) — NOT canvas-019
directly (the persistence is substrate-agnostic; it stores view-state, not
render state) and NOT design-system-006 (no new visuals, persistence only).

This task spans canvas (the interaction + IPC wiring) and touches
project-registry's DB layer (schema + CRUD). Held in canvas because the
interaction is canvas's; the DB CRUD is thin and follows the established
project-registry-004 pattern. If refinement decides the DB work is
substantial enough to be its own project-registry task, split then. The
`bc_positions` repurpose/retire is the one cross-BC coordination point —
flag to project-registry's owner.

Decision recorded: ADR-021 (retire `bc_positions`, add `bc_view_state`).

## Outcome

Persisted the kanban-accordion's per-BC arrangement (collapse flag +
drag-reordered `sort_order`) in GUPPI's own SQLite view-state (ADR-004), so a
user's frame arrangement survives restart.

**Decision (ADR-021):** the dead `bc_positions` table (orphaned when canvas-020
retired BC bubbles) was **retired** — DROPped in a new schema v5→v6 migration —
and replaced by a fresh `bc_view_state (project_id, bc_name, collapsed,
sort_order)` table at the same `(project_id, bc_name)` grain with the same
`ON DELETE CASCADE` (so it inherits ADR-005 soft-delete preservation /
hard-delete cascade). Retire-and-recreate over `ALTER TABLE` because an (x,y)
bubble coordinate carries no collapse/order information and there was nothing to
migrate; the data-preservation contract upheld is that `projects` /
`tile_positions` / `preferences` survive untouched.

**Key files:**
- `src-tauri/src/db.rs` — `CURRENT_SCHEMA_VERSION` 5→6; v5→v6 migration step
  (DROP `bc_positions`, CREATE `bc_view_state`); `BcViewStateRow` struct;
  `save_bc_view_state` / `bc_view_state` / `bc_view_states` CRUD (replacing the
  dead `bc_position` CRUD). New tests: v5→v6 data-preservation,
  `bc_positions`-is-dropped, round-trip, NULL-sort_order, batch, project
  isolation, hard-delete cascade, soft-delete preservation, concurrent-saves
  race-freedom, close/reopen persistence, fresh-DB-at-v6.
- `src-tauri/src/lib.rs` — IPC commands `save_bc_view_state` /
  `load_bc_view_state` / `load_bc_view_states` (replacing the dead
  `save_bc_position` / `load_bc_position(s)`); handler registration updated.
- `src/lib/ipc.ts` — `BcViewState` interface + `saveBcViewState` /
  `loadBcViewState` / `loadBcViewStates` (replacing the dead `saveBcPosition` /
  `loadBcPosition(s)`).
- `src/lib/Canvas.svelte` — `bcViewState` rune (collapse + order, batch-loaded
  on frame paint via `primeBcViewState`); `isExpanded`/`toggleAccordion` now
  read/write-through it; `orderBcs` comparator (explicit `sortOrder` first,
  stable BC-name fallback) feeding `mountedInteriors`; `reorderBc` (dense
  renumber + persist whole frame); HTML5 drag-to-reorder on accordion headers
  (drag handle, click-toggle suppressed on a genuine drag); `grab`/`grabbing`
  cursor + `.dragging` opacity affordance; `role="list"`/`role="listitem"` for
  a11y.
- `.agentheim/knowledge/decisions/ADR-021-retire-bc-positions-for-bc-view-state.md`
  — the retire-vs-repurpose decision.
- `.agentheim/contexts/canvas/README.md` — updated **BC accordion row** +
  **Layout** entries; added **BC view-state** vocabulary entry.

**Cross-BC flag for project-registry's owner:** `bc_positions` originated in
`project-registry-004` (DB-layer owner). This task removed the table and its
`save_bc_position`/`load_bc_position(s)` IPC + TS surface. project-registry's
README may still document `bc_positions` as a surface — it now no longer exists
(retired at schema v6, ADR-021). Per the held-in-canvas scoping rule, this
worker did NOT edit project-registry's README; its owner should reconcile.

**Gates:** `pnpm check` 0/0/0; `cargo test --lib` 149 passed / 0 failed.
