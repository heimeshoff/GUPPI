---
id: canvas-012
title: Drag state can stick after pointerup — subsequent mouse moves pan the canvas
status: todo
type: bug
context: canvas
created: 2026-05-17
completed:
commit:
depends_on: []
blocks: []
tags: [drag, input, regression, drag-controller]
related_adrs: [ADR-003]
related_research: []
prior_art: [canvas-002, canvas-007]
---

## Why

Hands-on verification on 2026-05-17 surfaced an intermittent stickiness in
the canvas drag controller: after a `pointerdown` → `pointermove` →
`pointerup` sequence, the canvas sometimes remains in active-drag mode.
Subsequent bare mouse-movement (without a pressed button) continues to pan
the world. The implicit contract — pointerup ends the drag — is violated.

The canvas is GUPPI's primary interaction surface (per `vision.md`); a
stuck drag breaks every gesture downstream of it. This blocks confident
hands-on use and undermines the v1 "ambient overview" experience.

## What

### Canonical reproducer (Marco, 2026-05-19 hands-on)

> "Clicking on a project box and then releasing the mouse cursor and then
> moving the mouse around makes the whole screen follow."

In canvas vocabulary: `pointerdown` on a project frame's BODY (not the
header bar — the header bar is the frame-drag handle; the body is
intentionally pass-through so empty regions inside a frame still let the
camera pan, per `canvas-007`), `pointerup` with zero or near-zero motion,
then bare `pointermove` → the camera pans following the cursor. The
"whole screen follows" wording is the tell: the **empty-canvas pan
claim** is the one sticking, not the frame drag or the BC drag.

### Source-pinned diagnosis (2026-05-19 refinement pass)

The Pixi `frame` object's `hitArea` covers only the header bar
(`Canvas.svelte` ~L2029–2035), so a `pointerdown` on the frame body
falls through to `app.canvas.addEventListener('pointerdown', …)` at
~L1109. That handler sets the **function-scope** `panning = true` at
~L1123. The `window`-level `pointerup` listener at ~L1186 clears
`panning` ONLY **inside an `if (panning) { … }` branch** at ~L1213,
AFTER two early-return guards for the BC-drag branch (~L1187) and the
frame-drag branch (~L1204). When any of those guards return early
without falling through to L1213 — and the pointerup is the gesture
that started a pan — `panning` survives.

The structural problem is that **four pieces of drag state are spread
across two scopes**: `dragProjectId`, `dragBcName`, `dragOriginX/Y` at
module scope (~L535–551), and `panning` at function scope inside the
empty-canvas pointerdown closure (~L1109–1129). The four-variable
spread is the reason the bug is invisible to the eye scanning a single
function. The minimal fix — clear all four unconditionally at the top
of the `window.pointerup` listener, then handle persistence in the
branches — closes the canonical repro by itself.

`pointercancel` and `pointerleave` are **NOT currently wired at all**
on the controller. The existing controller has zero listeners for
either. Both are required by the ACs below (multiple edges where
pointerup never arrives — touch interruption, pointer leaves the
window mid-drag — currently leave drag-state stuck). This is new
defensive coverage, not "tighten the existing handlers".

### Structural fix — extract a pure drag-controller module

Pull the drag-state logic into `src/lib/drag-controller.ts`, a peer to
`tile-layout.ts` / `bc-layout.ts` / `snapshot-patch.ts` (no Svelte, no
Pixi imports — the same "verification surface" pattern that
`canvas-007` / `canvas-008` use). The module owns:

- **State (discriminated union):**
  ```ts
  type DragState =
    | { kind: 'idle' }
    | { kind: 'panning'; lastX: number; lastY: number }
    | { kind: 'frame'; projectId: number; originX: number; originY: number }
    | { kind: 'bc'; projectId: number; bcName: string; originX: number; originY: number };
  ```
- **Target descriptor (discriminated union):** the pointerdown call
  site builds one of `{ kind: 'empty' } | { kind: 'frameHeader',
  projectId } | { kind: 'bcBubble', projectId, bcName }` and hands it
  to the controller.
- **Transition functions** (pure; each returns the new state plus an
  optional side-effect descriptor the caller executes):
  - `onPointerDown(state, target, x, y) → { next, ... }`
  - `onPointerMove(state, x, y, zoom) → { next, delta?: { kind, dx, dy, ... } }`
    — the caller applies the delta to `camera` / `entry.pos` /
    `entry.bcPositions`.
  - `onPointerUp(state) → { next, persist?: 'tile' | 'bc' | 'camera' }`
    — the caller calls the matching IPC save when `persist` is set.
  - `onPointerCancel(state) → { next: { kind: 'idle' } }` —
    unconditional terminal; cancelled drags do NOT persist.
  - `onPointerLeave(state) → { next: { kind: 'idle' } }` — same
    terminal contract.
- **Guards:** `button === 2` (right-click) never enters a drag state;
  it's a no-op on the controller. The consumer handles menu open.

**What stays in `Canvas.svelte`:** the three event-listener wirings
(empty-canvas at ~L1109, frame header at ~L2048, BC bubble at ~L2085);
the new `window`-level `pointercancel` / `pointerleave` listeners (added
as peers of the existing `pointerup` listener); the two/three
persistence calls (`saveTilePosition`, `saveBcPosition`, `saveCamera`);
the world/screen coordinate math; the `renderScene` call. Each
pointer-handler call site shrinks to 3–5 lines: build the descriptor,
call the controller, apply the delta and/or persist, render.

