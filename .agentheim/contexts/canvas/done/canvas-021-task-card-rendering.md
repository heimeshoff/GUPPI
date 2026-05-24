---
id: canvas-021
title: Task-card rendering (id, title, tags, status glyph, live-agent indicator)
status: done
type: feature
context: canvas
created: 2026-05-24
completed: 2026-05-24
commit: 721f5a3
depends_on: [canvas-019, canvas-020, design-system-006, project-registry-005, agent-awareness-002]
blocks: [canvas-022]
tags: [rendering, task-card, kanban, agent-indicator, pivot]
related_adrs: [ADR-003, ADR-020]
related_research: []
prior_art: [canvas-020]
---

## Why

canvas-020 establishes the kanban board structure; this task renders the
individual **task cards** inside the columns with their full content per the
reference (`.agentheim/contexts/design-system/references/kanban.png`): id
label, title, tag chips, status glyph, and the live-agent indicator line
("orchestrator · waiting 2m 14s"). The card is the smallest interactive unit
of the pivot and the click target that opens the detail panel (canvas-022).

## What

- Render each task card from a `Task` record (project-registry-005):
  id label (e.g. `#03`/`canvas-019`), title (wrap/clamp per
  design-system-006), tag chips (`thumb-gen`, etc.), and the status glyph
  from the §2.2 four-state palette.
- **Live-agent indicator** — when agent-awareness-002 reports a running or
  blocked state for `(project_id, bc, task_id)`, render the agent label +
  elapsed line ("orchestrator · waiting 2m 14s"); the elapsed duration is
  formatted on the canvas from the `since` timestamp agent-awareness supplies
  (no per-second event spam — a local timer refreshes the visible label).
  Blocked cards take the blocked accent treatment (the red card in the
  reference).
- **Card states** — default / hover / selected (the card whose detail panel
  is open) / blocked, per design-system-006.
- **Card click → open detail panel** — clicking a card selects it and opens
  the docked detail panel for that task (the panel itself is canvas-022; this
  task wires the click → selection → "open panel for task X" signal).
- Live updates: a card's content updates in place on `TaskChanged`
  (canvas-020's patch path) and its agent indicator updates on
  agent-awareness-002 state changes.

## Acceptance criteria

- [ ] Each task card renders id, title, tag chips, and status glyph from its
      `Task` record + §2.2 status palette, per design-system-006.
- [ ] Running/blocked cards show the live-agent indicator
      ("orchestrator · waiting 2m 14s") driven by agent-awareness-002; the
      elapsed value advances on screen from the supplied `since` timestamp
      without an event per tick.
- [ ] Card states (default/hover/selected/blocked) render per
      design-system-006; the blocked card takes the blocked accent.
- [ ] Clicking a card selects it and signals "open detail panel for this
      task" (panel rendering is canvas-022); the selected card shows the
      selected state.
- [ ] Card content patches in place on `TaskChanged`; agent indicator
      patches on agent-awareness state change. No full resync.
- [ ] Built against `STYLEGUIDE.md` (design-system-006); no raw hex/sizing.
      `pnpm check` 0/0/0.

## Notes

depends_on canvas-020 (the board it draws cards into), project-registry-005
(card data), agent-awareness-002 (live indicator), design-system-006 (card
component), canvas-019 (substrate). The selection signal it emits is consumed
by canvas-022.

ADR-020 records the two seams this task decided: the selection signal
(`onTaskSelected` prop + `selectedTask` rune) and the single gated 1 Hz
elapsed clock.

## Outcome

Filled in the per-task card CONTENT inside the canvas-020 kanban-accordion DOM
interior. Each card now renders its id + 2-line-clamped title + tag chips
(structure from canvas-020) plus, for an in-flight (DOING) task with a live
agent, the §3.11 **live-agent indicator line** ("orchestrator · waiting
2m 14s"): agent label + a locally-timed elapsed string. Running reads brand-blue
`▶` with the one sanctioned `durationPulse` breathe; blocked reads static red
`◆`. The four card **states** render per §3.11 — default / hover (CSS) /
selected (blue accent border) / blocked (red accent border). Clicking a card
selects it and emits the "open detail panel for this task" signal
(`onTaskSelected` callback + `selectedTask` rune) that canvas-022 consumes.

Mechanism:
- Pure `formatElapsed(sinceMs, nowMs)` added to `src/lib/frame-interior.ts`
  (the tested verification surface — 6 new cases pin the `Ns` / `Mm Ss` /
  `Hh Mm` scales + clock-skew clamp). 40/40 tests pass.
- Per-task live state lives in a `taskAgentStates` `$state` map keyed
  `(project_id, bc, task_id)` — a separate read model from the snapshot
  (ADR-018). Primed via `get_task_agent_state` for DOING tasks on
  refresh/refreshOne/task-event, and patched in place straight from the
  `task_agent_state_changed` event payload (no IPC re-fetch on the event).
- The elapsed string advances from ONE `setInterval(…, 1000)` ticking a
  `nowMs` rune, gated by a `hasLiveAgent` `$derived` so an idle canvas does no
  per-second work. Reduced-motion users get a static glyph.

Key files:
- `src/lib/frame-interior.ts` — `formatElapsed` helper.
- `src/lib/frame-interior.test.ts` — 6 new `formatElapsed` tests.
- `src/lib/Canvas.svelte` — `onTaskSelected` prop + `selectedTask` /
  `taskAgentStates` / `nowMs` stores + `selectTask`/`isSelected`/`agentLine`/
  `liveAgentState`/`refreshTaskAgentStates` helpers + the gated elapsed-clock
  `$effect` + the `task_agent_state_changed` per-card patch + the card markup
  (click/keydown → select, selected/blocked classes, live-agent line) + the
  §3.11 CSS (states, agent line, pulse keyframe).
- `.agentheim/knowledge/decisions/ADR-020-task-card-selection-and-elapsed-clock.md`
  (new).
- `.agentheim/contexts/canvas/README.md` — task-card vocabulary + live-state
  read-model update.

`pnpm check` 0/0/0; `pnpm test` 40/40.
