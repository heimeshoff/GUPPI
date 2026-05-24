---
id: ADR-018
title: Per-task live agent-state projection — unified read model, runner-primary v1, read-only
status: Accepted
scope: bc
bc: agent-awareness
date: 2026-05-24
related_tasks:
  - agent-awareness-002
related_adrs: [ADR-009, ADR-008, ADR-006]
---

# ADR-018: Per-task live agent-state projection

**Status:** Accepted
**Scope:** bc (agent-awareness)

## Context

agent-awareness owns GUPPI's "what's running, what's idle, what needs an answer"
view. Its v1 surface was deliberately modelled per **bounded context** (one
running/idle/blocked state per BC tile). The canvas kanban-accordion pivot
(canvas-019/020, ADR-017) moved the granularity to per **task**: a card shows
"orchestrator · waiting 2m 14s", the docked detail panel renders an "AGENT NEEDS
AN ANSWER" callout when a task is blocked, and the accordion header rolls this up
per BC ("1 active · 2 blocked · 1 idling").

This re-homes the concern the deleted `agent-awareness-001-blocked-question-
callout-spec` carried into the new card-and-panel design. canvas-020 (and
canvas-021/022) consume whatever read model and event contract this task defines,
so the contract must be settled here.

Three things needed deciding: (1) the per-task projection shape; (2) which signal
source carries which fidelity at v1 (the BC README's open question — runner-only
or runner + filesystem); and (3) how much of the answer/defer/edit action surface
v1 ships, given the vision's read-only v1 stance.

## Decision

### 1. A unified per-task projection keyed `(project_id, bc, task_id)`

`agent_state::AgentStateProjection` (`src-tauri/src/agent_state.rs`) holds a
`(project_id, bc, task_id) -> TaskAgentState` map. `TaskAgentState` is:

- `activity: running | idle | blocked_on_question`
- `agent_label: Option<String>` — the acting agent ("orchestrator", "worker",
  "observed") for running/blocked; `None` for idle
- `since: Option<u64>` — a Unix-millisecond **transition timestamp** held in this
  BC, never stored on disk; `None` for idle
- `question: Option<String>` — the live callout body for `blocked_on_question`

Idle is the implicit default: a task with no live signal is idle and is **not**
stored (the map stays bounded to live tasks; `state_for` returns the idle default
for any miss). The per-BC roll-up (`BcRollup { active, blocked, idling }`) is
derived: `idling = total_tasks − active − blocked`, so idling tasks need no map
entry. `total_tasks` is supplied by the caller from the project-registry snapshot
(project-registry-005).

**Elapsed duration is the canvas's job.** This BC supplies `since`; the card
formats "waiting 2m 14s" locally on its render ticker. The BC never emits a
per-second event — a duplicate same-activity signal preserves the existing
`since` so the elapsed base does not reset, and a genuine activity transition
adopts the new timestamp.

### 2. Source split — runner-primary, filesystem fallback; runner-only is v1's live fidelity

The README's two-source model is preserved structurally: a single
`AgentStateProjection::ingest(AgentSignal)` folds both fidelities into the same
`TaskAgentState`, so the canvas sees one shape. `AgentSignal` has runner variants
(`RunnerRunning` / `RunnerBlocked` / `RunnerIdle`, carrying the acting
`agent_label`) and a `FilesystemBlocked` variant (a `doing/` task with an on-disk
`blocked_question:` frontmatter, labelled `observed` since the filesystem cannot
attribute an acting agent).

**v1 wires only the runner path as a live producer.** The bus consumer in
`lib.rs` folds `DomainEvent::SessionBlockedOnQuestion` (runner) into the
projection and re-emits `TaskAgentStateChanged`. The `FilesystemBlocked` fold
**exists and is tested** so the model is proven source-agnostic, but no
filesystem producer is wired at v1 — runner is the live fidelity, the on-disk
`blocked_question` is the fallback question text (project-registry-005), and the
filesystem-observed producer is deferred. This matches the README's v1 open
question ("ship runner-only; canvas-only MVP says no live observation in v1").

**Question text is sourced live from the runner**, with the disk
`blocked_question:` frontmatter as the fallback — the reference callout is
live-state-shaped.

### 3. Read-only v1 — no answer/defer/edit write round-trip

The vision's v1 is read-only. This task delivers the **read** side: per-task
state, question text, blocked flag, and the per-BC roll-up. The
answer/defer/edit action **wiring** (submitting an answer back to a blocked
session) is a command-path concern, out of scope here and captured separately for
the v2 command path (a new backlog item). The docked panel renders the actions
**visual-only / disabled** at v1; no IPC write command is added.

### Contract surfaced to the canvas (ADR-009)

Two `DomainEvent` variants are added (reconciled into ADR-009 with a dated note,
following project-registry-005's precedent — no new event channel; the frontend
bridge forwards both under the existing `guppi://event` name):

- `SessionBlockedOnQuestion { project_id, bc, task_id, agent_label, question }` —
  agent-awareness's **input** (a `claude-runner` producer, declared now, produced
  when the runner gains session→task attribution).
- `TaskAgentStateChanged { project_id, bc, task_id, state, agent_label?, since?,
  question? }` — agent-awareness's **output**: the canvas patches a card's
  indicator (and the panel callout when blocked) in place.

Two read IPC commands hydrate on mount / resync: `get_task_agent_state` and
`get_bc_agent_rollup`. The frontend mirrors `TaskAgentState`, `BcRollup`,
`AgentActivity`, and both events in `src/lib/types.ts`.

## Consequences

- (+) The canvas sees one unified shape regardless of source — the README's
  value proposition, now at task granularity.
- (+) `since`-based elapsed timing means no per-second event spam.
- (+) The projection is reachable and queryable today even before a runner
  producer exists; the moment the runner attributes sessions to tasks, the
  consumer folds it with no further wiring.
- (−) v1 has no live signal until the runner gains session→task attribution — the
  surface is "ready but quiet". Acceptable: the model + contract are the
  load-bearing deliverable canvas-020 needs.
- (−) The filesystem-observed producer is deferred; observed (bare-terminal)
  sessions show idle until it lands.

## Reversibility

The projection is a pure in-core read model behind `ingest` / `state_for` /
`rollup`. Adding the filesystem producer, or changing the runner correlation, is
contained behind `AgentSignal`. The event variants honour ADR-009's "expected to
grow" contract — adding the filesystem producer or the v2 write command touches
no existing consumer.
