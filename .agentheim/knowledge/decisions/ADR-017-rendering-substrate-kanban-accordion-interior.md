---
id: ADR-017
title: Rendering substrate for the kanban-accordion frame interior — hybrid Pixi shell + DOM interior
status: Accepted
scope: bc
bc: canvas
date: 2026-05-24
related_tasks:
  - canvas-019
related_adrs: [ADR-003, ADR-015, ADR-016, ADR-002]
supersedes_in_part: [ADR-015]
---

# ADR-017: Rendering substrate for the kanban-accordion frame interior — hybrid Pixi shell + DOM interior

**Status:** Accepted
**Scope:** bc (canvas)

## Context

The canvas is pivoting. Every project frame's interior becomes a **vertical
accordion of collapsible bounded contexts**, each holding a kanban board
(BACKLOG → TODO → DOING → DONE) of individual task cards, with a docked
right-edge detail panel that animates in/out. A locked user decision drives
the whole pivot: **every project always renders as a full kanban-accordion
frame** — there is no separate small/summary representation; the user pans
and zooms between full frames on the infinite surface (call this
**decision #1**).

The current canvas is PixiJS v8 / WebGL (ADR-003) with a persistent scene
graph and camera-as-stage-transform (ADR-016, shipped in canvas-015). The
frame chrome, BC bubbles, and the spring-electrical intra-project edges
(ADR-015, canvas-007) are all drawn as Pixi `Graphics` / `Text`.

A scrollable kanban with collapsible accordions, dozens of text-heavy cards
per frame, and an animated docked panel is the quintessential DOM/HTML UI.
Decision #1 makes this acute: the worst case is **N full text-heavy
interactive frames simultaneously on a pannable/zoomable surface**. This
forces a revisit of ADR-003's substrate choice before any interior task can
proceed (canvas-020 / canvas-021 / canvas-022 all `depends_on` this decision).

The decision had to settle: (1) the substrate; (2) ADR-015's fate; (3) the
viewport-culling + level-of-detail policy for N full frames; (4) how the
docked detail panel composes; (5) what survives versus what is retired from
the ADR-016 / canvas-015 investment.

## Options considered

### Option A — Hybrid: Pixi shell + DOM interior (CHOSEN)

Pixi keeps the world-space camera transform (ADR-016) and the **frame shell**
(border + header bar + drag hit-area), drawn in world coordinates as today.
Each frame's **kanban-accordion interior** AND the **docked detail panel**
render as HTML/DOM overlays. The interior is an absolutely-positioned element
whose `left/top` come from `camera.worldToScreen(frame.wx, frame.wy)` (the
overlay contract ADR-003 already names and ADR-016 explicitly preserved), and
whose `transform: scale(z)` matches the Pixi zoom so the interior tracks the
shell exactly. Native scroll, native text layout/wrapping/selection, CSS
transitions for the accordion and panel slide, real focus/keyboard handling,
and the `tokens.css` overlay layer (maintained by design-system in parallel
with `tokens.ts`) all come for free.

### Option B — Full DOM canvas, no Pixi (COSTED FALLBACK, not chosen)

Drop Pixi entirely; a single CSS-`transform: scale()/translate()`-on-a-world
`<div>` carries the camera (the Miro / tldraw-lite pattern). This unifies
everything in one technology, and decision #1 removes Pixi's headline
advantage (there is no zoomed-out Pixi-only density mode to protect — every
frame is always a full kanban). It was costed honestly, not dismissed. It was
not chosen because (a) it throws away the bulk of recent canvas work —
ADR-016 / canvas-015 (persistent scene graph + stage transform),
canvas-012 (drag-controller), canvas-013 (crisp rendering); and (b) CSS
transforms on very large worlds hit subpixel / compositing limits that Pixi's
WebGL transform does not. Option B is retained as the **documented fallback**:
if hybrid's N-DOM-interior perf had failed and could not be recovered by
tuning the cost governors, this is where the decision would have gone. The
`canvas-019a` spike PASSED, so it was not needed.

### Option C — Pure Pixi: kanban in WebGL (REJECTED)

Build the kanban, scroll, accordion, and panel entirely in WebGL Pixi
`Graphics` / `Text`. Rejected outright: text-heavy, scrollable, interactive
UI in WebGL is the wrong tool. It would be a large, bug-prone reimplementation
of the browser's scroll containers, text reflow/wrapping/selection, and
accordion animation — and it directly contradicts ADR-003's own reasoning
(ADR-003 already prescribes HTML overlays "when a tile needs rich interactive
content … exactly Miro's and Figma's approach").

## Decision

**Adopt Option A — hybrid.** Pixi owns the camera transform and the frame
shell; DOM owns the kanban-accordion interior and the docked detail panel,
positioned to world coordinates via `camera.worldToScreen`.

This is an **extension of ADR-003**, not a reversal of it: ADR-003 mandated
exactly this HTML-overlay-for-rich-interactive-content pattern, and ADR-016's
Consequences explicitly preserved `camera.worldToScreen` as "the contract for
screen-space overlays … forthcoming agent-awareness badges." Hybrid is the
realisation of the architecture ADR-003 already described, applied to the
frame interior at the scale decision #1 demands.

### Cost governors (the perf policy for N full frames)

Decision #1 makes "N full DOM interiors at once" the real perf question. Two
cost governors keep hybrid viable and are part of this decision (recommended
starting thresholds from the `canvas-019a` spike; tunable on-machine):

- **Viewport culling.** Only frames whose screen-space AABB intersects the
  viewport (plus a margin `CULL_MARGIN_PX = 120`) mount a DOM interior.
  Off-screen frames render as the cheap persistent Pixi shell only — no DOM
  interior at all. The margin mounts interiors slightly off-screen so a fast
  pan does not flash an empty shell as a frame scrolls in. Tune the margin up
  if shell-flash is visible at high pan speed.
- **Zoom-threshold LOD.** Below a zoom floor `LOD_ZOOM_FLOOR = 0.45` ALL DOM
  interiors are suppressed (Pixi shells only) and re-mount on zoom-in. This is
  **rendering-layer level-of-detail, NOT a model-level summary** — the same
  frame renders at a different fidelity depending on zoom; there is no separate
  "summary representation" in the model, so decision #1 holds intact. At the
  floor an 11px card font renders sub-5px (illegible anyway), so suppressing it
  is visually free. Tune the floor up if mid-zoom DOM still janks; down if
  interiors vanish while still readable.

Two further refinements are available knobs, not load-bearing requirements:
CSS `content-visibility: auto` on BC sections / columns, and staggering
interior mount across two rAF ticks if a newly-mounted 120-card interior
spikes a single paint frame.

The load-bearing structural choice is **`transform: scale(z)` on the interior
root** rather than re-laying-out the interior at the current zoom: the flexbox
/ grid layout of the cards is computed once at zoom-1 layout size, and zoom
changes the layer's GPU scale, not the layout — a compositor transform, not a
main-thread reflow. Pan restyles only the `left/top/transform` of the handful
of mounted interior roots (≤9 in the worst case at default zoom), never the
hundreds of card children. Svelte's keyed `{#each}` (ADR-002) reconciles a
frame's card nodes across pan ticks so on-screen frames keep their DOM with no
create/destroy churn — only frames crossing the cull boundary mount/unmount.

### Docked detail panel composition

The right-edge docked detail panel composes as a **screen-space,
viewport-docked** DOM overlay (full detail in canvas-022), not a
world-positioned one. It is pinned to the viewport edge rather than tracking a
frame's world coordinates, so it does not pan or zoom with the canvas; it
animates in/out via CSS transition over the `tokens.css` overlay layer. It
sits in the same overlay layer as modals, context menus, and the voice
indicator — all the screen-space consumers ADR-016 enumerated for
`camera.worldToScreen` and viewport-relative positioning.

### What survives, what is retired

**Survives (preserved unchanged or near-unchanged):**

- `src/lib/camera.svelte.ts` — Camera state (pan + zoom) and `worldToScreen`.
  `worldToScreen` is now load-bearing for positioning the DOM interiors, exactly
  the contract ADR-016 promised would survive.
- `src/lib/drag-controller.ts` — the shared three-kind drag state machine
  (frame drag, BC drag, camera pan), canvas-012. Frame drag and camera pan are
  unchanged; BC-drag-inside-the-frame semantics change with the interior
  pivot (BCs are accordion sections, not draggable bubbles) and are reworked by
  the interior tasks, but the controller contract is preserved.
- `src/lib/tile-layout.ts` — deterministic outward-spiral auto-placement of
  frame origins on the canvas. Frames still need world positions; this is
  untouched.
- The **persistent Pixi scene graph + camera-as-stage-transform** (ADR-016) for
  the **frame shell**: each frame's border + header bar + title + drag hit-area
  remain persistent world-space Pixi objects, instantiated once and updated in
  place. Pan is still `world.position.set()` only; zoom is `world.position` +
  `world.scale` + in-place stroke repaint.

**Retired (removed from the production `Canvas.svelte` interior path):**

- `src/lib/bc-layout.ts` — the deterministic spring-electrical BC-bubble layout
  (ADR-015 canvas-layout consequence). The interior is now a vertical accordion,
  not a force-directed bubble cloud. (See "Relationship to ADR-015" — only the
  layout consequence is retired; the README-frontmatter relationship data model
  is NOT.)
- The **Pixi BC-bubble draw path** in `Canvas.svelte` (`BcDisplayObjects`,
  `updateBcDisplayObjects`, the `bcs` / `bcsRoot` members of
  `FrameDisplayObjects`). BCs are now DOM accordion sections inside the DOM
  interior.
- The **Pixi intra-project-edge draw path** in `Canvas.svelte` (the single
  per-frame `edges` Graphics, arrowheads, ACL notches). Intra-project BC↔BC
  relationship rendering, if it returns, returns as a DOM/SVG concern inside the
  interior — it is not part of this substrate decision.

## Relationship to ADR-015

ADR-015 carries two **separable** consequences. This ADR supersedes one and
preserves the other; the distinction is load-bearing:

- **Canvas-layout consequence — SUPERSEDED by this ADR.** ADR-015's decision
  that BCs are laid out as force-directed spring-electrical bubbles inside the
  frame via `src/lib/bc-layout.ts` is superseded: the kanban-accordion interior
  replaces the bubble cloud entirely. `bc-layout.ts`, the off-frame-pin edge
  case, the deterministic seeding, and the sticky-pin-on-drag-end contract for
  *bubble positions* all retire with it.
- **README-frontmatter relationship data model — NOT superseded; stays
  Accepted.** ADR-015's reliance on the per-BC `relationships:` block parsed
  from each BC's README frontmatter (the ADR-014 data model, owned by
  `project-registry-004`) is unaffected. That data model continues to exist and
  is consumed by whatever renders BC↔BC relationships in the new interior, and
  by the context-map tooling. The **data** survives; only its **force-directed
  layout consumer** retires.

ADR-015 is therefore marked **Superseded-in-part** (canvas-layout consequence
only), with this ADR as the superseding authority. A backlink note is added to
ADR-015's body. The index list and cross-link bookkeeping are the
orchestrator's responsibility.

## Empirical basis

This ADR ratifies hybrid on the strength of the `canvas-019a` perf spike
(`.agentheim/knowledge/research/canvas-hybrid-perf-2026-05-24/`), which built a
throwaway instrumented hybrid harness (`src/routes/spike-019a/`) reproducing
the worst case under decision #1 and measured it on a release build.

**VERDICT: PASS (release build, 2026-05-24).** Measured numbers cited:

- Worst-case sustained pan-circle — **N = 10 frames, all interiors mounted at
  default zoom, 5 BCs × 3 columns × 8 cards = 120 DOM cards/frame (1200 cards
  total)** — holds **`p95 = 8.5 ms`**. This is under the 16 ms (60 FPS) bar and
  at the 8 ms headroom target (the canvas-015 AC #10 target).
- The recovery-knob sweep to **3 cards/column returned the same 8.5 ms** →
  per-pan cost is **independent of card density**: hybrid scales with **frame
  count, not card count**, confirming the analytical prediction (pan never
  rebuilds Pixi and never re-creates DOM; it restyles ≤9 interior roots'
  `left/top/transform`, a compositor move, not a 1200-card reflow).
- The earlier `pnpm tauri dev` reading of **33.3 ms / 42 FPS was ~4× dev-mode
  overhead** (Svelte + Pixi dev builds + Vite HMR), retracted — it is not a
  hybrid cost. The provisional dev-mode FAIL does not stand.
- Both cost governors were validated live in the harness: viewport culling
  (`CULL_MARGIN_PX = 120`) and zoom-threshold LOD (`LOD_ZOOM_FLOOR = 0.45`),
  with `transform: scale(z)` keeping the 120-card layout frozen as a compositor
  transform (no reflow).

Per the spike's verdict rule, PASS ⇒ ratify hybrid and cite these numbers;
Option B (full-DOM) is not needed and stays the costed fallback. Had the spike
reported FAIL against its target with the recommended thresholds and recovered
nothing by tuning, this ADR would instead have ratified Option B and recorded
why. It PASSED, so hybrid stands.

## Consequences

- (+) Preserves the most prior investment — the entire ADR-016 / canvas-015
  persistent scene graph + camera transform, plus `camera.svelte.ts`,
  `drag-controller.ts`, `tile-layout.ts` — for the camera and frame shell. Only
  the interior swaps from Pixi to DOM.
- (+) Matches ADR-003's stated architecture; this is its realisation, not a
  reversal.
- (+) Native browser primitives for the interior: scroll, text wrapping /
  selection, CSS-transition accordion + panel slide, focus / keyboard, and the
  `tokens.css` overlay layer — none reimplemented.
- (+) The only real risk (N DOM interiors) is pushed onto viewport-culling +
  zoom-LOD, which are standard, contained, and empirically validated at the
  N≈10 target.
- (–) Two rendering technologies coexist (Pixi shell + DOM interior) and must
  stay in lockstep through the shared camera state — the `worldToScreen`
  positioning + `transform: scale(z)` contract is the seam that keeps them
  aligned, and it must be respected by every interior task.
- (–) A newly-mounted 120-card interior (a frame scrolling in from off-screen)
  can spike a single paint frame; mitigations (`content-visibility: auto`,
  staggered mount, larger `CULL_MARGIN_PX`) are tuning fixes, not
  hybrid-disqualifiers.
- (–) Retiring `bc-layout.ts` and the Pixi BC-bubble + edge draw paths is real
  deletion of working code (canvas-007 / canvas-015 interior work); the camera
  + shell half of that investment is preserved.

## Reversibility

Medium. The `camera.worldToScreen` + `transform: scale(z)` overlay seam is the
same contract ADR-003 / ADR-016 established, so the interior's substrate is
swappable behind it: if the N-DOM-interior cost ever regresses past tuning, the
costed Option B (full-DOM) is the documented fallback, and the camera
abstraction is portable to it. The retired `bc-layout.ts` and bubble draw paths
are recoverable from history if the bubble interior were ever revived.

## References

- `src/lib/Canvas.svelte` — the rendering surface (frame shell stays Pixi;
  interior becomes DOM in canvas-020/021/022).
- `src/lib/camera.svelte.ts` — Camera state + `worldToScreen` (the survivor the
  hybrid overlay positioning leans on).
- `src/lib/drag-controller.ts` — three-kind drag state machine (survivor).
- `src/lib/tile-layout.ts` — deterministic frame-origin auto-placement (survivor).
- `src/lib/bc-layout.ts` — retired (canvas-layout consequence of ADR-015).
- `.agentheim/knowledge/research/canvas-hybrid-perf-2026-05-24/README.md` — the
  `canvas-019a` perf spike findings note (the empirical basis; VERDICT PASS).
- `src/routes/spike-019a/` — the throwaway hybrid harness (retire via canvas-024
  now that this ADR has consumed the note).
- ADR-003 — PixiJS v8 + HTML overlays (the parent decision this extends).
- ADR-016 — persistent scene graph + camera-as-stage-transform (the investment
  this preserves for the shell).
- ADR-015 — BC layout (canvas-layout consequence superseded-in-part by this ADR;
  relationship data model preserved).
- ADR-002 — Svelte 5 + SvelteKit (the substrate both Pixi and the DOM overlay
  layer mount inside; keyed `{#each}` is what makes interiors reconcile cheaply).
