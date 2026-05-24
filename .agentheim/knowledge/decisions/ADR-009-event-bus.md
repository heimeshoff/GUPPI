---
id: ADR-009
title: IPC and event bus — Tokio broadcast channel in the core, Tauri events to the frontend
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-009-event-bus, infrastructure-014-fine-grained-fs-events, canvas-001-targeted-canvas-updates, design-system-004-light-theme]
---

# ADR-009: IPC and event bus — Tokio broadcast channel in the core, Tauri events to the frontend

**Status:** Accepted
**Scope:** global

## Context

GUPPI has multiple event producers — the filesystem watcher (ADR-005), the
`claude` PTY actors (ADR-006), the voice bridge (ADR-007), and command
handlers — that must notify multiple consumers: the canvas UI, the narrator,
and future telemetry/logging. The requirements are:

- **Typed events** — producers and consumers agree on a closed taxonomy, not
  stringly-typed payloads.
- **Fan-out** — one event reaches many consumers without producers knowing
  who is listening.
- **No UI polling** — the frontend learns about state changes by being told,
  not by asking.

ADR-001 already fixed the runtime as Tauri 2 with a Rust core and a web-tech
frontend, with IPC via Tauri's `invoke` / `emit`. This ADR decides the
*shape* of the event side of that IPC, both inside the Rust core and across
the core/frontend boundary.

## Options considered

1. **Polling from the frontend.** Trivial to build, but laggy and wasteful —
   the UI burns cycles asking for state that usually has not changed. Rejected
   for state-change notification.
2. **Tauri's `emit` API only.** Works for getting events to the WebView, but
   provides no fan-out *within* the Rust core itself — core-side consumers
   (narrator dispatcher, logger) would have nothing to subscribe to. Rejected.
3. **Tokio `broadcast` channel inside the core + Tauri `emit` at the frontend
   boundary.** Two layers. All core actors publish to and subscribe from a
   single broadcast channel carrying a typed `DomainEvent`. A thin
   "frontend bridge" task subscribes to that channel and forwards the
   frontend-relevant events to the WebView via `app_handle.emit()`.

## Decision

**Option 3.** A single `EventBus` in the Rust core wraps a Tokio
`broadcast` channel and exposes a typed `DomainEvent` enum. Producers hold a
sender clone and publish; consumers each hold their own `Receiver`.

### Broadcast-channel capacity

The channel is created with a **capacity of 1024**. This is a deliberate
starting point: large enough that no realistic burst (a filesystem rescan, a
voice transcript stream) overruns a well-behaved consumer, small enough to
bound memory. If a slow consumer lags past 1024, `broadcast` drops the oldest
messages for that receiver and signals `RecvError::Lagged` — consumers must
treat that as "resync from source of truth," never block the channel.

### Initial `DomainEvent` taxonomy

```rust
enum DomainEvent {
    // Project registry (ADR-005)
    ProjectAdded { project_id, path },
    ProjectMissing { project_id },

    // Filesystem observation (ADR-008 / infrastructure-014 / canvas-001 /
    // project-registry-005 — see 2026-05-24 note for TaskAdded payload growth
    // + the new TaskChanged variant)
    TaskMoved { project_id, bc, from, to, task_id },
    TaskAdded { project_id, bc, state, task_id, title, type_, tags },
    TaskChanged { project_id, bc, task_id, title, type_, tags, blocked_question },
    TaskRemoved { project_id, bc, state, task_id },
    BCAppeared { project_id, bc },
    BCDisappeared { project_id, bc },
    ResyncRequired { project_id },  // lag-only; see canvas-001 note below

    // claude PTY sessions (ADR-006)
    SessionSpawned { project_id, session_id },
    SessionExited { project_id, session_id, status },
    SessionBlockedOnQuestion { project_id, session_id, question },

    // Voice bridge (ADR-007)
    VoiceWakeWord,
    VoiceTranscript { text, final_: bool },

    // User preferences (design-system-004 — see 2026-05-16 reconciliation
    // note below). Generic key/value so future preferences reuse the same
    // shape.
    PreferenceChanged { key, value },
}
```

The enum is the contract. It is expected to grow as new producers land
(logger, narrator dispatcher, telemetry); adding a variant does not touch
existing producers or consumers that do not care about it. The variants for
`TaskMoved` / `TaskAdded` / `TaskRemoved` / `BCAppeared` / `BCDisappeared` are
kept aligned with the filesystem-observation work in ADR-008.