**No frontend test infrastructure is set up in this task.** The
extracted module is *prepared* for tests when `infrastructure-017`
lands; it is not unit-tested here. The pure-module shape is the
end-state per Marco's scope lock.

### Pointer Capture API — intentionally OUT of scope

Marco's lock permitted PC migration "if the audit reveals it's
cleaner". The audit reveals it isn't, here: the bug is a one-line
state-machine defect (the unconditional clear) plus two missing
handlers (`pointercancel` / `pointerleave`), not a structural failure
of the window-listener model. PC's main benefit is making the browser
own pointerup routing (which would dissolve the "overlay swallows
pointerup before `window` sees it" failure mode the original task body
flagged as a hypothesis). Its cost is non-trivial — rewiring all three
claim sites against the underlying DOM canvas (PixiJS v8 events are
synthetic; PC needs the real DOM target) — and the SM extraction gives
us a single audit surface that PC would also need.

**Future trigger for PC migration:** if a bug surfaces at the
overlay-intercept boundary AFTER this fix (a Modal / context menu /
error toast swallows pointerup-during-pan and the controller misses
the terminal event), file a follow-up task. The SM extraction makes
the migration cheap — one module's `onPointerUp` reroute, not three
sprawling handlers.

## Acceptance criteria

- [ ] **Canonical repro is fixed:** `pointerdown` on a project frame
      BODY, `pointerup` with zero or near-zero motion, then bare
      `pointermove` — no pan occurs until a new `pointerdown` lands.
- [ ] After any sequence of `pointerdown` → `pointermove` →
      `pointerup` (with `pointerup` landing on any target — window,
      off-canvas, HTML overlay, frame body, BC bubble), no further
      `pointermove` event pans the canvas or moves a frame/BC until a
      new `pointerdown` lands.
- [ ] Verified across all three drag kinds: project-frame drag (header
      bar), BC-bubble drag (bubble surface), and pan-empty-canvas
      drag (canvas background).
- [ ] `pointercancel` mid-drag clears active-drag state. (NEW listener
      at `window` — not currently wired anywhere in the controller.)
- [ ] `pointerleave` on `window` mid-drag clears active-drag state.
      (NEW listener at `window` — not currently wired.)
- [ ] Right-click during an in-flight left-drag does not leave the
      drag stuck after pointerup.
- [ ] Drag-state logic is extracted to `src/lib/drag-controller.ts`,
      a pure module with **no Svelte and no Pixi imports** (mirrors
      `tile-layout.ts` / `bc-layout.ts` / `snapshot-patch.ts`'s
      test-surface-ready stance). `Canvas.svelte`'s three claim sites
      call into the controller; the module exports its discriminated-
      union `DragState` type, the `Target` descriptor union, and the
      five transition functions. **No frontend test infrastructure**
      is set up here — that's `infrastructure-017`'s scope; the module
      is prepared for tests when 017 lands.
- [ ] Reproducer documented (steps + observed vs expected) in the
      task notes or as a comment in the fix commit, so the failure
      mode is captured for the next eyes on this code.
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- Shared drag controller currently lives in `Canvas.svelte`:
  - State declarations: ~L535–551 (module scope).
  - Empty-canvas claim: ~L1109–1129 (`panning = true` at ~L1123 — the
    one function-scope state variable; the four-variable spread is
    the root structural defect).
  - `window` `pointermove`: ~L1138–1185.
  - `window` `pointerup` (the leak site): ~L1186–1217; clear at
    ~L1213 inside `if (panning) { … }` branch.
  - Frame header claim: ~L2048–2068.
  - BC bubble claim: ~L2085–2104.
  - Capture-phase outside-click dismisser: ~L1255 (fires on every
    pointerdown, including a gesture's start — currently only nulls
    `menu`; flag in code review if it grows).
  - `Modal.svelte` + context-menu `onpointerdown={(e) =>
    e.stopPropagation()}` at ~L2204 are pointerup-routing risks but
    are made irrelevant by the new `pointercancel` / `pointerleave`
    listeners — both events still fire and route to the controller's
    terminal-state branch.
- Frame drag persists position via `saveTilePosition` (`canvas-002`);
  BC drag persists via `saveBcPosition` (`canvas-007` /
  `project-registry-004`); camera pan persists via `saveCamera`
  (ADR-004). The fix changes none of these — only the controller's
  state-machine.
- **The empty-frame body is intentionally pass-through** (`canvas-007`):
  the Pixi `frame.hitArea` covers only the header bar (~L2029–2035),
  so `pointerdown` on the body falls through to the empty-canvas
  handler. This is correct UX — empty regions inside a frame still
  let the camera pan. Do **not** change.
- **Pointer Capture API migration is intentionally out of scope for
  this task.** Rationale + future trigger named above.
- **ADR candidate (not pre-written):** the drag-controller extraction
  is a local architectural decision (scope: `bc: canvas`, not global).
  Suggested id when filed: the next free ADR number. **Write after
  the diff, not before** — the module's public API stabilises during
  implementation; speculative ADRs rot fast. If the extraction turns
  out to be larger than this body suggests (e.g., reveals a
  parallel-event coordination problem with `canvas-006`'s serialised
  live-add chain, or forces a change to the persistence call sites),
  the worker flags for an ADR mid-task.
