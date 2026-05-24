---
id: canvas-019
title: Rendering substrate for the kanban-accordion frame interior (ADR-003 revisit)
status: todo
type: decision
context: canvas
created: 2026-05-24
completed:
commit:
depends_on: [canvas-019a]
blocks: [canvas-020, canvas-021, canvas-022]
tags: [architecture, rendering, decision, pivot, kanban, pixi, dom]
related_adrs: [ADR-003, ADR-015, ADR-016, ADR-002]
related_research: []
prior_art: [canvas-007, canvas-015, canvas-013]
---

## Why

The canvas is pivoting: every project frame's interior becomes a vertical
accordion of collapsible bounded contexts, each holding a kanban board
(BACKLOG → TODO → DOING → DONE) of individual task cards, with a docked
right-edge detail panel that animates in/out (reference:
`.agentheim/contexts/design-system/references/kanban.png`). Locked user
decision: **every project always renders as a full kanban-accordion frame**
— there is no separate small/summary representation; you pan/zoom between
full frames on the infinite surface.

The current canvas is PixiJS v8 / WebGL (ADR-003) with a persistent scene
graph and camera-as-stage-transform (ADR-016, canvas-015), frame chrome +
BC bubbles + spring-electrical intra-project edges drawn as Pixi
`Graphics`/`Text`. A scrollable kanban with collapsible accordions, dozens
of text-heavy cards per frame, and an animated docked panel is a
quintessential DOM/HTML UI. Decision #1 (every frame always full) makes
this acute: N full text-heavy interactive frames on a pannable/zoomable
surface. This forces an **ADR-003 revisit** before any interior task can
proceed.

## What

Decide and record (as a new ADR) the rendering substrate for the frame
interior — and, if the decision warrants, the whole canvas:

- **Option A — Hybrid (lead recommendation).** Pixi keeps the world-space
  camera transform (ADR-016) and the frame shell (border + header bar +
  drag hit-area). Each frame's kanban-accordion interior AND the docked
  detail panel render as HTML/DOM overlays positioned to world coordinates
  via `camera.worldToScreen` (the affordance ADR-003 already names). Native
  scroll/text/accordion/CSS-transition; `tokens.css` overlay layer.
- **Option B — Full DOM canvas.** Drop Pixi entirely; a single
  CSS-`transform`-on-a-world-`<div>` camera (Miro/tldraw-lite). Unifies
  tech, but discards ADR-016/canvas-015/canvas-012/canvas-013.
- **Option C — Pure Pixi.** Kanban + scroll + accordion + panel in WebGL.
  Rejected in the recommendation; included for completeness.

The decision MUST settle: (1) substrate choice; (2) ADR-015's fate (its
canvas *layout* consequence is superseded; its relationship *data model*
in README frontmatter is NOT — keep that distinction explicit); (3) the
viewport-culling + level-of-detail policy for N full DOM frames (which
frames mount their DOM interior, what off-screen/zoomed-out frames render
as); (4) how the docked detail panel composes (screen-space, viewport-
docked — see canvas-022); (5) what survives from ADR-016/canvas-015
(camera, drag-controller, tile-layout) vs. what is retired (Pixi BC
bubbles, spring-electrical edges, bc-layout.ts).

## Acceptance criteria

- [ ] A new ADR (proposed ADR-017) is written and committed in
      `.agentheim/knowledge/decisions/`, scope `bc, canvas`, recording the
      substrate decision with the three options, the reasoning, and the
      rejected alternatives.
- [ ] The ADR states ADR-003's relationship to the decision (extension vs.
      partial supersession) and explicitly moves ADR-015's *canvas-layout*
      consequence to Superseded while preserving its README-frontmatter
      relationship model (owned by project-registry-004).
- [ ] The ADR records the viewport-culling + LOD policy and the perf
      stance for N full frames (target: smooth pan/zoom at N≈10 frames).
- [ ] The ADR cites `canvas-019a`'s measured findings (N≈10 populated DOM
      interiors with viewport-culling + zoom-threshold LOD) as the empirical
      basis for ratifying hybrid. If `canvas-019a` reported FAIL against its
      perf target, the ADR overturns the hybrid recommendation and costs
      Option B (full-DOM) instead, recording why.
- [ ] The ADR enumerates which existing modules survive
      (`camera.svelte.ts`, `drag-controller.ts`, `tile-layout.ts`) and
      which are retired (`bc-layout.ts`, Pixi BC-bubble + intra-project-edge
      draw paths in `Canvas.svelte`).
