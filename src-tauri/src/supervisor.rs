//! Multi-project watcher orchestration — the `WatcherSupervisor` of ADR-008.
//!
//! ADR-008 settles "one debounced watcher per registered project, each scoped
//! to that project's `.agentheim/` directory only" and notes a central
//! `WatcherSupervisor` owning a `project_id -> watcher` map. `watcher.rs` is
//! the single-project primitive (`AgentheimWatcher`); this module composes it
//! into the supervisor.
//!
//! ## Concurrency shape (`project-registry-001` reconciliation)
//!
//! ADR-008 sketched the supervisor as a Tokio task fed by a command channel.
//! `project-registry-001` simplified it to an `Arc<Mutex<HashMap<…>>>`: add and
//! remove are infrequent, called from Tauri IPC handlers that want a synchronous
//! return, and the "single owner" intent ADR-008 cares about is preserved —
//! exactly one `WatcherSupervisor` instance owns the map. Recorded in the
//! ADR-008 reconciliation note.
//!
//! ## Add semantics (ADR-005 "missing" state)
//!
//! `add` on a project whose `.agentheim/` is missing does **not** crash the
//! caller: it returns `SupervisorError::AgentheimMissing` and leaves the
//! supervisor map empty for that id. The project is "registered-but-unwatched"
//! (ADR-005's "missing" state), to be retried if `.agentheim/` reappears.
//!
//! On successful add the supervisor publishes `DomainEvent::ProjectAdded` on
//! the bus — `setup()` therefore no longer publishes it itself, removing a
//! double-publish that would arise if both did.
//!
//! Add is idempotent on `project_id`: a second `add` with the same id is a
//! no-op and does *not* republish `ProjectAdded`. This keeps the live-add path
//! safe in the face of resync or accidental double-registration.

use crate::events::{DomainEvent, EventBus};
use crate::watcher::{AgentheimWatcher, WatcherError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    /// The project's `.agentheim/` directory is missing — the project stays
    /// registered-but-unwatched (ADR-005 "missing" state). The map gains no
    /// entry; the caller is not unwound.
    #[error("the project at {path} has no .agentheim directory")]
    AgentheimMissing { path: PathBuf },
    /// Wrapping a lower-level `notify` failure that is not a missing-directory
    /// case. Lets the supervisor surface real watcher-startup problems without
    /// leaking the `notify` crate's error type into the IPC layer.
    #[error("could not start a watcher for {path}: {source}")]
    StartFailed {
        path: PathBuf,
        #[source]
        source: WatcherError,
    },
}

/// The central per-project watcher orchestrator (ADR-008). Cheap to clone;
/// every clone shares the same map.
#[derive(Clone)]
pub struct WatcherSupervisor {
    inner: Arc<Inner>,
}

struct Inner {
    bus: EventBus,
    watchers: Mutex<HashMap<i64, AgentheimWatcher>>,
}

impl WatcherSupervisor {
    pub fn new(bus: EventBus) -> Self {
        Self {
            inner: Arc::new(Inner {
                bus,
                watchers: Mutex::new(HashMap::new()),
            }),
        }
    }

    /// Begin watching `<project_path>/.agentheim/` for the given `project_id`,
    /// inserting the watcher into the supervisor map and publishing
    /// `ProjectAdded`.
    ///
    /// **Idempotent on `project_id`**: if the id is already watched, this is a
    /// no-op — no second watcher is created, no second `ProjectAdded` is
    /// published. Useful when add is wired into both startup and a live "add
    /// project" affordance that might both fire for the same id.
    ///
    /// **`.agentheim/` missing**: returns `SupervisorError::AgentheimMissing`
    /// without touching the map. The project stays registered-but-unwatched
    /// (ADR-005 "missing" state) — caller may retry later if the directory
    /// reappears, or transition the tile to the missing state.
    pub fn add(
        &self,
        project_id: i64,
        project_path: &Path,
    ) -> Result<(), SupervisorError> {
        // Idempotency check first: hold the lock through the entire add so two
        // concurrent calls with the same id cannot both think they are first.
        let mut guard = self.inner.watchers.lock().unwrap();
        if guard.contains_key(&project_id) {
            tracing::debug!(project_id, "supervisor.add: already watching, no-op");
            return Ok(());
        }

        match AgentheimWatcher::start(project_id, project_path, self.inner.bus.clone()) {
            Ok(watcher) => {
                guard.insert(project_id, watcher);
                // Release the map lock before publishing so a synchronously-
                // dispatched subscriber on the same thread cannot re-enter.
                drop(guard);
                self.inner.bus.publish(DomainEvent::ProjectAdded {
                    project_id,
                    path: project_path.to_string_lossy().into_owned(),
                });
                Ok(())
            }
            Err(WatcherError::PathMissing(_)) => Err(SupervisorError::AgentheimMissing {
                path: project_path.to_path_buf(),
            }),
            Err(other) => Err(SupervisorError::StartFailed {
                path: project_path.to_path_buf(),
                source: other,
            }),
        }
    }

