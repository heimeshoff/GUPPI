---
id: canvas-003-focus-zoom
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

# Focus-zoom — click-or-keyboard "zoom to focus"

## Why

The vision names "click-or-keyboard 'zoom to focus'" as part of the v1
irreducible core, alongside pan/zoom/drag. On a canvas with many project tiles,
free pan-and-zoom is not enough — "where am I" cognitive overhead is one of the
vision's stated pains. Focus-zoom is the move that says "take me to *that* one".

## What

- **Focus a target** — clicking a tile (or BC node), or selecting it via
  keyboard, frames it: the camera animates to centre the target and zoom to a
  level that fits it comfortably in the viewport.
- **Keyboard navigation** — move focus between tiles without the mouse
  (exact key scheme is a refinement question).
- The camera animation respects the styleguide's motion budget
  (`STYLEGUIDE.md` — "restrained motion", per the 2026-05-14 sign-off).
- Focus is a *viewport* operation — it does not move tiles or mutate layout
  (that's drag, already in the skeleton).

## Acceptance criteria

- [ ] Clicking a tile animates the camera to frame that tile.
- [ ] A keyboard affordance moves focus between tiles and frames the focused
      one.
- [ ] Focusing never mutates tile positions or persisted layout.
- [ ] The focus animation stays within the styleguide motion budget.

## Notes

Surfaced from the v1 "finish v1 first" capture pass (2026-05-14).

Frontend gate: built against `contexts/design-system/STYLEGUIDE.md`.

Soft ordering: most meaningful *after* `canvas-002` (focus-zoom across many
tiles), but not hard-blocked by it — focus logic works against a single tile
too. No `depends_on` link to `canvas-002`; refinement can add one if the team
prefers strict sequencing.

Open questions for refinement:
- Keyboard scheme — arrows to move focus + Enter to zoom? Tab cycling? A
  command-palette-style jump?
- Does focus also frame a project's BC nodes, or only project tiles?
- Camera state on focus — does it persist (reopen app focused where you left
  off) or always reset to a default viewport?
