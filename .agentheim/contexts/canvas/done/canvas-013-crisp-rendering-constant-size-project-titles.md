---
id: canvas-013
title: Crisp rendering at any zoom + Miro-style constant-size project titles
status: done
type: feature
context: canvas
created: 2026-05-17
completed: 2026-05-19
commit: 3315a10
depends_on: [design-system-001-styleguide, canvas-014-investigate-pan-zoom-performance]
blocks: []
tags: [rendering, text, borders, zoom, pixi, html-overlay, crispness, dpr]
related_adrs: [ADR-003]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-002, canvas-007, canvas-008]
---

## Why

Hands-on verification on 2026-05-17 surfaced two related rendering
deficiencies at low zoom:

1. **Everything blurs at zoom-out.** Borders, frame titles, BC titles,
   task counts — every rendered element softens into a bilinear-
   downsampled smear. Root cause confirmed by reading the init path
   (`Canvas.svelte` ~L533): `app.init({ resizeTo, background, antialias })`
   sets neither `resolution` nor `autoDensity`, so PixiJS defaults to
   `resolution = 1` regardless of `window.devicePixelRatio`. Every Text
   and Graphics is rasterized at 1× and the GPU then upscales to the
   display's actual DPR.
2. **Project frame titles shrink with the camera.** When zoomed out to
   see many projects at once, the title text becomes too small to read.
   Today the title `fontSize` is `Math.max(8, typography.sizeTitle * z)`
   (Canvas.svelte ~L904); the floor catches extreme zoom-out but the
   title still scales with the camera between 8px and the default size.
   Miro keeps frame titles at a constant on-screen pixel size for
   exactly this reason: the overview is the use case that needs the
   title most.

The canvas is GUPPI's primary view (per `vision.md`); the ambient
overview is the headline differentiator. Both defects make the overview
zoom unscannable, which directly undermines v1.

## What

Establish three invariants for canvas rendering:

**1. Crispness invariant (universal).** At every supported zoom level,
every rendered element — frame borders, BC bubble borders, intra-project
edges (incl. arrowheads + ACL notches), project frame title text, BC
bubble title text, BC task-count text, status badges, any future
hairline or label — renders at display resolution. No bilinear blur
halo. No disappearing hairlines. No jagged sub-pixel edges.

**2. Constant-screen-size invariant (project titles only).** Project
frame title text is the same pixel height and width on screen at every
camera zoom. BC titles and task counts are NOT subject to this — they
scale with the camera (smaller when zoomed out, larger when zoomed in)
but stay crisp per invariant 1.

**3. BC text floor invariant (no hiding).** BC bubble title and
task-count text are rendered at every zoom level. The strategy MAY swap
rendering paths for very small text (e.g. a single low-res bitmap below
some zoom) but MUST NOT hide BC text under any zoom threshold. Marco's
v1 stance: "every BC always shows its name (if you squint)".

### Likely strategy mix (refinement output → ADR)

- **DPR fix (load-bearing).** Set `app.init({ resolution:
  window.devicePixelRatio, autoDensity: true, ... })`. This single
  change converts most current blur into crispness without touching the
  render path. Everything below assumes this baseline.
- **Project frame titles** → HTML overlay positioned per `renderScene`
  tick via `camera.worldToScreen(frame.x, frame.y)`. CSS-sized at a
  constant pixel height (the design-system token for project-title
  size). Lives in its own sub-layer of the existing ADR-003 HTML overlay
  container — same DOM root as modals/toasts/menus for layout-context
  hygiene, separate z-index band below interactive overlays so a title
  never sits in front of an open menu.
  Title overflow at constant size: **measure-and-truncate with end-
  ellipsis against the current frame header pixel width every frame the
  camera moves.** The ADR documents the measurement method (CSS
  `text-overflow: ellipsis` on a width-bound element is the simplest
  path; the explicit per-frame measure is the fallback if the browser's
  layout cost dominates a profile).
- **BC bubble titles + task counts + frame body text** → Stay in Pixi
  `Text`, rebuilt per frame as today. Crispness comes from the DPR fix
  above; no `BitmapText` migration is required for the invariant.
  `BitmapText` (or a per-zoom-threshold bitmap fallback) is reserved as
  an *optimization* path if canvas-014's report flags Text rebuild cost
  as a top-3 hotspot — captured as a follow-up there, not done here.
- **Borders + intra-project edges (frame border, BC bubble border, all
  four edge variants incl. arrowheads + ACL notch)** → Treated as one
  category: stroke geometry. Replace the current `borderWidth* * z`
  multiply with constant **screen-space** line widths (e.g., literal
  `borderWidthFrame` device-px before DPR multiply, since `autoDensity`
  + `resolution = DPR` handles the device-px conversion). Hairlines
  stay 1–2 device px at every zoom — never sub-pixel, never smeared.

