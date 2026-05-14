---
id: ADR-003
title: Canvas rendering — PixiJS v8 (WebGL) with HTML overlays for interactive content
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-003-canvas-rendering]
related_adrs: [ADR-002]
---

# ADR-003: Canvas rendering — PixiJS v8 (WebGL) with HTML overlays for interactive content

**Status:** Accepted
**Scope:** global

## Context

ADR-002 settled the frontend: **Svelte 5 + SvelteKit (static adapter)** inside
the Tauri 2 WebView. This ADR picks the rendering technology for GUPPI's
infinite canvas, which mounts inside that Svelte frontend.

v1 is canvas-only: pan, zoom, drag, hundreds of tiles, child connections.
Future requirements: live indicators, status badges, and embedded terminal
panels — interactive text rendered *inside* a tile.

Three rendering approaches exist:

1. **SVG** (e.g. via svelte-flow / d3) — DOM-native interactions, accessible,
   but performance degrades around a few hundred nodes once connectors and
   re-renders are in play. A poor fit for a Miro-like zoomed-out density.
2. **HTML5 canvas 2D** (Konva, fabric.js, or hand-rolled) — a good middle
   ground, handling roughly thousands of objects, but text rendering and
   embedded interactive panels are awkward.
3. **WebGL via PixiJS** — the highest performance, scaling to many thousands
   of nodes with smooth zoom/pan and a mature API. Embedding interactive
   content (a terminal panel) is done by *overlaying* HTML on top of the
   canvas, synced to PixiJS coordinates — exactly Miro's and Figma's approach.

## Decision

Use **PixiJS v8** (WebGL) for the canvas — tiles, edges, badges, and
hit-testing.

When a tile needs rich interactive content (markdown viewer, emulated
terminal), render an **HTML overlay positioned to match the tile's world
coordinates**. A single source of truth for camera state (pan + zoom),
modelled as Svelte 5 runes per ADR-002, is subscribed to by both the PixiJS
scene and the HTML overlay layer so the two stay in lockstep.

This builds directly on ADR-002: PixiJS mounts inside a Svelte component, and
the overlay layer is plain Svelte markup driven by the same reactive camera
state.

## Consequences

- (+) Future-proof for terminal panels, dense canvases, and fluid pan/zoom.
- (+) Mature library with extensive prior art for infinite canvases.
- (–) More upfront work than a "drop in svelte-flow" approach — the first
  tile takes longer than it would in SVG.
- (–) Accessibility (screen readers) is poor on a canvas. Acceptable here:
  GUPPI is a single-user tool, not a public product.

## Reversibility

Medium. The world-coordinate abstraction — camera state plus the mapping
between world and screen coordinates — is portable. The rendering layer
behind it could be swapped (e.g. to canvas 2D) without disturbing the
overlay layer or the rest of the frontend, though it would still be a
non-trivial rework.
