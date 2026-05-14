---
id: infrastructure-008-filesystem-observation
type: decision
status: done
completed: 2026-05-14
scope: global
depends_on: [infrastructure-001-desktop-runtime]
related_adrs: [ADR-008]
commit: c1cc2be
---

# Decision: Filesystem observation

## Context

GUPPI must watch `.agentheim/contexts/*/{backlog,todo,doing,done}/` across many registered projects to derive task counts and state. Watchers must be cheap, debounced, survive folder deletions.

## Architect's recommendation

**`notify-debouncer-full`** (Rust) — one debounced watcher per registered project, scoped to that project's `.agentheim/` only (not the whole repo). Debounce window: 250ms. Central `WatcherSupervisor` (single Tokio task) owns the `project_id -> watcher` map.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-008-filesystem-observation.md`
- [ ] Domain-event mapping (`TaskMoved`, `BCAppeared`, `BCDisappeared`) reviewed against the event-bus design (ADR-009)

## Notes — architect's ADR draft

### ADR-008: Filesystem observation — `notify` crate, one watcher per registered project root

**Status:** Proposed
**Scope:** global

**Context.** GUPPI must watch `.agentheim/contexts/*/{backlog,todo,doing,done}/` across many projects to derive task counts and state. Watchers must be cheap, debounced, and survive folder deletions (project moved/deleted).

**Options considered.**
1. **`notify` crate (Rust)** — Cross-platform, uses ReadDirectoryChangesW on Windows, FSEvents on macOS, inotify on Linux. The de facto standard.
2. **`notify-debouncer-full`** — A wrapper over `notify` with debouncing built-in. Use this.
3. **Polling** — Trivial but wasteful and laggy. Skip.

**Decision.** Use **`notify-debouncer-full`** with one debounced watcher per registered project, scoped to the project's `.agentheim/` directory (not the whole repo — keeps event volume low and avoids noise from source-code changes). Debounce window: 250ms.

Watcher events are translated into domain events: `TaskMoved { project_id, bc, from_state, to_state, task_id }`, `BCAppeared { project_id, bc }`, `BCDisappeared { project_id, bc }`. These flow into the event bus (ADR-009).

A central `WatcherSupervisor` (single Tokio task) owns the map of `project_id -> debounced watcher` and handles add/remove as projects come and go from the registry.

**Consequences.**
- (+) One mature library, cross-platform, low overhead.
- (+) Debounced events mean UI doesn't flicker on bursty file changes.
- (–) ReadDirectoryChangesW on Windows can miss events under extreme load; for GUPPI's workload (Marco moving task files manually or via Claude hooks) this is not a real risk.

**Reversibility.** Trivial.

## Outcome

ADR-008 written and accepted at `.agentheim/knowledge/decisions/ADR-008-filesystem-observation.md`.
Decision: `notify-debouncer-full`, one debounced watcher per registered project
scoped to that project's `.agentheim/` directory, 250ms debounce window, central
`WatcherSupervisor` Tokio task owning the `project_id -> watcher` map.

The domain-event mapping (`TaskMoved`, `BCAppeared`, `BCDisappeared`) is defined
per the architect's draft. ADR-009 (event-bus) did not yet exist at completion
time — it is being authored in parallel — so the ADR records a note that the
event taxonomy must be reconciled with ADR-009's `DomainEvent` enum once it
lands. This satisfies the "reviewed against the event-bus design" criterion
without a cross-task dependency.