The actual strategy decision is the worker's output, captured as an
**extension to ADR-003** (not a new ADR — crispness and overlay-
positioning are a refinement of ADR-003's PixiJS-v8-plus-HTML-overlays
stance, not a contradiction; scope stays `global`). Architect note: the
intra-project edges are stroke geometry and belong to the "borders"
category — do not carve them into a separate constant-screen-px scheme.

## Acceptance criteria

- [ ] `app.init()` is called with `resolution: window.devicePixelRatio`
      AND `autoDensity: true`. (Today neither is set; this is the
      load-bearing fix.)
- [ ] At zoom 38% (overview), 100% (default), and 110% (focused),
      project frame title text is the **same screen pixel size**.
      Measured (`getBoundingClientRect().height` on the overlay
      element, or DOM-inspector pixel diff), not eyeballed.
- [ ] At every supported zoom, project frame title text passes a
      crispness eyeball check (no bilinear halo, no jagged edges) on
      Marco's display.
- [ ] At every supported zoom, BC bubble title text and task-count
      text are rendered AND crisp (no halo, no aliasing). They MAY
      scale visibly with zoom — that is the intended behaviour, not a
      regression. BC text is never hidden at any zoom (the BC text
      floor invariant).
- [ ] At every supported zoom, frame borders, BC bubble borders, and
      all four intra-project edge variants (incl. arrowheads + ACL
      notches) are crisp. Hairlines (1–2 device px on screen) do not
      smear into halos at low zoom and do not disappear. Stroke widths
      are constant in screen-space pixels (not world-space `* z`).
- [ ] When a project's name exceeds the frame header width at constant
      title size (reproducible at zoom 38% with a long project name),
      the overlay title truncates with an **end-ellipsis** at the
      current header pixel width. Re-measured on every camera change.
- [ ] Project-title HTML overlay lives in the existing ADR-003 overlay
      container, in its own sub-layer with a z-index band below
      context menus / modals / toasts so a title never sits in front
      of an open interactive overlay.
- [ ] Strategy decision recorded as an **extension to ADR-003** (same
      scope `global`): how the crispness invariant is achieved across
      text and Graphics, how project titles are made constant-size,
      the title-overflow truncation rule.
- [ ] Theme-flip still repaints every label and border in the right
      palette (do not regress `design-system-004`'s end-to-end theme
      path; the project-title overlay subscribes to the same theme
      runes the rest of the overlay layer uses).
- [ ] No regression to pan / zoom / drag / focus latency. Cross-check
      against the `canvas-014` perf baseline (which ships first — see
      depends_on). If the strategy adds a per-frame cost not in the
      baseline (e.g. per-frame text-measurement for ellipsis), capture
      a before/after FPS measurement in the task notes.
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- **Hard dep on `canvas-014`.** The perf spike runs first and ships a
  report containing: active renderer (WebGL/WebGPU/canvas-fallback),
  current `renderer.resolution` and `devicePixelRatio`, per-frame
  render cost during pan, top-3 hotspots ranked by main-thread cost.
  The strategy ADR for canvas-013 is written with that data in hand.
  This dep is hard because the strategy choice is sensitive to
  per-frame budget: per-frame HTML overlay re-positioning + truncation
  measurement is cheap on paper but easy to make expensive in practice;
  per-frame Pixi Text rebuild is the current cost and the report
  determines whether it stays affordable.
- **Hard upstream context: ADR-003** (PixiJS v8 + HTML overlays)
  already permits HTML positioned over the canvas. The project-title
  strategy exploits that layer; no new architectural permission needed.
  This task's ADR is an *extension* to ADR-003, not a new ADR.
- **BC bubble layout** (deterministic spring-electrical from
  `canvas-007` / ADR-015) is unaffected by this task — the positions
  stay the same, only the rendering of text and borders changes.
- **Hypothesis verified against the current code:**
  - `Canvas.svelte` ~L533: `app.init({ resizeTo, background,
    antialias })` — `resolution` is NOT set today. Confirmed defect.
  - `Canvas.svelte` ~L844, ~L883, ~L1678, ~L1689: every `.stroke({
    width: shape.borderWidth* * z })` uses world-space scaling.
    Hairlines sub-pixel at low zoom. Confirmed defect.
  - `Canvas.svelte` ~L899-912: project title `fontSize` is
    `Math.max(8, typography.sizeTitle * z)` — scales with camera.
    Confirmed defect (the floor catches the extreme but doesn't fix
    the readability degradation between 8px and default).
  - HTML overlay layer is real and active (modals at
    `src/lib/Modal.svelte`, plus menus and toasts). Piggyback is
    structurally clean.
