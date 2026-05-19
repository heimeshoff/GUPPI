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

## Extension 2026-05-19 — Crispness invariant + zoom-scaling project titles

**Status:** Accepted (extension)
**Scope:** global (unchanged)
**Driver:** [canvas-013-crisp-rendering-constant-size-project-titles](../../contexts/canvas/done/canvas-013-crisp-rendering-constant-size-project-titles.md)
**Companion:** the [`canvas-perf-2026-05-17`](../research/canvas-perf-2026-05-17/README.md)
spike report (canvas-014) — referenced for the per-frame cost baseline.

### Same-day revision (2026-05-19, hands-on)

The first draft of this extension specified project frame titles at a
**constant screen-space size** (Miro-style HTML overlay). Marco ran the
shipped commit (`3315a10`) in `pnpm tauri dev` the same afternoon and
reverted that choice: project titles read out of place when they
don't scale with their frame at zoom — the visual rhyme with the BC
titles inside the frame matters more than the Miro-style overview
affordance. Same-day revision lands as:

- **Project titles scale with zoom**, like BC titles do — same
  relative size to their frame. Project titles are Pixi `Text` (not
  HTML overlays); the title-overlay sub-layer / z-band / `frame-title*`
  CSS are removed.
- **Truncation is container-width-based, not zoom-based.** Both
  project titles and BC titles end-truncate with an ellipsis when
  their rendered width would exceed the available container width
  (project: frame header width minus padding minus counts pill;
  BC: bubble width minus padding minus counts pill). Implemented via
  a small `truncateTextToWidth(text, fullText, maxWidth)` helper that
  binary-searches the largest prefix that fits with `…` appended.
- **Crispness invariant (DPR fix) is unchanged.** Invariant #1 below
  still holds; it was the load-bearing fix and is what makes the
  zoom-scaling titles + BC titles render crisply at every zoom in the
  first place.
- **Screen-space stroke widths are unchanged.** Invariants #4 / #5 /
  #6 below also hold; the only invariants touched by this revision
  are #2 and #3.

The body below has been rewritten in place — invariants #2 and #3 carry
the revised wording. The "first draft" wording lives in git history at
commit `3315a10`.

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

#### 2. Project titles scale with zoom (same relative size to frame)

Project frame titles render as Pixi `Text` with `fontSize:
Math.max(8, typography.sizeTitle * z)` — same scaling convention as
BC titles inside the frame (`Math.max(8, typography.sizeBody * z)`).
The visual rhyme between a frame's title and its child BC titles is
load-bearing: a project that grows visually when zoomed in and
shrinks when zoomed out reads as one coherent object; a project whose
title stays the same size while its body grows / shrinks reads as
two unrelated UI layers stacked on top of each other.

The `Math.max(…, 8)` floor catches extreme zoom-out so the title
doesn't shrink to sub-pixel; below the floor the title is rendered at
8 CSS px (still crisp via the DPR fix below). This matches what the
BC titles do.

No HTML overlay, no separate z-band. The Pixi `Text` allocation is
inside `drawProjectFrame`, immediately after the counts pill is
measured (the title's truncation budget — see invariant #3 — depends
on the pill's rendered width). Per-render cost: one Pixi `Text`
allocation per project (matches the BC bubbles' per-render cost
profile).

#### 3. Title overflow — end-ellipsis at the container width

Both project titles and BC titles **end-truncate with an ellipsis
(`…`)** when their rendered width would exceed the available
container width:

- **Project title:** available width = `fw - 2 * framePadding -
  countsText.width - gap`, where `fw` is the frame header's screen
  width, `framePadding` is `shape.framePadding * z` (zoom-scaled),
  `countsText` is the right-aligned "N tasks" pill that gets measured
  before the title is built, and `gap` is `framePadding * 0.5`
  (breathing room between title and pill).
- **BC title:** available width = `pillX - bcTitleX - gap`, where
  `pillX` is the BC counts pill's left edge (the pill is measured
  first inside `makeBcBubble`, the title's truncation budget reads
  off `pillX`), and `gap` is the same `framePadding * 0.5`.

Truncation is implemented via a small helper, `truncateTextToWidth(t,
fullText, maxWidth)`, that binary-searches the largest prefix of
`fullText` whose rendered width (with `…` appended) is `<= maxWidth`.
If even the ellipsis alone doesn't fit (a degenerate zoom-out), the
text renders empty (the bubble / frame still draws, just without a
label — the title is never allowed to overflow). The helper is a
sibling of `deriveBcStatus` in `Canvas.svelte`.

The cost is amortised: the Pixi `Text`'s `.width` getter triggers a
layout, but each truncation iteration is O(log N) in the label length,
and titles are typically short (10–40 characters). Per the
`canvas-perf-2026-05-17` report, per-frame Pixi `Text` rasterisation
is part of the existing render cost — the truncation helper does not
add any rasterisations the title would not have triggered anyway. If
a future profile flags the binary-search measurement as material, a
straightforward optimisation is to cache `lastFullText → truncated`
keyed by `(maxWidth, fontSize)` per BC / per project across renders;
not done today.

The pre-revision wording (HTML overlay + CSS `text-overflow:
ellipsis`) is left behind in commit `3315a10`'s ADR diff; the same-day
hands-on revert is the reason invariants #2 and #3 are JS-side now.

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

- (+) The overview reads crisply at every zoom. The DPR fix is
  universal crispness for a one-line change.
- (+) Project titles + BC titles share one rendering model (Pixi
  `Text`, zoom-scaled, end-truncated to container width). A future
  reader of the canvas code does not have to remember which titles
  are HTML and which are Pixi.
- (+) Theme-flip continues to repaint titles, borders, and counts in
  the right palette: all title `Text` instances read `color.*` tokens
  built from the active palette, and the Pixi scene reruns
  `renderScene` on the `onThemeChange` listener (design-system-004
  path, unchanged).
- (–) Titles below ~8 CSS px (extreme zoom-out) clamp to the 8-px
  floor and stop scaling proportionally, matching the BC titles'
  behaviour. Acceptable; this is the same floor the BC titles already
  have. Removing the floor would let titles shrink to true sub-pixel,
  which the DPR fix can't save (Pixi rasterises at the configured
  font size, not below it).
- (–) The per-render cost of a project title is one Pixi `Text`
  allocation (returned by the HTML overlay revert). The
  `canvas-perf-2026-05-17` report has this in the "no top-3 hotspot"
  zone; the BitmapText follow-up is still the documented escape hatch
  (invariant #5).

### Reversibility

Each invariant is independently reversible. The zoom-scaling title
revert is a small diff in `drawProjectFrame` (the pre-revision form
is in commit `3315a10`'s ADR diff). The DPR fix is a two-property
change at the `app.init({ … })` site. The screen-space stroke-width
policy is a search-and-replace from `shape.borderWidth* * z` back to
the `* z` form. The `truncateTextToWidth` helper is one function with
no callers outside the two title sites.

### Implementation pointers

- `src/lib/Canvas.svelte`:
  - `app.init({ … })` call (the DPR fix).
  - `truncateTextToWidth(...)` helper — script-level, sibling of
    `deriveBcStatus`.
  - `drawProjectFrame` body — project title creation now follows the
    counts pill so the title's `maxWidth` can read off the pill's
    measured width.
  - `makeBcBubble` body — pill block now precedes title block for the
    same measure-then-truncate reason.
  - `drawIntraProjectEdges` / `drawArrowhead` / `drawAclNotch` —
    screen-space stroke widths.
  - `makeBcBubble` border path — screen-space stroke widths.
- No separate file; no HTML overlay layer for titles.
