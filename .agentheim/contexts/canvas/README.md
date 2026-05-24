---
name: canvas
classification: core
relationships:
  - to: project-registry
    type: customer-supplier
    direction: upstream
  - to: agent-awareness
    type: customer-supplier
    direction: upstream
  - to: infrastructure
    type: shared-kernel
---

# canvas

## Purpose

The Miro-like infinite surface that is GUPPI's primary view. Every Agentheim project appears as a tile, its bounded contexts as connected child nodes, with status badges per BC and task counts (backlog / doing / done). Supports pan, zoom, drag-to-reposition, click-or-keyboard focus-zoom, and a project-detail view that renders markdown (vision.md, research/*.md, ADRs, BC READMEs). The v1 MVP is canvas-only and read-only — when it lands, the "no overview" pain from the vision is gone.

This BC also owns the rendered-markdown detail pane (originally considered a separate `document-viewer` context; folded in because it has no distinct language or actor of its own).

## Classification

**Core.** GUPPI exists to provide this ambient overview surface. The canvas is one of GUPPI's two headline differentiators (the other being live agent-awareness).

## Frontend gate

This BC has a frontend. Every frontend task in this BC must `depends_on` the styleguide task `design-system-001-styleguide` in `contexts/design-system/`, and must be implemented against the styleguide itself: [`contexts/design-system/STYLEGUIDE.md`](../design-system/STYLEGUIDE.md) — the visual vocabulary (tokens, component states, motion budget) that keeps the canvas coherent. No UI work here is promoted to `doing/` before the styleguide is signed off.

The styleguide was signed off in person by Marco on 2026-05-14, so the gate is open — but the `depends_on` link and the "build against `STYLEGUIDE.md`" rule still apply to every frontend task.

## Ubiquitous language (seed)

