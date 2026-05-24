---
id: canvas-004-styleguide-visuals
type: feature
status: done
scope: bc
depends_on:
  - design-system-001-styleguide
related_adrs:
  - ADR-003
related_research: []
prior_art: []
completed: 2026-05-16
commit:
subsumed_by: canvas-008
---

## Closure note (2026-05-16)

**Subsumed by `canvas-008-apply-revised-tokens`.** No work was done on
this task; it is closed without a commit. Reasoning:

- Originally captured as "replace greybox tiles/nodes/edges/counts/badge-slot
  with `STYLEGUIDE.md` tokens" against the orbit baseline.
- `canvas-002` retired the single-tile greybox path.
- `canvas-007` (commit `e2296c2`, 2026-05-16) shipped the project-frame
  rendering against the §3.6/§3.7/§3.8 tokens — most of what this task
  was scoped to cover.
- The 2026-05-16 design (`references/claude-design-2026-05-16/`)
  revises the palette + status colors + v1 dimensions. Re-applying the
  revised tokens against the existing canvas is the residual scope of
  this task — captured fresh as `canvas-008-apply-revised-tokens` with
  explicit acceptance criteria (token-application audit, theme-flip
  end-to-end test, visual fidelity cross-check against the design
  artboards).
- Carrying canvas-004 forward against `canvas-008` would have meant two
  near-identical tasks with the older one's scope already half-
  obsolete. Closure is cleaner than refinement.

Audit trail preserved here; original captured scope below.

---

# Apply styleguide visuals — greybox → STYLEGUIDE.md

## Why

Everything the canvas draws today is **greybox** — plain rectangles and lines,
explicitly sanctioned by the walking skeleton's scope ("the styleguide hasn't
been signed off yet"). It has now been signed off (Marco, 2026-05-14), so the
gate is open. v1 is the surface that delivers "relief" from the vision — and a
greybox surface doesn't land that. This task makes the canvas *look like GUPPI*.

## What

Replace the greybox rendering with the `STYLEGUIDE.md` visual vocabulary:

- **Project tiles** — the rounded-rectangle tile treatment, tokens, typography
  from the styleguide (the sign-off confirmed dark-only, rounded-rectangle
  tiles).
- **BC nodes** — styled child nodes, the supporting/core/generic distinction if
  the styleguide expresses one.
- **Connections** — styled edges between tile and BC nodes.
- **Task counts** — the count display (backlog / doing / done) in styleguide
  type and colour, not raw text.
- **Status badge placeholders** — the per-BC badge slot styled per the
  styleguide, even though `agent-awareness` doesn't feed it yet (the badge is
  v1-shaped, the live data is Beyond-v1).
- All values pulled from styleguide tokens — no hardcoded colours/sizes.

## Acceptance criteria

- [ ] Tiles, BC nodes, connections, and task counts render using
      `STYLEGUIDE.md` tokens and component definitions — no greybox left on the
      canvas.
- [ ] No hardcoded visual values; everything resolves from styleguide tokens.
- [ ] The status-badge slot is visually present and styled, even with no live
      `agent-awareness` data.
- [ ] Visuals stay coherent at different zoom levels.

## Notes

Surfaced from the v1 "finish v1 first" capture pass (2026-05-14).

Frontend gate: built against `contexts/design-system/STYLEGUIDE.md` — this task
*is* the gate being exercised.

Soft ordering: best done against the multi-tile canvas (`canvas-002`) so the
styling covers the real v1 surface, but the styling work itself does not hard-
depend on it. No `depends_on` link to `canvas-002`; refinement can sequence.

Open question for refinement: how much of the `STYLEGUIDE.md` component set
already exists as built components vs. needs building here — depends on what
`design-system-001` actually shipped beyond the document.
