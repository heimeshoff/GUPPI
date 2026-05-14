//! Filesystem observation — `notify-debouncer-full`, scoped to `.agentheim/`
//! (ADR-008).
//!
//! The full ADR-008 design is a central `WatcherSupervisor` owning a
//! `project_id -> debounced watcher` map, translating debounced FS events into
//! the fine-grained `TaskMoved` / `BCAppeared` / `BCDisappeared` domain events.
//! The walking skeleton needs only the *spine* of that: one debounced watcher
//! on the one hard-coded project's `.agentheim/`, emitting a coarse
//! `AgentheimChanged` so the frontend re-fetches `get_project`. The skeleton
//! task explicitly calls this "crude — real event-bus mapping comes after".
//!
//! What is real here and validated by this code: `notify-debouncer-full`
//! against an actual `.agentheim/` tree, the 250ms debounce window, and
//! delivery onto the ADR-009 event bus.

use crate::events::{DomainEvent, EventBus};
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::path::Path;
use std::time::Duration;

/// ADR-008: long enough to coalesce a burst from one logical change, short
/// enough that the canvas still feels live.
pub const DEBOUNCE_WINDOW: Duration = Duration::from_millis(250);

#[derive(Debug, thiserror::Error)]
pub enum WatcherError {
    #[error("the .agentheim directory to watch does not exist: {0}")]
    PathMissing(String),
    #[error("notify error: {0}")]
    Notify(#[from] notify::Error),
}

/// A live filesystem watcher. Holding this value keeps the watcher running;
/// dropping it tears the watcher down (ADR-008's "drops a watcher when a
/// project is removed", in single-project form).
pub struct AgentheimWatcher {
    // The debouncer owns the underlying `notify` watcher and the debounce
    // thread; keeping it alive in the struct keeps observation running.
    _debouncer: notify_debouncer_full::Debouncer<
        notify::RecommendedWatcher,
        notify_debouncer_full::FileIdMap,
    >,
}

impl AgentheimWatcher {
    /// Begin watching `<project>/.agentheim/` recursively. Every debounced
    /// batch of filesystem events publishes a single `AgentheimChanged` for
    /// `project_id` onto the bus — the frontend reacts by re-fetching the
    /// project snapshot.
    pub fn start(
        project_id: i64,
        project_path: &Path,
        bus: EventBus,
    ) -> Result<Self, WatcherError> {
        let agentheim = project_path.join(".agentheim");
        if !agentheim.is_dir() {
            return Err(WatcherError::PathMissing(
                agentheim.to_string_lossy().into_owned(),
            ));
        }

        let mut debouncer = new_debouncer(DEBOUNCE_WINDOW, None, move |result: DebounceEventResult| {
            // The debouncer hands us either a coalesced batch of events or a
            // batch of errors. Either way the skeleton's response is the same:
            // tell the frontend to resync. A watched directory vanishing shows
            // up here as an error batch; we do not crash on it (ADR-008's
            // "survive folder deletions"), we simply stop emitting because the
            // debouncer will have nothing more to report.
            match result {
                Ok(events) if !events.is_empty() => {
                    bus.publish(DomainEvent::AgentheimChanged { project_id });
                }
                Ok(_) => {}
                Err(errors) => {
                    for error in errors {
                        tracing::warn!(?error, "filesystem watcher reported an error");
                    }
                }
            }
        })?;

        debouncer
            .watcher()
            .watch(&agentheim, RecursiveMode::Recursive)?;

        tracing::info!(
            project_id,
            path = %agentheim.display(),
            "watching .agentheim for changes"
        );

        Ok(Self {
            _debouncer: debouncer,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use std::fs;
    use std::path::PathBuf;

    fn scratch_project() -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "guppi-watcher-test-{}-{:?}",
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

    #[test]
    fn refuses_to_watch_a_missing_agentheim_directory() {
        let dir = std::env::temp_dir().join(format!("guppi-no-agentheim-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let bus = EventBus::new();
        match AgentheimWatcher::start(1, &dir, bus) {
            Err(WatcherError::PathMissing(_)) => {}
            Err(other) => panic!("unexpected error: {other:?}"),
            Ok(_) => panic!("expected watching a missing .agentheim to fail"),
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn moving_a_task_file_emits_a_change_event() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Watch\n").unwrap();
        fs::write(dir.join(".agentheim/contexts/canvas/backlog/x.md"), "x").unwrap();

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let _watcher = AgentheimWatcher::start(42, &dir, bus).unwrap();

        // Give the OS watcher a moment to arm before mutating the tree.
        tokio::time::sleep(Duration::from_millis(200)).await;
        fs::rename(
            dir.join(".agentheim/contexts/canvas/backlog/x.md"),
            dir.join(".agentheim/contexts/canvas/doing/x.md"),
        )
        .unwrap();

        // Debounce window is 250ms; allow generous slack for CI/Windows.
        let event = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("a change event should arrive within the timeout")
            .expect("the bus should deliver the event");

        match event {
            DomainEvent::AgentheimChanged { project_id } => assert_eq!(project_id, 42),
            other => panic!("unexpected event: {other:?}"),
        }

        fs::remove_dir_all(&dir).ok();
    }
}
