---
id: canvas-002-render-multiple-project-tiles
type: feature
status: done
completed: 2026-05-15
scope: bc
depends_on:
  - project-registry-001-multi-project-snapshot-model
  - design-system-001-styleguide
related_adrs:
  - ADR-003
  - ADR-004
related_research: []
prior_art:
  - canvas-001-targeted-canvas-updates
---

# Render multiple project tiles

## Why

The skeleton's `Canvas.svelte` renders exactly one tile: module-level
single-valued `$state` — `snapshot: ProjectSnapshot | null`, `tilePos: Point`,
`projectId: number | null` — and a `renderScene()` that draws that one tile with
its BC nodes radiating around it. The vision's v1 is the ambient overview of
**every** project at once — the single Miro-like surface that kills the "no
overview" pain. One tile is not an overview.

`project-registry-001` delivers the data layer: `list_projects() -> Vec<ProjectSnapshot>`,
`get_project(project_id)`, `WatcherSupervisor::add` (which publishes
`ProjectAdded { project_id, path }`), and `save_tile_position` /
`load_tile_position` reshaped to take `project_id`. This task makes the canvas
actually consume N projects — restructuring `Canvas.svelte` from single-valued
state to keyed collections without breaking `canvas-001`'s targeted patching or
ADR-003's camera model.

## What

Restructure `src/lib/Canvas.svelte` from one tile to one tile **per project**,
keyed by `project_id`.

**State restructure**
- `snapshot: ProjectSnapshot | null` → a keyed collection of snapshots by
  `project_id`. `tilePos: Point` → keyed positions by `project_id`. `projectId`
  (the "which project am I" learn-from-event scalar) is **removed** — every
  event now routes by its own `project_id` into the collection.
- Use a Svelte 5 `$state` `Map<number, …>` for both, or a `$state` array of
  `{ id, snapshot, pos }` records — pick one and justify in a code comment. Lean
  recommendation: a single `$state` array of records (`projects`), because
  `renderScene` already iterates and Svelte 5 array reactivity is well-trodden;
  a `Map` in `$state` reacts on reassignment/method calls but is easy to mutate
  unreactively by mistake. Whichever is chosen, **deep mutation of a snapshot's
  `bcs` must stay reactive** — `applyDomainEvent` mutates `bcs` in place and the
  ticker's `renderScene()` must pick it up next frame, exactly as today.

**`ProjectSnapshot.id`**
- `ProjectSnapshot` gains `id: number` in `src/lib/types.ts` (mirrors the Rust
  `i64` added by `project-registry-001` — see Coordination). The frontend keys
  everything off `snapshot.id`; no separate id-tracking scalar.

**Render N tiles**
- `renderScene()` loops over the project collection; for each project it draws
  the tile + BC nodes + edges exactly as the skeleton draws the single one.
- `bcWorldPosition(i, count)` gains a per-tile origin parameter — it currently
  radiates around the module-level `tilePos`; it must radiate around the
  argument tile's position instead.
- Node `key`s must be project-scoped so hover/focus rings don't collide across
  tiles: `project:${id}` and `bc:${id}:${name}` instead of today's `project` /
  `bc:${name}`.

**Per-project layout persistence**
- On mount: after `list_projects()`, for each project call
  `loadTilePosition(project_id)`. If a saved position exists, use it.
- If none exists, **auto-place** (below) and **persist immediately** via
  `saveTilePosition(project_id, pos)` — auto-placement runs once per project
  ever and is stable across restarts even if the tile is never dragged.
- Drag persists the dragged position via `saveTilePosition(project_id, pos)`.

**Auto-placement — spiral-out from world origin**
- A project with no saved position is placed by registration order on an
  outward spiral from world origin (compact, no overlap, cheap). Extract this as
  a pure function in its own module (e.g. `src/lib/tile-layout.ts`,
  `spiralPosition(index: number): Point`) so it is unit-testable without Svelte
  / Pixi — the project has no frontend test infra, so pure-module extraction is
  the verification strategy (same approach as `snapshot-patch.ts` in
  `canvas-001`).