- [ ] canvas INDEX.md ADR list + the global knowledge index updated to
      reference the new ADR; ADR-015 marked Superseded-in-part with a
      backlink.

## Notes

This is the gate task for the whole pivot's canvas work: canvas-020/021/022
all `depends_on` it. project-registry-005 and design-system-006 do NOT
depend on it (data contract + visual vocabulary are substrate-agnostic),
so they can proceed in parallel.

**The empirical perf gate was split out** (refine, 2026-05-24): the old AC #4
"run a perf spike" is now its own `type: spike` task **`canvas-019a`**
(validate hybrid only — Pixi shell + N≈10 DOM interiors with viewport-culling +
zoom-threshold LOD), which `canvas-019` now `depends_on`. The spike produces a
findings note; this ADR cites its numbers to ratify hybrid (or overturns toward
Option B if the spike fails). `canvas-019a` was promoted straight to `todo/` as
an unblocked root of the pivot — it starts in parallel with project-registry-005
and design-system-006. Full-DOM (Option B) stays the costed fallback only;
pure-Pixi (Option C) remains rejected.

### Architect's recommendation (for ratification — overturn with reasons if the spike contradicts it)

> **Lead recommendation: HYBRID — Pixi keeps the camera/world transform and the frame shell (border + header bar + drag hit-area); each frame's kanban-accordion interior and the docked detail panel render as HTML/DOM overlays positioned to world coordinates via the existing `camera.worldToScreen` contract.**
>
> Reasoning. ADR-003 already mandates exactly this pattern ("when a tile needs rich interactive content… render an HTML overlay positioned to match the tile's world coordinates… exactly Miro's and Figma's approach"), and ADR-016's Consequences explicitly preserve `camera.worldToScreen` as "the contract for screen-space overlays… forthcoming agent-awareness badges." A scrollable kanban with collapsible accordions, text-heavy cards, and an animated docked panel is the quintessential DOM UI: native scroll, native text layout/wrapping/selection, CSS transitions for the accordion + panel slide, real focus/keyboard handling, and the `tokens.css` overlay layer (which design-system already maintains in parallel with `tokens.ts`) all come for free. Rebuilding scroll containers, text reflow, and accordion animation in Pixi `Graphics`/`Text` would be a large, bug-prone reimplementation of the browser.
>
> The countervailing force is decision #1: every frame is *always* a full kanban, so there is no zoomed-out Pixi-only density mode to protect, and N full DOM frames each with multiple scroll containers + dozens of cards is the actual perf question. Mitigations that keep hybrid viable: (a) **viewport culling** — only mount the DOM interior for frames intersecting the viewport (off-screen frames render as a cheap Pixi shell or nothing); (b) **zoom-threshold detail** — below a zoom threshold, suppress the DOM interior and show a Pixi summary shell, *re-mounting* DOM on zoom-in (this softens decision #1 at the rendering layer without reintroducing a separate "summary representation" in the model — same frame, LOD rendering); (c) CSS `content-visibility: auto` on cards/columns. The investment to preserve is substantial — ADR-016/canvas-015's persistent scene graph, `drag-controller.ts`, `tile-layout.ts`, `camera.svelte.ts` — and hybrid preserves all of it for the camera + frame shell; only the *interior* (currently Pixi BC bubbles + spring edges) is swapped for DOM.
>
> **Full-DOM-canvas (no Pixi)** is the honest alternative and should be costed, not dismissed: a single CSS-`transform: scale()/translate()`-on-a-world-`<div>` camera is a well-trodden Miro/tldraw-lite pattern, would unify everything in one tech, and decision #1 removes Pixi's headline advantage. Its costs: throws away ADR-016/canvas-015/canvas-012/canvas-013 (the bulk of recent canvas work), and CSS transforms on very large worlds hit subpixel/compositing limits Pixi doesn't. **Pure-Pixi (kanban in WebGL)** is rejected outright — text-heavy scrollable interactive UI in WebGL is the wrong tool and contradicts ADR-003's own reasoning.
>
> Net: **hybrid** preserves the most prior investment, matches ADR-003's stated architecture, and pushes the only real risk (N DOM interiors) onto viewport-culling + LOD, which are standard and contained. Recommend hybrid; require the worker to spike viewport-culling perf with N=10 frames before ratifying, and to record the chosen LOD/culling policy in the new ADR.
