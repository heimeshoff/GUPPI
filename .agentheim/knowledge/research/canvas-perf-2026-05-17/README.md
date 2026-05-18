---
slug: canvas-perf-2026-05-17
title: Canvas pan/zoom sluggishness — per-frame cost investigation
type: research
created: 2026-05-18
authored_for: canvas-014
context: canvas
related_adrs: [ADR-003, ADR-015]
tags: [performance, rendering, pixi, profiling, spike]
---

# Canvas pan/zoom sluggishness — per-frame cost investigation

Spike task: `canvas-014-investigate-pan-zoom-performance` (2026-05-17).
Hands-on symptom: pan and zoom on the canvas feel sluggish — camera drags
as if every frame is far over the 60 FPS budget.

This report is the static-analysis half of the investigation. The
in-browser measurement half (Chrome trace + JS console snapshot) is
documented below as a reproducer protocol; the Tauri dev shell is
operator-driven (Marco) and is the source of truth for the actual
numbers. The report records the **expected** numbers, the structural
findings that produce them, and the fixes — with one trivial win
already landed in `Canvas.svelte` as part of canvas-014.

## TL;DR

**Single dominant hotspot**, identified by static analysis:

> The PixiJS ticker was unconditionally rebuilding the entire scene
> graph every frame (~60 Hz) via `app.ticker.add(() => renderScene())`,
> even when the camera was idle and no input was occurring. `renderScene`
> does `world.removeChildren()` followed by re-allocating every
> `Graphics`, `Text`, and `Container` for every project frame, every
> intra-project edge, every BC bubble, every status badge, every counts
> pill, plus the voice indicator. With N=10 frames carrying ~5 BCs each
> that is **≈ 250+ Pixi object allocations per frame, 60 times per
> second**, on top of GC pressure for the discarded children.

**Landed in this spike (trivial win)**: the ticker callback is now
guarded by `if (cameraTarget)`. Steady-state per-frame cost from the
ticker rebuild drops to zero. Window-resize re-projection is now
served by an explicit `resize` listener. Pan, wheel, drag, and hover
already called `renderScene()` explicitly on their own, so the
interactive feel during gestures is unchanged — only the *idle and
between-gesture* cost was waste. Theme flips and event-driven topology
changes also still call `renderScene()` explicitly.

Three additional hotspots remain, each spun out as a backlog task:

1. **`renderScene` rebuilds the entire scene every call** —
   `canvas-015-persistent-scene-graph-and-stage-transform-pan`
2. **The voice indicator allocates two Pixi objects on every render** —
   `canvas-016-cache-screen-space-overlays`
3. **No `hitArea` on the frame body / world container; Pixi hit-testing
   walks the scene graph on every pointer event** —
   `canvas-017-broad-phase-hit-rejection-for-empty-canvas-pan`

A fourth concern — `renderer.resolution` / `autoDensity` not being set,
producing blur on HiDPI displays — is **explicitly already** in
`canvas-013`'s acceptance criteria (AC #1) and is **not** touched here
to avoid scope drift. Static analysis confirms the gap at
`Canvas.svelte` L533 — see "Renderer configuration" below.

## Active renderer (expected) — to be confirmed in operator console

PixiJS v8's `Application.init` auto-selects a renderer. The init call at
`Canvas.svelte` L533 passes no `preference` option:

```ts
app = new Application();
await app.init({
    resizeTo: host,
    background: color.canvasBg,
    antialias: true
});
```

Per PixiJS v8's documented `Application.init` contract, with no
`preference`, the runtime probes for WebGL first (WebGPU is opt-in via
`preference: 'webgpu'`), falling back to the canvas2D renderer only if
WebGL acquisition fails. On Marco's Windows 11 dev machine with a
working browser-engine `WebView2`, the expected active renderer is
**WebGL** — the v1-correct path per ADR-003.

**Operator confirmation step** (record actual values in `console-snapshot.md` next to this report):

