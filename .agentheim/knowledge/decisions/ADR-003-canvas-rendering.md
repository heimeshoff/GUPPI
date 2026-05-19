---
id: ADR-003
title: Canvas rendering — PixiJS v8 (WebGL) with HTML overlays for interactive content
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-003-canvas-rendering, canvas-013-crisp-rendering-constant-size-project-titles]
related_adrs: [ADR-002]
---

# ADR-003: Canvas rendering — PixiJS v8 (WebGL) with HTML overlays for interactive content

**Status:** Accepted
**Scope:** global

## Context

ADR-002 settled the frontend: **Svelte 5 + SvelteKit (static adapter)** inside
the Tauri 2 WebView. This ADR picks the rendering technology for GUPPI's
infinite canvas, which mounts inside that Svelte frontend.

v1 is canvas-only: pan, zoom, drag, hundreds of tiles, child connections.
Future requirements: live indicators, status badges, and embedded terminal
panels — interactive text rendered *inside* a tile.

Three rendering approaches exist:

1. **SVG** (e.g. via svelte-flow / d3) — DOM-native interactions, accessible,
   but performance degrades around a few hundred nodes once connectors and
   re-renders are in play. A poor fit for a Miro-like zoomed-out density.
2. **HTML5 canvas 2D** (Konva, fabric.js, or hand-rolled) — a good middle
   ground, handling roughly thousands of objects, but text rendering and
   embedded interactive panels are awkward.
3. **WebGL via PixiJS** — the highest performance, scaling to many thousands
   of nodes with smooth zoom/pan and a mature API. Embedding interactive
   content (a terminal panel) is done by *overlaying* HTML on top of the
   canvas, synced to PixiJS coordinates — exactly Miro's and Figma's approach.

## Decision

Use **PixiJS v8** (WebGL) for the canvas — tiles, edges, badges, and
hit-testing.

When a tile needs rich interactive content (markdown viewer, emulated
terminal), render an **HTML overlay positioned to match the tile's world
coordinates**. A single source of truth for camera state (pan + zoom),
modelled as Svelte 5 runes per ADR-002, is subscribed to by both the PixiJS
scene and the HTML overlay layer so the two stay in lockstep.

This builds directly on ADR-002: PixiJS mounts inside a Svelte component, and
the overlay layer is plain Svelte markup driven by the same reactive camera
state.

## Consequences

- (+) Future-proof for terminal panels, dense canvases, and fluid pan/zoom.
- (+) Mature library with extensive prior art for infinite canvases.
- (–) More upfront work than a "drop in svelte-flow" approach — the first
  tile takes longer than it would in SVG.
- (–) Accessibility (screen readers) is poor on a canvas. Acceptable here:
  GUPPI is a single-user tool, not a public product.

## Reversibility

Medium. The world-coordinate abstraction — camera state plus the mapping
between world and screen coordinates — is portable. The rendering layer
behind it could be swapped (e.g. to canvas 2D) without disturbing the
overlay layer or the rest of the frontend, though it would still be a
non-trivial rework.

---

## Extension 2026-05-19 — Crispness invariant + constant-size project titles

**Status:** Accepted (extension)
**Scope:** global (unchanged)
**Driver:** [canvas-013-crisp-rendering-constant-size-project-titles](../../contexts/canvas/done/canvas-013-crisp-rendering-constant-size-project-titles.md)
**Companion:** the [`canvas-perf-2026-05-17`](../research/canvas-perf-2026-05-17/README.md)
spike report (canvas-014) confirmed that nothing in the per-frame budget
contradicts the HTML-overlay-for-titles strategy — Pixi `Text`
allocations are part of the per-render cost; HTML overlays positioned
via the camera transform ride CSS layer compositing on the GPU without
re-rasterising on every camera change.

### Context (refinement)

Hands-on verification on 2026-05-17 surfaced two related rendering
deficiencies at low zoom:

1. **Everything blurred at zoom-out.** PixiJS v8 defaults to
   `resolution = 1` regardless of `window.devicePixelRatio`; every Text
   and Graphics rasterised at 1× and the GPU then upscaled to device
   pixels, producing a bilinear-smear halo.
