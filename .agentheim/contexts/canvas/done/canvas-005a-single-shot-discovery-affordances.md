---
id: canvas-005a-single-shot-discovery-affordances
type: feature
status: done
completed: 2026-05-15
scope: bc
depends_on:
  - project-registry-003-manual-add-remove-and-missing-projects
  - design-system-001-styleguide
related_adrs:
  - ADR-003
  - ADR-005
related_research: []
prior_art:
  - canvas-002-render-multiple-project-tiles
  - canvas-001-targeted-canvas-updates
---

# Single-shot discovery affordances — Add / Remove / Missing tile

## Why

ADR-005 names three user-facing discovery affordances that live in the canvas
BC: **"Add project…"**, **"Remove project"** (single, with 30-day undo), and
the **"missing" tile state** for registered-but-unwatched projects. With
`project-registry-003` shipping the backing IPC (`register_project` /
`remove_project` with soft-delete, the `missing: bool` snapshot extension, and
the new `project_removed` event), this task lands their UI. `canvas-005b`
separately ships the heavier scan-folder/scan-root surface.

This task also introduces the **right-click context-menu pattern** that
canvas-005b extends. The shell + styling contract are owned here.

## What

### Right-click empty canvas → context menu

A new menu pattern: pointer-down on the canvas background (not on any tile)
with the right mouse button opens a contextual menu at click coordinates.

This task contributes **one** item to that menu: **"Add project…"**.
canvas-005b will append "Scan folder for projects…" and "Manage scan roots…"
to the same shell. Design the menu component so additional items append
cleanly (an array of `{ label, onClick, hidden? }` entries is fine).

Selecting "Add project…":
1. Open a Tauri-native folder picker via `@tauri-apps/plugin-dialog`'s
   `open({ directory: true })`. If the plugin is not already wired in
   (Cargo.toml + Tauri capabilities), wire it.
2. On a chosen path, invoke `registerProject(path)`.
3. **On success** (`Ok(project_id)`): the backend fires `ProjectAdded` via
   `WatcherSupervisor::add`; the existing live-add path in `Canvas.svelte`
   renders the tile.
4. **On rejection** (`"not an Agentheim project"`): show an error toast (see
   "Error toast" below). No state change to the registry.
5. **On user-cancelled picker:** silent close, no error.

### Right-click project tile → context menu

A second context menu for tiles, contributing one item this task:
**"Remove project"**.

Selecting "Remove project":
1. Invoke `removeProject(project_id)` immediately. **No confirmation step.**
   ADR-005's 30-day undo window (re-add restores the tile in place via the
   preserved `tile_positions` row) is the safety net.
2. Backend soft-deletes the row, tears down the watcher, fires
   `ProjectRemoved { project_id }`.
3. The frontend subscribes to `project_removed` in `onDomainEvent` and drops
   the entry from `projects`, removes its tile-position cache entry, and
   removes its position from the in-memory layout. The DB cascade handles
   the row.

This `project_removed` handler is the single canonical listener — canvas-005b
relies on the same handler for the cascade-remove case (one event variant,
fired by both code paths in project-registry-003). canvas-005b must NOT
duplicate this wiring.

### "Missing" tile rendering

For any `ProjectSnapshot` where `missing: true`:
- Tile body at **50% opacity**.
- Border swapped from `tileBorder` (`#8a8ad0`) to `statusMissing` (`#d05a8a`).
- A `✕` glyph in the top-right corner of the tile body, `statusMissing`
  colour, size `spacing.lg` (16px world-space).
- No BC nodes are rendered (the snapshot's `bcs: []` for missing tiles
  handles this naturally — but a missing tile must NOT be filtered out of
  the per-project collection; project-registry-003's frontend-tolerance
  acceptance criterion sets this up).
- Right-click on a missing tile still shows "Remove project"; selecting it
  soft-deletes as usual.

### Context-menu styling contract

