---
id: infrastructure-003-canvas-rendering
type: decision
status: todo
scope: global
depends_on: [infrastructure-002-frontend-framework]
---

# Decision: Canvas rendering library

## Context

v1 is canvas-only: pan, zoom, drag, many tiles, child connections. Future: live indicators, status badges, embedded terminal panels (interactive text inside a tile).

Three approaches: SVG, HTML5 `<canvas>` 2D, WebGL (PixiJS / Konva).

## Architect's recommendation

**PixiJS v8** (WebGL) for the canvas, with **HTML overlays positioned to match world coordinates** for tiles needing rich interactive content (markdown viewer, emulated terminal). Same approach Miro/Figma use.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-003-canvas-rendering.md`
- [ ] Justification matches architect's draft or Marco's amendments
- [ ] `scope: global` in frontmatter

## Notes — architect's ADR draft

### ADR-003: Canvas rendering — PixiJS (WebGL) with HTML overlays for interactive content

**Status:** Proposed
**Scope:** global

**Context.** v1 is canvas-only: pan, zoom, drag, hundreds of tiles, child connections. Future: live indicators, status badges, embedded terminal panels (interactive text inside a tile). Three rendering approaches exist: SVG, HTML5 `<canvas>` 2D, WebGL (typically via PixiJS or a higher-level lib like Konva).

**Options considered.**
1. **SVG (e.g. via svelte-flow / react-flow / d3)** — Easy DOM-native interactions, accessible, but performance degrades around a few hundred nodes with connectors and re-renders. Bad fit for "miro-like" zoom-out density.
2. **HTML5 canvas 2D (Konva, fabric.js, or hand-rolled)** — Good middle ground, ~thousands of objects, but text rendering and embedded interactive panels are awkward.
3. **PixiJS (WebGL)** — Highest performance, scales to many thousands of nodes, smooth zoom/pan, mature API. Embedding interactive content (a terminal panel) is done by *overlaying* HTML on top, synced to PixiJS coordinates. This is exactly Miro/Figma's approach.

**Decision.** Use **PixiJS v8** for the canvas (tiles, edges, badges, hit-testing). When a tile needs rich interactive content (markdown viewer, emulated terminal), render an **HTML overlay positioned to match the tile's world coordinates**. Use a single source of truth for camera state (pan + zoom) that both the PixiJS scene and the overlay layer subscribe to.

**Consequences.**
- (+) Future-proof for terminal panels, dense canvases, fluid pan/zoom.
- (+) Mature library, lots of prior art for infinite canvases.
- (–) More upfront work than a "drop in react-flow" approach. The first tile takes longer than it would in SVG.
- (–) Accessibility (screen readers) is poor on canvas. Acceptable here — single user, not a public product.

**Reversibility.** Medium. The world-coordinate abstraction is portable; the rendering layer behind it could be swapped if needed.
