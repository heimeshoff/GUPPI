---
id: design-system-006
title: "Styleguide: BC accordion row, kanban board/column, task card, docked detail panel"
status: done
type: feature
context: design-system
created: 2026-05-24
completed: 2026-05-24
commit:
depends_on: []
blocks: [canvas-020, canvas-021, canvas-022, agent-awareness-002]
tags: [styleguide, tokens, kanban, accordion, task-card, detail-panel, gate]
related_adrs: [ADR-002, ADR-004]
related_research: []
prior_art: [design-system-002, design-system-003, design-system-004]
---

## Why

The canvas pivot introduces visual elements the styleguide does not yet
define: the collapsible BC accordion row (with its active/blocked/idling
roll-up), the kanban board + its four columns (BACKLOG/TODO/DOING/DONE), the
task card (per status, per type, with live-agent indicator), and the docked
right-edge detail panel with its blocked-question callout and its in/out
motion (reference: `.agentheim/contexts/design-system/references/kanban.png`).
This BC is the **styleguide GATE**: every canvas frontend task in the pivot
(`canvas-020/021/022`) must `depends_on` this task and build against
`STYLEGUIDE.md`. No interior UI work promotes to `doing/` before these
component contracts + tokens exist.

## What

Extend `STYLEGUIDE.md` (new sections, following the §3.6–3.8 pattern) and
mirror tokens across `tokens.ts` (PixiJS-ready numerics — for whatever the
frame SHELL keeps in Pixi) and `tokens.css` (CSS custom properties — the
HTML overlay layer the interior likely uses; canvas-019 decides the
substrate, but tokens.css is needed regardless since ADR-003 already runs an
overlay layer):

- **BC accordion row** — collapsed + expanded states; the disclosure
  chevron; the row header (BC name, the active/blocked/idling roll-up pills,
  total task count); divider treatment; the collapse animation budget.
- **Kanban board + column** — the four columns (BACKLOG/TODO/DOING/DONE),
  column header label treatment, column min/max width, inter-column gap,
  horizontal-scroll affordance, empty-column state.
- **Task card** — id label, title (wrap/clamp rule), tag chips, the status
  glyph (reuse the §2.2 four-state status palette: idle/running/blocked/
  missing), and the live-agent indicator line ("orchestrator · waiting
  2m 14s") with its own treatment for running vs. blocked. Card states:
  default / hover / selected (the selected card is the one whose detail
  panel is open) / blocked (the red-accent card in the reference).
- **Docked detail panel** — the right-edge panel: width, header (task id +
  status pill + tags), body typography (reuse the canvas-009 reader pins
  where they fit — 14px Inter, line-height 1.65), the "AGENT NEEDS AN
  ANSWER" callout (the warm-accent boxed region with answer/defer/edit
  buttons), and the slide-in/out **motion** (duration + easing — align with
  the existing canvas-003 motion pins: 320ms / `cubic-bezier(.16,.84,.36,1)`
  unless the panel warrants its own; decide and document).