A new pattern in this codebase. Inlined here pending a possible follow-up
design-system entry.

- **Layer:** screen-space HTML overlay positioned absolutely at click
  coordinates (ADR-003's overlay-layer convention; same idea as the existing
  voice-state indicator at viewport bottom-right). Stays anchored when the
  canvas pans/zooms underneath — but dismisses on pan/zoom (see below).
- **Background:** `var(--guppi-tile-fill)` (`#262636`).
- **Border:** 1px `var(--guppi-tile-border)` (`#8a8ad0`), radius
  `var(--guppi-radius-tile)` (12px).
- **Text:** `var(--guppi-tile-text)` on default, `var(--guppi-font-family)`
  stack, size `var(--guppi-size-body)` (12px).
- **Item padding:** `var(--guppi-spacing-sm)` (8px) vertical,
  `var(--guppi-spacing-md)` (12px) horizontal.
- **Hover item:** background `var(--guppi-canvas-bg-raised)` (`#1e1e26`).
- **Dismissal:** any item click; pointer-down anywhere outside the menu DOM
  node; `Escape` keypress; any pan/zoom gesture (wheel, drag on canvas,
  middle-button pan).
- **Viewport clamping:** if the click is near a viewport edge, flip/shift the
  menu so it stays fully visible (the simplest viable: clamp top/left so
  `top + height <= viewport.height` and `left + width <= viewport.width`).

### Error toast

A second new screen-space-overlay pattern, for the "not an Agentheim project"
rejection. Inlined contract:
- **Position:** top-center of the viewport, pinned in screen-space.
- **Background:** `var(--guppi-tile-fill)`, border 1px `statusMissing`
  (`#d05a8a`) — refusal, not failure.
- **Text:** `var(--guppi-tile-text)`, `sizeBody`, padding
  `spacing.md`/`spacing.lg`.
- **Behaviour:** appears immediately on rejection, auto-dismisses after
  **3000ms**. Click-to-dismiss optional but not required.
- One toast at a time; a new toast replaces the current one rather than
  stacking.

## Scope (in)

- `src/lib/Canvas.svelte`:
  - Pointer-down handlers for right-button on the empty canvas background and
    on tiles.
  - Render a screen-space context-menu overlay with the styling contract
    above; an items-array shape so canvas-005b can append two more entries.
  - Wire "Add project…" → folder picker → `registerProject` → toast on
    rejection.
  - Wire "Remove project" → `removeProject(project_id)`.
  - Subscribe to `project_removed` in `onDomainEvent`; drop tile + position
    cache + layout entry.
  - Render `missing: true` tiles dim + magenta-bordered + `✕` glyph.
- `src/lib/ipc.ts`: add `registerProject(path)` and `removeProject(projectId)`
  wrappers. (project-registry-003 may have already added these — if so, no
  duplication.)
- Tauri config: ensure `@tauri-apps/plugin-dialog` is enabled (Cargo.toml +
  capability JSON) for the folder picker. If missing, add it.

## Scope (out)

- "Scan folder for projects…" and "Manage scan roots…" entries — canvas-005b
  appends them to the menu shell this task ships.
- Cascade-related handling: the `project_removed` handler this task wires up
  already covers canvas-005b's cascade fan-out (one event variant, two
  emitters in project-registry-003). canvas-005b must not duplicate the
  listener.
- Frontend test runner. Same `pnpm check`-only verification surface as
  canvas-001 / canvas-002 / canvas-006.
- Rust-side changes (project-registry-003).
- A `STYLEGUIDE.md` entry for the context-menu / toast patterns — a possible
  follow-up design-system task; not in scope.

## Acceptance criteria

