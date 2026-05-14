---
id: ADR-008
title: Filesystem observation — notify-debouncer-full, one watcher per registered project
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-008-filesystem-observation, infrastructure-014-fine-grained-fs-events, project-registry-001-multi-project-snapshot-model]
related_adrs: [ADR-001, ADR-005]
---

# ADR-008: Filesystem observation — `notify-debouncer-full`, one watcher per registered project

**Status:** Accepted
**Scope:** global

## Context

GUPPI derives task counts and per-state task placement by watching the
Agentheim layout on disk: `.agentheim/contexts/*/{backlog,todo,doing,done}/`
across **many** registered projects. When Marco (or a Claude hook) moves a task
file from `doing/` to `done/`, GUPPI must notice and update the canvas without
the user refreshing anything.

The watching layer must:

- be **cheap** — many projects watched in parallel, low idle overhead;
- be **debounced** — task moves and editor saves come in bursts; the UI must
  not flicker on every intermediate inotify/ReadDirectoryChangesW event;
- **survive folder deletions** — a registered project can be moved, deleted, or
  live on a drive that gets unmounted (the "missing" state from ADR-005);
- stay on the **Rust core** side of the Tauri 2 boundary (ADR-001), which
  already owns filesystem responsibilities.

## Options considered

1. **`notify` crate (Rust)** — the de facto standard cross-platform watcher.
   Uses `ReadDirectoryChangesW` on Windows, `FSEvents` on macOS, `inotify` on
   Linux. Raw — no debouncing; the caller must coalesce event bursts itself.
2. **`notify-debouncer-full`** — a maintained wrapper over `notify` that adds
   debouncing and event coalescing out of the box, including correct handling
   of rename pairs.
3. **Polling** — periodically `readdir` every watched directory. Trivial to
   implement and immune to dropped-event edge cases, but wasteful at idle and
   laggy under any reasonable poll interval. Rejected.

## Decision

Use **`notify-debouncer-full`** with **one debounced watcher per registered
project**, each scoped to that project's **`.agentheim/` directory only** —
not the whole repository. Scoping to `.agentheim/` keeps event volume low and
avoids noise from source-code edits, builds, and VCS churn that have nothing to
do with task state.

**Debounce window: 250ms.** Long enough to coalesce a burst of file events from
a single logical change, short enough that the canvas still feels live.

A central **`WatcherSupervisor`** — a single Tokio task in the Rust core — owns
the `project_id -> debounced watcher` map. It is the one place that:

- creates a watcher when a project is added to the registry (ADR-005);
- drops a watcher when a project is removed;
- handles a watched `.agentheim/` directory disappearing underneath a live
  watcher (project folder deleted or drive unmounted) without crashing — the
  watcher is torn down and the project transitions to the "missing" state
  defined in ADR-005.

### Domain-event mapping

Debounced filesystem events are translated by the watcher into domain events
before they leave the Rust core:

- **`TaskMoved { project_id, bc, from, to, task_id }`** — a task file appeared
  in one `{backlog,todo,doing,done}/` directory and disappeared from another,
  for the **same `task_id`**, within the same debounce window.
- **`TaskAdded { project_id, bc, state, task_id }`** — an *unpaired* create: a
  brand-new task file appeared with no matching delete in the window (`model`
  writes new task files straight into `backlog/` / `todo/` all the time).
- **`TaskRemoved { project_id, bc, state, task_id }`** — an *unpaired* delete:
  a task file was removed outright with no matching create in the window.
- **`BCAppeared { project_id, bc }`** — a new `contexts/<bc>/` directory was
  created.
- **`BCDisappeared { project_id, bc }`** — a `contexts/<bc>/` directory was
  removed.

These domain events flow into the event bus (ADR-009).

> **Reconciliation note (project-registry-001, 2026-05-14):** The
> `WatcherSupervisor` "downstream" implementation promised below has landed. It
> lives in `src-tauri/src/supervisor.rs` and composes the single-project
> `AgentheimWatcher` from `watcher.rs`. Concurrency shape: the supervisor owns
> its `project_id -> watcher` map behind an `Arc<Mutex<…>>` rather than a
> dedicated Tokio task with a command channel — `add`/`remove` are infrequent
> (registration / removal / startup-seed only, never the hot path) and
> synchronous calls keep the Tauri IPC command bodies simple. The "single
> owner" intent of this ADR holds: exactly one `WatcherSupervisor` instance owns
> the map. `add` publishes `ProjectAdded`; a missing `.agentheim/` at add time
> leaves the project registered-but-unwatched (the ADR-005 "missing" state)
> rather than erroring the caller.

> **Reconciliation note (infrastructure-014, 2026-05-14):** ADR-009 has landed
> and this section is reconciled with its `DomainEvent` enum. `TaskMoved` uses
> the field names `from` / `to` — this ADR's original draft said
> `from_state` / `to_state`; ADR-009's shorter `from` / `to` won. The "sensible
> fallback" for an unpaired create or delete (see Consequences below) is now
> decided: the first-class variants `TaskAdded` / `TaskRemoved`, added above —
> no silent drop, no coarse re-fetch fallback for those cases. The correlation
> is implemented in the **single-project** watcher (`src-tauri/src/watcher.rs`),
> not yet the `WatcherSupervisor`, and is covered by unit tests there.

## Consequences

- (+) One mature, cross-platform library — `notify` is already named as a core
  strength in ADR-001. `notify-debouncer-full` gets us debouncing for free
  instead of hand-rolling burst coalescing.
- (+) Debounced events mean the canvas does not flicker on bursty file changes
  (a task move is at minimum a create + a delete).
- (+) Scoping each watcher to `.agentheim/` keeps event volume proportional to
  *task activity*, not *source-code activity* — cheap even with many projects.
- (+) The `WatcherSupervisor` gives a single, testable seam for the
  registry-driven lifecycle (add/remove project) and for graceful handling of
  vanished folders.
- (–) `ReadDirectoryChangesW` on Windows can drop events under extreme load.
  For GUPPI's workload — a human moving task files, or a Claude hook doing so —
  this is not a realistic risk. If it ever bites, a periodic reconciliation
  pass (re-`readdir` on a long interval) is a contained add-on.
- (–) Translating raw FS events into `TaskMoved` requires the supervisor to
  correlate a create and a delete within one debounce window; an unpaired
  create or delete needs a sensible fallback. This correlation logic lives in
  the supervisor and must be covered by tests when the watcher is implemented.

## Downstream

Implementation of the `WatcherSupervisor`, the FS-event-to-domain-event
translation, and its wiring to the project registry (ADR-005) and the event bus
(ADR-009) are infrastructure BC implementation tasks, to be created once
ADR-009 has landed and the event taxonomy is reconciled.

## Reversibility

Trivial. Swapping `notify-debouncer-full` for raw `notify` (with hand-rolled
debouncing), or for polling, is a contained change behind the
`WatcherSupervisor` seam. No other ADR depends on the watcher's internals —
only on the domain events it emits.
