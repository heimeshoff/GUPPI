---
id: agent-awareness-001
title: Blocked-question callout — 240px tethered, pulse-synced with BC
status: backlog
type: feature
context: agent-awareness
created: 2026-05-16
completed:
commit:
depends_on: [design-system-001, design-system-003, design-system-004]
blocks: []
tags: [blocked-state, callout, in-canvas-overlay, v2]
related_adrs: []
related_research: []
prior_art: []
---

## Why

The vision: "When a BC is `blocked`, the actual question the agent is
asking should be rendered as a small tethered callout near that BC on
the canvas, so the user can see across the whole workspace which agents
need them." The 2026-05-16 design pins it: §3 of the artboard set +
`guppi-canvas-views.jsx`'s `ViewBlockedMoment`.

When an agent is blocked, the dev shouldn't have to dig for it. The
canvas should tell him from across the room: a red badge against a calm
backdrop, the question itself rendered IN PLACE so he can read the
prompt without opening a session. The callout is tethered with a dashed
line back to its BC so "where is this happening?" is preimmediate.

This is `agent-awareness`'s job per `context-map.md` — the BC owns the
"what's running / waiting / blocked" derivation; the rendering surface
is the canvas. The callout is the surfaced consequence.

## What

Spec in `references/claude-design-2026-05-16/project/guppi-canvas-views.jsx`
(`ViewBlockedMoment`) + §3 rationale in `GUPPI.html`. Pinned decisions:

**The callout box:**

- 240px wide, 1px red (`--g-status-blocked`) border, surface-1 fill.
- 12px Inter question text, line-height 1.5.
- Pulses on the **same 1600ms cycle** as the blocked BC's badge — the
  visual coupling tells the eye "this callout belongs to that BC".
- Tether: 1px dashed red line from BC edge to callout.
- Anchor dot: 2.5px solid red on the BC edge where the tether attaches.

**Answer affordances inside the callout:**

- Quick-answer chips (e.g. "yes" / "no" / "explain") — 11px Inter,
  hairline-strong border. Click to send.
- "type ↵" hint right-aligned, fg-3 — points to the input bar in the
  terminal panel for free-form answers.

**Secondary blocked indicators:**

- BCs blocked elsewhere on the canvas (not the focused one) appear as
  smaller notices — 200px wide, fg-2 text, opacity 0.78.
- Timer: 10px mono, fg-4 — "2m 14s" since blocked.

**Focus interaction:**

- The focused blocked BC gets the focus ring (`--g-focus-ring`,
  brand-orange).
- Keyboard / mouse can cycle through blocked BCs.

**Light theme:**

- Inherits all tokens. Red value swaps via `design-system-004`'s
  light palette.

## Acceptance criteria

- [ ] When a BC transitions to `blocked`, a callout renders near it
      with the question text from `agent-awareness`'s derivation.
- [ ] Callout has the 240×? geometry, red 1px border, surface-1 fill,
      Inter 12px body.
- [ ] Tether dashes from BC edge to callout; anchor dot on BC edge.
- [ ] Callout pulses synced with the BC's badge — same 1600ms cycle,
      same phase.
- [ ] Quick-answer chips render and send their value as the answer.
- [ ] Secondary blocked BCs render the small-notice variant.
- [ ] Timer increments live ("2m 14s") since each blocked event.
- [ ] Light theme renders cleanly.
- [ ] When a blocked BC is resolved (unblocked), the callout dismisses
      with the same easing as the camera transition.

## Notes

**Status:** **backlog** — v2 (per vision: "live agent-awareness... with
the question rendered at the BC's location"). Capture only.

**Hard upstream:** `agent-awareness` must publish a structured `blocked`
state with the question text. That requires:
- `claude-runner` emits an event taxonomy that includes blocked-on-question
  with the prompt text (cross-BC, see `canvas-010`'s upstream note).
- OR a filesystem signal (Agentheim writes a `blocked.json` or similar
  for bare-terminal sessions).

Both paths are open per `context-map.md`. The callout is the consumer
either way.

**Open questions for refinement:**

- Callout placement — to the side of the BC? Above? Algorithm to avoid
  overlap with other BCs and frames?
- Multiple blocked BCs in the same project — stacked callouts? Cycled?
- Quick-answer chip catalogue — fixed ("yes"/"no"/"explain") or
  agent-supplied per question?
- Click-callout-to-focus behaviour: should clicking the callout zoom
  in to the BC's session?

**Bundle reference:** `references/claude-design-2026-05-16/project/guppi-canvas-views.jsx`
(`ViewBlockedMoment`) + `GUPPI.html` §3.
