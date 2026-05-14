---
id: infrastructure-009-event-bus
type: decision
status: done
scope: global
depends_on: [infrastructure-001-desktop-runtime]
completed: 2026-05-14
related_adrs: [ADR-009-event-bus]
commit: a1d21d5
---

# Decision: IPC and event bus

## Context

Multiple producers (filesystem watcher, PTY actors, voice bridge, command handlers) must notify multiple consumers (canvas UI, narrator, future telemetry). Need typed events, fan-out, no UI polling.

## Architect's recommendation

**Two-layer: Tokio `broadcast` channel inside the core + Tauri `emit` at the frontend boundary.** A single `EventBus` in the core exposes a typed `DomainEvent` enum; a thin "frontend bridge" task subscribes and forwards relevant events to the WebView via `app_handle.emit()`. UI listens via `listen()` and updates frontend stores.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-009-event-bus.md`
- [ ] Initial `DomainEvent` enum reviewed (event taxonomy)
- [ ] Broadcast-channel capacity decided (architect suggests 1024)

## Notes — architect's ADR draft

### ADR-009: IPC and event bus — Tokio broadcast channel in the core, Tauri events to the frontend

**Status:** Proposed
**Scope:** global

**Context.** Multiple producers (filesystem watcher, PTY actors, voice bridge, command handlers) must notify multiple consumers (canvas UI, narrator, future telemetry). Need: typed events, fan-out, the UI doesn't poll.

**Options considered.**
1. **Polling from frontend** — Trivial, but laggy and wasteful. Reject for state changes.
2. **Tauri's `emit` API only** — Works, but you lose the ability to fan out within the Rust core itself.
3. **Tokio `broadcast` channel inside core + Tauri `emit` at the frontend boundary** — Two-layer. Core actors all subscribe to the same broadcast; a thin "frontend bridge" task subscribes and forwards relevant events through `app_handle.emit()`.

**Decision.** **Option 3.** A single `EventBus` in the core exposes a typed enum:

```rust
enum DomainEvent {
    ProjectAdded { project_id, path },
    ProjectMissing { project_id },
    TaskMoved { project_id, bc, from, to, task_id },
    BCAppeared { project_id, bc },
    SessionSpawned { project_id, session_id },
    SessionExited { project_id, session_id, status },
    SessionBlockedOnQuestion { project_id, session_id, question },
    VoiceWakeWord,
    VoiceTranscript { text, final_: bool },
    // ...
}
```

The frontend bridge subscribes and emits `tauri::Builder::default().emit_all("guppi://event", payload)` with a JSON shape. The frontend listens via `listen()` and updates Svelte stores. The UI never polls for state changes.

**Consequences.**
- (+) Clean, decoupled producers and consumers.
- (+) Easy to add new consumers (logger, narrator dispatcher) without touching producers.
- (–) Broadcast channels drop messages for slow consumers — must be sized appropriately (start with 1024) and consumers must not block.

**Reversibility.** High.

## Outcome

ADR written and accepted at `.agentheim/knowledge/decisions/ADR-009-event-bus.md` with **Status: Accepted** (the architect's recommendation stood; no Marco open question on this task).

Decisions folded into the ADR:

- **Two-layer event bus** — a single `EventBus` in the Rust core wrapping a Tokio `broadcast` channel carrying a typed `DomainEvent` enum, plus a thin frontend-bridge task that subscribes and forwards frontend-relevant events to the WebView via `app_handle.emit()` under the `guppi://event` name. The bridge is the only place Tauri's `emit` is called for domain events, keeping the core runtime-agnostic per ADR-001.
- **Broadcast-channel capacity: 1024** — architect's suggestion accepted. Consumers must handle `RecvError::Lagged` by resyncing from the source of truth and must not block in their receive loop.
- **Initial `DomainEvent` taxonomy** recorded, covering project registry, filesystem observation (`TaskMoved`/`BCAppeared`/`BCDisappeared`, kept aligned with the parallel infrastructure-008 work), `claude` PTY sessions, and the voice bridge. The enum is the contract and is expected to grow without touching existing producers/consumers.

No code change required (decision-only task). Infrastructure BC README left untouched — it already carries the generic IPC ubiquitous language and the ADR is the artifact.