2. **Project frame titles shrank with the camera.** The
   `fontSize: Math.max(8, typography.sizeTitle * z)` path tied the
   title height to the zoom level; the overview — the use case that
   needs the title most — was unreadable below ~50% zoom.

Both defects undermined GUPPI's headline ambient-overview differentiator
(per `vision.md`).

### Decision (extension)

The original ADR-003 PixiJS-v8-plus-HTML-overlays stance stands. This
extension refines it with four invariants for canvas rendering:

#### 1. Crispness invariant (load-bearing DPR fix)

`app.init({ … })` is called with **both** `resolution:
window.devicePixelRatio` AND `autoDensity: true`. This is the single
fix that converts most current blur into crispness without touching
the render path:

- `resolution = DPR` tells PixiJS to render at the framebuffer's
  real resolution.
- `autoDensity = true` tells PixiJS to downscale the canvas via CSS,
  so the WebView still composites at the logical CSS-pixel grid.
- Coordinate inputs (positions, line widths, font sizes) stay in CSS
  pixels — PixiJS multiplies by `resolution` internally. The rest of
  the scene math is unchanged by this flip.

The fix is implemented at the single boot site (`Canvas.svelte`'s
`app.init({ … })` call).

#### 2. Constant-screen-size project titles (Miro-style)

Project frame titles render as **HTML overlays** positioned over the
PixiJS canvas, at a CONSTANT screen-space CSS pixel size at every
camera zoom (`var(--guppi-size-title)`, currently 16px). BC titles,
counts, and the frame header's "N tasks" label are NOT subject to this
— they stay in Pixi `Text` and scale with the camera (smaller when
zoomed out, larger when zoomed in) but stay crisp per invariant 1.

The title overlay lives in the existing ADR-003 overlay container
(same DOM root as modals / menus / toasts) in its own sub-layer with
a dedicated z-index band:

| Layer | z-index | Notes |
| --- | --- | --- |
| PixiJS canvas | (default 0) | The underlying scene |
| **Project title overlay** | **5** | Above canvas, below interactive overlays |
| Context menus | 10 | (canvas-005a) |
| Error toasts | 11 | (canvas-005a) |
| Modal backdrops | 20 | (canvas-005b) |

`pointer-events: none` on the overlay container so a title never
swallows the canvas's pan gesture, a right-click on the frame header,
or a wheel-zoom.

The overlay subscribes to camera + project + theme reactive state via
Svelte 5 runes — `camera.pan_x` / `camera.pan_y` / `camera.zoom`, the
`projects` `$state` array, and the active CSS-token palette flipped by
the `[data-theme="light"]` attribute. Pan / zoom / theme flip / live-
add / live-remove re-evaluates the template automatically; no per-tick
push from `renderScene` required, and per the `canvas-perf-2026-05-17`
report the ticker is dormant in steady state anyway.

#### 3. Title overflow — end-ellipsis at the current header width

When a project's name exceeds the frame header width at the constant
title size (reproducible at zoom ~38% with a long project name), the
title truncates with an **end-ellipsis** at the current header CSS
width.

The implementation uses CSS `text-overflow: ellipsis` on a width-bound
element. The element's `width` is rebound on every camera change by
the reactive template (Svelte updates the inline `style="width: …px"`
string), and the browser re-measures synchronously on the next style
recalc. **No per-frame JavaScript measurement.**

The explicit per-frame JS measurement path remains the documented
fallback if a future profile flags browser layout cost as material
during pan. The handoff signal is "ellipsis lag becomes visible during
a pan gesture" — at that point, replace the CSS path with a manual
`measureText` against the current zoom + frame width and set the
displayed string explicitly. Not done today; not expected to be
needed.

#### 4. Screen-space stroke widths for the borders category

All "borders" — frame border, BC bubble border, header divider, focus
ring, and the four intra-project edge variants (incl. arrowheads + ACL
notches) — use **constant screen-space CSS pixel** stroke widths.
The previous `Math.max(1, shape.borderWidth* * z)` pattern produced
sub-pixel hairlines at zoom-out (smeared into halos by the GPU
upscale) and chunky strokes at zoom-in (out of line with the rest of
the stroke vocabulary). Removing the `* z` multiplier — with
`autoDensity = true` handling the CSS-px-to-device-px conversion —
yields true device-px hairlines at every zoom.

