---
id: canvas-014
title: Investigate canvas pan/zoom sluggishness — find the per-frame cost
status: done
type: spike
context: canvas
created: 2026-05-17
completed: 2026-05-18
commit: eea3d09
depends_on: []
blocks: []
tags: [performance, rendering, pixi, gpu, profiling, spike]
related_adrs: [ADR-003, ADR-015]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-002, canvas-007]
---

## Why

Hands-on verification on 2026-05-17: pan and zoom on the canvas feel
sluggish — the camera drags as if every frame is costing significantly
more than a 60-FPS budget. Marco's suspicion: the GPU may not actually be
in use, or expensive work is happening per frame in the main thread.

The canvas is the primary view; "ambient overview" requires that pan and
zoom feel instantaneous. A laggy canvas undermines the vision-level
promise. We do not yet know the cause — measurement first, fix second.

## What

This is a **spike**. Output is a research report — not feature code —
plus follow-up tasks for whatever the report surfaces. Any trivial wins
(a stray RAF loop, a missing `app.renderer.resolution`, an obvious
per-frame rebuild) may land in this same task; non-trivial fixes get
their own backlog entries.

### Investigation surface

- **Which renderer is actually active?** PixiJS v8 supports WebGL,
  WebGPU, and a canvas fallback. Confirm what's running on Marco's
  machine (read `app.renderer` type + GPU info). A canvas-fallback or a
  software path would explain everything in one line.
- **Per-frame `renderScene` cost during pan.** How often is `renderScene`
  invoked while the camera is moving? Is it once per pan event, once per
  RAF tick, once per pointermove? Profile main-thread time per call with
  Chrome devtools.
- **What does `renderScene` actually rebuild?** Are Pixi Graphics for
  every project frame and BC bubble being torn down and re-instantiated
  each call, or are they persistent and only re-positioned via the stage
  transform? Audit `Canvas.svelte`'s render function.
- **HTML overlay reflow cost.** Context menus, modals, toasts, and the
  theme toggle all live in the ADR-003 overlay layer. Confirm none of
  them reflow on every pan tick.
- **Stage-transform path vs full re-render.** Pan should ideally be a
  single `app.stage.position` update plus a draw call, not a re-build of
  every child. Confirm.
- **`hitArea` and event hit-testing.** PixiJS hit-testing without an
  explicit `hitArea` can walk the entire scene graph on every pointer
  event. With N projects × M BCs, this is a candidate hotspot.
- **Resolution and DPI.** `app.renderer.resolution` should match
  `devicePixelRatio`. Mismatches cause either blur (too low) or wasted
  fillrate (too high). Capture the current value and the display's DPR.
- **Theme-flip cost (less urgent).** `design-system-004`'s `onThemeChange`
  calls `renderScene` and resets `renderer.background.color`. Measure
  this isn't accidentally running on every frame.

### Suggested measurement protocol