```js
// In the Tauri devtools console while `pnpm tauri dev` is running:
const app = /* obtain Application instance — exposed via a debug global in a follow-up */;
console.log('renderer.type     =', app.renderer.type);            // expect 2 (WebGL) or 4 (WebGPU)
console.log('renderer.resolution =', app.renderer.resolution);    // expect 1 on default init
console.log('devicePixelRatio  =', window.devicePixelRatio);      // typically 1, 1.25, 1.5, 2
console.log('ticker.FPS        =', app.ticker.FPS);               // expect ≈60 in steady state
console.log('renderer.gl?.getParameter(?.UNMASKED_RENDERER_WEBGL)'); // GPU string
```

If the actual `renderer.type` returns the **canvas-fallback** number
(PixiJS v8: `CANVAS` = 1), every line of this report's analysis still
applies — but the symptom severity is multiplied 5–10x because canvas2D
re-rasterises everything on the CPU. Marco's hands-on symptom is
consistent with both the over-rebuilding-on-WebGL case and the
canvas-fallback case; the console snapshot disambiguates.

Marco — when next at the machine, please paste the four `console.log`
outputs into `console-snapshot.md` in this directory. If `renderer.type`
is `1` (canvas fallback), promote `canvas-018` (not yet spawned)
to investigate the WebGL acquisition failure.

## Renderer configuration — DPR

`Canvas.svelte` L533 omits both `resolution` and `autoDensity` from
`app.init`. PixiJS defaults `resolution = 1` regardless of
`window.devicePixelRatio`. On a 1.25–2.0 DPR display every Pixi text
glyph and stroked rect rasterises at framebuffer resolution and then
the OS upscales the canvas element, producing the "crispness" gap
canvas-013 was filed for.

This is **NOT** a per-frame performance hotspot — it is a per-pixel
fidelity gap. It is canvas-013's AC #1 and is intentionally left for
that task. The reason: the correct fix is
`resolution: window.devicePixelRatio, autoDensity: true` plus a
re-think of how world-space sizes scale (`Math.max(8, size * z)` floor
guards already assume framebuffer pixels). canvas-013 owns the deep
think; canvas-014 only flags the gap here so a future reader does not
re-discover it as a perf issue.

## Per-frame `renderScene` cost during pan (analytical model)

`renderScene` (Canvas.svelte L561–589) does the following per call:

| Step | Cost class | Notes |
| --- | --- | --- |
| `world.removeChildren()` | O(K) where K = total children added last render | also drops references → GC churn |
| `drawIntraProjectEdges` per project | O(E) — one new `Graphics` per edge | each Graphics has `moveTo` / `lineTo` / `stroke` calls + arrowhead/notch triangles |
| `drawProjectFrame` per project | O(1) `Container` + 1–4 `Graphics` + 2 `Text` (title + counts) + optional missing glyph + optional focus ring | every Text creates a new text-atlas entry until cached |
| `makeBcBubble` per BC | 1 `Container` + 2 `Graphics` (body, optional focus ring) + 2 `Text` (name, counts pill) + 1 `Graphics` (pill bg) + 1 `Container` (badge w/ 1 `Graphics` + 1 `Text`) | ≈ 8 Pixi objects per BC bubble |
| `makeVoiceIndicator` | 1 `Container` + 1 `Graphics` + 1 `Text` | screen-space overlay, only depends on `app.renderer.width/height` and `voiceState` |

For Marco's typical scene of N=4–10 projects × ~5 BCs each:

- Edges: ~4 per project × N = 16–40 `Graphics`
- Project frames: ~5 Pixi objects × N = 20–50
- BC bubbles: ~8 Pixi objects × 5 × N = 160–400
- Voice indicator: 3 fixed

**Total: ~200–500 new Pixi objects allocated per `renderScene` call.**

`Text` is the most expensive per-allocation cost in Pixi v8 — every
construction triggers a text-atlas rasterise (paint to off-screen
canvas → upload texture) the first time a given string + style appears.
Pure repeats are atlas-cached but the per-`Text`-instance constructor
still walks the style object.

### What multiplies that by 60

`app.ticker.add(() => renderScene())` (was at L1323; now guarded). The
ticker is the PixiJS-managed RAF loop and fires every animation frame
the browser permits — ~60 Hz on a 60 Hz display. So in steady state,
with nothing happening, the canvas was paying **200–500 Pixi
allocations × 60 = 12 000–30 000 allocations per second**, all of which
are immediately orphaned by the next tick's `removeChildren()`. The GC
pause that results is the obvious source of "the camera drags".

