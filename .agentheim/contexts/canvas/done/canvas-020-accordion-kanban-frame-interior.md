---
id: canvas-020
title: "Frame interior: BC accordion + kanban board layout (retire bubbles + edges)"
status: done
type: feature
context: canvas
created: 2026-05-24
completed: 2026-05-24
commit:
depends_on: [canvas-019, design-system-006, project-registry-005]
blocks: [canvas-021, canvas-022, canvas-023]
tags: [rendering, kanban, accordion, pivot, interior, retire-bubbles]
related_adrs: [ADR-003, ADR-015, ADR-016]
related_research: []
prior_art: [canvas-007, canvas-015]
---

## Why

The pivot replaces each frame's interior — currently Pixi BC bubbles in a
spring-electrical layout with intra-project relationship edges (canvas-007 /
ADR-015) — with a vertical accordion of collapsible bounded contexts, each
holding a kanban board (BACKLOG → TODO → DOING → DONE) of task cards
(reference: `.agentheim/contexts/design-system/references/kanban.png`). This
is the spine of the canvas pivot: it establishes the interior that the
card-rendering (canvas-021), detail-panel (canvas-022), and persistence
(canvas-023) tasks build on. It RETIRES the BC bubbles, the intra-project
edges, and `bc-layout.ts`.

## What

Build the frame interior per the canvas-019 substrate decision (hybrid /
full-DOM / pure-Pixi — this task is written substrate-agnostic at the
contract level; the implementation follows ADR-017):

- **BC accordion** — each BC renders as a collapsible row (chevron + BC name
  + the active/blocked/idling roll-up from agent-awareness-002 + total task
  count). Rows stack vertically; expanding shows the BC's kanban board,
  collapsing hides it. Default expand/collapse state per BC (all expanded at
  first paint; persisted state is canvas-023).
- **Kanban board per BC** — four columns BACKLOG/TODO/DOING/DONE, each
  listing that BC's `tasks[]` (project-registry-005) filtered by
  `task.column`. Horizontal scroll if columns overflow the frame width;
  vertical scroll within a column if cards overflow.
- **Frame sizing** — the frame still auto-fits / has a sensible default
  size; the interior scrolls within it. Reconcile with the canvas-019
  culling/LOD policy (off-screen or zoomed-out frames may render a cheaper
  shell).
- **Retire** the Pixi BC-bubble draw path, the intra-project edge draw path
  (`drawIntraProjectEdges` / `drawArrowhead` / `drawAclNotch`), and
  `bc-layout.ts`. The frame SHELL (border + header bar + drag handle +
  right-click menu + missing-tile visual) is preserved per canvas-019.
- **Live updates** — the canvas-001 targeted-update model extends to tasks:
  `TaskAdded` adds a card to a column, `TaskMoved` moves a card between
  columns, `TaskRemoved` drops a card, `TaskChanged` updates a card's
  metadata, `BCAppeared`/`BCDisappeared` add/remove an accordion row — all
  without a full resync (resync only on `resync_required`). Extend
  `snapshot-patch.ts` for the task-level events from project-registry-005.

## Acceptance criteria

- [x] Every project frame's interior renders as a vertical accordion of BC
      rows; expanding a row shows that BC's four-column kanban board of task
      cards (card CONTENT detail is canvas-021; this task establishes the
      board/column/accordion structure + placement).
- [x] Columns are BACKLOG/TODO/DOING/DONE; each lists the BC's tasks by
      `task.column`; horizontal + per-column vertical scroll work.
- [x] Accordion rows collapse/expand on click (persistence is canvas-023;
      this task ships the interaction with an in-memory default of all
      expanded).
- [x] BC bubbles, intra-project edges, and `bc-layout.ts` are removed;
      `Canvas.svelte` no longer draws §3.7/§3.8 interior elements. Frame
      shell (border/header/drag/right-click/missing-tile) preserved and all
      canvas-005a/005b/006/012 affordances still work.
- [x] Targeted updates: `TaskAdded`/`TaskMoved`/`TaskRemoved`/`TaskChanged`/
      `BCAppeared`/`BCDisappeared` patch the interior in place via
      `snapshot-patch.ts`; no full `get_project` except on `resync_required`.
      (`canvas-001` invariant preserved at task granularity.)
- [x] Pan/zoom/drag-to-reposition of the whole frame still works (camera +
      drag-controller preserved per canvas-019); the culling/LOD policy from
      ADR-017 is implemented.
