---
id: canvas-010
title: Emulated terminal panel — orchestrator + sub-agents view
status: backlog
type: feature
context: canvas
created: 2026-05-16
completed:
commit:
depends_on: [design-system-001, design-system-003, design-system-004]
blocks: []
tags: [terminal-panel, claude-stream, v2]
related_adrs: [ADR-003, ADR-006]
related_research: []
prior_art: []
---

## Why

The vision lists "Emulated terminal panel for interactive Claude sessions
— shallow view of orchestrator + sub-agent comms, same as a real terminal
would show" under "Beyond v1". The 2026-05-16 design pins it: §6 of
the artboard set + `guppi-panels.jsx`'s `ViewTerminalPanel`.

A raw terminal is hostile to a manager watching multiple sessions. This
panel keeps the terminal's grammar (mono, time-stamped lines, agent
labels) but lifts the noise: orchestrator vs sub-agents are color-coded,
sub-agents are collapsible threads with a left rule, and the "I need an
answer" moment becomes a real card with quick-answer chips — not a
buried prompt.

Cross-BC: this consumes `claude-runner`'s stream contract (orchestrator
events, sub-agent events, session lifecycle). The rendering is canvas's;
the data is runner's. See context-map.md — runner publishes, canvas
subscribes (canvas as one of multiple subscribers alongside `agent-awareness`).

## What

Spec in `references/claude-design-2026-05-16/project/guppi-panels.jsx`
(`ViewTerminalPanel`) + §6 rationale in `GUPPI.html`. Pinned decisions:

**Layout / chrome:**

- Surface `--g-surface-3`, 12px panel radius, hairline-strong border.
- Header carries: session identity (project / BC), running-count + blocked-
  count mini-badges (same vocabulary as the project frame's counts).
- Body is a scrollable transcript.
- Input bar at the bottom — 44px tall, brand-orange `›` prompt; same
  vocabulary as voice and the command palette (consistent across
  modalities).
- "⌘↩" hint right-aligned in fg-4.

**Typography:**

- Mono body: 12px Cascadia Code, line-height 1.55.
- Timestamps: 10.5px, fg-4, 50px gutter.
- Agent column: 88px wide, status-colored label (orchestrator vs
  sub-agent vs blocked sub-agent).

**Sub-agent threads:**

- 2px left rule, 10px indent.
- Collapsible (click to expand/collapse).
- A blocked sub-agent renders the rule in red (`--g-status-blocked`)
  with a 3px accent.

**The "agent is asking" moment:**

- Question rendered as a card (12px **Inter sans**, deliberately not
  mono — it's a human-shaped question, not a log line), red-glow fill.
- Quick-answer chips: 11px Inter, hairline-strong border. Click to
  send.
- Otherwise type in the input bar.

**Light theme:**

- Inherits all tokens from `design-system-004`. Red-glow becomes
  `--g-status-blocked-glow` light variant.

## Acceptance criteria

- [ ] Panel renders the design's chrome (surface, border, radius,
      header structure, input bar).
- [ ] Live orchestrator + sub-agent transcript renders from the
      `claude-runner` stream (real session, not a mock).
- [ ] Sub-agent threads are collapsible with the 2px left rule.
- [ ] A blocked sub-agent renders the red-rule treatment and the
      question card with quick-answer chips.
- [ ] Quick-answer chip click sends the chip's text as the answer.
- [ ] Free-form input via the input bar (`⌘↩` submits).
- [ ] Light theme renders cleanly.
- [ ] Per-BC counts in the header stay live (subscribe to the same
      `BcAppeared`/etc events `canvas-001` consumes).

## Notes

**Status:** **backlog** — v2 (per vision roadmap: "Emulated terminal
panel..."). Capture only; do not promote.

**Hard upstream:** `claude-runner` must expose a structured stream
(orchestrator events, sub-agent events, blocked events). The runner's
spike (`infrastructure-013`) proved the PTY mechanics; the *structured
event taxonomy on top of the stdio* is a separate piece of work that
likely deserves its own claude-runner task before canvas-010 can
even refine fully. Note as a refinement-time check.

**Open questions for refinement:**

- Where does the terminal panel live spatially — overlaid on the
  canvas, docked below, full-screen toggle? Design suggests overlay.
- Multiple terminal panels (one per active session) or just one
  focused-session view?
- Search / scroll-back? Persistent transcript across app restart?
- Copy-out (select-text in mono region) — terminal-like or
  rich-text-like?

**Bundle reference:** `references/claude-design-2026-05-16/project/guppi-panels.jsx`
(`ViewTerminalPanel`) + `GUPPI.html` §6.