> **Reconciliation note (infrastructure-014, 2026-05-14):** the
> filesystem-observation block above is reconciled with ADR-008's
> domain-event mapping and the implemented single-project watcher.
> `TaskAdded` and `TaskRemoved` were added — ADR-008 left the unpaired
> create/delete case as an open "sensible fallback"; `infrastructure-014`
> decided it as these two first-class variants. `TaskMoved`'s `from` / `to`
> field names (this ADR's original shape) are kept; ADR-008's earlier
> `from_state` / `to_state` draft was reconciled away. The skeleton's coarse
> `AgentheimChanged` variant still exists in the implemented enum alongside
> these — it is a deliberate compatibility seam, retired by `canvas-001`.

> **Reconciliation note (canvas-001, 2026-05-14):** the coarse
> `AgentheimChanged` variant is **retired and renamed** — but not deleted, it
> still had a second job. `AgentheimChanged` carried *two* roles: (1) the
> skeleton's normal-path "any `.agentheim/` change → re-fetch" event, and
> (2) the lag-resync signal this ADR's *Consequences* section requires —
> emitted by the frontend bridge's `Lagged` arm when the broadcast receiver
> falls behind capacity. `canvas-001` splits these:
>
> - **Normal path** — the watcher now publishes *only* the fine-grained
>   `TaskMoved` / `TaskAdded` / `TaskRemoved` / `BCAppeared` / `BCDisappeared`
>   events. The frontend patches its `ProjectSnapshot` model in place from
>   them (counts tick, BC nodes appear/disappear) with no `get_project`
>   round-trip. Role (1) of the coarse event is gone.
>
> - **Lag resync** — role (2) survives, renamed `AgentheimChanged` →
>   **`ResyncRequired { project_id }`** (same payload). It is emitted **only**
>   by `lib.rs`'s `Lagged` arm — never by the watcher — and is the **only**
>   event that triggers a full `get_project` re-fetch in the frontend. By
>   definition the bridge has *lost* events when it lags and cannot
>   reconstruct them from the fine-grained stream, so an explicit
>   resync-from-source-of-truth signal is still needed; this is that signal,
>   under a name that says what it is for.

> **Reconciliation note (design-system-004, 2026-05-16):** the taxonomy gains
> a new variant — **`PreferenceChanged { key: String, value: String }`** —
> for cross-session user preferences persisted in the v5 SQLite
> `preferences (key, value)` table (ADR-004). The first inhabitant is
> `key: "theme", value: "dark" | "light"`, fired by the `set_preference`
> IPC command so the canvas (PixiJS) and the HTML overlay layer can
> re-render against the newly-active palette without polling — and so
> future sibling surfaces (voice "Bob, go dark"; command palette) can flip
> the same source. Future preferences (font scale, reduced-motion override,
> light-mode-by-time-of-day, etc.) reuse the same generic key/value shape;
> consumers ignore keys they do not recognise. The frontend bridge forwards
> the variant under the existing `guppi://event` name — no new event channel.

> **Reconciliation note (project-registry-005, 2026-05-24):** the
> filesystem-observation taxonomy grows for the canvas kanban pivot, which
> draws **individual task cards** (not a counts pill) and so needs per-task
> metadata on the wire. Two changes:
>
> 1. **`TaskAdded` payload extended** — it now carries the just-created task
>    file's frontmatter metadata: `{ project_id, bc, state, task_id, title,
>    type_, tags }` (was `{ project_id, bc, state, task_id }`). The
>    single-project `AgentheimWatcher` re-reads the file's frontmatter when
>    emitting it (`correlate` stays pure and emits empty metadata; the watcher
>    closure enriches via `enrich_task_added`). A malformed/missing frontmatter
>    degrades to empty `title`/`type_`/`tags` with the filename-stem `task_id`
>    — never aborts the event. `TaskMoved` is **unchanged** (a move carries no
>    metadata change); `TaskRemoved` is **unchanged**.
> 2. **New variant `TaskChanged { project_id, bc, task_id, title, type_, tags,
>    blocked_question }`** — fires when a task file's frontmatter changes *in
>    place* (a `Modify(Data)`/`Modify(Any)` on an existing
>    `contexts/<bc>/<state>/<task_id>.md`, no column move) so the canvas can
>    patch the matching card's metadata without a resync. The watcher's
>    `changed_task_files` detector filters out task_ids that also moved/added/
>    removed in the same debounce batch (those are placement changes already
>    covered by `correlate`). `type_` is serialised as `type_` (not `type`) to
>    dodge the JS reserved word on the frontend mirror; `blocked_question` is
>    the on-disk fallback only — the **live** "AGENT NEEDS AN ANSWER" callout
>    text is `agent-awareness-002`'s surface.
>
> Both changes honour the enum's "expected to grow" contract: existing
> producers/consumers that do not care are untouched, and the frontend bridge
> forwards the new/extended variants under the existing `guppi://event` name —
> no new event channel. The frontend mirrors them in `src/lib/types.ts`
> (`task_added` gains the fields, `task_changed` added) and patches cards in
> place via `src/lib/snapshot-patch.ts`.

