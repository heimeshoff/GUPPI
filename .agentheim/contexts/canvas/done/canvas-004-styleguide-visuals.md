---
id: canvas-004-styleguide-visuals
type: feature
status: backlog
scope: bc
depends_on:
  - design-system-001-styleguide
related_adrs:
  - ADR-003
related_research: []
prior_art: []
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