- Reconcile against decisions #1/#2/#3: the accordion+kanban interior FULLY
  REPLACES the §3.7 BC bubble + §3.8 intra-project edge vocabulary on the
  canvas surface. Mark §3.7/§3.8 as **superseded for the canvas interior**
  in STYLEGUIDE.md (do not delete — the relationship vocabulary may resurface
  in a future cross-project view; annotate, don't erase).

## Acceptance criteria

- [ ] `STYLEGUIDE.md` gains new sections for: BC accordion row, kanban
      board/column, task card (with all states), docked detail panel +
      callout + motion. Each cites tokens by name.
- [ ] New tokens land in BOTH `tokens.ts` and `tokens.css` (mirror), for
      every dimensional + colour + motion value the new sections reference;
      reuse existing §2.2 status palette + §2.5 motion tokens where they fit
      rather than duplicating.
- [ ] Both themes covered: every new colour token has a `colorDark` +
      `colorLight` value (design-system-004 dual-palette pattern) and a
      `[data-theme="light"]` entry in `tokens.css`.
- [ ] §3.7 (BC bubble) + §3.8 (intra-project edges) annotated as superseded
      for the canvas interior, with a pointer to the new sections and to
      canvas-019; the underlying relationship vocabulary preserved, not
      deleted.
- [ ] A `references/` sketch (ASCII / pseudocode, the design-system-002
      pattern) fixes the layout vocabulary for sign-off without prejudging
      pixel-true layout. The `kanban.png` reference is the visual anchor.
- [ ] `pnpm check` 0/0/0; `cargo test --lib` unchanged (no Rust in this
      task — tokens + styleguide only, like design-system-002/003).

## Notes

GATE task. canvas-020/021/022 `depends_on` this. It does NOT depend on
canvas-019 (the visual vocabulary is substrate-agnostic; tokens.ts AND
tokens.css both get values so whichever substrate canvas-019 picks is
served). agent-awareness-002 depends on this for the card agent-indicator
+ panel callout components.

Marco's styleguide sign-off pattern: like design-system-002, the in-person
visual confirmation happens when the first consumer (canvas-020) runs in
`pnpm tauri dev`; this task fixes the contract + tokens + sketch, sign-off
is the deferred human gate.

## Outcome

Extended `STYLEGUIDE.md` with the canvas-pivot interior vocabulary:
new contract sections **§3.9 BC accordion row**, **§3.10 kanban
board/column**, **§3.11 task card** (default / hover / selected / blocked
states + the live-agent indicator line), **§3.12 docked detail panel** (+ the
"AGENT NEEDS AN ANSWER" callout + slide-in motion). Each section cites tokens
by name. The §2.1 colour table, §2.3 typography table (reader pins 14px /
1.65), §2.5 shape table, and §2.6 motion table each gained a
kanban-accordion-interior sub-grouping. §3.7 (BC bubble) + §3.8 (intra-project
edges) annotated as **superseded for the canvas interior** — preserved, not
deleted, with pointers to §3.9–3.12 and canvas-019 / ADR-017 (the vocabulary
may resurface in a future cross-project relationship view).

Mirrored every new colour + dimensional + motion value across
`src/lib/design/tokens.ts` (PixiJS-ready numerics, so a future Pixi consumer
can read them and the dual-file contract holds) **and**
`src/lib/design/tokens.css` (the DOM-overlay layer's actual consumer —
canvas-019 / ADR-017). Both themes covered: every new colour token has a
`colorDark` + `colorLight` value in `tokens.ts` (the `Palette` type enforces
parity) and a `[data-theme="light"]` entry in `tokens.css`. The §2.2 status
palette + §2.6 `durationPulse`/`glow` are **reused** by the interior (card
glyphs, roll-up pills, running-pulse) rather than duplicated.

New `references/kanban-accordion-sketch.md` ASCII layout reference for the
deferred sign-off (the design-system-002 pattern; `kanban.png` is the pixel
anchor). Recorded the open-question defaults in **STYLEGUIDE §5 Q11** (panel
motion gets its own `easePanel` curve at the shared 320ms budget; accordion
240ms; column width 200–280px fluid; card 2-line clamp; warm-orange callout;
DOM-overlay substrate per ADR-017) — no new design-system ADR (consistent
with Q4–Q10; substrate decision lives in ADR-017 from canvas-019).

`pnpm check` = 0/0/0. No Rust touched (`cargo test --lib` unchanged). Key
files: `STYLEGUIDE.md` §2.1/§2.3/§2.5/§2.6 + §3.7/§3.8 annotations +
§3.9–3.12 + §5 Q11; `src/lib/design/tokens.ts`; `src/lib/design/tokens.css`;
`references/kanban-accordion-sketch.md`; BC `README.md` (ubiquitous language
+ styleguide section). Consumers `canvas-020/021/022` + `agent-awareness-002`
now build against §3.9–3.12.