- [ ] Right-click on the empty canvas background opens a context menu at the
      click coordinates with exactly the items canvas-005a contributes ("Add
      project…"). The menu stays within the viewport (clamps near edges).
      Menu dismisses on item click, pointer-down outside, `Escape`, and any
      pan/zoom gesture. Verified by inspection + a manual exercise.
- [ ] Selecting "Add project…" opens a native folder picker. Picking a folder
      that contains `.agentheim/` results in a new tile appearing within a
      reasonable latency (the existing live-add path is reused). Picking a
      folder without `.agentheim/` shows an error toast displaying
      `"not an Agentheim project"` that auto-dismisses after ~3s. Cancelling
      the picker is silent.
- [ ] Right-click on a project tile opens a context menu with "Remove
      project". Selecting it: the tile and its BC-node group disappear (via
      `project_removed` handling); the `tile_positions` row in SQLite is
      **preserved** (verifiable by inspecting `%APPDATA%\guppi\guppi.db` —
      the row stays, only `projects.deleted_at` is set by the backend).
- [ ] Right-clicking on the empty canvas while a tile context menu is open
      swaps to the empty-canvas menu (one menu at a time, last click wins).
- [ ] A `ProjectSnapshot` with `missing: true` renders as a tile at 50%
      opacity, `statusMissing` magenta border (`#d05a8a`), and a `✕` corner
      glyph at `spacing.lg` size in `statusMissing` colour. No BC nodes are
      drawn around it.
- [ ] Right-click on a missing tile shows "Remove project"; selecting it
      removes the tile via the same path as the present case.
- [ ] All colours / sizes / typography pulled from `src/lib/design/tokens.ts`
      (Pixi-side) and `--guppi-*` CSS custom properties (overlay-side); no
      hard-coded values introduced in this task's diff.
- [ ] `pnpm check` is clean — `0 errors, 0 warnings`.

## Notes

- The right-click context menu and the error toast are **new patterns** in
  this codebase. The inlined contracts above are good enough for v1; if the
  patterns proliferate (canvas-005b's modals, terminal panel context menus
  later, etc.), a follow-up design-system task can codify them as
  `STYLEGUIDE.md` components.
- The folder-picker plugin: as of 2026-05-15 it has not been wired in (no
  consumer existed before this task). Worker step zero is to add it; if it
  turns out to already be present, that step is a no-op.
- **Re-adding a folder that maps to a soft-deleted registry row revives the
  tile at its old spot.** This is enforced by project-registry-003's backend
  acceptance (clears `deleted_at`, preserves `tile_positions`). This UI task
  does NOT need to test it explicitly — the tile arrives via the standard
  `ProjectAdded` live-add path, the position-load reads the preserved row.
- **No ADR.** The right-click context-menu pattern and missing-tile
  rendering are component-internal inside ADR-003 (PixiJS + HTML overlays)
  and the styleguide's existing token vocabulary. If the pattern proliferates,
  a styleguide entry is the right next step.

## Coordination

- `canvas-005b-scan-flow-and-scan-root-management` (also in `todo/`)
  appends its two menu items to the shell this task ships. The two tasks
  can be worked in either order — they share the menu data structure but
  not the items, and canvas-005b's hard-`depends_on` on canvas-006 means
  canvas-005a may well land first. The `project_removed` handler is shared,
  established here and reused by canvas-005b without duplication.

## Outcome

