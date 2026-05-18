---
id: canvas-017
title: Broad-phase hit rejection on the world container (cheaper pointermoves over empty canvas)
status: backlog
type: feature
context: canvas
created: 2026-05-18
completed:
commit:
depends_on: [canvas-014]
blocks: []
tags: [performance, input, pixi, hit-testing]
related_adrs: [ADR-003]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-007]
---

## Why

PixiJS's hit-test walks the display tree on every pointer event to
decide which interactive child should receive `pointerover` /
`pointerout`. Today, BC bubbles and frame header bars correctly
declare `hitArea` predicates (point-in-rect), so the leaf-level test
is cheap. The cost path the `canvas-perf-2026-05-17` report flagged is
the OTHER direction: every `pointermove` over **empty canvas** still
walks the world container's children to discover that none of them
match. With N×M interactive children, this is O(N·M) per pointermove,
which during a pan happens at the browser-coalesced pointermove rate
(often ~60–120 events/sec).

Two cheap mitigations close this:

- `world.eventMode = 'passive'`: Pixi skips the world container itself
  for hit-testing (children still receive events as today).
- `world.hitArea = <union bounds of the scene>`: broad-phase rejection
  for any pointermove that lands outside the scene's bounds becomes
  O(1) instead of O(N·M).

## What

Add the two property assignments and the union-bounds hitArea to the
canvas init path. Update the world's `hitArea` when the scene's
bounds change (project add/remove, frame drag end). The union is
already computed by `sceneWorldBounds()` for zoom-to-fit — reuse it.

After `canvas-015` (persistent scene graph) lands, the union-bounds
update can be a single per-render check; before it lands, we either
recompute on every event-driven render or accept slightly-stale
hit-area bounds (acceptable — the cost we're avoiding is a leaf walk,
not a hit miss).

## Acceptance criteria

- [ ] `world.eventMode = 'passive'` is set during canvas init.
- [ ] `world.hitArea` is set to the current scene's union bounds (in
      world space — Pixi applies the world transform).
- [ ] The hitArea updates when `projects` add/remove, when a frame is
      dragged to a new position, or when the BC layout's frame width/
      height changes on a re-layout.
- [ ] Pan over empty canvas: pointermove dispatches no Pixi hit-test
      below the world container's broad-phase rejection (verify via
      Pixi devtools / a debug `console.count` in a BC bubble's
      `pointerover`).
- [ ] BC and frame hover affordances still work (the focus ring
      appears on hover, disappears on hoverout).
- [ ] Frame drag and BC drag still claim correctly via `pointerdown`.
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- After `canvas-015` lands, the camera transform is on `world` itself
  — so `world.hitArea` in world coords needs no `z` scaling. Before
  it lands, hitArea is in screen coords (since today's rendering
  pre-projects to screen).
- Coordinate sequencing with `canvas-015`: cleaner to land canvas-015
  first (the world container's hit-area semantics become simpler), but
  this task is independently valuable as an interim measure on the
  current screen-coords rendering.
- Estimate: ½ day on the current renderer; even cheaper after
  canvas-015.
- See `canvas-perf-2026-05-17` report, Hotspot 4.
