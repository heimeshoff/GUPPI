---
id: canvas-024
title: Retire the throwaway spike-019a perf harness
status: backlog
type: chore
context: canvas
created: 2026-05-24
completed:
commit:
depends_on: [canvas-019]
blocks: []
tags: [cleanup, spike, throwaway, kanban, pivot]
related_adrs: []
related_research: [canvas-hybrid-perf-2026-05-24]
prior_art: [canvas-019a]
---

## Why

`canvas-019a` built a clearly-marked THROWAWAY perf harness at the route
`/spike-019a` (`src/routes/spike-019a/+page.svelte`) plus a
`window.__guppiSpike` instrumentation seam, so Marco could capture
hybrid Pixi-shell + DOM-interior frame-times for the `canvas-019`
substrate decision. Once `canvas-019` has decided (and its production
implementation tasks `canvas-020/021/022` are landing the real interior),
the harness has served its purpose and should be deleted so it does not
rot or get mistaken for production code.

## What

- Delete `src/routes/spike-019a/`.
- Confirm nothing imports from it (it should be self-contained; only
  `$lib/camera.svelte` was imported INTO it, never the reverse).
- `pnpm check` + `pnpm build` stay clean after removal.

## Acceptance criteria

- [ ] `src/routes/spike-019a/` removed.
- [ ] `pnpm check` clean and `pnpm build` succeeds.
- [ ] No dangling references to `__guppiSpike` or the route in the
      codebase.

## Notes

Do this only AFTER `canvas-019` has consumed the findings note
(`canvas-hybrid-perf-2026-05-24`) — hence `depends_on: [canvas-019]`. If
hybrid was ratified, the production interior lands in `canvas-020`+; if
Option B was chosen, the harness is moot either way. The findings note
and operator `console-snapshot.md` are knowledge artifacts and stay.
