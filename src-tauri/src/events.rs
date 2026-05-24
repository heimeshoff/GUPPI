//! In-core typed event bus — ADR-009.
//!
//! A single `EventBus` wraps a Tokio `broadcast` channel carrying a typed
//! `DomainEvent`. Producers (the filesystem watcher today; PTY actors and the
//! voice bridge later) hold a sender clone and publish; consumers each hold
//! their own `Receiver`. A thin frontend-bridge task subscribes and forwards
//! the frontend-relevant subset to the WebView via Tauri's `emit` — that
//! bridge lives in `lib.rs`, keeping Tauri APIs out of the rest of the core.

use serde::Serialize;
use tokio::sync::broadcast;

/// ADR-009: deliberate starting capacity. Large enough that no realistic burst
/// overruns a well-behaved consumer, small enough to bound memory.
pub const EVENT_BUS_CAPACITY: usize = 1024;

/// The closed event taxonomy. ADR-009 is the contract; the enum is expected to
/// grow as new producers land. Only the variants the walking skeleton actually
/// produces are populated today — the rest are declared so the contract is
/// visible and adding producers later does not touch this file's consumers.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
// `ProjectMissing` is part of the ADR-009 contract but has no producer in the
// skeleton (the registry's missing-path detection is post-skeleton work);
// keeping it declared makes the taxonomy reviewable and adding its producer
// later touches no consumer.
#[allow(dead_code)]
pub enum DomainEvent {
    // Project registry (ADR-005)
    ProjectAdded { project_id: i64, path: String },
    ProjectMissing { project_id: i64 },
    /// A registered project just left the registry — either via a single
    /// "Remove project" (soft-delete, `project-registry-003`) or via a
    /// `remove_scan_root` cascade (`project-registry-002b` / ADR-013, hard
    /// delete). The frontend drops the matching tile on receipt; the registry
    /// has already torn down the watcher and removed/soft-deleted the row by
    /// the time this fires.
    ProjectRemoved { project_id: i64 },

    // Filesystem observation (ADR-008 / ADR-009) — the fine-grained taxonomy.
    // `infrastructure-014` makes the single-project watcher correlate raw
    // debounced FS events into these. `canvas-001` made them the *only*
    // normal-path filesystem events: the watcher publishes nothing else for a
    // debounced batch, and the frontend patches its model in place from them.
    //
    // `from` / `to` / `state` are Agentheim task states: one of
    // `backlog`, `todo`, `doing`, `done`.
    /// A paired create + delete of the *same* `task_id` across two task-state
    /// directories within one debounce window — the task file moved.
    TaskMoved {
        project_id: i64,
        bc: String,
        from: String,
        to: String,
        task_id: String,
    },
    /// An unpaired create — a brand-new task file appeared (ADR-008's
    /// "sensible fallback", decided in `infrastructure-014`: no silent drop).
    ///
    /// `project-registry-005` extended the payload with the task file's
    /// frontmatter metadata (`title`, `type_`, `tags`) so the canvas can draw
    /// the new card in place without a `get_project` resync. The watcher reads
    /// the just-created file's frontmatter when emitting this; a
    /// malformed/missing frontmatter degrades to a filename-stem `task_id`,
    /// empty `title`/`type_`, and empty `tags` (never aborts the event).
    TaskAdded {
        project_id: i64,
        bc: String,
        state: String,
        task_id: String,
        title: String,
        #[serde(rename = "type_")]
        type_: String,
        tags: Vec<String>,
    },
    /// An unpaired delete — a task file was removed outright.
    TaskRemoved {
        project_id: i64,
        bc: String,
        state: String,
        task_id: String,
    },
    /// A task file's frontmatter changed *in place* — an edit to `title`,
    /// `type`, `tags`, or `blocked_question` without the file moving between
    /// task-state directories (a move is `TaskMoved`; a create is `TaskAdded`).
    /// `project-registry-005`: the canvas patches the matching card's metadata
    /// in place without a resync. The watcher detects a `Modify(Data)` /
    /// `Modify(Any)` on an existing `contexts/<bc>/<state>/<task_id>.md` and
    /// re-reads its frontmatter to fill this payload. ADR-009's enum is
    /// "expected to grow"; adding this variant touches no existing consumer.
    TaskChanged {
        project_id: i64,
        bc: String,
        task_id: String,
        title: String,
        #[serde(rename = "type_")]
        type_: String,
        tags: Vec<String>,
        blocked_question: Option<String>,
    },
    /// A new `contexts/<bc>/` directory was created.
    BCAppeared { project_id: i64, bc: String },
    /// A `contexts/<bc>/` directory was removed.
    BCDisappeared { project_id: i64, bc: String },
    /// A BC's `README.md` was written and the parsed `relationships:`
    /// frontmatter changed (deep-equal compared against the in-memory cached
    /// previous value). Consumed by `canvas-007`: the canvas patches the BC's
    /// `relationships` array in place and re-runs intra-project edge layout.
    /// README writes that don't change the parsed relationships set (e.g.
    /// prose-only edits) do **not** fire this event — see
    /// `watcher::RelationshipsCache` (`project-registry-004`).
    BcRelationshipsChanged { project_id: i64, bc: String },

