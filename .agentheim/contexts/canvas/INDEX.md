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
- [canvas-005b-scan-flow-and-scan-root-management](done/canvas-005b-scan-flow-and-scan-root-management.md) — `feature` — Shipped the scan-folder flow (folder picker → `addScanRoot` → discovery checklist modal: greyed-pre-ticked-disabled already-imported rows + "imported" badge + Select-all/none over togglable rows + disabled-on-zero-new-picks Import button → `importScannedProjects`, tiles arrive via canvas-006's `liveAddChain` at distinct spiral slots) and the scan-roots management surface (hidden-when-empty menu entry, list with per-row child counts via new thin `list_projects_by_scan_root` IPC wrapper, Rescan opens checklist flagged " (rescan)", Remove opens cascade-confirmation that names the path + N child count + explicit "tile state will not be retained" warning). New generic `Modal.svelte` primitive extracts the header/body/footer/backdrop/Escape contract for all three modals (extraction triggered by three consumers — styleguide entry deliberately deferred per task scope). `project_removed` handler from 005a reused without duplication; cascade fan-out drops tiles cleanly. `pnpm check` 0/0/0 (938 files). Commit `4979e27`. (2026-05-15)
- [canvas-005a-single-shot-discovery-affordances](done/canvas-005a-single-shot-discovery-affordances.md) — `feature` — Shipped ADR-005's three single-shot discovery affordances: right-click empty canvas → context menu with "Add project…" (Tauri folder picker → `registerProject` → rejection surfaces exact `"not an Agentheim project"` toast); right-click tile → "Remove project" (no confirmation, ADR-005's 30-day undo as safety net); missing-tile rendering (50% opacity, `statusMissing` magenta border, `✕` corner glyph at `spacing.lg`). New `@tauri-apps/plugin-dialog` wired in (Cargo.toml + capability + `tauri_plugin_dialog::init()`); HTML-overlay context-menu and toast patterns inlined per ADR-003 (no new ADR — component-internal until they proliferate); `project_removed` handler established as THE canonical listener for canvas-005b to reuse without duplication. `pnpm check` 0/0/0. Commit `8ff6a1e`. (2026-05-15)
- [canvas-006-live-add-race-on-concurrent-project-added](done/canvas-006-live-add-race-on-concurrent-project-added.md) — `bug` — Serialised the `project_added` → `addLiveProject` path through a single promise chain (`liveAddChain` + `enqueueLiveAdd`); N back-to-back arrivals now process strictly sequentially so each tile lands at a distinct spiral slot in memory and in `tile_positions`. Post-await `findProject` re-check kept as defence in depth. Mount path + `project_missing` branch untouched. `pnpm check` 0/0/0. Commit `f83222c`. (2026-05-15)
- [canvas-002-render-multiple-project-tiles](done/canvas-002-render-multiple-project-tiles.md) — `feature` — `Canvas.svelte` restructured from single-tile → keyed-by-`project_id` collection. New pure `tile-layout.ts` (square-spiral auto-placement, no Svelte/Pixi imports). Shared window-level drag controller replaces N per-tile listeners. Per-id event routing; `project_added` idempotent live-add; `sceneWorldBounds` unions every tile. `pnpm check` 0/0/0. Commit `e690fd3`. (2026-05-15)
- [canvas-001-targeted-canvas-updates](done/canvas-001-targeted-canvas-updates.md) — `type: feature`. Canvas patches its `ProjectSnapshot` in place from the fine-grained FS events (`task_moved`/`added`/`removed`, `bc_appeared`/`disappeared`) instead of re-fetching; coarse `AgentheimChanged` retired → renamed `ResyncRequired` (lag-only resync signal). ADR-009 amended in place. Commit `5fa7080`.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
