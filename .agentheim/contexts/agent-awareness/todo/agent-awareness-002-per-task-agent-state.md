---
id: agent-awareness-002
title: Per-task live agent state + blocked-question content
status: todo
type: feature
context: agent-awareness
created: 2026-05-24
completed:
commit:
depends_on: [project-registry-005, design-system-006]
blocks: [canvas-021, canvas-022]
tags: [agent-state, per-task, blocked-question, events]
related_adrs: [ADR-009, ADR-008, ADR-006]
related_research: []
prior_art: []
---

## Why

The canvas pivot surfaces live agent state on individual task cards
("orchestrator · waiting 2m 14s") and renders an "AGENT NEEDS AN ANSWER"
callout (question text + answer/defer/edit actions) in the docked detail
panel when a task is blocked (reference:
`.agentheim/contexts/design-system/references/kanban.png`). The accordion
header rolls this up per BC ("1 active · 2 blocked · 1 idling").

agent-awareness today models state per **BC** (running/idle/blocked-on-
question) — its v1 surface was deliberately minimal (README open question).
The pivot needs state per **task**. This task re-homes the concern that the
now-deleted `agent-awareness-001-blocked-question-callout-spec` carried, into
the new card-and-panel design.

## What

- A per-task state projection keyed by `(project_id, bc, task_id)` — the
  task identity established by project-registry-005. State per task:
  `running | idle | blocked-on-question`, plus, for running/blocked, the
  acting agent label ("orchestrator") and an elapsed duration the card
  renders as "waiting 2m 14s". Duration is derived from a transition
  timestamp held in this BC, not stored on disk.
- Blocked-on-question carries the **question text** (the callout body) and,
  where the source provides them, the suggested actions surface — the panel
  renders answer / defer / edit (the action WIRING is canvas-022 / a later
  command-path task; this task supplies the question text + the blocked
  flag, not the answer-submission round-trip, which is post-v1 per the
  vision's read-only v1 stance — confirm scope in refinement).
- Two signal sources, same split as the BC-level model: rich signals from
  `claude-runner` (`SessionBlockedOnQuestion { project_id, session_id,
  question }` — extend correlation to a `task_id` where the runner can
  attribute the session to a task) for owned sessions; best-effort
  filesystem signals (a task in `doing/`, hook outputs, mtimes) for
  observed sessions. The unified per-task state model survives the split —
  the canvas sees one shape.
- A per-BC roll-up (active / blocked / idling counts) derived from the
  per-task states, for the accordion header.
- Surfaced to the canvas via the ADR-009 bus / a query the canvas reads;
  the existing per-BC indicator contract extends to per-task. (Define the
  exact event/read shape in refinement — likely a `TaskAgentStateChanged
  { project_id, bc, task_id, state, agent_label?, since? }` variant.)

## Acceptance criteria

- [ ] A per-task agent-state projection keyed `(project_id, bc, task_id)`
      exists, with states `running | idle | blocked-on-question` and an
      acting-agent label + transition timestamp for running/blocked.
- [ ] Blocked-on-question carries the question text the docked panel's
      callout renders.
- [ ] A per-BC roll-up (active/blocked/idling counts) is derived from the
      per-task states for the accordion header.
- [ ] Canvas can read per-task state for a `(project_id, bc, task_id)` and
      the BC roll-up; the contract is documented (event variant and/or query
      shape) and the agent-awareness README ubiquitous language extends from
      per-BC to per-task ("task state").
- [ ] The runner-vs-filesystem source split is preserved: an owned-session
      blocked signal and a filesystem-observed blocked signal both land in
      the unified per-task model. Tested at whatever fidelity v1 ships
      (runner-only acceptable if filesystem-observed sessions remain
      deferred per the README's v1 open question — record the chosen v1
      scope).
- [ ] Elapsed-duration ("waiting 2m 14s") is derivable on the canvas from
      the supplied transition timestamp (this BC supplies `since`, the canvas
      formats it — no per-second event spam).

## Notes

depends_on project-registry-005 (needs `task_id` as the projection key) and
design-system-006 (the card agent-indicator + the panel callout are
design-system components). It does NOT depend on canvas-019 — the state
model is substrate-agnostic.

Re-homes deleted `agent-awareness-001-blocked-question-callout-spec`'s
concern. The answer/defer/edit ACTION round-trip (submitting an answer back
to a blocked session) is a command-path concern — v1 is read-only per the
vision, so this task delivers the read side (state + question text); the
write side is captured separately when the v2 command path opens. Confirm
in refinement whether the v1 cut ships the actions disabled/visual-only or
omits them.

Open question for refinement: is `blocked_question` text sourced live from
`claude-runner`'s `SessionBlockedOnQuestion.question` (lean: yes), from disk
frontmatter (project-registry-005 fallback), or both? The reference's
callout is live-state-shaped, so lean live with a disk fallback.