**Shared drag controller**
- `makeDraggable` today registers `window`-level `pointermove`/`pointerup`
  listeners per tile — with N tiles that is N listener sets leaking and all
  firing on every move. Replace with a **single shared drag controller**:
  one set of `window` listeners, tracking which `project_id` is the active drag
  target (set on a tile's `pointerdown`, cleared on `pointerup`). On
  `pointerup`, persist that one project's position.

**Targeted updates per project (`canvas-001` preserved)**
- The `onDomainEvent` handler's fine-grained branch inverts: today it is
  "ignore if `event.project_id !== projectId`". Now it is "find the project in
  the collection by `event.project_id`; if found, `applyDomainEvent(that
  project's snapshot, event, warn)`; if not found, ignore (it is a project the
  canvas is not rendering — or the live-add race below)."
- `resync_required`: find the project by `event.project_id` and re-fetch just
  that one via `get_project(event.project_id)`, replacing its snapshot in the
  collection (keep its existing position).

**Live-add path**
- `project_added` carries only `{ project_id, path }` — no bcs/counts. On
  `project_added` for a `project_id` **not already in the collection**:
  `get_project(project_id)` to obtain the full snapshot, auto-place it (next
  spiral slot), persist the position, add it to the collection. `renderScene()`
  picks it up on the next ticker frame — no manual refresh.
- **Idempotency:** the startup seed project arrives BOTH via `list_projects()`
  on mount AND via a `project_added` event from `supervisor.add`. The add path
  must be a no-op if the `project_id` is already in the collection.

**Zoom-to-fit framing all tiles**
- `sceneWorldBounds()` currently bounds the single tile + its BCs. Extend it to
  union the bounding box of **every** tile and its BCs in the collection, so
  pressing `f` frames the whole overview.

## Scope (in)

- `src/lib/Canvas.svelte` — full single-tile → keyed-collection restructure.
- `src/lib/types.ts` — add `id: number` to `ProjectSnapshot`.
- `src/lib/ipc.ts` — `getProject` → `getProject(projectId)`, add
  `listProjects()`, `saveTilePosition(projectId, pos)` /
  `loadTilePosition(projectId)` reshaped to pass `project_id` (matching the
  `project-registry-001` command signatures).
- New `src/lib/tile-layout.ts` — pure spiral auto-placement, with the canvas
  consuming it.
- `bcWorldPosition`, `sceneWorldBounds`, `renderScene`, `makeDraggable` (→ shared
  controller), the `onDomainEvent` handler — all adapted to the keyed model.

## Scope (out)

- **Project removal / "missing" tile state — `canvas-005`.** `project_missing`
  stays a **no-op** in this task's `onDomainEvent` handler (confirmed: the
  handler keeps its `case 'project_missing': return;` branch unchanged). This
  task must not break when `project_missing` fires, but renders nothing
  different for it.
- **Empty state** (`list_projects()` returns zero projects) — out of scope. The
  `project-registry-001` hardcoded seed keeps the project set non-empty; a
  dedicated empty-state surface is later work. The canvas must not crash on an
  empty list, but no empty-state UI is built here.
- **Visual vocabulary** — turning the greybox into the real styleguide visuals
  is the sibling task `canvas-004`. This task reuses the skeleton's existing
  `makeNode` / token-driven rendering unchanged; it is purely structural.
- **`project-registry-002`** (filesystem scan + register) — not required; the
  live-add path is exercised by the seed's `project_added` event today and by
  scan-registered projects once `-002` lands.
- Rust-side changes beyond the `ProjectSnapshot.id` field — owned by
  `project-registry-001`.

## Acceptance criteria

- [ ] On mount, `listProjects()` is called once and **every** returned project
      renders as its own tile with BC nodes and connecting edges. With the
      seed-only registry that is one tile; the rendering path is genuinely
      N-keyed (verified by reading the code, not tile count).
- [ ] `ProjectSnapshot` has an `id: number` field in `types.ts`; the canvas keys
      all per-project state (`snapshot`, position, drag target, event routing)
      off it — no single-valued `projectId` scalar remains.
- [ ] Each tile's position persists and restores independently: a project with a
      saved position loads at it; a project with none is spiral-placed AND its
      position is written via `saveTilePosition(project_id, …)` on first
      placement (verified: restart shows the never-dragged tile in the same
      spot).
