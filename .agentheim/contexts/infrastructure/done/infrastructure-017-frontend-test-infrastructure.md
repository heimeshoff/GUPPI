---
id: infrastructure-017-frontend-test-infrastructure
type: feature
status: done
completed: 2026-05-19
scope: bc
depends_on: []
related_adrs:
  - ADR-002
  - ADR-003
related_research: []
prior_art:
  - canvas-001-targeted-canvas-updates
  - canvas-002-render-multiple-project-tiles
  - canvas-006-live-add-race-on-concurrent-project-added
  - canvas-007-project-as-frame
---

# Frontend test infrastructure — Vitest for the pure modules

## Why

Three pure, Svelte/Pixi-free modules under `src/lib/` have accumulated
without a runnable test surface:

- `src/lib/snapshot-patch.ts` — targeted in-place patching of
  `ProjectSnapshot` from fine-grained filesystem events (canvas-001).
- `src/lib/tile-layout.ts` — deterministic outward-spiral auto-placement
  of project frames (canvas-002).
- `src/lib/bc-layout.ts` — deterministic one-shot force-directed layout
  of BC bubbles inside a project frame (canvas-007).

Each of these modules was extracted from `Canvas.svelte` *specifically*
so it could be unit-tested in isolation when test infrastructure lands;
each canvas task's done note has flagged the absence of that infra as
the reason no `*.test.ts` files were added.

The verification gap has been tolerable while the modules were small,
but `bc-layout.ts` is the first one with real algorithmic complexity
(spring-electrical simulation, deterministic seed, pinned-position
contract, frame auto-fit). Future tasks against any of these will land
faster and safer with vitest in place.

## What

- Add `vitest` to `package.json` devDependencies, with a thin
  `vitest.config.ts` that ignores the SvelteKit pipeline (the pure
  modules don't need it).
- Add a `pnpm test` script that runs `vitest run`.
- Backfill tests for each pure module's invariants:
  - `snapshot-patch.ts`: lazy-create on `task_*` before `bc_appeared`;
    count-clamp-at-0 on underflow; BC ordering after lazy-create matches
    the Rust `get_project` sort; `bc_relationships_changed` returns
    `false` and ensures the node exists.
  - `tile-layout.ts`: `spiralPosition(0) = (0,0)`; consecutive indices
    are adjacent; spiral-ring growth (right, down, left×2, up×2,
    right×3, …); pure same-input-same-output check.
  - `bc-layout.ts`: deterministic same-input-same-output across
    multiple runs; pinned positions are sticky (output frame-local
    coords match input saved positions); related BCs cluster (edge-
    length metric); frame auto-fits to BC union; empty BC list returns
    the inner-min frame.

## Acceptance criteria

- [ ] `pnpm test` runs the full suite green from a clean clone.
- [ ] Each pure module has ≥ 3 tests covering its load-bearing
      invariants.
- [ ] `pnpm check` stays at 0 errors / 0 warnings.
- [ ] CI integration (if/when there is CI) runs `pnpm test` alongside
      `cargo test --lib`.

## Scope (in)

- `vitest.config.ts`, `package.json` devDependency + script.
- `*.test.ts` files in `src/lib/`.

## Scope (out)

- Svelte component tests (the pure modules are the testable surface;
  components are exercised manually via `pnpm tauri dev`).
- E2E / WebDriver tests.

## Notes

Captured during `canvas-007-project-as-frame` (2026-05-16): the
project-as-frame work added the third pure module to the canvas BC, all
designed for isolated unit testing but currently un-tested. Pre-existing
across canvas-001 / canvas-002 / canvas-006 done notes.

## Outcome

Vitest 2 is now the frontend test runner for GUPPI's pure modules. Twenty-
nine characterisation tests across three test files cover each pure module's
load-bearing invariants as captured in its originating task's Outcome:

- `src/lib/tile-layout.test.ts` (9 tests) — `spiralPosition(0)` at origin,
  the documented leg pattern (right, down, left×2, up×2, right×3, …),
  adjacency of consecutive indices, injectivity over the first 50 indices,
  purity, defensive fallback for negative / non-finite indices, and the
  `spiralPositions(n)` helper.
- `src/lib/snapshot-patch.test.ts` (10 tests) — lazy zero-count BC creation
  on `task_*` before `bc_appeared`, idempotent `bc_appeared` after lazy
  create, alphabetical sort matching the Rust `get_project` order,
  count-clamp-at-0 on `task_removed` underflow (and on the `from` side of
  a `task_moved`), normal `task_moved` / `bc_disappeared` path,
  `bc_relationships_changed` returning `false` while still ensuring the
  node exists and preserving existing counts / relationships, and the
  no-op-and-`false` contract for non-patch events.
- `src/lib/bc-layout.test.ts` (10 tests) — empty-input minimum frame size,
  determinism across runs *and* across input-array reorderings, sticky
  pinned positions (single + multiple pins), finite unpinned positions
  around pins, clustering of related BCs measured via total edge length,
  minimum-frame floor for one small BC, frame grows when a pin is dragged
  far out, and unpinned-only content sits at the documented `framePadding`
  / `frameHeaderHeight + framePadding` offsets.

Key new files:
- `vitest.config.ts` (project root) — minimal config; node env, no
  SvelteKit pipeline, includes `src/lib/**/*.test.ts`.
- `package.json` — added `vitest` ^2.1.0 devDep and `pnpm test` script
  (`vitest run`).
- `src/lib/{snapshot-patch,tile-layout,bc-layout}.test.ts` — the three
  characterisation test files.
- `infrastructure/README.md` — new vocabulary block: **pure module** and
  **characterisation test**.

Acceptance:
- `pnpm test` — 29 passed (3 test files), ~1s.
- `pnpm check` — 0 errors / 0 warnings (990 files).
- No CI yet; the `pnpm test` script is shape-ready for whenever CI lands.