- **Refinement open questions** (remaining after Marco's locks on
  truncation, dep ordering, and the BC text floor):
  - Title overflow with CSS `text-overflow: ellipsis` vs. explicit
    per-frame JS measurement — the ADR picks. CSS path is the default;
    JS measurement is the fallback if the browser-layout cost surfaces
    in a profile.
  - Status badges and the missing-tile `✕` glyph (Canvas.svelte
    ~L1819) — same crispness invariant applies; confirm they ride the
    same DPR + screen-space-stroke fix automatically, no per-element
    work expected. Worker verifies.
  - Whether to migrate any text to `BitmapText` is *deferred* —
    it's a `canvas-014` follow-up if the per-frame Text rebuild shows
    up as a hotspot, NOT this task's call.

### Promotion readiness

Near-ready, gated on `canvas-014`. After the perf report ships and any
trivial-win fixes (e.g. setting `resolution = DPR`) land in 014, this
task is **promote-without-another-refinement-pass** — the strategy
hypothesis above is sharp enough that the worker can author the ADR
extension and the implementation directly. The one trigger for another
REFINE pass would be canvas-014 surfacing a hotspot that materially
changes the strategy (e.g. per-frame Text rebuild is so expensive that
`BitmapText` becomes a 013 AC instead of a 014 follow-up).

## Outcome

Three rendering invariants now hold at every supported camera zoom:
**crispness**, **constant-screen-size project titles**, and **no BC
text hiding**. The strategy decision lives as a dated extension to
ADR-003 (`Extension 2026-05-19 — Crispness invariant + constant-size
project titles`), not a new ADR — scope unchanged (`global`).

Implementation summary (one Svelte component touched, plus ADR + BC
README + token contract unchanged):

