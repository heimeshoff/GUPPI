---
id: canvas-003-focus-zoom
type: feature
status: backlog
scope: bc
depends_on:
  - design-system-001-styleguide
  - canvas-007-project-as-frame
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

### Decided during refinement (2026-05-15)

- **Frame target:** focusing a project frames the **project's bounded region
  including its BCs** (option "B"). This is stable across the visual redesign
  in `canvas-007` — once `canvas-007` lands, "the project's bounded region"
  is literally the project frame; until then, it means the union of the
  project tile and its orbiting BCs (same as today's `f` scoped to one
  project).
- **Sequencing:** hard-blocked on `canvas-007-project-as-frame`. The redesign
  ships first; this task is refined against the new model so the keyboard
  scheme and BC-focus question can be answered in terms of frames-and-bubbles
  rather than tiles-and-orbits.

### Open questions for refinement (still)

- Keyboard scheme — arrows to move focus + Enter to zoom? Tab cycling? A
  command-palette-style jump? (The styleguide flags command-palette-style as
  a `canvas` BC item.)
- Are BCs themselves first-class focus targets, or only projects? (Re-ask in
  the canvas-007 model — bubbles-inside-frame make BC focus more obvious.)
- Camera state on focus — persist (reopen app focused where you left off) or
  always reset to a default viewport?
- ESC / re-press behaviour — does focusing a focused tile zoom back to fit?
  Does ESC restore the previous viewport?

### Design pins (2026-05-16 — `references/claude-design-2026-05-16/`)

Marco's 2026-05-16 design (§7 of the artboard set +
`guppi-canvas-views.jsx`'s `ViewZoomOverview` / `ViewZoomFocused`) pins
the camera behaviour:

- **Zoom levels:** overview 38%, focused 110% (numbers exact).
- **Easing:** `cubic-bezier(.16, .84, .36, 1)` — matches `motion.easeStandard`.
- **Duration:** 320ms — matches `motion.durationCamera`.
- **Transform origin:** centre of the target frame.
- **Visual identity through the transition:** the focused frame keeps its
  shape, name, and BC positions — only scale and crop change. The transition
  reads as a *camera move*, not a screen change.
- **Focused state affordance:** the focused frame gets a `--g-focus-ring`
  (1.5px brand orange) — same affordance the project frame already has on
  hover after `design-system-003`.
- **Overview state affordance:** non-focused frames render at 0.6 opacity
  with the warm-faint border (`--g-periwinkle-faint`).
- **Trigger parity:** the same gesture from three input modalities — `⌘1`
  / double-click / "focus X" voice command — all run this transition.

This **unblocks the task structurally** (canvas-007 shipped 2026-05-16,
commit e2296c2) and **resolves several open questions**:

- "Camera state on focus" — design implies it doesn't persist; entering
  focus is always a transition from current viewport.
- "ESC behaviour" — design implies ESC restores the overview camera
  (the inverse 320ms transition).
- "BC focus targets" — the design only shows project-level focus; BC-level
  focus stays open for refinement.

Pending refinement (now actionable):

- Keyboard scheme final answer — does `canvas-011-command-palette-spec`
  (backlog, v2) cover the third-modality jump, or does this task need an
  in-canvas-only key set first?
- BC focus targets — re-ask in a Suggestor-mode refinement pass.