- [ ] Auto-placement is a pure function in `src/lib/tile-layout.ts` with no
      Svelte/Pixi imports; given indices `0..n` it returns non-overlapping
      world positions spiralling out from origin.
- [ ] Dragging tile A moves only tile A and persists only A's position; with ≥2
      tiles, exactly one shared set of `window` `pointermove`/`pointerup`
      listeners exists (verified by code review — not N sets).
- [ ] A fine-grained FS event (`task_*` / `bc_*`) updates **only** the matching
      project's tile: the handler finds the project by `event.project_id` and
      `applyDomainEvent`s its snapshot; an event for a `project_id` not in the
      collection is ignored without error.
- [ ] `resync_required { project_id }` re-fetches exactly that project via
      `getProject(project_id)` and replaces its snapshot, preserving its
      on-canvas position.
- [ ] A `project_added` event for a project not yet rendered triggers
      `getProject(project_id)` + auto-place + persist + render with no manual
      refresh; a `project_added` for an already-rendered `project_id` (the
      startup seed double-add) is a no-op.
- [ ] `project_missing` remains a no-op; firing it does not crash or alter the
      canvas.
- [ ] Pressing `f` frames all rendered tiles and their BCs in the viewport
      (`sceneWorldBounds()` unions every tile's box).
- [ ] `pnpm check` (svelte-check) passes clean — the project has no frontend
      test runner; svelte-check plus the extracted pure `tile-layout.ts` (and
      the untouched-and-still-passing `snapshot-patch.ts`) are the verification
      surface.

## Notes

Surfaced from the v1 "finish v1 first" capture pass (2026-05-14). Refined
2026-05-15 with Marco's decisions baked in.

**Decisions taken with Marco (do not re-litigate):**
1. `project_id` plumbing — `ProjectSnapshot` gains an `id` field (Rust
   `src-tauri/src/project.rs` + `src/lib/types.ts`) so id flows with the
   snapshot everywhere. Requires the coordination note below on
   `project-registry-001`.
2. Auto-placement is **spiral-out from world origin** by registration order.
3. Auto-placed positions **persist immediately** (`saveTilePosition` on first
   placement) — placement runs once per project ever, stable across restarts.
4. Zoom-to-fit (`f`) frames **all** tiles — `sceneWorldBounds` unions every box.

**Coordination — appended to `project-registry-001` (still in `todo/`, unworked):**
`project-registry-001`'s Notes carries:
> `canvas-002` requires `ProjectSnapshot` to carry an `id: i64` field (added in
> `src-tauri/src/project.rs`, mirrored in `src/lib/types.ts`). Both
> `list_projects()` and `get_project(project_id)` must populate it so the id
> flows with the snapshot to the frontend. (Decision with Marco, 2026-05-15.)

**No ADR.** This is implementation within ADR-003 (PixiJS/WebGL camera) and
ADR-004 (tile-position persistence). The state-shape choice (`Map` vs
array-of-records) and the shared-drag-controller pattern are component-internal
structure, not cross-cutting decisions — no future-maintainer context to
preserve. Recorded here so the next refiner does not re-open the question.

**Holds as one task.** The `Canvas.svelte` restructure converts `snapshot`,
`tilePos`, and `projectId` to keyed collections simultaneously, and
`renderScene` / `makeDraggable` / `bcWorldPosition` / `sceneWorldBounds` / the
`onDomainEvent` handler all close over them — there is no independently
shippable half-state. Same coupling logic that kept `canvas-001` and
`project-registry-001` whole.

**Frontend gate:** built against `contexts/design-system/STYLEGUIDE.md` (signed
off 2026-05-14). This task is structural — it reuses the skeleton's existing
token-driven `makeNode` rendering untouched; the greybox-to-real-visuals work is
`canvas-004`.

`prior_art`: `canvas-001` — the targeted-update patching (`snapshot-patch.ts`)
this task must keep working per-project, and the pure-module-extraction
verification strategy this task reuses for `tile-layout.ts`.

Open questions from the original capture — **resolved:**
- Auto-placement algorithm → spiral-out from origin (decision 2).
- Does the registry emit a live event or does the canvas re-poll → the registry
  emits `ProjectAdded` from `WatcherSupervisor::add`; the canvas reacts to it
  (confirmed in `project-registry-001` Notes → Coordination).

## Outcome

Completed 2026-05-15.

`Canvas.svelte` was restructured from a single-tile component to a per-project
collection rendered tile-by-tile, keyed by `ProjectSnapshot.id`. Every
per-project concern — saved position, drag target, fine-grained domain-event
routing — now keys off the id; no `projectId` scalar remains.

**Frontend changes:**
- `src/lib/tile-layout.ts` — new pure module. `spiralPosition(index)` returns a
  deterministic outward-spiral world-space slot from origin; `spiralPositions`
  yields the first `n`. No Svelte / Pixi imports — the verification surface,
  mirroring `snapshot-patch.ts` from canvas-001 (the project still has no
  frontend test runner).
- `src/lib/Canvas.svelte` — full rewrite of the script section:
  - Single `$state` array of `ProjectEntry = { id, snapshot, pos }` records
    (justified inline: array reactivity is well-trodden in Svelte 5; a `Map<…>`
    in `$state` reacts only on reassignment/method calls and is easy to mutate
    unreactively by mistake).
  - `renderScene()` loops over `projects`; each entry draws its tile + BC nodes
    + edges with `bcWorldPosition` now taking a per-tile `origin` parameter.
  - Node keys are project-scoped: `project:${id}` / `bc:${id}:${name}` — hover
    rings cannot collide across tiles.
  - One shared `window` `pointermove`/`pointerup` pair handles both camera-pan
    and active tile drag, with `dragProjectId` identifying which tile is being
    dragged. No per-tile listener leak. `pointerup` persists exactly the
    dragged project's new position via `saveTilePosition(id, pos)`.
  - On mount: `listProjects()` → `buildEntry(snapshot, spiralIndex)` per
    project; `buildEntry` restores the saved position if any, otherwise picks
    `spiralPosition(index)` and **persists immediately** so an undragged tile
    survives a restart in place.
  - `onDomainEvent` routes: `project_added` is no-op if already in the
    collection (startup seed double-add), otherwise `get_project` + auto-place
    + persist + append (idempotency double-checked after the await). Fine-
    grained `task_*`/`bc_*` events look up the entry by id and
    `applyDomainEvent` its snapshot; events for unknown ids are ignored.
    `resync_required` re-fetches that one project, preserving its `pos`.
    `project_missing` is an explicit no-op (canvas-005 territory).
  - `sceneWorldBounds()` unions every tile + every BC orbit; `f` frames the
    whole overview.
- `src/lib/types.ts` — unchanged in this task; the `ProjectSnapshot.id` field
  it already had (added by `project-registry-001`, commit `d594ad5`) was the
  hinge.
- `src/lib/ipc.ts` — unchanged in this task; the per-project
  `saveTilePosition(projectId, …)` / `loadTilePosition(projectId)` /
  `getProject(projectId)` / `listProjects()` shapes were already in place
  from `project-registry-001`.

**README updates:** `contexts/canvas/README.md` "How the canvas stays live"
paragraph reworded to describe per-project routing (was single-project filter
language). Added a new "Rendering N projects" section capturing the keyed-
collection model, shared drag controller, spiral auto-placement, live-add
idempotency, and union-bounds zoom-to-fit.

**ADRs:** none — implementation within ADR-003 (PixiJS camera model) and
ADR-004 (tile-position persistence), as the task Notes called.

**Verification:** `pnpm check` (svelte-kit sync + svelte-check) — 0 errors,
0 warnings, 936 files checked. The pure `tile-layout.ts` is unit-test-ready
(no runtime deps) once frontend test infra arrives; same posture as
`snapshot-patch.ts`. The untouched `snapshot-patch.ts` continues to power
fine-grained patching — now per-entry rather than against a module-level
single snapshot.

**Coordination:** ran in parallel with `project-registry-002a-scan-roots-and-walk`;
no overlap (this task touched only `src/lib/*` and the canvas BC README; the
other worker touched only `src-tauri/*` and the project-registry BC README).

**Key files:** `src/lib/Canvas.svelte`, `src/lib/tile-layout.ts`,
`.agentheim/contexts/canvas/README.md`.
