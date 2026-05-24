---
id: ADR-015
title: BC layout inside a project frame — deterministic one-shot spring-electrical with sticky pins
status: Superseded-in-part
scope: bc
bc: canvas
date: 2026-05-16
related_tasks:
  - canvas-007-project-as-frame
related_adrs: [ADR-003, ADR-014, ADR-017]
superseded_in_part_by: ADR-017
---

# ADR-015: BC layout inside a project frame — deterministic one-shot spring-electrical with sticky pins

**Status:** Superseded-in-part by ADR-017 (canvas-layout consequence only)
**Scope:** bc (canvas)

> **Superseded-in-part by [ADR-017](ADR-017-rendering-substrate-kanban-accordion-interior.md) (2026-05-24).**
> The kanban-accordion pivot replaces the force-directed BC-bubble interior
> with a vertical accordion of collapsible BCs, each holding a kanban board.
> ADR-015's **canvas-layout consequence** — that BCs are laid out as
> spring-electrical bubbles inside the frame via `src/lib/bc-layout.ts` — is
> therefore SUPERSEDED; `bc-layout.ts` and the Pixi BC-bubble draw path retire.
> ADR-015's **README-frontmatter relationship data model** (the per-BC
> `relationships:` block from ADR-014, owned by `project-registry-004`) is NOT
> superseded and stays Accepted — the data survives; only its force-directed
> layout consumer retires. See ADR-017 for the substrate decision.

## Context

`canvas-007-project-as-frame` ships the visual shift from project-as-bubble
(BCs orbiting one tile) to project-as-frame (BCs as interior bubbles, with
intra-project BC↔BC edges by relationship type — ADR-014 supplies the
relationship data). With BCs now living inside a bounded region, the canvas
needs an algorithm to place them.

The task's acceptance criteria pinned three invariants:

1. **Deterministic.** Same input -> same output, so a never-dragged BC lands
   in the same spot across restarts. No `requestAnimationFrame` loop; one-
   shot run on input change.
2. **Manual positions sticky.** Per-BC drag positions persist via
   `saveBcPosition` (`project-registry-004`); on the next layout pass, a BC
   with a saved position uses it.
3. **Re-layout only on topology change.** BC add / remove / relationship-
   change triggers a single layout pass. Task-count tickers don't.

The task left algorithm choice open ("force or spring-electrical, deterministic
seed"). Several real decisions had to be made:

- **Which force model?** Pure spring (Hooke's law on edges) tends to collapse
  unconnected components. Fruchterman-Reingold-style spring-electrical
  (springs on edges + universal Coulomb-style repulsion + cooling schedule)
  separates unconnected nodes naturally and is the textbook one-shot
  algorithm — it terminates in bounded iterations rather than relying on
  energy convergence.
- **How does a "sticky pin" interact with re-layout?** If the layout post-
  processes by translating the simulated point cloud into the frame's inner
  content rectangle (the natural "auto-fit" step), pinned positions get
  re-translated too — silently rewriting the saved coords. That breaks
  invariant #2.
- **What happens when a pinned BC is dragged outside the frame body's
  bounds?** The frame could (a) grow to cover it, (b) silently shift every
  position to re-anchor at the new origin, or (c) just render that BC
  outside the frame.

## Decision

Implement BC layout as a pure module `src/lib/bc-layout.ts` alongside
`tile-layout.ts`, with:

1. **Fruchterman-Reingold-style spring-electrical** force-directed simulation.
   `ITERATIONS = 120` (small constant — single-project BC counts are at most
   ~10–20 in v1). Linear cooling schedule. Initial temperature caps the per-
   iteration displacement so a high-energy first step cannot fling nodes
   off-canvas.
2. **Deterministic seeding via mulberry32 + djb2(bcName).** Each BC's
   initial position is a deterministic function of its name (stable across
   runs) plus a deterministic-jittered grid (so coincident initial positions
   don't pin the simulation). BC iteration order is sorted by name. The
   edge set is deduplicated by unordered endpoint pair. Result: same
   `bcs[]` + same `savedPositions` -> bit-identical output, every run.
3. **Pin contract: pinned positions render at their authored frame-local
   coords, period.** The post-simulation translation step is zero when any
   pin exists; the frame auto-fits the union of (pinned + simulated)
   rectangles. When no pin exists, the simulation's centroid is translated
   into the inner content rectangle as the natural auto-fit.
4. **Off-frame pinned BCs render outside the frame body.** If the user
   drags a BC to frame-local coords whose rectangle extends beyond the
   frame's right/bottom edge, the frame grows to cover it. If the user
   drags into negative frame-local coords, the BC visually sits to the
   left/above `entry.pos` — accepted as the minor cost of preserving the
   simpler invariant "frame top-left = `entry.pos`" and never silently
   rewriting the user's saved position. The user can drag the BC back.
5. **Manual drag pins on drag-end, not on drag-start.** During a BC drag,
   `Canvas.svelte` mutates `bcPositions` + `bcLayout.positions` in place
   for instant visual feedback; on `pointerup`, it calls
   `recomputeBcLayout(entry)` so the rest of the BCs flow around the new
   pin (a single one-shot pass).

## Consequences

- (+) Identical input -> identical output, both within a session and across
  restarts. Invariant #1 satisfied without an animation loop.
- (+) Sticky pins are byte-stable. Save coords match render coords. Invariant
  #2 satisfied; the "drag, restart, BC still there" UX works.
- (+) Algorithm is generic — handles disconnected BCs (e.g. the voice BC's
  no-edge-in-v1 state from ADR-014) naturally; the repulsion term pushes
  them away from the cluster.
- (+) Off-frame pinned BCs are recoverable (user drag-back). The alternative
  (silent re-anchoring) would corrupt saved data.
- (–) The off-frame edge case can produce a slightly confusing visual
  ("BC inside the frame's silhouette but rendered to its left"). Acceptable
  for a single-user tool; if it becomes a pain point, a v2 affordance can
  add a "snap back to frame" command without changing the layout contract.
- (–) Spring-electrical with `O(N^2)` repulsion is fine at N ≤ 50 but would
  need a Barnes-Hut acceleration if BC counts grew an order of magnitude.
  Not a v1 concern.

## Reversibility

Medium. The `BcFrameLayout` interface (`{ width, height, positions: Map<…>}`)
is portable. The algorithm behind it can be swapped — a different force
model, a constraint solver, manual grid — without disturbing the rendering
code or the saved-position contract, as long as the new algorithm respects
the same determinism + sticky-pin invariants.
