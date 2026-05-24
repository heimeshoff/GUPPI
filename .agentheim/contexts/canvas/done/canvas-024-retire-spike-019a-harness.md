---
id: canvas-024
title: Retire the throwaway spike-019a perf harness
status: done
type: chore
context: canvas
created: 2026-05-24
completed: 2026-05-24
commit: cef866b
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

## Outcome

The throwaway `canvas-019a` perf harness has been retired. Deleted the
self-contained route `src/routes/spike-019a/+page.svelte` (the only file
in that directory) along with the `window.__guppiSpike` instrumentation
seam it carried. `svelte-kit sync` regenerated route types, dropping the
stale `.svelte-kit/types/.../spike-019a/$types.d.ts` stub.

Verification:
- Codebase grep for `spike-019a` / `__guppiSpike` under `src/` returns
  zero hits — the harness was self-contained and only imported INTO it
  (`$lib/camera.svelte`); nothing in production referenced it.
- `pnpm check` clean: 990 files, 0 errors, 0 warnings.
- `pnpm build` succeeds; the `spike-019a` route is gone from the build
  output.

Knowledge artifacts preserved: `canvas-hybrid-perf-2026-05-24/README.md`
and `console-snapshot.md` were not touched (they remain the permanent
record of the spike's findings that fed ADR-017).
