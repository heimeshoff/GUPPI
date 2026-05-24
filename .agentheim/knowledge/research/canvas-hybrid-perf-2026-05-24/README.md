---
slug: canvas-hybrid-perf-2026-05-24
title: Hybrid Pixi-shell + DOM kanban-interior perf spike — findings note
type: research
created: 2026-05-24
authored_for: canvas-019a
context: canvas
related_adrs: [ADR-003, ADR-016, ADR-002]
tags: [performance, rendering, pixi, dom, hybrid, kanban, spike, pivot]
---

# Hybrid Pixi-shell + DOM kanban-interior perf spike — findings note

Spike task: `canvas-019a-perf-spike-hybrid-dom-interior` (2026-05-24).
This is the empirical gate for `canvas-019` (the substrate-decision ADR
for the kanban-accordion pivot). `canvas-019` `depends_on` this note: it
ratifies **hybrid** on PASS, or overturns toward **Option B (full-DOM)**
on FAIL.

The product of this spike is THIS note plus a throwaway, instrumented
harness. As with `canvas-014`, the Tauri dev shell / real GPU cannot be
driven from a worker's shell, so the headline frame-time numbers are
**operator-pending** (Marco runs the reproducer; section "Operator
protocol" below). Everything that CAN be settled in-shell — harness
design, culling/LOD thresholds, the structural cost model, the static
checks — is settled here.

## TL;DR

- **Harness built and instrumented**, Option A (hybrid) only:
  `/spike-019a` route (`src/routes/spike-019a/+page.svelte`), a clearly
  marked THROWAWAY kept entirely out of the production `Canvas.svelte`
  path. `pnpm check` clean (992 files, 0/0/0); `pnpm build` compiles the
  route into the static bundle.
- **Worst-case load reproduced** per decision #1: N=10 frames, each a
  full populated kanban-accordion = 5 BCs × 3 columns × 8 cards =
  **120 DOM cards/frame, 1200 cards total** if all interiors mounted.
- **Two cost governors implemented and live in the harness**:
  - *Viewport culling* — only frames whose screen-space AABB intersects
    the viewport (+120px margin) mount a DOM interior; off-screen frames
    are a cheap persistent Pixi shell only.
  - *Zoom-threshold LOD* — below `z = 0.45` ALL interiors are suppressed
    (Pixi shells only) and re-mount on zoom-in. Rendering-layer LOD, NOT
    a model-level summary; decision #1 holds.
- **The structural argument for PASS is strong** (see "Why hybrid should
  PASS, analytically"): pan/zoom never rebuilds Pixi (ADR-016 stage
  transform) and never re-creates DOM nodes — pan only restyles the
  `left/top/transform` of the ≤9 mounted interiors, which the compositor
  handles off the main thread.
- **PASS/FAIL verdict is operator-pending.** Run the protocol; record
  numbers in `console-snapshot.md` next to this note. Decision rule and
  the fallback trigger are spelled out below so `canvas-019` can be
  written the moment the numbers land.

## Harness design (what was built)

Route: `src/routes/spike-019a/+page.svelte`. Self-contained; the ONLY
production import is `$lib/camera.svelte` (the `Camera` survivor — leaned
on per the task, not re-derived). No tokens, no brand palette — dummy
content, so the spike is exempt from the design-system styleguide gate.

Rendering split (Option A / hybrid):

- **Pixi (WebGL) draws the SHELL** of every frame in WORLD coordinates:
  a persistent `Container` per frame holding a border `Graphics`, a
  header-bar `Graphics`, and a title `Text`. Instantiated ONCE on mount
  (ADR-016 persistent scene graph); never rebuilt. The `world` container
  carries the camera transform (`world.position` = pan, `world.scale` =
  zoom). Stroke width is pre-divided by `z` and re-drawn ONLY on zoom
  change, never on pan (ADR-016 §3 invariant).
- **DOM draws the INTERIOR** of each on-screen frame: a vertical
  accordion of BCs, each a 3-column kanban (BACKLOG/DOING/DONE) of dummy
  task cards inside a real `overflow-y: auto` scroll container (the
  scroll container is part of decision #1's cost, so it is reproduced).
  Each interior is an absolutely-positioned div whose `left/top` come
  from `camera.worldToScreen(frame.wx, frame.wy)` (ADR-003 / ADR-016
  overlay contract) and whose `transform: scale(z)` matches the Pixi
  zoom so the interior tracks the shell exactly.

Why `transform: scale(z)` instead of re-laying-out the interior at the
current zoom: laying out 120 cards' flexbox at a new size every pan tick
would be a main-thread reflow storm. A single CSS `scale()` on the
interior root is a compositor transform — the browser rasterises the
interior's layout ONCE at zoom-1 layout size and the GPU scales the
layer. This is the load-bearing design choice for hybrid's viability and
is exactly the ADR-003 "overlays participate in GPU compositing without
re-rasterising on every camera change" idiom that `canvas-perf-2026-05-17`
flagged as the strength of the HTML-overlay path.

### Chosen thresholds (and why)

| Knob | Value | Rationale |
| --- | --- | --- |
| `N_FRAMES` | 10 | task target N≈10; matches Marco's real 7–10 projects |
| `BCS_PER_FRAME` | 5 | matches the canvas-perf baseline's "~5 BCs each" |
| `CARDS_PER_COLUMN` | 8 | "dozens of cards each" → 24/BC, 120/frame — deliberately heavy |
| `LOD_ZOOM_FLOOR` | 0.45 | below this, an 11px card font renders sub-5px — illegible anyway, so suppressing is visually free. Tune on-machine: raise if mid-zoom DOM still janks; lower if interiors vanish while still readable |
| `CULL_MARGIN_PX` | 120 | mount interiors slightly off-screen so a fast pan doesn't flash an empty shell as a frame scrolls in. Tune up if shell-flash is visible at high pan speed |

These are the spike's recommended starting thresholds for `canvas-019`'s
ADR. They are deliberately conservative; the operator run may justify
loosening `LOD_ZOOM_FLOOR` (more zoom range with live interiors) if the
numbers have headroom.

## Instrumentation (the measurement seam)

The harness exposes `window.__guppiSpike` (canvas-018 dev-seam idea), a
rolling frame-time sampler over `requestAnimationFrame` deltas:

```js
__guppiSpike.config        // { N_FRAMES, BCS_PER_FRAME, CARDS_PER_COLUMN,
                           //   cardsPerFrame, LOD_ZOOM_FLOOR, CULL_MARGIN_PX }
__guppiSpike.reset()       // clear the sample window
__guppiSpike.autopan(5)    // scripted 5s pan-circle, auto-resets the sampler
__guppiSpike.stats()       // { count, avgMs, p95Ms, maxMs, fps }
__guppiSpike.mountedInteriorCount()  // how many DOM interiors are live now
__guppiSpike.lodActive()             // true when below the LOD floor
```

`autopan(5)` drives the camera around a circle for 5 seconds without the
operator having to hand-pan, giving a repeatable sustained-pan stance for
AC #4. Stats are reported as average ms/frame, p95 ms/frame (the number
that matters for "feels smooth" — a single 40ms hitch is felt), max
ms/frame, and derived FPS.

## Operator protocol (Marco-runnable — fills the PASS/FAIL numbers)

This reuses the `canvas-perf-2026-05-17` reproducer protocol (Chrome
trace + JS console snapshot), adapted to the harness route. Paste outputs
into `console-snapshot.md` next to this README.

1. `pnpm tauri dev` (or `pnpm dev` + open `http://localhost:1420`), then
   navigate the WebView to **`/spike-019a`**.
2. Wait for the console line `[spike-019a] harness ready`.
3. Open WebView2 devtools (`Ctrl+Shift+I`).
4. **Renderer confirmation** (same as canvas-perf-2026-05-17):
   ```js
   const a = __guppiSpike.app;
   console.log('renderer.type      =', a.renderer.type);          // expect 2 (WebGL)
   console.log('renderer.resolution=', a.renderer.resolution);
   console.log('devicePixelRatio   =', window.devicePixelRatio);
   console.log('GPU                =',
     a.renderer.gl?.getParameter(
       a.renderer.gl.getExtension('WEBGL_debug_renderer_info')?.UNMASKED_RENDERER_WEBGL));
   ```
   If `renderer.type` is `1` (canvas fallback), STOP and note it — the
   whole calculus shifts (canvas-018-class WebGL-acquisition issue).
5. **Sustained-pan measurement (AC #4)** — default zoom (interiors live):
   ```js
   __guppiSpike.autopan(5);          // pan-circle for 5s; resets sampler
   // wait ~5s, then:
   __guppiSpike.stats();             // record count, avgMs, p95Ms, maxMs, fps
   ```
   Confirm `__guppiSpike.mountedInteriorCount()` is > 0 during the run
   (interiors are actually mounted, not silently culled to nothing).
6. **Zoom measurement (AC #4)** — `__guppiSpike.reset()`, then wheel-zoom
   in and out across the LOD floor for ~5s, then `__guppiSpike.stats()`.
   Watch the HUD: it should show "LOD: interiors suppressed" below z=0.45
   and the mounted count should climb as you zoom in.
7. **Chrome Performance trace** — Performance tab → Record → `autopan(5)`
   → Stop. Save as `trace.json` next to this note. Annotate the
   Scripting vs Rendering vs GPU split and the longest frame. The signal
   to watch: with hybrid working, Scripting per pan frame should be tiny
   (camera math + ≤10 overlay placements), with cost shifting to
   Rendering/Compositing (the GPU scaling DOM layers) — that split is
   the healthy hybrid signature.

### Verdict rule (so canvas-019 can be written immediately)

- **PASS** if the sustained pan-circle AND the cross-LOD zoom both hold
  `p95Ms ≤ 16` (sustained 60 FPS) at default zoom with interiors mounted.
  Ideally `avgMs ≤ 8` (the canvas-015 AC #10 headroom target). On PASS,
  `canvas-019` ratifies hybrid and cites this note's numbers.
- **FAIL** if either gesture sustains `p95Ms > 16` with the recommended
  thresholds AND tightening `LOD_ZOOM_FLOOR` / lowering
  `CARDS_PER_COLUMN` toward realistic counts does not recover it. On
  FAIL, this note's verdict line is set to FAIL and `canvas-019` is
  directed to cost **Option B (full-DOM)** as the fallback. (Do NOT build
  Option B here — it is out of this spike's scope.)

**Current verdict: OPERATOR-PENDING.** No fabricated frame-times. The
analytical case (below) predicts PASS; the on-machine run confirms or
refutes it.

## Why hybrid should PASS, analytically

The `canvas-perf-2026-05-17` report nailed the original sluggishness to
per-frame scene-graph REBUILDS (200–500 Pixi allocations × 60 Hz).
ADR-016 already eliminated that for the shell: pan is `world.position`
only, zoom is `world.position` + `world.scale` + an in-place stroke
repaint, zero allocation. The hybrid harness preserves that exactly.

The new question decision #1 raises is whether the DOM interiors
reintroduce a per-frame cost. They do not, by construction:

1. **DOM nodes are created once per mount, not per frame.** Svelte's
   `{#each}` keyed by `frame.id` reconciles: a frame that stays on-screen
   keeps its 120 card nodes across pan ticks — no create/destroy churn.
   Only frames crossing the cull boundary mount/unmount, at most a
   handful per second during a pan.
2. **Pan restyles ≤9 interior roots' `left/top/transform`.** That is ~9
   style writes per frame, not 1200. `top/left` on an absolutely
   positioned, `transform`-scaled element is a compositor-layer move, not
   a layout reflow of the 120 children.
3. **`transform: scale(z)` keeps the interior layout frozen.** The
   flexbox/grid layout of the 120 cards is computed once at zoom-1 size;
   zoom changes the layer's GPU scale, not the layout. No reflow on zoom.
4. **Viewport culling caps the live DOM at what's visible** (typically
   2–6 frames at default zoom for N=10 in a 4-wide grid), and **LOD caps
   it at zero** when zoomed out — exactly the regime where a full-DOM
   approach would otherwise pay for all 1200 cards at once.

The residual risk the operator run must rule out: the browser's initial
*paint/raster* of a newly-mounted 120-card interior (a frame scrolling in
from off-screen) could spike a single frame. Mitigations if observed:
`content-visibility: auto` on the BC sections, or staggering interior
mount across two rAF ticks. The harness margins (`CULL_MARGIN_PX = 120`)
buy a little head-start; if mount-spikes appear in the trace, that is the
knob to grow, and it is a tuning fix, not a hybrid-disqualifier.

## Scope notes

- **Throwaway, not merged.** The harness lives at its own route and is
  never imported by `Canvas.svelte` or linked from the app shell. To
  retire it after `canvas-019` decides, delete
  `src/routes/spike-019a/`. (Captured as a backlog cleanup task — see
  below.)
- **Option B not built** (per task scope). It is the fallback the FAIL
  branch flags.
- **No ADR written by this spike.** The substrate DECISION ADR is
  `canvas-019`'s job; this note is the empirical input it cites.

## Files

- `src/routes/spike-019a/+page.svelte` — throwaway hybrid harness +
  `window.__guppiSpike` instrumentation seam
- `src/lib/camera.svelte.ts` — reused unchanged (the `Camera` survivor)
- `.agentheim/knowledge/research/canvas-hybrid-perf-2026-05-24/console-snapshot.md`
  — operator pastes numbers here (created as a stub)

## Checks

- `pnpm check`: clean — 992 files, 0 errors, 0 warnings.
- `pnpm build`: succeeds; `spike-019a/_page.svelte.js` chunk emitted.
- Frame-time numbers: OPERATOR-PENDING (Tauri/GPU not driveable from the
  worker shell — canvas-014 precedent).