### What multiplies that during a gesture

Pan moves the cursor → `pointermove` → `renderScene()` (interactive
path) → next tick → `renderScene()` (ticker path, before the fix) →
next pointermove → `renderScene()` (interactive) → next tick →
`renderScene()` (ticker). The interactive path was being silently
**doubled** by the ticker. After the fix the ticker is dormant and the
interactive path runs once per pan event.

## Hotspots — ranked

### Hotspot 1 — Ticker fires `renderScene` every tick (LANDED)

Severity: dominant. Per-frame cost: full rebuild × 60 Hz, idle and
during gestures alike. Effort: trivial. **Landed in canvas-014.**

Change (Canvas.svelte L1323–1326 → guarded variant):

```ts
// BEFORE
app.ticker.add(() => {
    if (cameraTarget) stepCameraTransition();
    renderScene();
});

// AFTER
app.ticker.add(() => {
    if (cameraTarget) {
        stepCameraTransition();
        renderScene();
    }
});
window.addEventListener('resize', () => {
    if (!disposed) renderScene();
});
```

Why this is safe:
- Pan, drag, wheel, hover, and event-driven topology changes all
  already call `renderScene()` explicitly — they did so before this fix.
- The previous ticker `renderScene()` was named in the in-line comment
  as "Re-render on PixiJS ticker so a window resize re-projects". The
  explicit `resize` listener covers that case more directly.
- The eased camera transition (`f` key zoom-to-fit) still drives
  `renderScene()` from the ticker as long as `cameraTarget` is set.
- The `bc_relationships_changed` handler comment at the old L1293
  mentioned "the ticker's `renderScene()` on the next frame"; static
  analysis confirmed the targeted-event default branch
  (`task_*` / `bc_*`) DID NOT call `renderScene()` itself and relied
  on the now-removed ticker rebuild. The canvas-014 patch therefore
  adds an explicit `renderScene()` call at the end of that branch,
  immediately after the (optional) `recomputeBcLayout` step. This is
  load-bearing — without it, task-count ticks, bc-appeared, and
  bc-disappeared events would silently stop visually updating the
  canvas. Both edits ship in the same patch.

### Hotspot 2 — Full scene-graph rebuild per `renderScene` call

Severity: high during interactive gestures (pan / drag / hover).
Per-event cost: 200–500 Pixi allocations + texture-atlas
look-ups + GC churn. Effort: medium-to-high.

The whole `renderScene` strategy is "tear down and rebuild". The
PixiJS-native model would be:

- One persistent `Container` per project frame, kept across renders.
- One persistent `Graphics` per edge / per body / per header / per
  pill, kept across renders, with `.clear()` + new draw commands only
  when geometry actually changes.
- One persistent `Text` per project title and per BC name, with its
  `position` updated on render but text/style only updated when the
  data changes.
- Pan and zoom drive `world.position` and `world.scale` (the camera
  transform on the parent container), NOT a redraw of every child at
  pre-transformed screen coordinates.

The current implementation does the camera transform manually in JS
(every `Graphics` is positioned at `camera.worldToScreen(...)` screen
coordinates rather than at world coordinates with the camera applied
via `world.position`/`world.scale`). That choice is the root reason a
rebuild is needed per pan — none of the existing Graphics survive a
camera change because they were drawn at the old screen coordinates.

This is the substantive fix and lives in
`canvas-015-persistent-scene-graph-and-stage-transform-pan`. Estimate:
1–3 days of focused work; touches `Canvas.svelte` extensively and
requires careful regression testing of drag, hit-testing, focus rings,
intra-project edges, and the missing-tile glyph.

### Hotspot 3 — Screen-space overlays allocate per render

Severity: low individually, but the voice indicator is the only
overlay still wired into world-space rendering today and it allocates
a fresh `Container` + `Graphics` + `Text` every render. Adding the
forthcoming overlays (status badges from `agent-awareness`, panels,
toasts) without a cache pattern will compound this.