- [x] Built against `STYLEGUIDE.md` (design-system-006 sections); no raw
      hex/sizing. `pnpm check` 0/0/0; `cargo test --lib` unchanged
      (frontend-only unless the patch layer needs Rust — not expected).

## Outcome

Implemented the ADR-017 hybrid frame interior. The Pixi **shell** (border +
header bar + drag hit-area + missing glyph + focus ring) survives unchanged;
the BC-bubble + intra-project-edge interior and `bc-layout.ts` are **deleted**.
The kanban-accordion interior is now a **DOM overlay** positioned at
`worldToScreen(frame.pos)` + `transform: scale(z)`.

Key files:
- `src/lib/frame-interior.ts` (new) — pure helpers + the tested verification
  surface: `bucketTasksByColumn`, `frameSize`, `frameScreenAabb`,
  `shouldMountInterior` (the ADR-017 culling + LOD policy with
  `CULL_MARGIN_PX = 120` / `LOD_ZOOM_FLOOR = 0.45`), `COLUMN_ORDER`.
  Tested in `src/lib/frame-interior.test.ts` (11 tests).
- `src/lib/Canvas.svelte` — retired the BC-bubble/edge interior path,
  `bc-layout` import, `recomputeBcLayout`/`bcCenterWorld`/`drawArrowhead`/
  `drawAclNotch`/`drawIntraProjectEdges*`/`createBcDisplayObjects`/
  `updateBcDisplayObjects`/`createStatusBadge`/`updateStatusBadge`/
  `attachBcInteractivity`/`toggleBcFocusRing`; `ProjectEntry` is now
  `{ id, snapshot, pos, size }`; added the `mountedInteriors` `$derived`
  (cull/LOD gate) + the `cameraVersion` reactive bridge (ADR-019) + the DOM
  interior markup (accordion rows → kanban columns → task cards) + styles (all
  `--guppi-*` tokens, styleguide §3.9–3.11) + the accordion expand/collapse
  state (in-memory all-expanded default) + the `bcRollups` store wired to
  `get_bc_agent_rollup` and refreshed on `task_agent_state_changed` + task
  events.
- `src/lib/ipc.ts` — added `getTaskAgentState` + `getBcAgentRollup` (the
  agent-awareness-002 read contract).
- Deleted `src/lib/bc-layout.ts` + `src/lib/bc-layout.test.ts` (ADR-017 retires
  them; no production importer remains).

The snapshot-patch layer needed **no extension** — project-registry-005 already
patches the task records and BCs in place; the DOM interior derives reactively
from the `$state` snapshot. The live agent state is a separate read model (not
on the snapshot), so the accordion roll-up re-fetches `get_bc_agent_rollup`.

Decision recorded: **ADR-019** (kanban-accordion DOM-interior overlay) — the
`cameraVersion` reactive bridge between the imperative Pixi camera and the
reactive DOM overlay; the fixed frame-shell size (interior scrolls within);
the one runtime-hex exception (data-driven roll-up glyph colour from a token
numeric).

The drag-controller's `bc` kind is preserved (ADR-017 contract) but is now a
defensive no-op — no Pixi hit-area starts a BC drag. `bc_positions` schema +
`save/load_bc_position` IPC remain in project-registry-004, now unused by the
canvas (harmless; out of scope to remove).

Gates: `pnpm check` 0/0/0; `pnpm test` 34/34 passing; `pnpm build` succeeds.
No Rust changes (`cargo test --lib` unchanged). Styleguide §5 Q11 sign-off is a
deferred human gate (Marco runs it in `pnpm tauri dev`), not a blocker.

## Notes

The interior-rendering SPINE. canvas-021/022/023 build on it. depends_on
canvas-019 (substrate), design-system-006 (gate), project-registry-005
(task data). The active/blocked/idling roll-up in each accordion row reads
agent-awareness-002 — but this task can ship the row with the count slot
present and wired to agent-awareness-002 if it has landed, or stubbed to the
column-count roll-up if not (coordinate sequencing in refinement; the roll-up
DETAIL is agent-awareness-002 + canvas-021).

This is large; refinement may split it (accordion structure vs. kanban
board vs. live-update patching). Held as one for now because the interior is
one coherent layout problem and splitting mid-structure creates churny
half-states. Decompose at REFINE if the substrate decision (canvas-019)
makes a natural seam.