- **Canvas** — the infinite surface itself.
- **Frame** — the surrounding visual region drawn around a project (`canvas-007`). Replaces the orbit-baseline single-bubble tile: project name, status badges, and task counts ride a header bar across the top edge. The header bar is the project's drag handle and right-click target. *Pivot (ADR-017 / ADR-019, canvas-020):* the frame is now **hybrid** — the **shell** (border + header + drag hit-area + missing glyph) is a PixiJS world-space object; the **interior** (the BC accordion + kanban) is a DOM overlay positioned at `worldToScreen(frame.pos)` + `transform: scale(z)`. The shell is a **fixed default size** (`frame-interior.frameSize`); the interior scrolls within it (the retired `bc-layout.ts` auto-fit-to-content is gone).
- **Frame interior** — the DOM-overlay half of the hybrid frame (ADR-017 / ADR-019, canvas-020). A vertical **BC accordion**; each row expands to a four-column **kanban board**. Mounted only for frames passing the ADR-017 cost governors (viewport culling `CULL_MARGIN_PX = 120` + zoom-floor LOD `LOD_ZOOM_FLOOR = 0.45`); off-screen / zoomed-out frames render the cheap Pixi shell only. The mounted set + per-frame `left/top/scale(z)` is the `mountedInteriors` `$derived`, recomputed on every camera change via the `cameraVersion` reactive-bridge counter (ADR-019). Pure cull/LOD/bucketing/`frameSize` helpers live in `src/lib/frame-interior.ts` (the tested verification surface).
- **BC accordion row** — a bounded context inside a frame interior, rendered as a **collapsible DOM row** (replacing the retired Pixi BC bubble). Collapsed = a 44px header band (chevron + BC name + the active/blocked/idling **roll-up** + total task count); expanded = the header plus the BC's kanban board. Rows default to expanded and to the registry's stable BC-name order; both the collapse flag and the drag-reordered position **persist** per `(project_id, bc_name)` via **BC view-state** (canvas-023, below). The header doubles as a **drag-to-reorder handle** (HTML5 drag-and-drop; a genuine drag suppresses the click-toggle). The roll-up reads `agent-awareness-002`'s `get_bc_agent_rollup` (one capsule per non-zero state, each with its colourblind-safe `statusGlyph`), re-fetched on `task_agent_state_changed` + task events.
- **Kanban board / column / task card** — inside an expanded accordion row: four fixed columns (BACKLOG · TODO · DOING · DONE) of the BC's `tasks[]` (`project-registry-005`) bucketed by `task.column`. The board scrolls horizontally past four columns; each column's card stack scrolls vertically. A **task card** renders id + 2-line-clamped title + tag chips (canvas-020 structure) plus, for an in-flight (DOING) task with a live agent, the **live-agent indicator line** ("orchestrator · waiting 2m 14s", canvas-021): the agent label + a locally-timed elapsed string from agent-awareness-002's `since` timestamp (running = brand-blue `▶` with the one sanctioned `durationPulse` breathe; blocked = static red `◆`). Card **states** (canvas-021, §3.11): default / hover / **selected** (blue accent border — the card whose detail panel is open) / **blocked** (red accent border + the blocked agent line). Clicking a card **selects** it and emits the "open detail panel for this task" signal (the `onTaskSelected` prop callback + the `selectedTask` rune — ADR-020); the docked panel itself is canvas-022.
- **Bubble** — *retired (ADR-017 / canvas-020).* The orbit-baseline / canvas-007 inside-frame BC bubble (a draggable Pixi rounded-rect with a counts pill + status badge) is removed from the interior. BCs are now **BC accordion rows** (above). The §3.7 vocabulary is preserved in the styleguide for a possible future zoomed-out per-BC density view, but no current code path draws a BC bubble.
- **Tile** — pre-`canvas-007` term for a project's visual (single bubble + orbiting BCs). Retired; "frame" replaces it. Some IPC names still carry the legacy term (`tile_positions`, `saveTilePosition`, `loadTilePosition`) — those persist a project's frame-origin position and are read as "frame position" semantically.
- **Intra-project edge** — *retired for the canvas interior (ADR-017 / canvas-020).* The BC↔BC relationship line between in-frame BC bubbles (four variants — directional `customer-supplier`/`conformist`, non-directional `shared-kernel`/`partnership`, notched `anticorruption-layer`) is no longer drawn, because there is no longer a free-positioned bubble graph for edges to connect (BCs are stacked accordion rows). The per-BC `relationships:` README-frontmatter **data model** (ADR-014, `project-registry-004`) survives; only its canvas renderer retired. The geometry vocabulary is preserved in styleguide §3.8 for a possible future cross-project relationship view.
- **Connection** — pre-`canvas-007` term for the line between a tile and its BC nodes (project→BC). Retired; containment (BC inside frame) replaces it. The line vocabulary now applies only to BC↔BC intra-project edges.
- **Viewport** — the currently visible window onto the canvas (pan position + zoom level).
- **Focus** — a "zoom to" operation that frames a specific project frame or BC bubble.
- **Layout** — positions of frames on the canvas (persisted in GUPPI's own state directory, not in the target project's `.agentheim/`). Frame positions ride `tile_positions`. *Pivot (canvas-020):* per-BC manual-drag positions (`bc_positions`) are no longer read by the canvas — BCs are accordion rows, not draggable bubbles. *Resolved (canvas-023 / ADR-021):* the dead `bc_positions` table + its `save_bc_position`/`load_bc_position(s)` IPC were **retired** (DROPped at schema v6) and replaced by **BC view-state** (`bc_view_state`, below).
- **BC view-state** — GUPPI's persisted memory of a BC accordion row's arrangement inside a frame: `collapsed` (folded shut or open) + `sort_order` (the user's drag-reordered position; NULL = unset → falls back to stable BC-name order). Stored in the `bc_view_state (project_id, bc_name, collapsed, sort_order)` SQLite table (schema v6, ADR-004/ADR-021), keyed per `(project_id, bc_name)` with `ON DELETE CASCADE` so it inherits the exact ADR-005 retention semantics the old `bc_positions` had (survives soft-delete, cascades on hard-delete). CRUD: `save_bc_view_state` / `load_bc_view_state` / `load_bc_view_states` IPC (mirrored as `saveBcViewState` / `loadBcViewState` / `loadBcViewStates` in `ipc.ts`), following the `project-registry-004` position pattern. The frame paint batch-loads view-state on mount (`primeBcViewState`); collapse-toggle and drag-reorder write through optimistically. Per ADR-009, these are plain IPC round-trips, not `DomainEvent`s (no second consumer).
- **BC layout** — *retired (ADR-017 / canvas-020, supersedes-in-part ADR-015).* The deterministic force-directed (`canvas-007`) `bc-layout.ts` that placed BC bubbles inside a frame is **deleted**: the interior is now a top-to-bottom DOM accordion, not a force-directed bubble cloud, so there is no spring-electrical positioning to compute. The per-BC `relationships:` README-frontmatter data model (ADR-014, `project-registry-004`) is NOT superseded and survives; only its layout consumer retired. The frame shell is a fixed default size (`frame-interior.frameSize`); the accordion + kanban scroll within it.
- **Status badge** — the per-BC visual indicator (running / idle / blocked-on-question dot), driven by `agent-awareness`.
- **Detail view** — the project-detail pane that renders markdown documents from a project.
- **Markdown pane** — the renderer for `vision.md`, `research/*.md`, ADRs, BC READMEs in the detail view.
- **Targeted update** — patching the client-side `ProjectSnapshot` in place from a fine-grained filesystem event instead of re-fetching the whole snapshot (`canvas-001`, `src/lib/snapshot-patch.ts`). Now at **task granularity** (`project-registry-005`): `task_added` adds a card record to a column, `task_moved` moves it, `task_removed` drops it, `task_changed` patches its metadata in place, `bc_appeared`/`bc_disappeared` add/remove an accordion row — all without a `get_project` round-trip. The deeply-reactive `$state` snapshot drives the DOM interior overlay, so a card add/move/remove re-renders the kanban with no imperative call. The separate **live agent state** (`task_agent_state_changed`, `agent-awareness-002`) is NOT on the snapshot — the canvas re-fetches the affected BC's `get_bc_agent_rollup` for the accordion-header slot AND patches the per-task `taskAgentStates` read model (keyed `(project_id, bc, task_id)`) straight from the event payload to drive the per-card live-agent indicator (canvas-021, ADR-020). The per-card elapsed string ("waiting 2m 14s") advances from a single 1 Hz local clock gated on live presence — agent-awareness supplies the `since` timestamp once, never a per-second event.
- **Resync** — the one remaining full `get_project` re-fetch, triggered only by the `resync_required` domain event. The Rust core's event bridge emits it when its broadcast receiver lags and loses events it cannot reconstruct (ADR-009 lag-resync strategy).
- **Context menu** — a screen-space HTML overlay (ADR-003 overlay layer) opened by right-click. The empty-canvas menu and tile menu share one items-array shape (`{ label, onClick, hidden? }`) so future menu items append cleanly. canvas-005a contributes "Add project…" (empty canvas) and "Remove project" (tile); canvas-005b appends "Scan folder for projects…" (always shown) and "Manage scan roots…" (hidden when `scanRootsCount === 0`, the cached count of registered scan roots — refreshed on mount, after `addScanRoot`, after `removeScanRoot`). Dismissed on item click, pointer-down outside, `Escape`, or any pan/zoom gesture (a window-level capture-phase pointerdown listener owns outside-click dismissal). Viewport-clamped after first paint.
- **Modal** — a centered HTML-overlay panel (ADR-003 overlay layer) above a dimmed backdrop (`--guppi-canvas-bg` @ 70%). Generic primitive at `src/lib/Modal.svelte`: header / body / footer snippet slots, `Escape` + backdrop-click dismissal. Three consumers shipped in canvas-005b — the **discovery checklist**, the **scan-roots management** surface, and the **cascade-remove confirmation**. Modals are mutually exclusive — EXCEPT the cascade-confirm dialog, which stacks ON TOP of the management modal (the lower modal stays mounted behind it; clicks on its surface are blocked by the upper modal's backdrop). A fourth consumer is the threshold for codifying buttons/modal as a STYLEGUIDE.md component entry.
- **Discovery checklist** — the modal that opens after `addScanRoot` or `rescanScanRoot` resolves. Lists every `ScanCandidate` from the walker as a row with a checkbox, mono-path, and nickname suggestion. `already_imported: true` rows render at 60% opacity with the checkbox pre-checked **and disabled** and a small `statusIdle` "imported" pill after the path — they cannot be unticked and are filtered out of the eventual `importScannedProjects` request. "Select all" / "Select none" header controls operate on togglable rows only. "Import selected" is disabled until at least one new (not-already-imported) candidate is ticked. The empty-candidates case (zero rows from `addScanRoot`) still opens the modal, with the "no Agentheim projects found" empty state and a single "OK" button — the scan root is persisted by the backend BEFORE the walk runs, so an empty subtree still leaves a rescannable root behind (ADR-013).
- **Scan-roots management** — the modal that lists every row from `listScanRoots()` with its per-row child-project count, plus a Rescan button (re-runs the walk + opens the checklist) and a Remove button (opens the cascade-remove confirmation). The per-row count comes from a thin `list_projects_by_scan_root` IPC wrapper (`canvas-005b`) over the existing `Db::list_projects_by_scan_root`; the frontend takes `.length` of the returned `Vec<i64>` because we never need the ids themselves at v1 — only the count. The empty state never renders because the menu item is hidden when zero roots exist.
- **Cascade-remove confirmation** — the small two-button dialog opened from the management modal's "Remove" button. Names the scan-root path AND the child-project count and explicitly states that tile state will not be retained — ADR-013 makes the cascade hard-delete, NOT subject to ADR-005's 30-day window. Confirming invokes `removeScanRoot(scanRootId)`; the backend fires `ProjectRemoved` per child BEFORE tearing watchers down, and the canvas-005a `project_removed` handler drops the tiles (one event variant, one listener — canvas-005b does NOT re-subscribe). After the cascade resolves, the management modal refreshes via `listScanRoots()` + per-row counts; if zero roots remain, it closes and the "Manage scan roots…" menu item hides on the next right-click.
- **Error toast** — a screen-space HTML overlay pinned top-center, `statusMissing` border (refusal, not failure). Auto-dismisses after 3000ms; one toast at a time. canvas-005a uses it for the `register_project` rejection path ("not an Agentheim project"), which is the exact IPC contract string and must surface verbatim.
- **Missing tile** — the canvas visual for a registered-but-unwatched project (`ProjectSnapshot.missing: true`). Frame body at 50% opacity, border swapped from `frameBorder` to `statusMissing`, `✕` glyph at `spacing.lg` in the top-right corner of the header bar. `bcs: []` on the snapshot keeps the frame empty (the empty-frame placeholder is suppressed in this state); the frame is NOT filtered out of the per-project collection (the missing visual is the affordance). Right-click on the header bar still offers "Remove project".
- **Title-fits-frame invariant** (canvas-013, revised 2026-05-19) — project frame titles and BC bubble titles **scale with camera zoom** (same relative size to their container) and **end-truncate with an ellipsis** when the rendered text width would exceed the available container width. Both kinds of title are Pixi `Text`. Truncation budget: project title is `frameWidth - 2*framePadding - countsPillWidth - gap`; BC title is `pillX - bcTitleX - gap`. Implemented via a single `truncateTextToWidth(t, fullText, maxWidth)` helper at script scope in `Canvas.svelte` (binary search for the largest prefix that fits with `…` appended). Floor: under canvas-015 (persistent scene graph + camera-as-stage-transform) the floor is enforced via a per-Text counter-scale `Math.max(1, 8 / (fontSize * z))` applied to four sites (project title, project missing-tile glyph, BC title, BC counts pill text) before truncation — the on-screen result is the same as the pre-canvas-015 `Math.max(8, fontSize * z)` fontSize-floor, with the on-screen text never dropping below 8 CSS-px. See `screenSpaceTitleScale` in `Canvas.svelte` and ADR-016 §4 (zoom-out floor). *Historical note:* the first draft of canvas-013 (commit `3315a10`) used an HTML-overlay layer with `text-overflow: ellipsis` to keep project titles at a constant screen-space size (Miro-style). Reverted same day after hands-on verification — the visual rhyme between a frame's title and its child BC titles is load-bearing.
- **Crispness invariant** (canvas-013) — the canvas-wide policy that every rendered element (frame borders, BC bubble borders, intra-project edges incl. arrowheads + ACL notches, project frame title text, BC bubble title text, BC task-count text, status badges, missing-tile glyph) renders at display resolution at every supported zoom. Two load-bearing pieces hold the invariant: (1) PixiJS booted with `app.init({ resolution: window.devicePixelRatio, autoDensity: true, … })` so Text and Graphics rasterise at the framebuffer's real resolution rather than the previous default 1× upscale; (2) stroke widths for the "borders" category (frame border, BC bubble border, header divider, focus ring, and all four intra-project edge variants incl. arrowheads + ACL notch geometry) are **constant screen-space CSS pixels** — the previous `Math.max(1, shape.borderWidth* * z)` pattern that produced sub-pixel hairlines at zoom-out and chunky strokes at zoom-in is retired. Focus-ring **inset** (positional offset) stays world-space; only the stroke width is constant-screen-space. Arrowhead `pullBack` (the tip-from-bubble distance) stays world-space because the bubble it pulls back from is `bcInsideWidth * z` in screen space.
- **Persistent scene graph** (canvas-015, ADR-016; trimmed by canvas-020/ADR-017) — every project's **frame-shell** display objects (frame container, body, header, title text, counts text, focus ring, missing glyph) are **instantiated once** when the project enters the scene (initial `refresh` or `project_added`) and held in a `frameObjects: Map<projectId, FrameDisplayObjects>` keyed by snapshot id. *Pivot:* the interior Pixi objects (intra-project edges Graphics, BC-bubble children, empty-state Text) are removed — the interior is a DOM overlay now; `FrameDisplayObjects` carries the shell only. Subsequent renders update geometry in place via `.clear()` + redraw on the same persistent `Graphics` instances; Text content/style updates via property writes; visibility flags toggle via `.visible`. No `world.removeChildren()`; no `new Graphics()` / `new Text()` per render. The pan path becomes a single `world.position.set()` call (zero allocation, zero clear-and-redraw); zoom adds `world.scale.set()` + a `repaint()` pass that rewrites stroke widths in place (still no allocation). Theme flips and topology changes go through the same in-place update pass. Hover focus rings toggle via `.visible` only; their geometry is laid down once and stays until the next zoom/topology update.
- **Camera as stage transform** (canvas-015, ADR-016) — the PixiJS `world` Container's `position` and `scale` carry the camera transform: `world.position = (camera.pan_x, camera.pan_y)`, `world.scale = camera.zoom`. Children are drawn in **world coordinates** (not the previous JS-pre-projected screen coordinates), and PixiJS's WebGL renderer applies the parent transform once per frame on the GPU — every child moves and scales for free under pan and zoom. `camera.worldToScreen()` remains the contract for screen-space overlays (modals, context menus, voice indicator, forthcoming agent-awareness badges); the **renderer's internal draw path** no longer uses it. Stroke widths in world space are pre-divided by `z` (`shape.borderWidthFrame / z`) so the parent's `world.scale = z` multiplication restores the constant CSS-pixel value on screen — Crispness invariant preserved (#4). Arrowhead and ACL-notch tip sizes follow the same pre-divide policy; `pullBack` distance stays in world space because the BC bubble it pulls back from is itself drawn in world space.

## How the canvas stays live

The canvas does not poll. The Rust core watches each project's `.agentheim/`
and emits fine-grained domain events; the frontend applies them to its
in-memory model as **targeted updates** (see `src/lib/snapshot-patch.ts`).
Robustness rules baked into the patching: a `task_*` event for a BC not yet in
the model lazily creates a zero-count BC (filesystem events can arrive before
the `bc_appeared` for the same batch, and the lazy-create initialises
`relationships: []` so the new BC participates in the next layout pass
edge-less until its README's frontmatter is read); a delta that would push a
count below zero is clamped at 0 and logged (the client model has drifted
from disk); and every event is routed by `project_id` to the matching frame
in the canvas's per-project collection (events for a `project_id` not in the
collection are ignored).

`bc_relationships_changed { project_id, bc_name }` (`project-registry-004`'s
fine-grained README frontmatter event) is the one filesystem-observation
event that the pure patcher cannot fully apply — the event payload is a
scoped "go refresh" signal, not the new relationship set itself. The canvas
handles it as a per-project `refreshOne(project_id)` against the fresh
`BoundedContext[]` (canvas-020: no BC layout recompute — the interior is a DOM
accordion that re-derives from the snapshot, and each BC's roll-up is re-fetched).
This keeps the `canvas-001` targeted-update invariant intact (no full
`list_projects` re-fetch, only the one affected project re-pulls). A full
re-fetch of a single project happens only on **resync**
(`resync_required { project_id }`).

## Rendering N projects

The canvas holds a keyed collection of project entries
(`{ id, snapshot, pos, size }` per project, keyed off `ProjectSnapshot.id`;
canvas-020 dropped `bcLayout`/`bcPositions` and added the fixed `size`);
`Canvas.svelte`'s `renderScene` iterates and reconciles one **persistent**
per-project **frame-shell** display-object struct per entry (frame body, header,
title, counts, focus ring, missing glyph) — see the **persistent scene graph**
vocabulary entry and ADR-016. The kanban-accordion interior is a separate DOM
overlay (the **Frame interior** entry / ADR-019), not part of `frameObjects`.
The Pixi world `Container` itself carries
the camera transform: `world.position` is the pan, `world.scale` is the zoom,
so children draw in world coordinates and the WebGL renderer composites the
camera once per frame on the GPU. `renderScene` is **event-driven**
(`canvas-014`): every interactive path (pan, wheel, drag, hover) and every
domain event (`task_*`, `bc_*`, `project_added`, `project_removed`,
`bc_relationships_changed`, `resync_required`, theme flip) drives the
appropriate sub-path explicitly. Pan calls only `world.position.set()` (no
reconcile, no clear-and-redraw). Wheel calls `world.position.set()` +
`world.scale.set()` + a `repaint()` pass (rewrites zoom-dependent stroke
widths in place — no allocation). Frame-drag / BC-drag write directly into
the one affected container's `position`. Topology changes and theme flips
go through `renderScene()` (or `repaint()`), which reconciles the
`frameObjects` map against the live `projects[]` array and runs in-place
update on every entry — instantiating new display objects only on
`project_added` and destroying them only on `project_removed`. The PixiJS
ticker is dormant in steady state and only stepped while an eased camera
transition (`f` zoom-to-fit) is in flight — the previous unconditional
per-tick rebuild was the dominant per-frame cost the
`canvas-perf-2026-05-17` spike retired. Window-resize re-projection is
served by an explicit `resize` listener. Per-frame state — saved frame position, persisted
per-BC drag positions, the deterministic BC layout output, drag target,
fine-grained event routing — is all keyed by id; no single-valued `projectId`
scalar exists. A **shared drag controller** owns the one set of `window`
`pointermove` / `pointerup` / `pointercancel` / `pointerleave` listeners; its
state machine — formerly four sibling variables spread across module scope
and a function-scope `panning` flag — is now a single discriminated-union
`DragState` in the pure `src/lib/drag-controller.ts` module (the canvas-012
extraction; same verification-surface stance as `tile-layout.ts` /
`bc-layout.ts` / `snapshot-patch.ts`, no Svelte / Pixi / IPC imports).
Two active drag kinds share the controller — frame drag (claimed by the frame
header bar's `pointerdown`, persisted via `saveTilePosition`) and camera pan
(claimed by a `pointerdown` on empty canvas, persisted via `saveCamera`). The
third kind, **BC drag**, is **retired** by canvas-020 (ADR-017): BCs are DOM
accordion rows, not draggable Pixi bubbles, so nothing starts a BC drag — the
controller's three-kind union is preserved (ADR-017) and the `bc` branches are
defensive no-ops. The frame body region is pass-through so empty regions inside
a frame don't swallow the camera pan, and the DOM interior overlay sets
`pointer-events: none` on its wrapper (re-enabled on the scrollable body) so the
header band + empty space still reach the Pixi pan/drag hit-areas beneath.
`pointercancel` and
`pointerleave` are unconditional terminal events that land state in `idle`
without persisting — they exist for the cases where `pointerup` never
arrives (touch interruption, OS-level pointer hijack, the cursor leaves
the window mid-drag). Right-click (`button === 2`) is a controller no-op:
the call site handles menu open and the in-flight drag, if any, keeps its
state until its own `pointerup` / cancel / leave terminates it.

**Auto-placement** for projects with no saved frame position is a
deterministic outward spiral from world origin (`src/lib/tile-layout.ts` —
a pure, no-Svelte/Pixi module, the verification surface alongside
`snapshot-patch.ts`); each auto-placed position is persisted immediately, so a
never-dragged frame lands in the same spot across restarts. The frame **shell**
is a fixed default size (`src/lib/frame-interior.frameSize` — pure, alongside
`snapshot-patch.ts` / `tile-layout.ts`); the kanban-accordion DOM interior
scrolls within it. The retired `bc-layout.ts` force-directed BC placement is
gone (ADR-017 / canvas-020).

A `project_added` for a new id triggers `get_project` + auto-place + persist
+ fixed-size shell + render + lazy per-BC roll-up fetch with no manual refresh;
a `project_added` for an already-rendered id (the startup seed double-add) is
a no-op. Live-adds are **serialised through a single promise chain**
(`canvas-006`) so a burst of N `project_added` events — the normal shape of
`import_scanned_projects` announcing N picks back-to-back on the event bus —
processes strictly sequentially. Without that chain, the concurrent closures
all read the same `projects.length` for the spiral-index step and the
`projects = [...projects, entry]` reassignment loses every loser to
last-write-wins; the colliding `saveTilePosition` rows also reach SQLite
before the array catches up. Treat "N concurrent arrivals" as the default
test stance for any future change to this handler, not the single-arrival
case. Zoom-to-fit (`f`) frames the union of every project frame in the
collection (each frame auto-fits to its interior BCs, so the union already
covers them).

## Discovery affordances

The canvas owns the user-facing affordances ADR-005 names. Single-shot
"Add project…" and "Remove project" (canvas-005a) live in two right-click
context menus — one on the empty canvas background, one on the frame
header bar (`canvas-007` retired the tile body as the right-click target;
the header bar replaces it). "Add project…" opens a Tauri-native folder
picker (`@tauri-apps/plugin-dialog`), invokes `registerProject(path)`, and
routes the rejection string `"not an Agentheim project"` to an **error
toast**; the success path is silent and rides the existing
`project_added` → `enqueueLiveAdd` chain. "Remove project" invokes
`removeProject(project_id)` with **no confirmation step** — ADR-005's
30-day undo window (re-add restores the frame via the preserved
`tile_positions` row) is the safety net. The frontend's `project_removed`
handler is THE canonical listener — canvas-005b reuses it for the scan-
root cascade fan-out without duplication (one event variant, two emitters
in the project-registry). A `ProjectSnapshot.missing` frame renders as a
**missing tile** (above); its right-click menu still surfaces "Remove
project" so the user can recover. The frame BODY is pass-through (not a
right-click target) so the empty canvas's right-click menu still opens
when the user clicks an empty region inside a frame.

The **scan-folder flow** and the **scan-root management surface**
(canvas-005b) ride the same right-click empty-canvas menu. "Scan folder
for projects…" opens the same folder picker → `addScanRoot(path)` →
**discovery checklist modal** with the returned `ScanCandidate` rows
(checkbox-per-row; already-imported rows pre-ticked-and-disabled with an
"imported" pill; "Select all" / "Select none" header controls on togglable
rows only; "Import selected" disabled until a new candidate is ticked).
The user's picks (with `already_imported` rows filtered out) feed
`importScannedProjects(scan_root_id, paths)`, which fires N back-to-back
`ProjectAdded` events — and canvas-006's serialised live-add chain is the
load-bearing piece that turns those N events into N distinct spiral
slots (without it, N-1 of the imports collide and silently drop). "Manage
scan roots…" opens the **scan-roots management modal** listing every
registered root with its live child-project count (via a thin
`list_projects_by_scan_root` IPC wrapper that returns `Vec<i64>` — the
frontend's `.length`), a Rescan button (re-walks the root → reopens the
checklist with the rescan flag set in the header), and a Remove button
(opens the cascade-remove confirmation, which stacks ON TOP of the
management modal — the explicit exception to one-modal-at-a-time). The
confirmation names the path AND the child count and warns that tile state
will not be retained (ADR-013 cascade hard-deletes, NOT subject to
ADR-005's 30-day window). On confirm: `removeScanRoot` fires N
`ProjectRemoved` events through the canvas-005a handler; the management
modal refreshes, and if zero roots remain it closes and "Manage scan
roots…" hides on the next right-click.

## Upstream dependencies

- `project-registry` — supplies the list of projects, their BCs, and task counts (customer-supplier; canvas is downstream).
- `agent-awareness` — supplies tile state and question-at-BC-location overlays (customer-supplier; canvas is downstream).
- `claude-runner` — supplies the orchestrator/sub-agent streams that the terminal panel inside the detail view renders (canvas owns the rendering component, runner owns the stream).
- `infrastructure` — canvas state persistence (tile positions, zoom, clusters) lives in GUPPI's own state directory via the infrastructure-provided persistence API.

## Open questions

- Terminal panel ownership boundary with `claude-runner` (rendering here, stream from there — confirm during walking-skeleton).
- Layout persistence format and location (foundation pass).
