---
id: canvas-009
title: Project detail panel — markdown reader, 520px right slide-in
status: backlog
type: feature
context: canvas
created: 2026-05-16
completed:
commit:
depends_on: [design-system-001, design-system-003, design-system-004]
blocks: []
tags: [detail-panel, markdown, long-form-reading, v1.5]
related_adrs: [ADR-003]
related_research: []
prior_art: []
---

## Why

The vision lists this surface under "Beyond v1" — "Project detail view:
rendered markdown for `vision.md`, `research/*.md`, ADRs, BC READMEs".
The 2026-05-16 design ships a pixel-detailed specification for it
(§5 of the artboard set + `guppi-panels.jsx`'s `ViewDetailPanel`).

Code agents produce a lot of writing — vision documents, ADRs, research
notes, BC READMEs. The dev needs to read them carefully, often for an
hour at a time, on a dark background. Most dark-mode reading surfaces
are awful: too much contrast, too much chrome, fonts not tuned for length.
This task captures the design pins so when the panel ships, it ships
correctly the first time.

## What

The spec is fully captured in `references/claude-design-2026-05-16/project/guppi-panels.jsx`
(component) + the §5 rationale in `GUPPI.html`. Summary of pinned
decisions:

**Layout / chrome:**

- Panel slides in from the right when a project is focused.
- 520px wide.
- Surface `--g-surface-3` (panels & modals).
- 12px corner radius (`--g-r-panel`), 1px hairline-strong border.
- Sits ON the canvas, not over — the project frame behind remains
  visible, faded, so spatial location is not lost.
- Tabset at top: vision / ADRs / research / BC READMEs jump targets
  within the same project, without leaving the canvas.
- 11px Inter tabs; active tab has a brand-orange underline.

**Typography:**

- Body text 14px Inter, line-height 1.65, max-width 620px (~75ch).
- Body color `--g-fg-1` (`#e6e6ec`) — deliberately not pure white,
  reduces fatigue over long sessions.
- `h1` 22px Inter 600, letter-spacing −0.015em.
- `h2` 15px Inter 600.
- Blockquote: 2px `--g-periwinkle-soft` (brand-orange-soft) left rule.
- Code blocks: 12px mono, `--g-code-surface` (surface-2 light,
  surface-1 dark), hairline border.

**Triggering:**

- Opens when a project is "focused" via `canvas-003-focus-zoom` (or
  a future direct trigger — keyboard, voice, double-click).
- Closes via ESC, click outside, or a "close" affordance.
- Slide animation respects `--g-camera` (320ms ease).

**Light theme:**

- Inherits all tokens from `design-system-004`. The blockquote rule
  becomes `--g-periwinkle-soft` in light mode (deeper orange for AA
  contrast on the light surface).

## Acceptance criteria

- [ ] Panel renders at 520px wide, slides from right on focus.
- [ ] Markdown rendered with the typography pins above; renders
      `vision.md`, ADRs (`/.agentheim/knowledge/decisions/*.md`),
      research (`/.agentheim/knowledge/research/*.md`), BC READMEs
      (`/contexts/<bc>/README.md`).
- [ ] Tabset at top jumps between document classes; active tab has
      brand-orange underline.
- [ ] Blockquote, code block, headings render per the design.
- [ ] Closes via ESC, click outside, or close affordance.
- [ ] Slide-in / slide-out respects the `--g-camera` motion budget.
- [ ] Light theme renders cleanly (post-`design-system-004`).
- [ ] Project frame behind panel remains visible (canvas not occluded).

## Notes

**Status:** **backlog** — beyond v1. The vision sequences this after
the v1 canvas-only MVP is done. Capture only; don't promote to todo
without confirming v1.5 scope is opening.

**Open questions for refinement (when activated):**

- Markdown library? The vision says "GUPPI renders markdown for viewing".
  Likely `marked` or `markdown-it` in JS, or a Rust crate piped through
  Tauri. Architect decision.
- Reactive content updates: if the file on disk changes while open
  (a worker writes an ADR), does the panel auto-refresh? Likely yes,
  via `infrastructure`'s file watcher.
- Tab persistence: does the panel remember which doc was last open
  per project, or always default to vision?
- Search inside the panel? (Likely v2+.)

**Bundle reference:** `references/claude-design-2026-05-16/project/guppi-panels.jsx`
+ `GUPPI.html` §5.

**Trigger source:** depends on `canvas-003-focus-zoom` shipping the
focus gesture; the panel is one of the things "focus" implies.
