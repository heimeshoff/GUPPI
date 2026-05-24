---
id: canvas-019a
title: Perf spike — hybrid Pixi-shell + DOM kanban interior at N≈10 frames
status: done
type: spike
context: canvas
created: 2026-05-24
completed: 2026-05-24
commit: d8a1f59
depends_on: []
blocks: [canvas-019]
tags: [architecture, rendering, spike, pivot, kanban, pixi, dom, perf]
related_adrs: [ADR-003, ADR-016, ADR-002]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-014, canvas-015]
---

## Why

`canvas-019` is the substrate-decision ADR for the kanban-accordion pivot, and
its lead recommendation is the **hybrid** (Pixi keeps the camera + frame shell;
each frame's kanban interior + the docked panel are DOM overlays positioned via
`camera.worldToScreen`). The one real risk the architect flagged is **decision
#1**: every project *always* renders as a full kanban, so the canvas is N full
text-heavy DOM interiors — multiple scroll containers and dozens of cards each —
on a pannable/zoomable surface. That perf question must be answered with
measured numbers *before* the ADR ratifies hybrid, not after. This spike is the
empirical gate; it was carved out of `canvas-019`'s old AC #4 so the decision
task can stay a clean ADR and this can be a clean throwaway.

## What

A **throwaway** harness that builds **Option A (hybrid) only** and measures it:

- Pixi keeps the existing camera/world transform (ADR-016, `camera.svelte.ts`)
  and draws each frame's shell (border + header bar).
- Each frame's interior renders as an HTML/DOM overlay positioned to world
  coordinates via the existing `camera.worldToScreen` contract — a populated
  kanban-accordion: a vertical accordion of BCs, each with BACKLOG→DONE columns
  holding dozens of dummy text cards.
- **Viewport culling**: only frames intersecting the viewport mount their DOM
  interior; off-screen frames render as a cheap Pixi shell (or nothing).