Shipped the three single-shot discovery affordances ADR-005 names — "Add
project…", "Remove project", and the missing-tile visual — and the two new
overlay patterns they introduce (context menu + error toast). The new patterns
are component-internal inside `Canvas.svelte` per the task scope; no ADR was
required (all decisions live inside ADR-003's overlay-layer convention and the
styleguide's existing token vocabulary).

Pieces:

- **`@tauri-apps/plugin-dialog` wired in** — `pnpm add` on the frontend,
  `tauri-plugin-dialog = "2"` in `src-tauri/Cargo.toml`,
  `.plugin(tauri_plugin_dialog::init())` in `src-tauri/src/lib.rs`'s builder,
  `"dialog:allow-open"` granted in `src-tauri/capabilities/default.json`. The
  picker now resolves end-to-end through the matching capability.
- **`registerProject` / `removeProject` IPC wrappers** added to
  `src/lib/ipc.ts`. The IPC contract string for rejection
  (`"not an Agentheim project"`) is documented inline; the canvas renders it
  verbatim.
- **Right-click context menu in `Canvas.svelte`** — an items-array shape
  (`{ label, onClick, hidden? }`) plus two openers (`openEmptyCanvasMenu`,
  `openTileMenu`). canvas-005b will append two more items to the empty-canvas
  menu without touching anything else. Dismisses on item click,
  pointer-down outside the menu DOM (window-level **capture-phase**
  listener, so "last click wins" falls out for free), `Escape` keypress,
  wheel zoom, and the existing left-button pan path's natural cleanup.
  Viewport-clamped via a post-paint measure (`$effect` → `queueMicrotask` →
  `getBoundingClientRect` → clamp `top`/`left`). HTML overlay styled
  entirely from `--guppi-*` CSS custom properties; no hard-coded values.
- **"Add project…" flow** — opens `openDialog({ directory: true })`,
  invokes `registerProject(path)`, routes the three terminal states:
  cancelled picker (silent), success (silent — the existing
  `project_added` → `enqueueLiveAdd` chain from canvas-002/006 renders the
  tile), rejection (toast). Any unexpected error is also toasted so a
  user-initiated affordance never fails silently.
- **"Remove project" flow** — no confirmation step; ADR-005's 30-day undo
  window is the safety net. Calls `removeProject(project_id)`; backend
  fires `ProjectRemoved`; the canvas drops the tile via the new
  `project_removed` handler.
- **`project_removed` handler in `onDomainEvent`** — THE canonical
  listener for both the single-remove and the cascade fan-out
  (`remove_scan_root` in project-registry-003 emits the same event
  variant). canvas-005b MUST NOT duplicate this — documented inline.
  Drops the entry from the `projects` array, clears a stale `hoveredKey`,
  re-runs `renderScene()`. The DB cascade / soft-delete is the backend's
  concern.
- **Missing-tile rendering** — for `ProjectSnapshot.missing: true`: tile
  body at 50% opacity (`tile.alpha = 0.5`), border swapped to
  `color.statusMissing`, `✕` glyph at `spacing.lg` world-space in the
  top-right corner (`makeMissingGlyph`). `bcs: []` from the backend
  keeps the orbit empty; the tile is NOT filtered out — the visual is
  the affordance. Right-click still surfaces "Remove project".
- **Error toast** — one toast at a time, replaces rather than stacks,
  auto-dismisses after 3000ms. Pinned top-center with `statusMissing`
  border to signal refusal (not failure).

Verification: `pnpm check` → `0 errors, 0 warnings, 0 files with problems`.
`cargo check --manifest-path src-tauri/Cargo.toml` clean. The fully wired
flows (folder picker, native dialog, end-to-end remove) need a `pnpm tauri
dev` hands-on exercise — the same posture as canvas-001 / canvas-002 /
canvas-006.

Files:
- `src/lib/Canvas.svelte` — context menu + toast overlays, missing-tile
  rendering, `project_removed` handler, right-click handlers on canvas
  and tiles, dispatched Add/Remove flows.
- `src/lib/ipc.ts` — `registerProject(path)` / `removeProject(projectId)`.
- `src-tauri/Cargo.toml` — `tauri-plugin-dialog = "2"`.
- `src-tauri/src/lib.rs` — `.plugin(tauri_plugin_dialog::init())`.
- `src-tauri/capabilities/default.json` — `"dialog:allow-open"`.
- `package.json` / `pnpm-lock.yaml` — `@tauri-apps/plugin-dialog ^2.7`.
- `.agentheim/contexts/canvas/README.md` — context menu / toast /
  missing tile entries in ubiquitous language; new "Discovery
  affordances" section.
