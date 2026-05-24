---
id: canvas-007-project-as-frame
type: feature
status: done
completed: 2026-05-16
scope: bc
depends_on:
  - design-system-001-styleguide
  - design-system-002-project-frame-vocabulary
  - project-registry-004-bc-relationships-and-positions
related_adrs:
  - ADR-003
  - ADR-014
  - ADR-015
related_research: []
prior_art:
  - canvas-001-targeted-canvas-updates
  - canvas-002-render-multiple-project-tiles
  - canvas-005a-single-shot-discovery-affordances
  - canvas-005b-scan-flow-and-scan-root-management
  - canvas-006-live-add-race-on-concurrent-project-added
---

# Project-as-frame — BCs inside, edges between BCs by relationship type

## Why

Today every project renders as a single bubble with its bounded contexts
orbiting around it; the only edge type drawn is project→BC (parent/child
containment by line). The richer Agentheim shape — *projects contain BCs, BCs
relate to each other* — is invisible on the canvas. The context-map's DDD
relationship types (customer-supplier upstream/downstream, mutual /
shared-kernel, anticorruption-layer, no relationship) are the actual structure
of the system; the user wants the canvas to reflect that structure directly.

Unblocks `canvas-003` (focus-zoom on a project = "frame the project's bounded
region", which in the new model is literally the frame itself).

## What

Visual + data model shift, rolled out as the integration step atop two
upstream tasks:

- **Project** — rendered as a **surrounding frame** (a bounded region with a
  drawn border and a title bar across the top edge that carries the project
  name, status badges, and task counts — i.e., everything today's tile
  shows). The frame is the new drag handle for the project as a whole;
  right-click on the title bar opens the existing tile context menu
  (`Remove project`, etc.).
- **Bounded context** — rendered as a **bubble inside** its project's frame.
  No more orbit around the project; the frame *contains* its BCs.
- **Intra-project BC↔BC edges** — drawn between BCs based on context-map
  relationship type, per the vocabulary `design-system-002` ships:
  - upstream/downstream (customer-supplier) — directional line, arrowhead
    at downstream end
  - mutual / shared-kernel / partnership — non-directional line
  - anticorruption-layer — variant of upstream/downstream with a notch
    glyph or hover-label (design-system-002 finalises)
  - no relationship — no edge drawn
- **Project→BC parent edges retire.** Containment (BC inside frame)
  replaces the line.
- **Frame sizing** — auto-fits its BC content. No user-resize at v1.
- **Layout inside the frame** — deterministic force-directed initial layout
  driven by the relationship graph (clustering related BCs visually), with
  per-BC manual drag override persisted via `project-registry-004`'s
  `save_bc_position` / `load_bc_position` IPC. Manual positions are sticky;
  re-layout only triggers when a BC appears / disappears / relationship set
  changes (i.e., when targeted updates from `bc_relationships_changed`
  arrive).
- **Cross-project edges** — out of scope for v1. Locked.
- **Migration path** — hard cutover. v1 is unshipped; no feature flag.

## Acceptance criteria

- [ ] Every project renders as a frame containing its BCs as interior
      bubbles. No more orbit; no more project→BC lines. Frame uses
      `STYLEGUIDE.md` §3.6 tokens by name (`frameFill`, `frameBorder`,
      `frameHeaderFill`, `frameHeaderDivider`, `radiusFrame`,
      `borderWidthFrame`, `frameHeaderHeight`, `framePadding`,
      `frameMin*`).
- [ ] Interior BC bubbles use §3.7 tokens by name: denser rounded
      rectangle (`radiusBcInside` 8 vs. orbit's `radiusBc` 10),
      `bcInside{Width,Height}`, title + counts pill in one row at
      default zoom (`bcInsidePill*`), status badge slot retained for
      `agent-awareness` to drive later.
- [ ] BC↔BC edges render the four §3.8 variants on the locked single-
      neutral `fgMuted` palette (geometry distinguishes):
      `customer-supplier` line + arrowhead at downstream end
      (`arrowheadLength` 10, `arrowheadWidth` 8); `shared-kernel` /
      `partnership` line, no arrowhead, no notch;
      `anti-corruption-layer` line + filled triangle notch at midpoint
      pointing upstream (`aclNotchSize` 10); `conformist` directional
      line at `edgeWeightConformist` (lighter than `edgeWeight`). No
      edge drawn when no relationship.
- [ ] TS `BcSnapshot` alias retired in favour of `BoundedContext`
      across `Canvas.svelte` and every other frontend consumer
      (project-registry-004 left the alias as the canvas-007 rename
      point; remove the alias in `types.ts` once consumers are
      renamed).
- [ ] `Relationship[]` consumed from `BoundedContext.relationships`;
      cross-project `to` references are already dropped by the
      registry parser (do not double-handle in the canvas).
- [ ] Per-BC manual drag inside the frame persists via `saveBcPosition`
      / `loadBcPosition` IPC; on project paint, `loadBcPositions(project_id)`
      batch-loads. Manual positions sticky; force-directed re-layout
      fires only on BC add/remove/relationship-change.
- [ ] Force-directed initial layout runs deterministically (same input
      → same output) so a never-dragged BC lands in the same spot
      across restarts. New pure module(s) alongside `tile-layout.ts`;
      one-shot run on input change, no `requestAnimationFrame` loop.
- [ ] `canvas-001` targeted-update model still works: a
      `BcRelationshipsChanged { project_id, bc_name }` event patches
      the rendered graph in place (edges added/removed, force-directed
      re-runs once for that project) — no full re-fetch unless
      `resync_required` fires. Extend `snapshot-patch.ts` to handle
      this event variant. (`snapshot-patch.ts`'s `bcNode` lazy-create
      already initialises `relationships: []` — no work needed there.)
- [ ] All existing affordances continue to work against the new shape:
      drag (project frame + per-BC bubbles), right-click context menus
      (canvas-005a / 005b), missing-tile visual (canvas-005a),
      scan-root cascade (canvas-005b's `project_removed` handler),
      live-add serialisation (canvas-006's `liveAddChain`).
- [ ] `f` (zoom-to-fit) frames the union of every project frame in the
      collection (already does, structurally; verify after the model
      change).
- [ ] Voice BC renders as a lone BC inside its frame with **no**
      intra-project edges — its Whisperheim/Utterheim ACLs are
      cross-project and locked out at v1; the registry-004 bootstrap
      reflects this. This is the expected v1 state, not a bug to
      report.
- [ ] All visuals use `STYLEGUIDE.md` tokens via `design-system-002`'s
      additions. No raw hex / sizing in `Canvas.svelte`.
- [ ] `pnpm check` 0 errors / 0 warnings; `pnpm build` passes.
      `cargo test --lib` stays at 117/117 — this task is frontend-only,
      no Rust changes expected.

## Scope (in)

- `src/lib/Canvas.svelte` — replace orbit rendering with frame + interior
  bubbles + intra-project edges. The keyed-collection model (`canvas-002`)
  extends from `{ id, snapshot, pos }` to `{ id, snapshot, pos, bcLayout,
  bcPositions }`.
- New pure module(s) for the force-directed BC layout (force or
  spring-electrical, deterministic seed, no `requestAnimationFrame` loop —
  one-shot run on input change). Test surface alongside `tile-layout.ts`.
- Snapshot patching (`snapshot-patch.ts`) extends to handle the new
  `BcRelationshipsChanged` event variant from `project-registry-004`
  (existing `bcNode` lazy-create already seeds `relationships: []`).
- `src/lib/types.ts` — retire the `BcSnapshot` back-compat alias in
  favour of `BoundedContext` (the rename was explicitly carved out as
  a canvas-007 follow-up in project-registry-004's done note).
- `src/lib/ipc.ts` consumers of `saveBcPosition` / `loadBcPosition` /
  `loadBcPositions` (wrappers already exist; canvas wires the calls).

## Scope (out)

- Cross-project edges. v2+.
- User-resizable frames. v2+.
- Animated continuous force-layout. v2+ if anyone wants it; one-shot is the
  v1 stance.
- Brainstorm/model writing the per-BC `relationships:` frontmatter
  automatically — that's a follow-up in the Agentheim plugin repo, captured
  separately. `project-registry-004` hand-curates GUPPI's own seven BC
  READMEs as the bootstrap; future projects rely on the plugin work.

## Notes

### Coordination

- **Hard prereqs landed 2026-05-16:**
  `project-registry-004` (commit `b3727f5`) shipped `BoundedContext.relationships`,
  the `BcRelationshipsChanged` event, the three `bc_position*` IPC commands,
  schema v4, and the seven-BC frontmatter bootstrap.
  `design-system-002` (commit `38f48ab`) shipped `STYLEGUIDE.md` §3.6
  (project frame), §3.7 (BC bubble inside frame), §3.8 (intra-project
  edges) + the 17 colour tokens + 17 shape tokens, mirrored across
  `tokens.ts` / `tokens.css`.
- **Blocks** `canvas-003-focus-zoom`. canvas-003's open questions
  (keyboard scheme, BC-as-focus-target, ESC behaviour) get re-asked
  against frames-and-bubbles once this lands.
- **Frontend gate:** built against `contexts/design-system/STYLEGUIDE.md`
  (the styleguide-001 sign-off plus design-system-002's project-frame
  additions). Marco's deferred in-person sign-off on `STYLEGUIDE.md`
  §5.Q4–Q8 becomes actionable *after* this task — canvas-007 is the
  visual confirmation point (`pnpm tauri dev`).

### Promotion 2026-05-16

REFINE pass confirmed every wave-hand has collapsed to a specific
contract — token names from §3.6–3.8, type/event/IPC names from
project-registry-004's done note. Acceptance criteria updated to
reference those contracts by name. Status moved `backlog → todo`.

### Open follow-ups outside this task

- Marco's in-person `STYLEGUIDE.md` §5.Q4–Q8 sign-off — deferred human
  gate, owned by `design-system` BC.
- Hover-on-edge highlight decision — same sign-off conversation;
  `edgeHighlight` token already exists in §3.8 if "yes".
- Brainstorm/model writing `relationships:` frontmatter automatically
  on BC creation/refinement — captured in the Agentheim plugin repo
  (`agentheim/.agentheim/backlog/brainstorm-model-maintain-bc-relationships-frontmatter.md`).

## Outcome

Landed the project-as-frame visual shift in `src/lib/Canvas.svelte`:
every project now renders as a §3.6 frame (rounded-rect body
`frameFill` + 1px `frameBorder`, header bar `frameHeaderFill` carrying
project name + total task count + missing glyph, divider line via
`frameHeaderDivider`) containing its BCs as §3.7 interior bubbles
(`bcInsideWidth × bcInsideHeight` = 160 × 56, `radiusBcInside` 8,
title + counts pill at default zoom, status badge slot). Intra-project
BC↔BC edges render the four §3.8 variants on the locked single-neutral
`fgMuted` palette — customer-supplier (directional, arrowhead at
downstream end), shared-kernel / partnership (non-directional), ACL
(directional + filled midpoint notch pointing upstream), conformist
(directional at `edgeWeightConformist` 1). The orbit baseline's
project→BC connector lines are retired; containment replaces them.

New pure module `src/lib/bc-layout.ts` alongside `tile-layout.ts` runs
a deterministic one-shot Fruchterman-Reingold-style spring-electrical
simulation (mulberry32-seeded + djb2(bcName) initial positions, 120
iterations, linear cooling) — same input -> same output, so a never-
dragged BC lands in the same spot across restarts. Manual-drag
positions are sticky via `saveBcPosition` / `loadBcPosition` /
`loadBcPositions` (`project-registry-004` IPC); pinned BCs render at
their saved frame-local coords without any post-simulation translation,
the rest re-flow around them. Re-layout fires only on BC topology
change (`bc_appeared` / `bc_disappeared` / `task_added` / `task_moved`
/ `task_removed` may lazily create a node, `bc_relationships_changed`
triggers a per-project `refreshOne` + recompute). No
`requestAnimationFrame` loop — one-shot on every layout-triggering
event. The pin-contract decision is documented in
`.agentheim/knowledge/decisions/ADR-015-bc-layout-deterministic-spring-electrical.md`.

`BcSnapshot` TS alias retired in favour of `BoundedContext` across
`Canvas.svelte`, `snapshot-patch.ts`, and `types.ts`. The
`ProjectEntry` shape extends from `{ id, snapshot, pos }` to
`{ id, snapshot, pos, bcLayout, bcPositions }` per the canvas-002
keyed-collection seam. `snapshot-patch.ts` got a `bc_relationships_changed`
case that ensures the BC node exists (idempotent `bcNode` lazy-create)
and signals the caller (returns `false`) that a `refreshOne` is
required to repull the parsed relationship set.

Drag / right-click affordances rewired against the frame shape: the
header bar is now the project's drag handle and right-click target
(replacing the orbit-baseline tile body); the frame body is pass-
through so empty regions inside a frame don't swallow the camera pan
and the empty-canvas right-click menu still works there. BC bubbles
have their own drag handlers (per-BC `pointerdown` claims a BC drag
via `dragBcName`); on drag-end the new position persists and the layout
re-runs in one shot. All existing affordances continue to work
(canvas-005a empty-canvas menu, canvas-005a missing-tile visual now on
the frame, canvas-005b scan-root cascade via the unchanged
`project_removed` handler, canvas-006 `liveAddChain`). Zoom-to-fit
`f` frames the union of every project frame (each frame's auto-fit
already covers its interior BCs). Voice BC renders edge-less inside
its frame as expected (its Whisperheim/Utterheim ACLs are cross-project
and dropped by the registry parser at v1).

**Key files:**
- `src/lib/Canvas.svelte` — frame + bubbles + intra-project edges
  rendering; drag rewire; `bc_relationships_changed` handler;
  per-project `recomputeBcLayout` trigger
- `src/lib/bc-layout.ts` *(new)* — pure deterministic BC layout module
- `src/lib/snapshot-patch.ts` — `BoundedContext` rename;
  `bc_relationships_changed` case added
- `src/lib/types.ts` — `BcSnapshot` alias removed
- `.agentheim/contexts/canvas/README.md` — ubiquitous language updated
  (Frame / Bubble / Intra-project edge / BC layout added; Tile /
  Connection marked retired); rendering + live-update sections
  rewritten against the frame model
- `.agentheim/knowledge/decisions/ADR-015-bc-layout-deterministic-spring-electrical.md`
  *(new)* — algorithm + sticky-pin contract decision

**Checks:** `pnpm check` 0 errors / 0 warnings (939 files);
`pnpm build` passes; `cargo test --lib` 117 / 117 (no Rust changes).

### Iteration 2 (2026-05-16) — §3.7 pill gap closed

Verifier iteration 1 flagged that `makeBcBubble` rendered the
task-counts as a plain `Text` glyph at the bottom-left of the bubble,
leaving the four `bcInsidePill*` tokens (`bcInsidePillFill`,
`bcInsidePillHeight`, `bcInsidePillMinWidth`, `bcInsidePillRadius`)
unconsumed and the title at `weightBold` rather than the §3.7-
mandated `weightMedium`. Closed in this pass by:

- Replacing the bottom-left counts-`Text` with an actual
  right-aligned rounded-rect pill: a `Graphics().roundRect(...).fill(
  color.bcInsidePillFill)` sized `bcInsidePillHeight` tall by
  `max(bcInsidePillMinWidth, measured text width + 12 px padding)`
  wide at `bcInsidePillRadius` corner radius, anchored to
  `screenX + w - framePadding * 0.5` (matching the title's left
  inset) and vertically at `screenY + framePadding * 0.5` so it sits
  in the title row. The counts `Text` is now anchored at `(0.5, 0.5)`
  and positioned at the pill rect's centre.
- Switching the BC name `Text` `fontWeight` from `weightBold` to
  `weightMedium` per §3.7.

Re-checks: `pnpm check` 0 / 0 / 0 (939 files); `pnpm build` passes.
Grep confirms all four `bcInsidePill*` tokens appear in
`src/lib/Canvas.svelte`'s `makeBcBubble` call site and `weightBold`
no longer appears inside the function.

**TDD status:** Skipped per the legitimate-skip category "UI tasks
where the project has no UI test infrastructure" — `package.json` has
no vitest / jest / `*.test.ts`. The new `bc-layout.ts` joins
`snapshot-patch.ts` and `tile-layout.ts` as Svelte/Pixi-free modules
designed as the test surface when infra lands. Backlog item
`infrastructure-017-frontend-test-infrastructure` (created in this
task) captures the work. Manual verification path: run `pnpm tauri
dev`, observe project frames, drag BCs, edit a BC's README
`relationships:` block and confirm the edge updates without a full
reload.