Fix in `canvas-016-cache-screen-space-overlays`: instantiate the
indicator once on mount, update its `.position` and tint on render
(or via an effect that watches `app.renderer.width/height` +
`voiceState`). Estimate: ½ day.

### Hotspot 4 — Pointer hit-testing has no broad-phase rejection

Severity: medium during pan. PixiJS's hit-test walks the display
tree to find interactive children unless a parent declares
`hitArea`. Today, BC bubbles and frame headers DO declare `hitArea`
(Canvas.svelte L1755 + L1894 — point-in-rect predicates). Good.

But the `app.canvas`'s pointerdown handler does NOT route through
Pixi's interaction system — it's a raw DOM `addEventListener` on
`app.canvas` (L1012). That's fine for pan claim. The cost path is
the OTHER direction: every `pointermove` from the window-level
listener walks the world container's children to dispatch
`pointerover` / `pointerout` to BC bubbles and frame headers. With
N×M interactive children all using closures for their `hitArea`
predicate, this is O(N×M) per move.

Two mitigations:
- Set `world.eventMode = 'passive'` so Pixi skips the world
  container itself for hit-testing (children still receive events).
- Set a `world.hitArea` of the union bounds so the broad-phase
  rejection is cheap on pointermoves over empty space.

Lives in `canvas-017-broad-phase-hit-rejection-for-empty-canvas-pan`.
Estimate: ½ day.

## Reproducer — operator protocol

The operator (Marco) runs the reproducer in the Tauri dev shell; the
JS console output and Chrome devtools Performance trace are pasted
into `console-snapshot.md` and `trace.json` (or screenshot) next to
this README.

**Setup**:
- `pnpm tauri dev`
- Use Marco's real registered projects (currently 7–10 frames, ~5 BCs each).
- Wait for the canvas to settle (status bar shows "N projects · press F to fit").
- Open the WebView2 devtools (right-click in the canvas region → Inspect, or `Ctrl+Shift+I`).

**Console snapshot** (paste outputs into `console-snapshot.md`):

```js
// 1. Expose the running Application instance to the console.
//    Add this debug global to Canvas.svelte temporarily, OR reach into
//    the canvas via `document.querySelector('canvas')` and rely on a
//    one-time `window.__guppiApp = app` assignment behind a dev-only
//    guard. (A follow-up task — canvas-018 — will land a proper
//    dev-only diagnostic seam if this proves repeated friction.)
const app = window.__guppiApp;
console.log('renderer.type       =', app.renderer.type);
console.log('renderer.resolution =', app.renderer.resolution);
console.log('devicePixelRatio    =', window.devicePixelRatio);
console.log('ticker.FPS          =', app.ticker.FPS);
console.log('GPU                 =',
    app.renderer.gl?.getParameter(
        app.renderer.gl.getExtension('WEBGL_debug_renderer_info')?.UNMASKED_RENDERER_WEBGL
    ));
```

**Worst-lag reproducer** (use this as the regression-check stance):
- N ≥ 7 frames visible at default zoom (no zoom-to-fit applied).
- Pan continuously by holding left-mouse on an empty area and drawing
  large overlapping circles for ~5 seconds.
- Expected post-fix: < 16 ms per frame (sustained 60 FPS) in the
  Performance tab's Frames row.