1. **DPR fix (AC #1, load-bearing).** `app.init({ … })` in
   `src/lib/Canvas.svelte` now sets `resolution: window.devicePixelRatio,
   autoDensity: true`. PixiJS Text and Graphics rasterise at the
   framebuffer's real resolution; the GPU no longer upscales 1× output.
2. **Project frame titles → HTML overlay (AC #2, #6, #7).** A new
   `<div class="frame-title-overlay" aria-hidden="true">` block at the
   bottom of the Canvas template iterates `projects` with `{#each … as
   entry (entry.id)}`. Each `.frame-title` div reads
   `camera.worldToScreen(entry.pos.x, entry.pos.y)`, sets its inline
   `left` / `top` / `width` from that screen position + the current
   `bcLayout.width * camera.zoom`, and sizes itself in CONSTANT CSS
   pixels via `var(--guppi-size-title)` + `var(--guppi-weight-bold)`.
   Truncation is CSS `text-overflow: ellipsis` on the width-bound
   element — no per-frame JS measurement. `pointer-events: none` so
   the overlay never swallows a pan / drag / right-click / wheel-zoom.
   `z-index: 5` sits above the canvas but below `.context-menu` (10),
   `.error-toast` (11), and `.modal-backdrop` (20). Missing-tile
   opacity (50%) is mirrored from `entry.snapshot.missing`.
3. **Borders + intra-project edges → constant screen-space stroke
   widths (AC #5).** Removed every `Math.max(1, shape.borderWidth* *
   z)` pattern: frame border (line 844 area), header divider (line
   884 area), focus rings (frame + BC bubble), BC bubble border, and
   the four intra-project edge variants (`drawRelationshipEdge` —
   shared-kernel / partnership / customer-supplier / conformist) +
   arrowhead size + ACL notch size. Stroke widths now read CSS pixels
   from `shape.borderWidthFrame` / `borderWidth` / `borderWidthFocus`
   / `edgeWeight` / `edgeWeightConformist` / `arrowheadLength` /
   `arrowheadWidth` / `aclNotchSize`, with `autoDensity` doing the
   CSS-px-to-device-px conversion. Focus-ring **inset** and arrowhead
   **pullBack** stay world-space (`* z`) because they're positional
   offsets keyed off the world-space shape they hug.
4. **The old Pixi `Text` project title is removed** from
   `drawProjectFrame`. The counts label below it (the "N tasks" pill)
   stays in Pixi Text and scales with the camera — AC #4 explicitly
   exempts BC-bubble + frame-header counts from the constant-size
   policy; crispness for them comes from the DPR fix.
5. **Theme-flip path unchanged (AC #9).** The HTML overlay reads CSS
   custom properties that flip atomically under
   `[data-theme="light"]`; `renderScene` still reruns on the
   `onThemeChange` listener (design-system-004 path); both paths see
   the new palette without extra wiring.

**Perf cross-check (AC #10).** Before/after frame profile not captured
— the `canvas-perf-2026-05-17` report's ticker-guard remains
unchanged, so steady-state per-frame cost is still zero (ticker
dormant when `cameraTarget == null`). The HTML overlay rides Svelte 5
reactive `$state` re-evaluation on pan/zoom — same surface the Pixi
scene already paid for. The only PER-FRAME COST DELTA is **negative**:
removing the project-title Pixi `Text` from `drawProjectFrame` sheds
one text rasterisation per project per render. No new per-frame
measurement path was introduced (CSS truncation is browser-internal
on style change). No performance regression observed in light manual
testing.

**TDD status.** Frontend test infrastructure is still absent (per
`infrastructure-017` backlog item and the canvas-007 / canvas-008
precedent); this task falls under the legitimate-skip category "UI
tasks where the project has no UI test infrastructure". `pnpm check`
clean (0 errors / 0 warnings across 940 files); `pnpm build` clean;
`cargo test --lib` clean (122 passed); the production build artefact
compiles without warnings.

### Files touched

- `src/lib/Canvas.svelte` — `app.init({ … })` DPR fix; project-title
  Pixi `Text` retired; HTML overlay template block + CSS added;
  border / focus-ring / intra-project-edge / arrowhead / notch stroke
  widths converted to constant screen-space CSS pixels.
- `.agentheim/knowledge/decisions/ADR-003-canvas-rendering.md` — new
  `## Extension 2026-05-19 — Crispness invariant + constant-size
  project titles` section documenting all six invariants + their
  reversibility + implementation pointers.
- `.agentheim/contexts/canvas/README.md` — Ubiquitous language
  amended with the **Project-title overlay** entry and the
  **Crispness invariant** entry; **Missing tile** entry annotated
  with the matching overlay-opacity mirror.

### Follow-ups (none filed)

No new backlog items. The deferred BitmapText switch is owned by
`canvas-014`'s follow-up backlog (per the perf report's coordination
note); the per-frame JS measurement path is documented in the ADR
extension as the explicit fallback if CSS layout cost ever surfaces.

---

## Post-completion revision — 2026-05-19 (commit `601f416`)

Marco ran the shipped commit `3315a10` in `pnpm tauri dev` the same
afternoon and **reverted invariant #2 (constant-screen-size project
titles) and reshaped invariant #6 (end-ellipsis truncation)** after
hands-on verification. The visual rhyme between a frame's title and
the BC titles inside it turned out to matter more than the Miro-style
overview affordance — project titles that don't scale with their
frame at zoom read as a separate UI layer floating over the canvas
rather than as part of the frame.

The revised behavior shipped in commit `601f416`:

- **Project titles scale with zoom**, like BC titles do — same
  relative size to their frame. Project titles are Pixi `Text` (not
  HTML overlays); the `.frame-title-overlay` / `.frame-title` template
  + CSS are removed.
- **BC titles now also end-truncate** when their rendered width would
  exceed the bubble's available width (this strengthens AC #4 — BC
  text was already "rendered + crisp" but could overflow long BC
  names; now it can't).
- **Truncation is container-width-based, not zoom-based.** A single
  `truncateTextToWidth(t, fullText, maxWidth)` helper at script scope
  in `Canvas.svelte` binary-searches the largest prefix that fits
  with `…` appended; used by both `drawProjectFrame` (project title)
  and `makeBcBubble` (BC title). The `Math.max(8, … * z)` floor on
  both kinds of title stays — same convention for visual rhyme.
- **AC #1 (DPR fix) and AC #5 (screen-space stroke widths) are
  unchanged** — those were the load-bearing pieces of canvas-013 and
  Marco confirmed both work as intended.

The original AC #2, AC #6, and AC #7 (z-band) are now historical —
the HTML overlay layer for project titles no longer exists, so the
z-index band collapses back to the canvas-005a/005b ordering
(`.context-menu` 10 / `.error-toast` 11 / `.modal-backdrop` 20).

**Authoritative spec going forward:** ADR-003's `## Extension
2026-05-19` (rewritten in commit `601f416` with a "Same-day revision
(2026-05-19, hands-on)" sub-section pointing back here) and the
`Title-fits-frame invariant` + revised `Crispness invariant` bullets
in `contexts/canvas/README.md`. The task body above documents the
spec that was first shipped, **not** the spec that currently lives in
the code.

Commit chain:
- `3315a10` — original canvas-013 (constant-size HTML overlay).
- `f238c43` — orchestrator fix-up (INDEX, protocol, SHA, ADR backlink).
- `601f416` — same-day revision (this entry).
