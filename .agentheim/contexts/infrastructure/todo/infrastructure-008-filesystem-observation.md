---
id: infrastructure-008-filesystem-observation
type: decision
status: todo
scope: global
depends_on: [infrastructure-001-desktop-runtime]
---

# Decision: Filesystem observation

## Context

guppi must watch `.agentheim/contexts/*/{backlog,todo,doing,done}/` across many registered projects to derive task counts and state. Watchers must be cheap, debounced, survive folder deletions.

## Architect's recommendation

**`notify-debouncer-full`** (Rust) — one debounced watcher per registered project, scoped to that project's `.agentheim/` only (not the whole repo). Debounce window: 250ms. Central `WatcherSupervisor` (single Tokio task) owns the `project_id -> watcher` map.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-008-filesystem-observation.md`
- [ ] Domain-event mapping (`TaskMoved`, `BCAppeared`, `BCDisappeared`) reviewed against the event-bus design (ADR-009)

## Notes — architect's ADR draft

### ADR-008: Filesystem observation — `notify` crate, one watcher per registered project root

**Status:** Proposed
**Scope:** global

**Context.** guppi must watch `.agentheim/contexts/*/{backlog,todo,doing,done}/` across many projects to derive task counts and state. Watchers must be cheap, debounced, and survive folder deletions (project moved/deleted).

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
- (–) ReadDirectoryChangesW on Windows can miss events under extreme load; for guppi's workload (Marco moving task files manually or via Claude hooks) this is not a real risk.

**Reversibility.** Trivial.
