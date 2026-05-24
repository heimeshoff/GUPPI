---
id: agent-awareness-004
title: Answer / defer / edit write round-trip for a blocked task (v2 command path)
status: backlog
type: feature
context: agent-awareness
created: 2026-05-24
completed:
commit:
depends_on: [agent-awareness-002]
blocks: []
tags: [agent-state, blocked-question, command-path, write, v2, deferred]
related_adrs: [ADR-018, ADR-006, ADR-009]
related_research: []
prior_art: [agent-awareness-002]
---

## Why

`agent-awareness-002` (ADR-018) delivered the **read** side of per-task agent
state: a blocked task surfaces its question text in the docked panel's
"AGENT NEEDS AN ANSWER" callout, with answer / defer / edit actions rendered
**visual-only / disabled** (v1 is read-only per the vision). The write side —
actually submitting an answer back to the blocked `claude-runner` session — was
deferred to the v2 command path.

## What

- An IPC command (and bus/contract pieces) to submit a human answer to a blocked
  owned session, routed to the right `ClaudeSession` (ADR-006 write channel),
  closing the loop the callout opens.
- Defer (snooze the callout) and edit (amend the question / context) affordances,
  wired to the panel actions the design-system kanban callout exposes.
- The projection transitions the task out of `blocked_on_question` once the
  answer is accepted.

## Acceptance criteria

- [ ] Submitting an answer from the docked panel writes it to the blocked owned
      session and the task transitions out of `blocked_on_question`.
- [ ] The action surface (answer/defer/edit) is enabled, replacing the v1
      visual-only rendering.
- [ ] Routing targets the correct session for the `(project_id, bc, task_id)`.

## Notes

Deferred from `agent-awareness-002` per ADR-018 (v1 read-only). Opens the v2
command path; coordinate with claude-runner's session→task attribution (the
same correlation `SessionBlockedOnQuestion` needs a producer for).
