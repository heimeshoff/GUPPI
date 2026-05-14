---
id: infrastructure-014-fine-grained-fs-events
type: feature
status: backlog
scope: global
depends_on:
  - infrastructure-012-walking-skeleton
related_adrs:
  - ADR-008
  - ADR-009
---

# Fine-grained filesystem domain events

## Why

The walking skeleton (`infrastructure-012`) ships a deliberately **crude**
watcher: any change inside a project's `.agentheim/` publishes a single coarse
`DomainEvent::AgentheimChanged { project_id }`, and the frontend reacts by
re-fetching the whole `get_project` snapshot. The skeleton's scope explicitly
sanctioned this ("crude — real event-bus mapping comes after the skeleton").

ADR-008 and ADR-009 specify a richer taxonomy that the skeleton did **not**
implement:

- `TaskMoved { project_id, bc, from, to, task_id }`
- `BCAppeared { project_id, bc }`
- `BCDisappeared { project_id, bc }`

These require the watcher to **correlate a create and a delete within one
debounce window** (ADR-008's stated correlation logic), and require ADR-008's
draft taxonomy to be **reconciled** with ADR-009's `DomainEvent` enum (names,
field shapes, the raw-FS-event vs domain-event boundary).

## Scope (in)

- Implement the create/delete correlation in the watcher to emit `TaskMoved`,
  `BCAppeared`, `BCDisappeared`.
- Reconcile the ADR-008 draft names with ADR-009's enum; update both ADRs if
  the reconciliation changes either.
- Decide the fallback for an *unpaired* create or delete (ADR-008 calls this
  out as needing "a sensible fallback").
- Cover the correlation logic with tests (ADR-008 requires this explicitly).
- Frontend: react to the fine-grained events with targeted updates instead of
  a full snapshot re-fetch.

## Scope (out)

- The multi-project `WatcherSupervisor` (one watcher per registered project) —
  that lands with the project registry, not here. This task can stay
  single-project or be sequenced after the supervisor; refiner decides.

## Notes

Surfaced during `infrastructure-012-walking-skeleton`. The skeleton's
`src-tauri/src/watcher.rs` and `events.rs` carry comments pointing here.