Caveats baked into the implementation:

- Focus-ring **inset** (the offset that places the ring outside the
  shape) stays world-space (`* z`), proportional to the shape it
  hugs at every zoom. Only the stroke width is constant-screen-space.
- Arrowhead `pullBack` (the distance the tip is pulled back so it
  doesn't bury under the BC bubble) stays world-space, because the
  bubble it pulls back from is drawn at `bcInsideWidth * z` in screen
  space. Only the arrowhead's `headLen` / `headW` are constant.

#### 5. BC text stays in Pixi `Text` (BitmapText deferred)

BC bubble title, BC task-count pill, the frame header's counts label,
status-badge glyphs, and the missing-tile `✕` glyph all remain in
Pixi `Text`. Crispness comes from the DPR fix (invariant 1); no
`BitmapText` migration is required for the invariant.

`BitmapText` (or a per-zoom-threshold bitmap fallback) is reserved as
an *optimisation* path. The `canvas-perf-2026-05-17` spike's hotspot
ranking did not flag per-frame Pixi `Text` allocation as a top-3 cost
beyond what `canvas-015` (persistent scene graph) already addresses;
once 015 lands, BC `Text` instances persist across renders and
re-rasterise only on string/style change. At that point a BitmapText
switch would shed the per-Text-style atlas lookup but the headline
pan/zoom cost is already gone. **No backlog item is filed for the
BitmapText switch from this task** — the trigger is "BitmapText shows
up as a top-3 hotspot in a future profile", not "we have time to do
it."

#### 6. BC text floor — no hiding

BC bubble title and task-count text are rendered at every zoom level.
The strategy MAY swap rendering paths for very small text (e.g. a
single low-res bitmap below some zoom) but MUST NOT hide BC text under
any zoom threshold. Marco's v1 stance: "every BC always shows its
name (if you squint)." This invariant is documented for future
performance-driven rework; no current code path violates it.

### Consequences

- (+) The overview reads at every zoom. The DPR fix is universal
  crispness for a one-line change; the constant-size title is the
  Miro-style affordance the canvas was missing.
- (+) The per-frame cost of rendering a project title drops to zero
  — the HTML overlay rides CSS compositing; no Pixi `Text`
  rasterisation per render; no per-frame allocation for the title.
- (+) Theme-flip continues to repaint titles, borders, and counts in
  the right palette: the title overlay reads CSS custom properties
  (which flip atomically under `[data-theme="light"]`); Pixi
  `renderScene` reruns on the `onThemeChange` listener
  (design-system-004 path, unchanged).
- (–) The title overlay is a second source of truth for project
  identity in the DOM (alongside what the Pixi scene knows). The
  divergence risk is small — both subscribe to the same `projects`
  `$state` array — but a future bug where one updates and the other
  doesn't would surface as a stale title floating over the wrong
  frame. Mitigation: both paths are keyed by `entry.id` and read the
  same `entry.snapshot.name`, with no intermediate caching.
- (–) The overlay container's `position: absolute; inset: 0` covers
  the full canvas region; the per-frame title divs are absolutely
  positioned children. Many projects (hundreds) means many child
  divs in the DOM. Acceptable for v1 (Marco's machine handles N≈10
  in profiling); a future cull-by-visible-viewport optimisation is
  the obvious knob if N grows to thousands.

### Reversibility

The HTML-overlay-for-titles strategy is straightforward to revert: the
overlay template block is one `{#each projects}` in the template plus
one CSS rule pair; the Pixi `Text`-based title path is the
`drawProjectFrame` body's previous form (one diff hunk to restore).
The DPR fix is a two-property change at the `app.init({ … })` site.
The screen-space stroke-width policy is a search-and-replace from
`shape.borderWidth* * z` back to the `* z` form. Each piece is
independently reversible.

### Implementation pointers

- `src/lib/Canvas.svelte` (the `app.init({ … })` call, the
  `drawProjectFrame` body, the `drawIntraProjectEdges` / arrowhead /
  ACL-notch helpers, the `makeBcBubble` border path, the
  `.frame-title-overlay` / `.frame-title` template + CSS).
- The HTML overlay layer in the same component (alongside
  `.context-menu` / `.error-toast` / `Modal` consumers); no separate
  file.
