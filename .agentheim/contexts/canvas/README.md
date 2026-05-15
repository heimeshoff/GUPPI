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
- **Tile** — visual representation of a project (large node).
- **Node** — visual representation of a bounded context (small node, child of a project tile).
- **Connection** — the line between a tile and its BC nodes.
- **Viewport** — the currently visible window onto the canvas (pan position + zoom level).
- **Focus** — a "zoom to" operation that frames a specific tile or node.
- **Layout** — positions of tiles/nodes on the canvas (persisted in GUPPI's own state directory, not in the target project's `.agentheim/`).
- **Status badge** — the per-BC visual indicator (running / idle / blocked-on-question dot), driven by `agent-awareness`.
- **Detail view** — the project-detail pane that renders markdown documents from a project.
- **Markdown pane** — the renderer for `vision.md`, `research/*.md`, ADRs, BC READMEs in the detail view.
- **Targeted update** — patching the client-side `ProjectSnapshot` in place from a fine-grained filesystem event (`task_moved` / `task_added` / `task_removed` / `bc_appeared` / `bc_disappeared`) instead of re-fetching the whole snapshot. A tile's task counts tick, a BC node appears/disappears, without a `get_project` round-trip (`canvas-001`).
- **Resync** — the one remaining full `get_project` re-fetch, triggered only by the `resync_required` domain event. The Rust core's event bridge emits it when its broadcast receiver lags and loses events it cannot reconstruct (ADR-009 lag-resync strategy).
- **Context menu** — a screen-space HTML overlay (ADR-003 overlay layer) opened by right-click. The empty-canvas menu and tile menu share one items-array shape (`{ label, onClick, hidden? }`) so future menu items append cleanly. canvas-005a contributes "Add project…" (empty canvas) and "Remove project" (tile); canvas-005b appends "Scan folder for projects…" (always shown) and "Manage scan roots…" (hidden when `scanRootsCount === 0`, the cached count of registered scan roots — refreshed on mount, after `addScanRoot`, after `removeScanRoot`). Dismissed on item click, pointer-down outside, `Escape`, or any pan/zoom gesture (a window-level capture-phase pointerdown listener owns outside-click dismissal). Viewport-clamped after first paint.
- **Modal** — a centered HTML-overlay panel (ADR-003 overlay layer) above a dimmed backdrop (`--guppi-canvas-bg` @ 70%). Generic primitive at `src/lib/Modal.svelte`: header / body / footer snippet slots, `Escape` + backdrop-click dismissal. Three consumers shipped in canvas-005b — the **discovery checklist**, the **scan-roots management** surface, and the **cascade-remove confirmation**. Modals are mutually exclusive — EXCEPT the cascade-confirm dialog, which stacks ON TOP of the management modal (the lower modal stays mounted behind it; clicks on its surface are blocked by the upper modal's backdrop). A fourth consumer is the threshold for codifying buttons/modal as a STYLEGUIDE.md component entry.
- **Discovery checklist** — the modal that opens after `addScanRoot` or `rescanScanRoot` resolves. Lists every `ScanCandidate` from the walker as a row with a checkbox, mono-path, and nickname suggestion. `already_imported: true` rows render at 60% opacity with the checkbox pre-checked **and disabled** and a small `statusIdle` "imported" pill after the path — they cannot be unticked and are filtered out of the eventual `importScannedProjects` request. "Select all" / "Select none" header controls operate on togglable rows only. "Import selected" is disabled until at least one new (not-already-imported) candidate is ticked. The empty-candidates case (zero rows from `addScanRoot`) still opens the modal, with the "no Agentheim projects found" empty state and a single "OK" button — the scan root is persisted by the backend BEFORE the walk runs, so an empty subtree still leaves a rescannable root behind (ADR-013).
- **Scan-roots management** — the modal that lists every row from `listScanRoots()` with its per-row child-project count, plus a Rescan button (re-runs the walk + opens the checklist) and a Remove button (opens the cascade-remove confirmation). The per-row count comes from a thin `list_projects_by_scan_root` IPC wrapper (`canvas-005b`) over the existing `Db::list_projects_by_scan_root`; the frontend takes `.length` of the returned `Vec<i64>` because we never need the ids themselves at v1 — only the count. The empty state never renders because the menu item is hidden when zero roots exist.
- **Cascade-remove confirmation** — the small two-button dialog opened from the management modal's "Remove" button. Names the scan-root path AND the child-project count and explicitly states that tile state will not be retained — ADR-013 makes the cascade hard-delete, NOT subject to ADR-005's 30-day window. Confirming invokes `removeScanRoot(scanRootId)`; the backend fires `ProjectRemoved` per child BEFORE tearing watchers down, and the canvas-005a `project_removed` handler drops the tiles (one event variant, one listener — canvas-005b does NOT re-subscribe). After the cascade resolves, the management modal refreshes via `listScanRoots()` + per-row counts; if zero roots remain, it closes and the "Manage scan roots…" menu item hides on the next right-click.
- **Error toast** — a screen-space HTML overlay pinned top-center, `statusMissing` border (refusal, not failure). Auto-dismisses after 3000ms; one toast at a time. canvas-005a uses it for the `register_project` rejection path ("not an Agentheim project"), which is the exact IPC contract string and must surface verbatim.
- **Missing tile** — the canvas visual for a registered-but-unwatched project (`ProjectSnapshot.missing: true`). Tile body at 50% opacity, border swapped from `tileBorder` to `statusMissing`, `✕` glyph at `spacing.lg` in the top-right corner. `bcs: []` on the snapshot keeps the orbit empty; the tile is NOT filtered out of the per-project collection (the missing visual is the affordance). Right-click still offers "Remove project".

## How the canvas stays live

The canvas does not poll. The Rust core watches each project's `.agentheim/`
and emits fine-grained domain events; the frontend applies them to its
in-memory model as **targeted updates** (see `src/lib/snapshot-patch.ts`).
Robustness rules baked into the patching: a `task_*` event for a BC not yet in
the model lazily creates a zero-count node (filesystem events can arrive before
the `bc_appeared` for the same batch); a delta that would push a count below
zero is clamped at 0 and logged (the client model has drifted from disk); and
every event is routed by `project_id` to the matching tile in the canvas's
per-project collection (events for a `project_id` not in the collection are
ignored). A full re-fetch of a single project happens only on **resync**
(`resync_required { project_id }`).

## Rendering N projects

The canvas holds a keyed collection of project entries (`{ id, snapshot, pos }`
per project, keyed off `ProjectSnapshot.id`); `Canvas.svelte`'s `renderScene`
iterates and draws one tile + its orbiting BC nodes + edges per entry. Per-tile
state — saved position, drag target, fine-grained event routing — is all keyed
by id; no single-valued `projectId` scalar exists. A **shared drag controller**
owns the one set of `window` `pointermove`/`pointerup` listeners; tiles claim
the active drag via their own `pointerdown`. **Auto-placement** for projects
with no saved position is a deterministic outward spiral from world origin
(`src/lib/tile-layout.ts` — a pure, no-Svelte/Pixi module, the verification
surface alongside `snapshot-patch.ts`); each auto-placed position is persisted
immediately, so a never-dragged tile lands in the same spot across restarts.
A `project_added` for a new id triggers `get_project` + auto-place + persist +
render with no manual refresh; a `project_added` for an already-rendered id
(the startup seed double-add) is a no-op. Live-adds are **serialised through a
single promise chain** (`canvas-006`) so a burst of N `project_added` events —
the normal shape of `import_scanned_projects` announcing N picks back-to-back
on the event bus — processes strictly sequentially. Without that chain, the
concurrent closures all read the same `projects.length` for the spiral-index
step and the `projects = [...projects, entry]` reassignment loses every loser
to last-write-wins; the colliding `saveTilePosition` rows also reach SQLite
before the array catches up. Treat "N concurrent arrivals" as the default test
stance for any future change to this handler, not the single-arrival case.
Zoom-to-fit (`f`) frames the union of every tile and its BCs.

## Discovery affordances

The canvas owns the user-facing affordances ADR-005 names. Single-shot
"Add project…" and "Remove project" (canvas-005a) live in two right-click
context menus — one on the empty canvas background, one on a tile.
"Add project…" opens a Tauri-native folder picker
(`@tauri-apps/plugin-dialog`), invokes `registerProject(path)`, and routes
the rejection string `"not an Agentheim project"` to an **error toast**;
the success path is silent and rides the existing `project_added` →
`enqueueLiveAdd` chain. "Remove project" invokes `removeProject(project_id)`
with **no confirmation step** — ADR-005's 30-day undo window (re-add
restores the tile via the preserved `tile_positions` row) is the safety
net. The frontend's `project_removed` handler is THE canonical listener —
canvas-005b reuses it for the scan-root cascade fan-out without
duplication (one event variant, two emitters in the project-registry).
A `ProjectSnapshot.missing` tile renders as a **missing tile** (above);
its right-click menu still surfaces "Remove project" so the user can
recover.

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
