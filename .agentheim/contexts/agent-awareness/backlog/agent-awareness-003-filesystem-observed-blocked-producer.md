---
id: agent-awareness-003
title: Filesystem-observed blocked-task producer (observed sessions)
status: backlog
type: feature
context: agent-awareness
created: 2026-05-24
completed:
commit:
depends_on: [agent-awareness-002]
blocks: []
tags: [agent-state, filesystem, observed-session, deferred]
related_adrs: [ADR-018, ADR-008]
related_research: []
prior_art: [agent-awareness-002]
---

## Why

`agent-awareness-002` (ADR-018) built the unified per-task agent-state
projection and wired the **runner** path as the only live producer. The
projection already has a tested `AgentSignal::FilesystemBlocked` fold (proving
the model is source-agnostic), but **no filesystem producer is wired** — so
bare-terminal "observed sessions" GUPPI did not spawn always show idle.

This is the deferred best-effort fidelity the BC README describes: a task in
`doing/` carrying an on-disk `blocked_question:` frontmatter should surface as
`blocked_on_question` (labelled `observed`), even with no owned `claude-runner`
session.

## What

- A producer that maps filesystem signals (a task present in `doing/` with a
  `blocked_question:` in its frontmatter; possibly hook outputs / mtimes) into
  `AgentSignal::FilesystemBlocked` and folds them into
  `agent_state::AgentStateProjection`, re-emitting `TaskAgentStateChanged`.
- Likely rides the existing ADR-008 per-project watcher (the `TaskChanged` /
  `TaskAdded` events already carry `blocked_question`) rather than a second
  watcher.
- A transition back to idle when the task leaves `doing/` or its
  `blocked_question` clears.

## Acceptance criteria

- [ ] A `doing/` task with an on-disk `blocked_question:` surfaces as
      `blocked_on_question` with `agent_label = "observed"` and the disk question
      text, with no owned runner session.
- [ ] The signal clears back to idle when the task moves out of `doing/` or the
      `blocked_question` is removed.
- [ ] Runner-sourced state wins over filesystem-sourced state for the same task
      (the rich signal takes precedence — define + test the precedence rule).

## Notes

Deferred from `agent-awareness-002` per ADR-018 (runner-only was v1's live
fidelity). The projection seam (`AgentSignal::FilesystemBlocked`,
`OBSERVED_AGENT_LABEL`) is already in place.
