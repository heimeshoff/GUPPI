# agent-awareness — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
*(None.)*
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
- [agent-awareness-002-per-task-agent-state](todo/agent-awareness-002-per-task-agent-state.md) — `type: feature`, depends on `project-registry-005` + `design-system-006` (both done). **Kanban-accordion pivot.** Extends the agent-state model from per-BC to per-**task** (keyed `(project_id, bc, task_id)`): `running | idle | blocked-on-question` + acting-agent label + `since` for the "orchestrator · waiting 2m 14s" card indicator; supplies the blocked-question text for the docked panel callout; per-BC active/blocked/idling roll-up for the accordion header. Re-homes the deleted `agent-awareness-001-blocked-question-callout-spec`. Promoted 2026-05-24.
<!-- todo-list:end -->

## Doing

<!-- doing-list:start -->
*(None yet.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
*(None yet.)*
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
