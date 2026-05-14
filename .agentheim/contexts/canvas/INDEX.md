# canvas — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
- [canvas-003-focus-zoom](backlog/canvas-003-focus-zoom.md) — `type: feature`, depends on `design-system-001`. Click-or-keyboard "zoom to focus" — camera frames a target tile/node within the styleguide motion budget; viewport-only, no layout mutation. v1 core.
- [canvas-004-styleguide-visuals](backlog/canvas-004-styleguide-visuals.md) — `type: feature`, depends on `design-system-001`. Replace greybox tiles/nodes/edges/counts/badge-slot with `STYLEGUIDE.md` tokens. The frontend gate being exercised. v1 core.
- [canvas-005-project-discovery-affordances](backlog/canvas-005-project-discovery-affordances.md) — `type: feature`, depends on `project-registry-002b` + `design-system-001`. The canvas-BC UI for ADR-005's affordances — "Add project…", "Scan folder…" + discovery checklist modal, "Remove project", the "missing" tile state. Under-refined stub. v1 core.
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
- [canvas-002-render-multiple-project-tiles](todo/canvas-002-render-multiple-project-tiles.md) — `type: feature`, depends on `project-registry-001` + `design-system-001`. Restructure `Canvas.svelte` from single-tile to keyed-by-`project_id` collections: render N tiles, `ProjectSnapshot.id`, spiral auto-placement (persisted immediately), per-project layout, shared drag controller, per-project `canvas-001` patching, `f` fits all tiles. v1 core.
<!-- todo-list:end -->

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [canvas-001-targeted-canvas-updates](done/canvas-001-targeted-canvas-updates.md) — `type: feature`. Canvas patches its `ProjectSnapshot` in place from the fine-grained FS events (`task_moved`/`added`/`removed`, `bc_appeared`/`disappeared`) instead of re-fetching; coarse `AgentheimChanged` retired → renamed `ResyncRequired` (lag-only resync signal). ADR-009 amended in place. Commit `5fa7080`.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
