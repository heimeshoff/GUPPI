---
id: canvas-015
title: Persistent scene graph + camera as stage transform (replace tear-down/rebuild render)
status: todo
type: feature
context: canvas
created: 2026-05-18
completed:
commit:
depends_on: [canvas-014]
blocks: []
tags: [performance, rendering, pixi, refactor]
related_adrs: [ADR-003, ADR-015]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-002, canvas-007]
---

## Why

`canvas-014`'s investigation surfaced that `renderScene` calls
`world.removeChildren()` and reinstantiates every `Graphics`, `Text`,
and `Container` for every project frame, BC bubble, edge, status
badge, counts pill, header bar, and focus ring on every render.

For Marco's typical scene (N≈7 projects × 5 BCs), that is **200–500
new Pixi objects per call**, allocated and discarded on every pan
event. The canvas-014 trivial-win patch killed the worst case (the
PixiJS ticker invoking the rebuild ~60×/sec when idle), but the
substantive cost during gestures remains: every pointermove during a
pan still rebuilds the whole scene from scratch.

The root reason a rebuild is needed at all is that the current
implementation does the camera transform **manually in JS**: every
`Graphics` is drawn at `camera.worldToScreen(...)` *screen*
coordinates rather than at world coordinates with the camera applied
via `world.position` / `world.scale`. So none of the existing
Graphics survive a camera change — they were drawn at the old screen
coords.

The PixiJS-native model is the opposite: draw children at world
coordinates once, then pan/zoom by setting `world.position` and
`world.scale`. The GPU does the transform for free.

## What

Restructure `Canvas.svelte`'s rendering so each project frame and its
contents are instantiated **once** and updated incrementally:

- One persistent `Container` per project frame (keyed by
  `entry.id`), held in a `Map<projectId, FrameDisplayObjects>`. Add
  on `project_added`, remove on `project_removed`.
- One persistent `Graphics` per frame body, per header bar, per
  intra-project edge, per BC body, per BC counts pill — held inside
  the per-frame display object struct so a single update can `.clear()`
  + redraw only when geometry or palette changes (e.g. theme flip).
- One persistent `Text` per project title and per BC name — `text`
  and `style` only updated when the underlying data changes
  (rename, count tick).
- Camera pan/zoom drives `world.position` and `world.scale` instead
  of triggering a redraw. Pan becomes a single property assignment;
  the GPU handles every child's transform.

This is a substantial rewrite of `renderScene`, `drawProjectFrame`,
`drawIntraProjectEdges`, `makeBcBubble`, and `makeStatusBadge`. The
public Svelte component contract does not change.

### Hit-areas and focus rings

- Frame header `hitArea` becomes a frame-local rect (the
  `world.scale` covers the zoom; the predicate no longer needs `z`).
- BC bubble `hitArea` likewise becomes a frame-local rect.
- Focus ring is a child `Graphics` toggled via `.visible` (do not
  destroy / re-add).

### Text scaling

When `world.scale` carries the zoom, child `Text` will rasterise at
its declared font-size and then be scaled by the GPU — which is
exactly what canvas-013's HTML-overlay-for-project-titles plus
BitmapText-for-BC-names strategy is for. Coordinate sequencing with
canvas-013: if canvas-013 lands first, `Text` for BC names stays in
this refactor's surface (canvas-013 deferred the BitmapText switch).
If canvas-014 follow-ups (this task) land first, project titles are
still Pixi `Text` here and canvas-013 swaps them to HTML overlay
afterward.

## Acceptance criteria

- [ ] `renderScene` no longer calls `world.removeChildren()`.
- [ ] Each project's display objects are instantiated once (on
      `project_added` or initial `refresh`) and updated in place on
      subsequent renders.
- [ ] Pan: a `pointermove` event during pan triggers ONLY a
      `world.position` update plus the implicit GPU draw call — no
      Pixi object allocation in the per-pan path.
- [ ] Wheel zoom: a `wheel` event triggers ONLY `world.scale` and
      `world.position` updates (zoom-around-anchor preserved).
- [ ] BC drag and frame drag still persist position via
      `saveBcPosition` / `saveTilePosition` on `pointerup` (unchanged
      contract).
- [ ] Intra-project edges update geometry when a BC drag ends and the
      one-shot layout re-runs (ADR-015 contract preserved).
- [ ] Theme flip still recolours every frame, header, edge, badge —
      via a `repaint()` pass that calls `.clear()` + re-stroke / re-fill
      with the new palette but does NOT re-instantiate the
      `Graphics`/`Text` instances.
- [ ] Missing-tile state still recolours the border + dims the body
      via property updates, not rebuilds.
- [ ] Hover focus ring still toggles via `.visible`, not by
      reinstantiation.
- [ ] Frame-time targets (`canvas-014`): ≤ 8 ms per frame at default
      zoom with 10+ project frames, sustained 60 FPS during continuous
      pan-circles for ≥ 5 seconds.
- [ ] No regressions: every existing behaviour from `canvas-002`,
      `canvas-005a`, `canvas-005b`, `canvas-006`, `canvas-007`,
      `canvas-008`, `design-system-004` (theme flip) still works.
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- This is the substantive follow-up to `canvas-014`. The trivial-win
  patch in canvas-014 (ticker-guard + explicit resize listener) is the
  prerequisite — without it, this task's measured gains are masked by
  the ticker's idle-time rebuild noise.
- Read the canvas-perf-2026-05-17 report end-to-end before starting.
  In particular, "Hotspot 2 — Full scene-graph rebuild per renderScene
  call" describes the structural change required.
- ADR-003 ("PixiJS v8 + HTML overlays") and ADR-015 (one-shot BC
  layout) both hold unchanged. This task implements ADR-003's
  "the GPU does the camera transform" path correctly for the first
  time.
- The current `camera.worldToScreen()` API stays useful — overlays
  (modals, context menus, voice indicator after canvas-016) still need
  to project world points to screen pixels. Only the renderer's
  internal drawing switches off it.
- Coordinate with `canvas-013` (crisp rendering + project-title HTML
  overlay) — see the canvas-perf-2026-05-17 report's "Coordination
  with canvas-013" section. Either order works; both end up with the
  rendering surface in the same shape.
- Suggest landing in stages: (1) frames-only persistence (project
  body, header, title), (2) BC bubbles, (3) edges, (4) the camera
  transform switch — each stage can `pnpm check` clean and smoke
  independently.
