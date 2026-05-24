---
id: ADR-020
title: Task-card selection signal + the live-agent elapsed clock
status: Accepted
scope: bc
bc: canvas
date: 2026-05-24
related_tasks: [canvas-021]
related_adrs: [ADR-019, ADR-018, ADR-017, ADR-003]
---

# ADR-020: Task-card selection signal + the live-agent elapsed clock

**Status:** Accepted
**Scope:** bc (canvas)

## Context

canvas-021 renders the per-task card CONTENT inside the kanban-accordion DOM
interior (ADR-019): the live-agent indicator line and the four card states
(default / hover / selected / blocked, §3.11). Two seams were left to the
implementing task and are non-obvious enough to record.

## Decision

### 1. The "open detail panel for task X" signal is a `selectedTask` rune + an `onTaskSelected` prop callback

Clicking a card does two things: it sets a `selectedTask` `$state` rune inside
`Canvas.svelte` (so the card can render its §3.11 `selected` blue border with no
round-trip) AND it invokes an optional `onTaskSelected({ projectId, bc, taskId })`
component prop — the **outbound** signal the canvas-022 detail-panel host will
subscribe to.

Rejected alternatives:
- A shared module-level event store (`detail-panel.svelte.ts`). Premature — the
  panel does not exist yet (canvas-022), and the selection-border rendering is a
  pure canvas concern that should not leak into a cross-module store before there
  is a second reader. When canvas-022 lands it may promote `selectedTask` to a
  shared store; until then the prop callback is the minimal contract.
- A Svelte `createEventDispatcher`/`dispatch`. Svelte 5 runes-mode prefers
  callback props over the legacy event-dispatcher; the codebase is runes-only.

`selectedTask` is cleared on `project_removed` for the owning project so a stale
card reference never lingers. Selection survives a `task_moved` (the card keeps
its identity across columns — the key is `(projectId, bc, taskId)`).

### 2. The elapsed "waiting 2m 14s" value advances from a single local 1 Hz clock, gated on live presence

agent-awareness-002 supplies the `since` transition timestamp ONCE per state
change (ADR-018 — no per-second event spam). The card derives the human elapsed
string locally via the pure `formatElapsed(sinceMs, nowMs)` helper in
`frame-interior.ts` (the tested verification surface — 6 cases pinning the
`Ns` / `Mm Ss` / `Hh Mm` scales + clock-skew clamp).

`nowMs` is a single `$state` ticked by ONE `setInterval(…, 1000)` for the whole
canvas, not one timer per card. The interval is gated by a `hasLiveAgent`
`$derived` (true iff any task carries a non-idle state): an idle canvas runs no
per-second work, and the timer starts/stops as the first/last live agent
appears/clears. This keeps the styleguide's single sanctioned ambient loop
(§2.6 / §5 Q3 — the running-line glyph `durationPulse` breathe) honest: it costs
nothing when nothing is live.

The per-task live state itself (`taskAgentStates`, keyed `(projectId, bc, taskId)`)
is a **separate read model** from the snapshot (ADR-018): it is patched in place
straight from the `task_agent_state_changed` event payload (no IPC re-fetch on
the event), and primed via `get_task_agent_state` for DOING tasks only (§3.11
shows the indicator only for in-flight tasks).

## Consequences

- canvas-022 plugs the docked panel into `onTaskSelected` + reads `selectedTask`;
  no rework of the click path.
- The elapsed string ticks visibly without flooding the bus; reduced-motion users
  get a static glyph (`@media (prefers-reduced-motion: reduce)`).
- A future "select via keyboard arrow" or multi-select would extend `selectedTask`
  (currently a single ref) — noted as out of scope for v1.