> **Reconciliation note (agent-awareness-002, 2026-05-24 — ADR-018):** the
> taxonomy gains two variants for the canvas's per-task live agent-state surface.
> Both honour the "expected to grow" contract (existing producers/consumers are
> untouched) and the frontend bridge forwards them under the existing
> `guppi://event` name — no new channel.
>
> 1. **`SessionBlockedOnQuestion { project_id, bc, task_id, agent_label, question
>    }`** — agent-awareness's *input*: the rich, live `claude-runner` signal that
>    an owned session is waiting for a human answer, attributed to a task. No
>    producer is wired at v1 (the runner does not yet attribute sessions to
>    tasks); declaring it makes the contract reviewable and lets the
>    agent-awareness projection ingest it the moment the runner correlation
>    lands. (This realises one of the ADR-009 v1-draft's planned `Session*`
>    variants, now with a `bc`/`task_id`/`agent_label` attribution payload.)
> 2. **`TaskAgentStateChanged { project_id, bc, task_id, state, agent_label?,
>    since?, question? }`** — agent-awareness's *output*: a task's live
>    `running | idle | blocked_on_question` state (the unified read model that
>    survives the BC's two signal sources). `since` is a Unix-millisecond
>    transition timestamp the canvas derives the "waiting 2m 14s" elapsed string
>    from locally — no per-second event spam. `agent_label`/`since` are absent for
>    idle; `question` (the live "AGENT NEEDS AN ANSWER" callout body, runner-live
>    with on-disk `blocked_question` as fallback — project-registry-005) is
>    present only when blocked. The canvas patches the matching card's agent
>    indicator + the docked panel's callout in place. v1 is read-only — the
>    answer/defer/edit write round-trip is post-v1.
>
> The frontend mirrors both variants plus the `TaskAgentState` / `BcRollup` /
> `AgentActivity` read types in `src/lib/types.ts`. Two read IPC commands
> (`get_task_agent_state`, `get_bc_agent_rollup`) hydrate on mount / resync.

### Frontend bridge

A dedicated bridge task subscribes to the `EventBus` and forwards the subset
of events the UI needs to the WebView under a single Tauri event name
(`guppi://event`) with a JSON payload. The frontend listens via Tauri's
`listen()` and updates its Svelte stores (ADR-002). The frontend never polls
for state changes. The bridge is the *only* place Tauri's `emit` is called
for domain events, keeping the rest of the core free of Tauri-specific APIs
(consistent with ADR-001's "IPC behind a thin abstraction").

## Consequences

- (+) Producers and consumers are cleanly decoupled — fan-out is structural,
  not wired by hand.
- (+) New consumers (logger, narrator dispatcher, telemetry) attach by taking
  a `Receiver`; no producer changes.
- (+) The typed `DomainEvent` enum gives a single reviewable contract for the
  whole event surface.
- (+) Only one task touches Tauri's `emit`, so the core stays runtime-agnostic
  and reversible per ADR-001.
- (–) `broadcast` channels drop messages for receivers that lag past capacity.
  Capacity must be sized (started at 1024) and **every consumer must handle
  `Lagged` by resyncing rather than assuming a perfect stream**.
- (–) Consumers must not block in their receive loop, or they risk lagging
  themselves and every event after.
- (–) The frontend bridge must decide, per variant, what is "frontend
  relevant" — a small ongoing maintenance point as the taxonomy grows.

## Reversibility

High. The `EventBus` is an internal abstraction; swapping `broadcast` for
another fan-out primitive (e.g. `tokio::sync::watch` per topic, or an actor
mailbox) is a localized change. The frontend bridge isolates the Tauri
dependency, so the core/frontend transport could change without touching
producers.
