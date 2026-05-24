# agent-awareness — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
- [ADR-018 — Per-task live agent-state projection](../../knowledge/decisions/ADR-018-per-task-agent-state-projection.md) — Accepted. `scope: bc`. Unified per-task read model (`running | idle | blocked-on-question`) keyed `(project_id, bc, task_id)` + per-BC roll-up; `TaskAgentStateChanged` output + `SessionBlockedOnQuestion` runner input on the ADR-009 bus. Runner-primary, read-only v1; filesystem producer + write round-trip deferred (agent-awareness-003/004). [agent-awareness-002]
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
- [agent-awareness-003-filesystem-observed-blocked-producer](backlog/agent-awareness-003-filesystem-observed-blocked-producer.md) — `type: feature`. The deferred filesystem-observed producer: fold a `doing/` task carrying an on-disk `blocked_question` into the unified per-task model (the projection's `FilesystemBlocked` fold already exists + is tested; this wires the producer). Carved out of agent-awareness-002.
- [agent-awareness-004-answer-defer-edit-write-roundtrip](backlog/agent-awareness-004-answer-defer-edit-write-roundtrip.md) — `type: feature`. The post-v1 command path: submit answer/defer/edit back to a blocked session (the write round-trip the read-only v1 leaves visual-only). Carved out of agent-awareness-002.
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
*(None.)*
<!-- todo-list:end -->

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [agent-awareness-002-per-task-agent-state](done/agent-awareness-002-per-task-agent-state.md) — `type: feature`, commit `cecbc96`. Per-task live agent-state read model + per-BC roll-up; `TaskAgentStateChanged`/`SessionBlockedOnQuestion` bus contract + two read IPC commands. Runner-primary, read-only v1 (ADR-018). Unblocks the accordion-header roll-up in canvas-020 and the card/panel wiring in canvas-021/022.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