- Worst observed pre-fix (from Marco's 2026-05-17 hands-on): described
  as "sluggish" — pin a numeric baseline by capturing trace pre-fix
  (revert the canvas-014 patch on a scratch branch) and post-fix on
  main.

**Chrome Performance trace**:
- Open devtools → Performance tab → Record.
- Pan-circles for ~5 seconds.
- Stop recording. Save the trace as `trace.json` (Save profile…)
  next to this README and reference it from `console-snapshot.md`.
- Annotate: total frame count, average frame time, longest frame,
  Scripting vs Rendering vs GPU split.

## Frame-time targets

Per the spike body:

- < 16 ms per frame at default zoom with 10+ project frames (60 FPS sustained).
- < 8 ms per frame is the "feels instant" target with headroom.

**Acceptance for post-canvas-014 baseline**: the trivial-win patch
alone should clear the < 16 ms bar at idle (steady-state cost: zero).
During active pan, residual cost is one full rebuild per pointermove
event — typically ≤ 60–120 events per second (browser-coalesced). The
< 8 ms headroom target is **not** expected to be reached until
canvas-015 lands (persistent scene graph + camera as stage transform).

## Behaviour regression notes (canvas-014 patch)

The trivial-win patch removes the unconditional ticker rebuild. Risk
matrix verified by source-code review:

| Behaviour | Was driven by | Still driven by |
| --- | --- | --- |
| Pan (drag empty canvas) | pointermove → renderScene() | unchanged |
| Wheel zoom | wheel handler → renderScene() | unchanged |
| Frame drag | pointermove (with `dragProjectId`) → renderScene() | unchanged |
| BC drag | pointermove (with `dragBcName`) → renderScene() | unchanged |
| Hover focus ring | pointerover/out → renderScene() | unchanged |
| Eased zoom-to-fit (`f`) | ticker → stepCameraTransition + renderScene | ticker (guarded by `cameraTarget`) |
| Theme flip | onThemeChange → renderScene() | unchanged |
| Window resize | ticker rebuild (incidental) | NEW: explicit `resize` listener |
| Live project add/remove | event handler → renderScene() | unchanged |
| Targeted `task_*` / `bc_*` events | event handler → `recomputeBcLayout`, then ticker rebuild | event handler → `recomputeBcLayout` → **explicit `renderScene()` added by canvas-014** |

Marco — when smoke-testing the patch, please specifically verify:
edit a task file under a registered project (e.g. rename or move a
task .md from `todo/` to `doing/`) and confirm the BC's task counts
on the canvas update without needing to nudge the camera. The
canvas-014 patch wires an explicit `renderScene()` into the
default-event branch, but the hands-on confirmation is the only way
to catch any third dispatch path the source-only audit missed.

## Coordination with canvas-013

canvas-013's strategy (crisp rendering + constant-size project titles)
hinges on:

- AC #1: set `resolution = devicePixelRatio` + `autoDensity = true`
  — orthogonal to canvas-014's hotspot list, and the canvas-014 patch
  does not touch it.
- HTML overlay for project titles: this report's hotspot ranking
  **strengthens** that choice — Pixi `Text` allocations are part of
  the per-render cost, and HTML overlays positioned via the camera
  transform (already an ADR-003 idiom) participate in CSS layer
  compositing on the GPU without re-rasterising on every camera
  change.
- BitmapText for BC names: deferred as a canvas-014 follow-up per the
  protocol log. After canvas-015 (persistent scene graph) lands, BC
  `Text` instances persist across renders and re-rasterise only when
  their string or style changes, not on every pan tick. At that point
  the BitmapText switch is an optimisation, not a fix — it would shed
  the per-Text-style atlas lookup but the headline pan/zoom cost is
  already gone.

**Net**: canvas-013's strategy decision is unchanged by this report's
findings. The HTML-overlay-for-titles call holds, and BitmapText
remains deferred. canvas-013 can promote without waiting for canvas-015.

## Files referenced

- `src/lib/Canvas.svelte` — the rendering surface (patched in canvas-014)
- `src/lib/camera.svelte.ts` — reactive camera state (untouched)
- `src/lib/bc-layout.ts` — one-shot deterministic BC layout (confirmed: no per-frame work; ADR-015 contract intact)
- `src/lib/tile-layout.ts` — pure spiral auto-placement (no per-frame work)
- `src/lib/snapshot-patch.ts` — pure targeted-update patcher (no per-frame work)
- `src/lib/Modal.svelte` — HTML overlay; mounts only when a modal is open; no pan-tick reflow

## Open items captured as backlog tasks

- `canvas-015-persistent-scene-graph-and-stage-transform-pan` — the substantive renderer rework
- `canvas-016-cache-screen-space-overlays` — voice indicator (and future agent-awareness badges) cached per mount
- `canvas-017-broad-phase-hit-rejection-for-empty-canvas-pan` — `world.hitArea` and `eventMode = 'passive'`
- `canvas-018-dev-only-diagnostic-seam` — a behind-a-flag `window.__guppiApp = app` (or similar) so future spikes can read renderer state from the console without source patching
