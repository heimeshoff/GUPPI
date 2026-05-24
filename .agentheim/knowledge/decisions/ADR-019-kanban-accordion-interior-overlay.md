---
id: ADR-019
title: Kanban-accordion DOM-interior overlay — camera-version reactive bridge + fixed frame shell
status: Accepted
scope: bc
bc: canvas
date: 2026-05-24
related_tasks: [canvas-020]
related_adrs: [ADR-017, ADR-016, ADR-003, ADR-002]
---

# ADR-019: Kanban-accordion DOM-interior overlay — camera-version reactive bridge + fixed frame shell

**Status:** Accepted
**Scope:** bc (canvas)

## Context

ADR-017 ratified the hybrid substrate: the project-frame **shell** stays PixiJS;
the **interior** (BC accordion → kanban board → task cards) becomes a DOM
overlay positioned via `camera.worldToScreen` + `transform: scale(z)`.
canvas-020 implements that interior. Two implementation seams needed deciding
that ADR-017 left to the implementing task, and both are non-obvious enough to
record.

## Decision 1 — `cameraVersion` reactive bridge

The Pixi camera path is **imperative**: pan writes `world.position.set(...)`
directly (the canvas-015 zero-allocation pan invariant — ADR-016), never
through a reactive read. The DOM interior overlay is **reactive** Svelte 5
markup (`{#each mountedInteriors}`), and `mountedInteriors` is a `$derived` that
must recompute its `left/top/scale(z)` on **every** camera change so the
interior tracks the shell.

The bridge is a single `cameraVersion = $state(0)` counter, read
(`void cameraVersion`) inside the `mountedInteriors` `$derived` and bumped
(`cameraVersion++`) by every camera mutation site: the pan `pointermove`, the
frame-drag `pointermove`, `repaint()` (which already runs on wheel-zoom + the
eased zoom-to-fit tick + theme flip), and `syncViewport()` (resize). This
guarantees the overlay re-positions on every pan/zoom/drag/resize even though
the camera's `$state` lives on an external `Camera` class instance that the
imperative pan path never reads reactively.

Rejected alternative: making the overlay subscribe to `camera.pan_x/y/zoom`
directly and dropping the counter. That works for paths that go through a
reactive read, but the load-bearing pan path deliberately does **not** read the
camera reactively (it writes `world.position` imperatively to stay
allocation-free), so a derived depending only on `camera.*` could miss pan
ticks. The explicit version counter is the robust seam and costs one integer
increment per camera event.

Per ADR-017's cost analysis the cost is bounded: the `$derived` restyles the
`left/top/transform` of ≤9 mounted interior roots, never the hundreds of card
children (those are frozen under the compositor `transform: scale(z)`).

## Decision 2 — fixed frame-shell size, interior scrolls within

The retired `bc-layout.ts` grew each frame to auto-fit a force-directed BC
bubble cloud. The kanban-accordion interior is unbounded content (N BCs × 4
columns × M cards), so auto-fitting the *shell* to it is wrong — the shell
would balloon. Instead `frame-interior.frameSize()` returns a **fixed default**
shell size (independent of BC count) and the DOM interior **scrolls within it**:
the accordion column scrolls vertically, each kanban board scrolls horizontally
past four columns, and each column's card stack scrolls vertically. This keeps
the world-space footprint of a frame predictable (so spiral auto-placement and
zoom-to-fit stay stable) and matches the styleguide's per-axis scroll affordances
(§3.10).

Consequence for accordion layout: an **expanded** row uses `flex: 1 1 auto;
min-height: 0` so it claims interior height and its board scrolls vertically,
rather than pushing sibling rows out of the fixed-height interior. Collapsed
rows are `flex: 0 0 auto` (just the 44px header band).

## Decision 3 — roll-up colour is the one runtime-hex exception

The styleguide rule is "no raw hex; use tokens". The accordion-header roll-up
pill's glyph colour is **data-driven per live-agent state** (`statusColor.running`
/ `.blocked` / `.idle` from agent-awareness-002's roll-up), not a static class,
so it is applied as an inline `color:` via a `hexColor(n)` helper that formats
the **token numeric** (`statusColor[...]`) — not a literal hex. The value still
comes from the token table; only the application is inline because the pill's
colour varies with data. Every other interior colour/size is a `--guppi-*` token
class.

## Consequences

- (+) The overlay tracks the Pixi shell exactly through the shared camera, with
  one cheap integer per camera event and zero per-pan card reflow (ADR-017).
- (+) Frame world footprint is stable and small; the unbounded kanban content
  lives behind native scroll, not in the world size.
- (+) BC-bubble drag, intra-project edges, and `bc-layout.ts` are deleted; the
  camera + shell half of the canvas-015 investment is untouched (ADR-017).
- (–) The `cameraVersion` counter is a manual reactivity bridge a future reader
  must understand; the alternative (fully reactive camera) is incompatible with
  the deliberately-imperative zero-allocation pan path.
- (–) The fixed frame size is a v1 default; a future per-content sizing pass
  (e.g. shrink-to-fit a single-BC project) can land behind `frameSize`'s
  already-present `bcCount` parameter without changing call sites.

## References

- `src/lib/frame-interior.ts` — pure cull/LOD + bucketing + `frameSize` helpers
  (the tested verification surface).
- `src/lib/Canvas.svelte` — the hybrid renderer: Pixi shell + `mountedInteriors`
  DOM overlay + the `cameraVersion` bridge.
- ADR-017 — the substrate decision this implements (hybrid; CULL_MARGIN_PX 120 /
  LOD_ZOOM_FLOOR 0.45; what survives / what retires).
- ADR-016 — persistent scene graph + camera-as-stage-transform (the imperative
  pan path this bridges to the reactive overlay).
- ADR-003 — HTML overlays positioned to world coordinates (the parent pattern).
