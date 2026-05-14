---
id: ADR-009
title: IPC and event bus — Tokio broadcast channel in the core, Tauri events to the frontend
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-009-event-bus]
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

    // Filesystem observation (infrastructure-008)
    TaskMoved { project_id, bc, from, to, task_id },
    BCAppeared { project_id, bc },
    BCDisappeared { project_id, bc },

    // claude PTY sessions (ADR-006)
    SessionSpawned { project_id, session_id },
    SessionExited { project_id, session_id, status },
    SessionBlockedOnQuestion { project_id, session_id, question },

    // Voice bridge (ADR-007)
    VoiceWakeWord,
    VoiceTranscript { text, final_: bool },
}
```

The enum is the contract. It is expected to grow as new producers land
(logger, narrator dispatcher, telemetry); adding a variant does not touch
existing producers or consumers that do not care about it. The variants for
`TaskMoved` / `BCAppeared` / `BCDisappeared` are kept aligned with the
filesystem-observation work in infrastructure-008.

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
