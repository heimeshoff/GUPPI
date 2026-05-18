---
id: canvas-016
title: Cache screen-space overlays (voice indicator) instead of allocating per render
status: backlog
type: feature
context: canvas
created: 2026-05-18
completed:
commit:
depends_on: [canvas-014]
blocks: []
tags: [performance, rendering, pixi, overlays, refactor]
related_adrs: [ADR-003]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-007]
---

## Why

`makeVoiceIndicator` (Canvas.svelte) builds a fresh `Container` +
`Graphics` + `Text` on every `renderScene` call, even though the
indicator's state (`voiceState`, viewport size) usually does not
change between renders. After `canvas-014`'s ticker-guard fix, the
indicator only re-allocates when something else triggers a render —
but every pan, wheel, drag, and hover still pays the cost
unnecessarily.

This is also the design template for future overlays the BC will
acquire: status badges from `agent-awareness`, ambient command-palette
chrome, future toast positioning. Establish the "cached overlay"
pattern now while the surface is small.

## What

Refactor `makeVoiceIndicator` into a persistent overlay that:

- Instantiates the `Container`, `Graphics`, and `Text` once during
  the canvas init phase.
- Reads `app.renderer.width` / `app.renderer.height` to position
  itself on init AND on window resize (hooked into the same `resize`
  listener canvas-014 added).
- Recolours the dot + label when `voiceState` changes (the rune /
  `$effect` path, not a full `renderScene`).
- Stays as the only screen-space overlay handled by `world` for now.
  (HTML chrome — toggle, status bar, menus, modals, toast — is
  already in the overlay layer per ADR-003.)

If a forthcoming task brings status badges from `agent-awareness`,
follow the same pattern: pre-allocate per BC on `bc_appeared`, update
properties on event, never re-allocate per render.

## Acceptance criteria

- [ ] The voice indicator's `Container`, `Graphics`, and `Text` are
      instantiated exactly once per canvas lifetime (verify with a
      Pixi devtools snapshot or by counting via a debug log).
- [ ] `renderScene` no longer calls `makeVoiceIndicator`.
- [ ] On window resize, the indicator repositions to the new
      bottom-right corner of the viewport.
- [ ] On `voiceState` change, the dot and label colours update without
      a full `renderScene`.
- [ ] Theme flip still recolours the indicator (it follows the
      `color.voice*` tokens).
- [ ] No regressions to the indicator's existing semantic (the
      "mic" / "muted" / "listening" label and dot stay correct in
      every state).
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- Coordinates with `canvas-015` (persistent scene graph): if
  canvas-015 lands first, this becomes a small case-by-case extension
  of its persistent-display-objects pattern. If canvas-016 lands first,
  it sets the pattern that canvas-015 then generalises.
- The voice BC owns the *real* voice-state wiring. This task does NOT
  touch that contract — it only changes how the canvas paints the
  current `voiceState` value.
- Estimate: ½ day.
- See `canvas-perf-2026-05-17` report, Hotspot 3.
