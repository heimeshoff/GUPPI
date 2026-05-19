---
id: ADR-016
title: Persistent PixiJS scene graph + camera as stage transform
status: Accepted
scope: bc
bc: canvas
date: 2026-05-19
related_tasks:
  - canvas-014-investigate-pan-zoom-performance
  - canvas-015-persistent-scene-graph-and-stage-transform-pan
related_adrs: [ADR-003, ADR-015]
---

# ADR-016: Persistent PixiJS scene graph + camera as stage transform

**Status:** Accepted
**Scope:** bc (canvas)

## Context

`canvas-014`'s perf spike (`research/canvas-perf-2026-05-17`) identified the
dominant cost driver in the canvas renderer: `renderScene` tore down and
rebuilt the entire Pixi scene graph on every call — `world.removeChildren()`
followed by re-instantiating every `Graphics`, `Text`, and `Container` for
every project frame, header, BC bubble, status badge, counts pill, and
intra-project edge. For Marco's typical scene (N≈7 projects × ~5 BCs),
that is **200–500 Pixi objects per render**, allocated and discarded on every
pan event.

The structural reason a rebuild was needed at all was the **camera transform
done manually in JS**: every `Graphics` was drawn at
`camera.worldToScreen(...)` *screen* coordinates rather than at world
coordinates with the camera applied via `world.position` / `world.scale`. So
none of the existing Graphics survived a camera change — they had been drawn
at the old screen coordinates.

ADR-003 already mandated PixiJS v8 + WebGL for the canvas. ADR-003's
Extension 2026-05-19 mandated the six rendering invariants (DPR fix,
constant screen-space stroke widths, zoom-scaled Pixi `Text` for project +
BC titles, `truncateTextToWidth` budget). All of those hold; this ADR
explains *how* the renderer satisfies them without paying the
tear-down-and-rebuild cost.

## Decision

The canvas renderer is structured around two long-lived abstractions:

### 1. Persistent per-project display objects

`Canvas.svelte` holds `frameObjects: Map<projectId, FrameDisplayObjects>`
keyed by the project's snapshot id. Each `FrameDisplayObjects` is
instantiated **once** when the project enters the scene (initial `refresh`
or a `project_added` event) and **destroyed** only when the project leaves
(`project_removed`).

The struct lays out one Pixi object per visual element:

```
FrameDisplayObjects = {
  container,    // Container at world-space (entry.pos.x, entry.pos.y)
  body,         // frame fill + border
  header,       // header bar fill + divider
  title,        // project name Text
  counts,       // total task count Text
  focusRing,    // hover halo (toggled via .visible)
  missingGlyph, // ✕ for missing-tile state (toggled via .visible)
  emptyText,    // "No bounded contexts yet" (toggled via .visible)
  edges,        // ALL intra-project edges in one Graphics (cleared+redrawn)
  bcsRoot,      // parent Container for BC bubbles
  bcs,          // Map<bcName, BcDisplayObjects>
}
```

Per BC bubble:

```
BcDisplayObjects = {
  container,   // Container at frame-local (bcPos.x, bcPos.y)
  body,        // bubble fill + border
  focusRing,   // hover halo (toggled via .visible)
  pillBg,      // counts-pill fill
  pillText,    // counts label
  title,       // BC name
  badge,       // status badge (body + glyph)
}
```

Updates flow through one of two functions:

- `updateFrameDisplayObjects(entry, obj, z)` — clears + redraws each
  Graphics's geometry in place (no allocation), updates Text contents and
  positions, reconciles BC bubble Map against the snapshot's `bcs[]`.
- `updateBcDisplayObjects(projectId, bc, local, obj, z)` — clears +
  redraws one BC bubble's body, focus ring, pill, title, badge.

Reconciliation in `renderScene` removes per-project Containers whose ids
are no longer in `projects[]`; the same shape inside the frame removes BC
display objects whose names are no longer in `entry.snapshot.bcs[]`.

### 2. Camera as stage transform

The `world` Container's `position` and `scale` carry the camera transform:

- `world.position = (camera.pan_x, camera.pan_y)` — pan in screen pixels.
- `world.scale = camera.zoom` — uniform zoom factor.

Children are drawn in **world coordinates** (not the previous JS-projected
screen coordinates). PixiJS's WebGL renderer applies `world.position` and
`world.scale` once per frame, transforming every child on the GPU.

The three interactive paths reduce to:

