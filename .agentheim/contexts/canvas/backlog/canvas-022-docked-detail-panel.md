---
id: canvas-022
title: Docked right-edge task detail panel + animation + blocked callout
status: backlog
type: feature
context: canvas
created: 2026-05-24
completed:
commit:
depends_on: [canvas-019, canvas-021, design-system-006, project-registry-005, agent-awareness-002]
blocks: []
tags: [rendering, detail-panel, animation, blocked-callout, pivot]
related_adrs: [ADR-003, ADR-009]
related_research: []
prior_art: [canvas-021, canvas-009]
---

## Why

Clicking a task card opens a detail panel docked to the right edge of the
viewport that animates in/out; when the task is blocked, the panel shows the
"AGENT NEEDS AN ANSWER" callout with the question text and answer/defer/edit
actions (reference: `.agentheim/contexts/design-system/references/kanban.png`).
This is the read surface for a single task and the home of the re-homed
blocked-question concern (formerly the deleted
`agent-awareness-001-blocked-question-callout-spec`).

## What

- A detail panel docked to the **viewport** right edge (screen-space, not a
  world-space frame edge — confirm against the reference: the panel spans the
  viewport height and is pinned to the right, independent of where the frame
  sits on the canvas). Opened by a card-selection signal (canvas-021),
  dismissed by close / Escape / selecting another card / clicking empty
  canvas.
- **Content** — task id + status pill + tags in the header; the task body /
  metadata in the panel body (typography per design-system-006, aligned with
  the canvas-009 reader pins where they fit).
- **Slide in/out animation** — per design-system-006's motion spec (align
  with canvas-003's 320ms / `cubic-bezier(.16,.84,.36,1)` unless
  design-system-006 gives the panel its own budget).
- **Blocked callout** — when agent-awareness-002 reports the selected task as
  blocked-on-question, render the "AGENT NEEDS AN ANSWER" callout with the
  question text and the answer / defer / edit action affordances. v1 is
  read-only per the vision: ship the actions per the agent-awareness-002
  scope decision (visual-only / disabled, OR wired if the command path is in
  scope — coordinate; lean visual-only for v1 with the write-path captured
  separately).
- Panel content updates live: `TaskChanged` updates the body; agent-awareness
  state change toggles the callout in/out.

## Acceptance criteria

- [ ] Clicking a task card opens the detail panel docked to the viewport
      right edge with that task's id/status/tags/body; the panel animates in.
- [ ] The panel dismisses on close button / Escape / selecting another card /
      empty-canvas click, animating out per design-system-006.
- [ ] When the selected task is blocked-on-question (agent-awareness-002),
      the "AGENT NEEDS AN ANSWER" callout renders with the question text and
      answer/defer/edit affordances (v1 behaviour per the agent-awareness-002
      scope decision — visual-only or wired).
- [ ] Panel content + callout update live (`TaskChanged`, agent-state change)
      without reopening the panel.
- [ ] The panel is screen-space (viewport-docked) and composes correctly over
      the canvas-019 substrate (camera pan/zoom of the canvas behind it does
      not move the panel); confirm coordinate space with the reference.
- [ ] Built against `STYLEGUIDE.md` (design-system-006); no raw hex/sizing.
      `pnpm check` 0/0/0.

## Notes

depends_on canvas-021 (the card selection signal it consumes), canvas-019
(substrate + the screen-space-overlay coordinate contract — ADR-016 keeps
`camera.worldToScreen` for exactly this), project-registry-005 (task body),
agent-awareness-002 (blocked state + question text), design-system-006 (panel
component + motion). Re-homes the deleted agent-awareness-001 callout concern.

Open question (flagged): the reference panel reads as viewport-docked (full
viewport height, right-pinned) — confirm it is NOT a world-space
frame-attached panel. Specced screen-space; revisit if the reference is meant
frame-attached.

The answer/defer/edit WRITE path (submitting an answer to a blocked session)
is a command-path concern; v1 read-only per the vision. Capture the write
path separately when the v2 command surface opens.