- **Zoom-threshold LOD**: below a zoom threshold the DOM interior is suppressed
  (Pixi summary shell) and **re-mounts** on zoom-in — this is rendering-layer
  LOD, NOT a separate model-level "summary representation" (decision #1 holds).

Full-DOM (Option B) is the **fallback only if hybrid fails** the target — do not
build it in this spike. Pure-Pixi is already rejected. Use dummy content; this
spike is exempt from the `design-system` styleguide gate (no production UI).

## Acceptance criteria

- [ ] Throwaway harness renders N≈10 project frames, each carrying a populated
      kanban-accordion DOM interior over the existing Pixi camera/world transform
      (branch or a clearly-marked throwaway dir — NOT merged into the production
      `Canvas.svelte` render path).
- [ ] Viewport culling implemented: only frames intersecting the viewport mount
      their DOM interior; off-screen frames render as a cheap Pixi shell or
      nothing.
- [ ] Zoom-threshold LOD implemented: below a zoom threshold the DOM interior is
      suppressed (Pixi summary shell) and re-mounts on zoom-in.
- [ ] Frame-time measured during a sustained ~5s pan-circle and during zoom, at
      N≈10 with populated interiors, using the `canvas-perf-2026-05-17`
      reproducer protocol (Chrome trace + JS console snapshot). Target: smooth
      pan/zoom — sustained ~60 FPS (≤ ~16 ms/frame), ideally clearing the ≤ 8 ms
      default-zoom target from canvas-015's AC #10.
- [ ] A short findings note records PASS/FAIL vs the target, the chosen
      viewport-culling + LOD thresholds, and any surprises — written so
      `canvas-019`'s ADR can cite it directly. If hybrid FAILS the target, the
      note says so and explicitly flags Option B (full-DOM) for costing in the
      ADR.

## Notes

This is the gate for `canvas-019` (which `depends_on` it). The findings note is
the spike's product — `canvas-019` reads it and either ratifies hybrid citing
the numbers, or (if FAIL) overturns toward Option B. `canvas-020/021/022` sit
behind `canvas-019`, so this spike de-risks the whole pivot's canvas spine.

Reuse what exists: the `canvas-perf-2026-05-17` research report's reproducer
protocol, and the `canvas-018` dev-only diagnostic-seam idea (read renderer
state from devtools without source-patching) if it speeds up measurement.
`camera.svelte.ts` / `drag-controller.ts` / `tile-layout.ts` are the survivors
the hybrid preserves — lean on them rather than re-deriving the camera.

## Outcome

Built a throwaway, instrumented **Option A (hybrid) only** harness and
authored the findings note that `canvas-019` will cite. As with
`canvas-014`, the headline frame-times are **operator-pending** (the
Tauri dev shell / real GPU cannot be driven from a worker shell); the
harness is built, runnable, and instrumented, and the note gives Marco
the exact reproducer steps to fill the numbers. This is a legitimate
spike SUCCESS per the canvas-014 precedent — no frame-times were
fabricated.

**Harness** — `src/routes/spike-019a/+page.svelte`, a clearly-marked
throwaway route kept entirely OUT of the production `Canvas.svelte` path
(only imports `$lib/camera.svelte`, the survivor). Reproduces decision
#1's worst case: N=10 frames, each a full populated kanban-accordion
(5 BCs × 3 columns × 8 cards = 120 DOM cards/frame, 1200 total if all
mounted). Pixi draws each frame's SHELL in world coordinates on the
persistent scene graph + camera-as-stage-transform (ADR-016); the DOM
draws each on-screen frame's INTERIOR positioned via
`camera.worldToScreen` (ADR-003 overlay contract), zoom-matched with a
single `transform: scale(z)` so pan/zoom never reflows the 120-card
layout.

**Both cost governors implemented (AC #2, AC #3):**
- *Viewport culling* — only frames whose screen-space AABB intersects
  the viewport (+120px margin) mount a DOM interior; off-screen frames
  are a cheap Pixi shell only.
- *Zoom-threshold LOD* — below `z = 0.45` all interiors are suppressed
  (Pixi shells) and re-mount on zoom-in. Rendering-layer LOD, NOT a
  model-level summary; decision #1 holds.

**Instrumentation (AC #4 enablement)** — `window.__guppiSpike` dev seam
(canvas-018 idea): a rolling rAF frame-time sampler with
`autopan(5)` (scripted 5s pan-circle), `stats()`
(count/avgMs/p95Ms/maxMs/fps), `reset()`, `mountedInteriorCount()`,
`lodActive()`, and `config`. The operator runs the
`canvas-perf-2026-05-17` reproducer protocol against `/spike-019a` and
pastes numbers into `console-snapshot.md`.

**Verdict** — OPERATOR-PENDING; the analytical case predicts PASS (pan
restyles ≤9 overlay roots, never rebuilds Pixi nor recreates DOM; zoom
is a compositor scale, not a reflow). The note spells out the PASS/FAIL
rule and the FAIL→Option B fallback trigger so `canvas-019` can be
written the instant the numbers land.

**Checks** — `pnpm check` clean (992 files, 0/0/0); `pnpm build`
succeeds (`spike-019a/_page.svelte.js` chunk emitted).

**Key files:**
- `src/routes/spike-019a/+page.svelte` — throwaway hybrid harness + seam
- `.agentheim/knowledge/research/canvas-hybrid-perf-2026-05-24/README.md`
  — findings note (the spike's product; `canvas-019` cites it)
- `.agentheim/knowledge/research/canvas-hybrid-perf-2026-05-24/console-snapshot.md`
  — operator stub for the pending numbers
- `.agentheim/contexts/canvas/backlog/canvas-024-retire-spike-019a-harness.md`
  — cleanup task to delete the throwaway after canvas-019 decides

**ADRs:** none. The substrate decision ADR is `canvas-019`'s job; this
spike produces the empirical input it cites. ADR-003 / ADR-016 / ADR-002
all hold unchanged and constrain the harness as built.