| Path | Camera transform | Persistent objects | Notes |
| --- | --- | --- | --- |
| **Pan** (`pointermove` with `dragState.kind === 'pan'`) | `world.position.set()` only | none touched | AC #3: zero allocation, zero clear+redraw. |
| **Zoom** (`wheel`) | `world.position.set()` + `world.scale.set()` | `repaint()` redraws stroke widths in place | Stroke widths must update because they are pre-divided by `z` to stay constant CSS-px on screen. |
| **Frame drag** (`pointermove` with `dragState.kind === 'frame'`) | unchanged | one frame's `container.position` | One project's container moves; nothing else touched. |
| **BC drag** (`pointermove` with `dragState.kind === 'bc'`) | unchanged | one BC's `container.position` + that project's edges Graphics | Edges follow the moving BC; other projects untouched. |
| **Theme flip** | unchanged | `repaint()` rewrites every Graphics's fill/stroke colours | No reinstantiation. |
| **Hover focus ring** | unchanged | toggle one Graphics's `.visible` | The ring's geometry is laid down by the last `updateXxx` call; the hover handler only flips `.visible`. |
| **Topology change** (`bc_appeared` / `bc_disappeared` / `project_added` / `project_removed`) | unchanged | `renderScene()` reconciles + updates affected entries | Only newly-added entries instantiate Pixi objects. |

### 3. Stroke-width invariant (ADR-003 #4) under world.scale

ADR-003 Extension 2026-05-19 #4 mandates constant screen-space stroke
widths for borders. With `world.scale = z` multiplying every child's
geometry by `z` on the GPU, a world-space stroke of width `w` renders on
screen at `w * z` CSS pixels. To keep the on-screen width constant at the
design token's `borderWidthFrame` (or `borderWidthFocus`, `edgeWeight`,
etc.), the world-space stroke width is pre-divided by `z`:

```ts
const strokeFrame = Math.max(1 / z, shape.borderWidthFrame / z);
obj.body
  .roundRect(0, 0, fw, fh, shape.radiusFrame)
  .fill(color.frameFill)
  .stroke({ width: strokeFrame, color: borderCol });
```

Arrowhead and ACL-notch tip sizes follow the same pre-divide pattern
(`shape.arrowheadLength / z`). The `pullBack` distance in
`drawArrowhead` stays in world space because it is the offset from the
bubble's edge, which lives in world space itself now (canvas-007's
inside-frame BC bubbles are positioned in frame-local world coords).

### 4. Text scaling

Pixi `Text` font-sizes are the world-space (zoom-1) token values
(`typography.sizeTitle`, `typography.sizeBody`, `typography.sizeCaption`).
The parent `world.scale = z` multiplies their on-screen size to
`fontSize * z`, satisfying the canvas-013 invariant that project and BC
titles **scale with zoom**.

At extreme zoom-in, text rasterized at the world-space `fontSize` is
GPU-upscaled by `z`, introducing slight bilinear softness. This is
acknowledged as imperfect; a follow-up BitmapText or dynamic-fontSize +
counter-scale strategy can sharpen it without changing the canvas-015
shape. canvas-013's `truncateTextToWidth` budget is computed against
world-space widths (`fw - 2 * framePadding - countsText.width - gap`) and
keeps producing correct truncation at every zoom because Text widths and
budget scale together.