    // Filesystem observation (ADR-008 / ADR-009) — the lag-only resync signal.
    // ADR-009: when the frontend bridge's broadcast receiver reports `Lagged`,
    // events have been *lost* and cannot be reconstructed from the fine-grained
    // stream. The bridge emits this so the frontend re-fetches the whole
    // `get_project` snapshot. This is the *only* event that triggers a full
    // re-fetch. It is emitted **only** by `lib.rs`'s `Lagged` arm — never by
    // the watcher's normal path. (Was `AgentheimChanged`; `canvas-001` renamed
    // it and dropped its normal-path role.)
    ResyncRequired { project_id: i64 },

    // Claude session ownership / PTY (ADR-006). A `ClaudeSession` actor's read
    // loop publishes raw PTY-master bytes as `SessionOutput` — no VT parsing
    // yet, deferred to the terminal-panel feature per ADR-006. `bytes` is not
    // UTF-8-guaranteed; it is whatever ConPTY emitted.
    SessionOutput { session_id: i64, bytes: Vec<u8> },

    // User preferences (`design-system-004-light-theme`). Fired by
    // `set_preference` so any subscriber (the PixiJS canvas, the HTML overlay
    // layer) can react. Theme is the first inhabitant — flipping
    // `('theme','dark'<->'light')` triggers a canvas re-render against the
    // newly-active palette and flips the `data-theme` attribute on
    // `<html>`. Future preferences (font scale, reduced-motion override,
    // etc.) reuse the same generic shape. See ADR-009's 2026-05-16
    // reconciliation note.
    PreferenceChanged { key: String, value: String },
}

/// The in-core pub/sub bus. Cloneable: every clone shares the same channel.
#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    pub fn new() -> Self {
        let (sender, _rx) = broadcast::channel(EVENT_BUS_CAPACITY);
        Self { sender }
    }

    /// Publish an event to all current subscribers. Returns the number of
    /// subscribers that received it (0 is fine — fan-out, producers do not
    /// care who is listening).
    pub fn publish(&self, event: DomainEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }

    /// Take a fresh receiver. Consumers must handle `RecvError::Lagged` by
    /// resyncing from the source of truth, never by blocking the channel.
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn published_event_reaches_a_subscriber() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.publish(DomainEvent::ResyncRequired { project_id: 7 });

        let received = rx.recv().await.expect("event should be delivered");
        match received {
            DomainEvent::ResyncRequired { project_id } => assert_eq!(project_id, 7),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn fans_out_to_every_subscriber() {
        let bus = EventBus::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();

        bus.publish(DomainEvent::ProjectAdded {
            project_id: 1,
            path: "C:/x".into(),
        });

        assert!(matches!(
            a.recv().await.unwrap(),
            DomainEvent::ProjectAdded { project_id: 1, .. }
        ));
        assert!(matches!(
            b.recv().await.unwrap(),
            DomainEvent::ProjectAdded { project_id: 1, .. }
        ));
    }

    #[test]
    fn publish_without_subscribers_does_not_panic() {
        let bus = EventBus::new();
        assert_eq!(bus.publish(DomainEvent::ProjectMissing { project_id: 9 }), 0);
    }

    #[tokio::test]
    async fn preference_changed_event_reaches_a_subscriber() {
        // `design-system-004-light-theme` acceptance: the `PreferenceChanged`
        // domain event is part of the ADR-009 taxonomy. Fired by
        // `set_preference` (the IPC command) so the canvas + HTML overlay can
        // re-render against the newly-active palette without polling.
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.publish(DomainEvent::PreferenceChanged {
            key: "theme".to_string(),
            value: "light".to_string(),
        });

        let received = rx.recv().await.expect("event should be delivered");
        match received {
            DomainEvent::PreferenceChanged { key, value } => {
                assert_eq!(key, "theme");
                assert_eq!(value, "light");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
}