- Open Chrome devtools' Performance tab while panning continuously for
  ~5 seconds with the canvas at its default zoom and a representative
  scene (Marco's actual projects, or seed N=10 frames).
- Record frame times, scripting time, rendering time, GPU activity.
- Snapshot the same with the JS console: `app.renderer.type`,
  `app.renderer.resolution`, `window.devicePixelRatio`, ticker FPS.

### Frame-time targets

- < 16ms per frame at default zoom with 10+ project frames (60 FPS sustained).
- < 8ms per frame is the "feels instant" target with headroom.

## Acceptance criteria

- [ ] Research report committed to `.agentheim/knowledge/research/canvas-perf-2026-05-17/`. Contents at minimum:
  - Active renderer (WebGL / WebGPU / canvas fallback) and GPU info.
  - Current `renderer.resolution` and `devicePixelRatio`.
  - Per-frame render cost during pan, with at least one Chrome performance trace embedded or referenced.
  - Top three identified hotspots, ranked by main-thread cost.
  - Proposed fix per hotspot, with rough effort estimate.
- [ ] Reproducer documented: project count, zoom level, interaction that triggers the worst lag — so future regression checks have a baseline.
- [ ] Follow-up backlog tasks captured in `canvas/backlog/` for every hotspot fix that's non-trivial.
- [ ] Any trivial wins landed in this same task (e.g., set `app.renderer.resolution = devicePixelRatio`, attach `hitArea` to project frames, kill a stray `requestAnimationFrame`).
- [ ] No regressions to behaviour — pan, zoom, drag (frame + BC + empty canvas), focus, theme-flip, modals, context menus, missing-tile rendering all still work.
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- ADR-003 (PixiJS v8 + HTML overlays) and ADR-015 (deterministic BC
  layout) frame the solution space. The BC layout is already one-shot
  (no RAF loop), so the per-frame cost should be coming from somewhere
  else — `renderScene`, the camera transform, or the overlay layer.
- Coordinate with `canvas-013` (crisp rendering + constant-size project
  titles) if both run concurrently. Switching project titles to HTML
  overlay vs Pixi text vs BitmapText all have different per-frame
  profiles — the data from this spike should inform the canvas-013
  strategy decision (and conversely, that decision's profile should
  show up in this spike's baseline if 013 ships first).
- Out of scope: actually fixing every hotspot. The spike's job is
  measurement + follow-up task capture. Trivial wins are a courtesy
  bonus, not the goal.

## Outcome

Research report at `.agentheim/knowledge/research/canvas-perf-2026-05-17/README.md`
identifies the dominant per-frame cost — `app.ticker.add(() => renderScene())`
was rebuilding the entire scene graph (`world.removeChildren()` + ~200–500
fresh Pixi `Graphics` / `Text` / `Container` allocations for N=7 frames × ~5
BCs each) ~60 times per second, idle and during gestures alike. Three
ranked hotspots and four follow-up tasks captured.

**Trivial wins landed in this task** (in `src/lib/Canvas.svelte`):
- Ticker callback guarded by `if (cameraTarget)` so it only steps the
  eased camera transition; steady-state per-frame cost drops to zero.
- Explicit `window.addEventListener('resize')` replaces the incidental
  resize-coverage the unconditional ticker rebuild provided.
- Explicit `renderScene()` call added to the targeted-event default
  branch (`task_*` / `bc_*`) so count ticks and BC topology changes still
  visually update with the ticker dormant. This was load-bearing
  correctness, not just optimisation.

**Renderer-config gap NOT touched** (deliberate, in canvas-013 scope):
`app.init` at `Canvas.svelte` L533 still omits `resolution` /
`autoDensity`; canvas-013 owns that fix as its AC #1.

**Follow-up backlog tasks** (under `.agentheim/contexts/canvas/backlog/`):
- `canvas-015` — Persistent scene graph + camera-as-stage-transform pan
  (the substantive renderer rework; depends on canvas-014)
- `canvas-016` — Cache screen-space overlays (voice indicator + future
  agent-awareness badges)
- `canvas-017` — Broad-phase hit rejection (`world.eventMode='passive'` +
  `world.hitArea` of scene bounds)
- `canvas-018` — Dev-only diagnostic seam (`window.__guppi` in dev
  builds) so future spikes read renderer state from the console without
  source-patching globals

**Measurement protocol**: the report includes a Marco-runnable
reproducer protocol (Chrome devtools Performance trace + JS console
snapshot) because the Tauri dev shell cannot be driven from this
worker's shell. Operator confirmation is needed to (a) confirm the
active renderer (expected WebGL; canvas-fallback would change the
severity calculus) and (b) smoke-test that `task_*` events still
visually update with the ticker dormant.

**Checks**: `pnpm check` clean (940 files, 0/0/0); `cargo test --lib`
122 passed (unchanged from baseline).

**ADRs**: none new. ADR-003 (PixiJS + HTML overlays) and ADR-015
(one-shot BC layout) both hold unchanged; the report extends ADR-003's
"GPU does the camera transform" path is the right model and stages the
work to actually get there (canvas-015).

**Key files**:
- `.agentheim/knowledge/research/canvas-perf-2026-05-17/README.md`
- `src/lib/Canvas.svelte` (L1300–1330 region)
- `.agentheim/contexts/canvas/README.md` ("Rendering N projects" section)
- `.agentheim/contexts/canvas/backlog/canvas-{015,016,017,018}-*.md`
