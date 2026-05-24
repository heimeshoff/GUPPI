---
name: agent-awareness
classification: core
relationships:
  - to: claude-runner
    type: customer-supplier
    direction: upstream
  - to: project-registry
    type: conformist
    direction: upstream
  - to: infrastructure
    type: shared-kernel
---

# agent-awareness

## Purpose

Aggregates a single, unified "what's running, what's idle, what's blocked-on-question" view across every observed session, regardless of source. The canvas's status badges and the question-at-BC-location overlay are driven entirely from this BC.

Two signal sources, two fidelities:

1. **Rich signals from `claude-runner`** for GUPPI-owned sessions: orchestrator stream events, prompt-waiting detection, lifecycle events. Subscribes to runner events.
2. **Best-effort filesystem signals** for bare-terminal sessions GUPPI didn't spawn: presence of tasks in `doing/`, hook output files, mtime patterns. Conforms to the Agentheim-on-disk shape.

The value-add is the *unified state model* that survives this source split: downstream consumers (the canvas) see one shape, not two.

## Classification

**Core.** "Live indicators of what's running and what needs attention" is one of GUPPI's two headline differentiators (the other being the canvas surface itself). The unified-state model — and especially the question-at-BC-location feature — is GUPPI-specific intelligence, not off-the-shelf plumbing.

## Ubiquitous language

- **Task state** / **per-task agent state** — the live status of a single task,
  keyed `(project_id, bc, task_id)`: running / idle / blocked-on-question. The
  canvas kanban-accordion pivot moved the granularity from per-BC to per-task
  (`agent-awareness-002`, ADR-018); **task state is the primary model** now,
  per-BC roll-up is derived from it.
- **Tile state** / **BC state** — retained as the *rolled-up* per-BC view
  (active / blocked / idling counts) the accordion header renders ("1 active ·
  2 blocked · 1 idling"). Derived from the per-task states, not a separate
  source.
- **Running** — at least one session is actively producing output for this task.
- **Idle** — no active session for this task (the implicit default — an unknown
  task is idle).
- **Blocked-on-question** — an active session is waiting for human input. Carries
  the **question text** (the "AGENT NEEDS AN ANSWER" callout body) the docked
  detail panel renders.
- **Acting agent label** — for running/blocked tasks, the agent doing the work
  ("orchestrator", "worker"); `observed` for a filesystem-attributed task (the
  filesystem cannot name the acting agent). The card renders it as
  "orchestrator · waiting 2m 14s".
- **Transition timestamp** (`since`) — the Unix-ms moment a task entered its
  current running/blocked state, held in this BC (never on disk). The canvas
  derives the "waiting 2m 14s" elapsed string from it locally — this BC emits
  **no per-second event**.
- **Owned session** — observed via `claude-runner` events (rich signal). The v1
  live source.
- **Observed session** — observed via filesystem only (best-effort signal). The
  v1 fallback fidelity (deferred producer — see Open questions).
- **Indicator** — the visual badge consumed by the canvas (per-card agent dot /
  "waiting …" label).
- **AGENT NEEDS AN ANSWER callout** — the question text rendered in the docked
  detail panel when a task is blocked. v1 renders answer/defer/edit actions
  visual-only (read-only v1 — the write round-trip is post-v1).
- **Signal source** — runner-event-stream | filesystem-watch.

### Read contract (canvas-facing, ADR-018)

- **Bus events** (ADR-009): `TaskAgentStateChanged { project_id, bc, task_id,
  state, agent_label?, since?, question? }` is agent-awareness's *output* (the
  canvas patches a card's indicator + the panel callout in place);
  `SessionBlockedOnQuestion { project_id, bc, task_id, agent_label, question }`
  is its *input* (the runner producer).
- **IPC reads**: `get_task_agent_state(project_id, bc, task_id) -> TaskAgentState`
  and `get_bc_agent_rollup(project_id, bc, total_tasks) -> BcRollup` hydrate on
  mount / resync.
- **Projection**: `agent_state::AgentStateProjection` in
  `src-tauri/src/agent_state.rs` is the in-core read model.

## Upstream / downstream

- **Downstream of:**
  - `claude-runner` (customer-supplier; subscribes to session events).
  - The filesystem / `project-registry` (conformist to the Agentheim-on-disk shape; reads `doing/` contents, hook outputs, mtimes).
- **Upstream of:** `canvas` (supplies status badges and question-at-location overlays).

## Open questions

- Shared watcher with `project-registry`, or independent? Foundation pass — likely one watcher, two consumers via the infrastructure event bus.
- "Blocked-on-question" detection — owned here as a state transition, but the underlying signal comes from `claude-runner`. Where does the heuristic live? Lean: runner detects the raw "stream went quiet after a `?`" signal; this BC interprets it as state.
- ~~v1 scope: runner-only or runner + filesystem?~~ **Resolved (ADR-018,
  `agent-awareness-002`):** v1 wires the **runner path as the only live
  producer**; the filesystem-observed producer is deferred (the projection's
  `FilesystemBlocked` fold exists and is tested so the model stays source-
  agnostic, but no filesystem producer is wired). v1 is **read-only** — the
  answer/defer/edit actions render visual-only, the command-path write
  round-trip is post-v1.
