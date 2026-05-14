---
id: ADR-002
title: Frontend framework — Svelte 5 + SvelteKit (static adapter)
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-002-frontend-framework]
related_adrs: [ADR-001]
---

# ADR-002: Frontend framework — Svelte 5 + SvelteKit (static adapter)

**Status:** Accepted
**Scope:** global

## Context

ADR-001 settled the desktop runtime: **Tauri 2** with a Rust core and a
web-tech frontend. This ADR picks the framework that mounts inside the Tauri
WebView and owns canvas rendering, tile UI, the command palette, and the
terminal panel UI.

Constraints for the frontend framework:

- **No SSR.** The frontend ships as static assets inside the Tauri bundle;
  there is no server.
- **All client-side.** Everything runs in the WebView (WebView2 on Windows,
  the day-one validated target per ADR-001).
- **Small bundles.** The Tauri ethos is lean; a heavy framework runtime works
  against that.
- **Must not fight a canvas-heavy app.** GUPPI renders an infinite canvas with
  hundreds of tiles. Render-loop code must stay tight, with minimal
  framework ceremony around hot paths.

The choice came down to **Svelte 5**, **React**, and **Solid** — a
framework-preference call, since nothing downstream depends on it except the
styleguide task.

- **React** has the largest ecosystem; every canvas library has a wrapper.
  Its runtime and hooks/effects model add more ceremony in tight render code.
- **Solid** offers Svelte-like ergonomics with a React-style ecosystem.
- **Svelte 5** has a tiny runtime, compiles away most of the framework, and
  pairs naturally with Tauri's lean approach. Its ecosystem is smaller than
  React's.

Marco reviewed all three and signed off on the architect's recommendation:
**Svelte 5**.

## Decision

Use **Svelte 5** with **SvelteKit** configured for
**`@sveltejs/adapter-static`**.

- The frontend is a **single-page app** shipped as **static assets** inside
  the Tauri bundle — no SSR, no server.
- SvelteKit is used for routing, project structure, and the static build
  pipeline; the static adapter produces the asset bundle Tauri serves.
- Svelte 5 **runes** are the reactivity model for component and canvas state.

This builds directly on ADR-001: Svelte/SvelteKit owns the frontend
responsibilities listed there (canvas rendering, tile UI, command palette,
terminal panel UI), and talks to the Rust core through the thin IPC
abstraction over Tauri's `invoke`/`emit`.

## Consequences

- (+) Tiny runtime and compile-away model mean fast updates and minimal
  framework ceremony in canvas code, where renders are tight.
- (+) Excellent fit with Tauri — the static adapter matches the no-SSR,
  all-client-side, ship-as-static-assets model exactly.
- (+) Small bundles, consistent with Tauri's lean ethos and ADR-001's
  ~10–20MB bundle expectation.
- (–) Smaller ecosystem than React. If a critical canvas library is
  React-only, we would need to wrap it or port it.
- (–) Svelte 5 runes are still maturing; expect occasional rough edges and
  churn in idioms.

## Reversibility

Low cost early, high cost late. Framework choice permeates every component, so
switching after the UI is built would be a near-total frontend rewrite. Decide
once and live with it. Keeping the IPC surface behind a thin abstraction
(per ADR-001) means the *core* is unaffected by a later frontend change, but
the frontend itself would not survive a framework swap.