    /// Stop watching `project_id`. No-op if the id is not in the map —
    /// the watcher-lifecycle surface paired with `Db::remove_project` by
    /// `project-registry-002b`'s `remove_scan_root` cascade and (eventually)
    /// `canvas-005`'s single "Remove project" affordance.
    pub fn remove(&self, project_id: i64) {
        let mut guard = self.inner.watchers.lock().unwrap();
        if guard.remove(&project_id).is_some() {
            tracing::info!(project_id, "supervisor.remove: dropped watcher");
        } else {
            tracing::debug!(project_id, "supervisor.remove: nothing to drop, no-op");
        }
    }

    /// Whether a given project id is currently watched. Used by tests and
    /// future diagnostic affordances; the IPC layer does not consult it.
    #[allow(dead_code)]
    pub fn is_watching(&self, project_id: i64) -> bool {
        self.inner.watchers.lock().unwrap().contains_key(&project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::DomainEvent;
    use std::fs;
    use std::path::PathBuf;
    use std::time::Duration;

    fn scratch_project_with_agentheim() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "guppi-supervisor-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join(".agentheim/contexts/canvas/backlog")).unwrap();
        fs::create_dir_all(dir.join(".agentheim/contexts/canvas/doing")).unwrap();
        dir
    }

    fn scratch_dir_without_agentheim() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "guppi-supervisor-missing-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[tokio::test]
    async fn add_starts_a_watcher_and_publishes_project_added() {
        // ADR-008 / `project-registry-001` acceptance: a successful add seats
        // the watcher in the map and publishes the `ProjectAdded` event on the
        // bus exactly once. `setup()` no longer publishes it directly; the
        // supervisor owns it.
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let sup = WatcherSupervisor::new(bus);
        let dir = scratch_project_with_agentheim();

        sup.add(7, &dir).unwrap();

        assert!(sup.is_watching(7), "watcher must be in the map after add");

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("ProjectAdded should arrive promptly")
            .expect("bus should deliver the event");
        match event {
            DomainEvent::ProjectAdded { project_id, path } => {
                assert_eq!(project_id, 7);
                assert!(path.contains("guppi-supervisor-test-"));
            }
            other => panic!("expected ProjectAdded, got {other:?}"),
        }

        sup.remove(7);
        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn add_is_idempotent_on_project_id() {
        // `project-registry-001` acceptance: a second `add` for the same
        // `project_id` is a no-op — the watcher map keeps the original entry
        // and `ProjectAdded` is not republished.
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let sup = WatcherSupervisor::new(bus);
        let dir = scratch_project_with_agentheim();

        sup.add(11, &dir).unwrap();
        sup.add(11, &dir).unwrap();

        assert!(sup.is_watching(11));

        // Drain the bus: must see exactly one ProjectAdded, then nothing more
        // within a short wait.
        let first = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("first ProjectAdded should arrive")
            .expect("bus delivers");
        assert!(matches!(first, DomainEvent::ProjectAdded { project_id: 11, .. }));

        // A second ProjectAdded would arrive within the debounce window; wait
        // long enough that absence is meaningful.
        let second = tokio::time::timeout(Duration::from_millis(300), rx.recv()).await;
        match second {
            Err(_) => { /* expected timeout — idempotent */ }
            Ok(Ok(DomainEvent::ProjectAdded { project_id: 11, .. })) => {
                panic!("idempotent add must not republish ProjectAdded");
            }
            Ok(other) => panic!("unexpected second event: {other:?}"),
        }

        sup.remove(11);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_without_agentheim_returns_error_and_does_not_seat_a_watcher() {
        // `project-registry-001` acceptance: `add` on a path with no
        // `.agentheim/` returns `SupervisorError::AgentheimMissing` without
        // unwinding the caller; the supervisor map gains no entry (the project
        // stays registered-but-unwatched — ADR-005 "missing" state).
        let bus = EventBus::new();
        let sup = WatcherSupervisor::new(bus);
        let dir = scratch_dir_without_agentheim();

        let err = sup.add(3, &dir).unwrap_err();
        match err {
            SupervisorError::AgentheimMissing { path } => assert_eq!(path, dir),
            other => panic!("expected AgentheimMissing, got {other:?}"),
        }
        assert!(!sup.is_watching(3), "no entry must be seated on failure");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remove_drops_the_watcher_from_the_map() {
        // `project-registry-001` acceptance: `remove` tears down the project's
        // watcher; the map no longer contains it. (Pairs with the lifecycle
        // surface `project-registry-002` will call after the row delete.)
        let bus = EventBus::new();
        let sup = WatcherSupervisor::new(bus);
        let dir = scratch_project_with_agentheim();

        sup.add(5, &dir).unwrap();
        assert!(sup.is_watching(5));
        sup.remove(5);
        assert!(!sup.is_watching(5), "remove must drop the entry");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn remove_of_unknown_id_is_a_silent_no_op() {
        let bus = EventBus::new();
        let sup = WatcherSupervisor::new(bus);
        sup.remove(999); // must not panic
        assert!(!sup.is_watching(999));
    }

    #[tokio::test]
    async fn two_projects_emit_events_carrying_their_own_project_id() {
        // `project-registry-001` acceptance: with >= 2 projects added to the
        // supervisor, a task-file move in either project produces a domain
        // event carrying the *correct* project_id on the single EventBus.
        // Integration shape: real filesystems, real notify watcher, real
        // debounce window — assert routing across the supervisor map.
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let sup = WatcherSupervisor::new(bus);
        let dir_a = scratch_project_with_agentheim();
        let dir_b = scratch_project_with_agentheim();

        sup.add(101, &dir_a).unwrap();
        sup.add(202, &dir_b).unwrap();

        // Drain the two ProjectAdded events so the assertion below only sees
        // the post-mutation TaskMoved events.
        for _ in 0..2 {
            let evt = tokio::time::timeout(Duration::from_secs(2), rx.recv())
                .await
                .expect("ProjectAdded should arrive")
                .expect("bus delivers");
            assert!(matches!(evt, DomainEvent::ProjectAdded { .. }));
        }

        // Seed a task file in each project and let the watcher arm.
        fs::write(
            dir_a.join(".agentheim/contexts/canvas/backlog/canvas-100.md"),
            "a",
        )
        .unwrap();
        fs::write(
            dir_b.join(".agentheim/contexts/canvas/backlog/canvas-200.md"),
            "b",
        )
        .unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;

        // Drain whatever those seed creates produced so the move below is
        // the first event we assert on.
        loop {
            match tokio::time::timeout(Duration::from_millis(400), rx.recv()).await {
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }

        // Move task in project A and verify routing.
        fs::rename(
            dir_a.join(".agentheim/contexts/canvas/backlog/canvas-100.md"),
            dir_a.join(".agentheim/contexts/canvas/doing/canvas-100.md"),
        )
        .unwrap();

        let mut saw_a = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !saw_a {
            let event = tokio::time::timeout_at(deadline, rx.recv())
                .await
                .expect("event from project A should arrive")
                .expect("bus delivers");
            if let DomainEvent::TaskMoved {
                project_id,
                ref bc,
                ref task_id,
                ..
            } = event
            {
                assert_eq!(project_id, 101, "must carry project A's id");
                assert_eq!(bc, "canvas");
                assert_eq!(task_id, "canvas-100");
                saw_a = true;
            }
        }

        // Move task in project B and verify routing.
        fs::rename(
            dir_b.join(".agentheim/contexts/canvas/backlog/canvas-200.md"),
            dir_b.join(".agentheim/contexts/canvas/doing/canvas-200.md"),
        )
        .unwrap();

        let mut saw_b = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !saw_b {
            let event = tokio::time::timeout_at(deadline, rx.recv())
                .await
                .expect("event from project B should arrive")
                .expect("bus delivers");
            if let DomainEvent::TaskMoved {
                project_id,
                ref bc,
                ref task_id,
                ..
            } = event
            {
                assert_eq!(project_id, 202, "must carry project B's id");
                assert_eq!(bc, "canvas");
                assert_eq!(task_id, "canvas-200");
                saw_b = true;
            }
        }

        sup.remove(101);
        sup.remove(202);
        fs::remove_dir_all(&dir_a).ok();
        fs::remove_dir_all(&dir_b).ok();
    }
}