**Zoom-out floor (ADR-003 Extension 2026-05-19 invariant #6).** Linear
scaling alone would let titles shrink below readability at extreme
zoom-out (e.g. `sizeBody = 14` at `z = 0.2` → 2.8 CSS-px). To preserve
"every BC always shows its name (if you squint)", four title `Text`
sites (project title, project missing-tile glyph, BC title, BC counts
pill text) apply a counter-scale `s = max(1, 8 / (fontSize * z))` via
`screenSpaceTitleScale(fontSize, z)` (a pure helper in `Canvas.svelte`).
At default zoom and above, `s = 1` and the cost is a single multiply +
compare per text per repaint pass. The counter-scale is applied BEFORE
`truncateTextToWidth` so the binary search measures the rendered width
that ends up on the GPU (the title may then exceed its world-space
budget at the floor; `truncateTextToWidth` shortens it correspondingly).
Repaint runs on every zoom change (wheel, eased zoom-to-fit, theme
flip), so the counter-scale is always current; it does NOT run on pan,
which is correct — pan does not change `z`.

### 5. Hit areas

Frame container `hitArea` is a frame-LOCAL rect (`0..fw × 0..headerH`) —
the header band. BC bubble container `hitArea` is a BC-LOCAL rect
(`0..bcInsideWidth × 0..bcInsideHeight`). Pixi's hit-test calls
`hitArea.contains(localX, localY)` with already-inverse-transformed
coords, so no `z` factor is needed in the predicates — `world.scale`
handles it.

The frame body is intentionally pass-through (frame's `hitArea` is the
header rect only); empty regions inside a frame fall through to the
DOM-level `app.canvas` `pointerdown` listener (camera pan claim).
Canvas-007's contract holds.

### 6. The drag-controller contract

The shared drag controller (`src/lib/drag-controller.ts`, canvas-012)
remains the public contract for all three drag kinds. canvas-015 only
changes how the **render** applies the delta: instead of calling
`renderScene()` after every `panBy` / `pos` mutation, the pan and frame /
BC drag paths now write directly into the persistent display objects
(`world.position.set` / `frame.container.position.set` /
`bcObj.container.position.set`) and the GPU re-renders at the next tick.

## Rationale

### Why persistent objects, not pooling?

A pool of pre-allocated Graphics+Text instances reuses memory but still
pays an allocation amortised over time. A persistent scene graph reuses
each Pixi object exactly as long as its visual concept exists in the model.
The lifecycle is governed by the model (`projects[]` + `bcs[]`), not by
the renderer — when a project is removed, its display objects are
destroyed; otherwise they live for the canvas's lifetime.

### Why `world.position` / `world.scale` instead of viewport scrolling?

PixiJS v8's WebGL renderer applies a parent container's transform on the
GPU at zero per-child cost. CSS-based viewport scrolling on the canvas
element would shift the whole `<canvas>` but not the screen-space
overlays (voice indicator, future panels, modals). Keeping the pan +
zoom inside Pixi keeps the camera's coordinate system shared with
overlays that compose against `app.renderer.width/height`.

### Why a single `edges` Graphics per project?

Per-edge Graphics instances would be persistent, but the number of edges
changes with the BC graph's topology. Edges are also strictly determined
by the relationship set — they don't carry their own state (hover, drag).
A single Graphics that is `.clear()`-ed and refilled on layout-change
events (BC drag end, topology change, theme flip, zoom change) is
allocation-free and the simplest correct shape. Per-pan cost is zero
because pan never touches this Graphics.

### Why pre-divide stroke widths instead of post-scale compensation on each child?

`Graphics.stroke({ width })` takes a number; Pixi v8 does not expose a
"keep stroke constant under parent scale" flag. Pre-dividing by `z` at
draw time produces the correct on-screen width with one multiplication
per stroked element. The alternative (a per-child counter-scale
`graphics.scale.set(1 / z)`) would compensate for the parent but require
counter-positioning every child — a layout regression.

## Consequences

### Wins

- **Pan path is allocation-free** (AC #3). Worst-pre-fix per-pointermove
  cost (200–500 Pixi allocations + atlas lookups) drops to a single
  `world.position.set()` call.
- **Zoom path allocates nothing** (AC #4). The `repaint()` pass that
  rewrites stroke widths uses persistent Graphics's `.clear()` + redraw
  — no `new Graphics()` anywhere on the zoom path.
- **Theme flip is in-place** (AC #7). `repaint()` clears + redraws every
  Graphics with the new palette numerics; Text fills are updated via
  `style.fill = newColor`.
- **Hover focus ring toggles via `.visible`** (AC #9). The ring's
  geometry is laid down by the last `updateXxx` call; the hover handler
  only flips visibility.
- **Frame-time budget** (AC #10). The structural cost driver (full scene
  rebuild × hover/pan/wheel events) is gone; pan-circles should sustain
  60 FPS at default zoom with 10+ frames. Hands-on verification per the
  canvas-perf-2026-05-17 reproducer protocol.

### Caveats

- **Text crispness at extreme zoom-in** is slightly soft (GPU upscale of
  world-space-rasterised Text). Acceptable for v1; a BitmapText switch
  (deferred from canvas-013) or a dynamic fontSize+counter-scale
  refinement can sharpen it without changing the persistent-scene-graph
  shape.
- **The renderer surface is wider in source**: `renderScene` now does
  reconcile + per-frame update instead of tear-down + rebuild. The total
  line count of `Canvas.svelte` is higher (a `createFrameDisplayObjects`
  + `updateFrameDisplayObjects` pair, plus the same for BC bubbles, plus
  the `frameObjects` map). The structural readability win is the
  separation of *what to draw* (geometry recipes in the `updateXxx`
  functions) from *when to draw it* (the dispatch paths in renderScene,
  the wheel handler, the pointermove handler).
- **`camera.worldToScreen` still exists** and is the contract for
  screen-space overlays (modals, context menus, voice indicator,
  forthcoming agent-awareness badges). The renderer's *internal* draw
  path no longer uses it.

## Open items captured as backlog

None new. The next renderer-side optimisations remain:
- `canvas-016-cache-screen-space-overlays` — apply the same persistent
  + update pattern to the voice indicator and future agent-awareness
  badges. Already partly done here: the voice indicator is now
  instantiated once and updated in place.
- `canvas-017-broad-phase-hit-rejection-for-empty-canvas-pan` —
  `world.hitArea` for cheap pan-over-empty-space rejection.
- `canvas-018-dev-only-diagnostic-seam` — `window.__guppiApp = app`
  behind a flag for future spikes.

## References

- `src/lib/Canvas.svelte` — the rendering surface.
- `src/lib/camera.svelte.ts` — Camera state; `worldToScreen` still
  used by overlays.
- `src/lib/drag-controller.ts` — drag state machine; canvas-015
  preserves its public API.
- `src/lib/bc-layout.ts` — BC layout output (`BcFrameLayout`); contract
  with canvas-015 preserved.
- `.agentheim/knowledge/research/canvas-perf-2026-05-17/README.md` —
  the spike that motivated this rewrite.
- ADR-003 — PixiJS v8 + HTML overlays (the parent decision).
- ADR-015 — BC layout inside a frame (read by this renderer).
