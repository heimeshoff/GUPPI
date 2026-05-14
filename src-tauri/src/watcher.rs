//! Filesystem observation — `notify-debouncer-full`, scoped to `.agentheim/`
//! (ADR-008).
//!
//! The full ADR-008 design is a central `WatcherSupervisor` owning a
//! `project_id -> debounced watcher` map. This module is the **single-project**
//! form of that: one debounced watcher on one project's `.agentheim/`. The
//! multi-project supervisor lands with the project registry, not here
//! (`infrastructure-014` scope-out).
//!
//! What this module does (`infrastructure-014`): it correlates each debounced
//! batch of raw filesystem events into the fine-grained ADR-008/ADR-009 domain
//! events — `TaskMoved`, `TaskAdded`, `TaskRemoved`, `BCAppeared`,
//! `BCDisappeared` — and publishes them onto the event bus. It *also* still
//! publishes the coarse `AgentheimChanged` for every batch, unchanged from the
//! walking skeleton: a deliberate seam so the skeleton frontend keeps working
//! while `canvas-001` migrates it to the fine-grained events and retires
//! `AgentheimChanged`.
//!
//! Correlation rule (ADR-008): a create and a delete of the **same `task_id`**
//! landing in the **same 250ms debounce window** is one `TaskMoved`. An
//! unpaired create is a `TaskAdded`; an unpaired delete is a `TaskRemoved`.
//! Creates and deletes of *different* `task_id`s in the same window are *not*
//! paired — they stay separate `TaskAdded` / `TaskRemoved`.

use crate::events::{DomainEvent, EventBus};
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

/// ADR-008: long enough to coalesce a burst from one logical change, short
/// enough that the canvas still feels live.
pub const DEBOUNCE_WINDOW: Duration = Duration::from_millis(250);

/// The four Agentheim task-state directory names. A path under
/// `contexts/<bc>/<state>/` is only a task file if `<state>` is one of these.
const TASK_STATES: [&str; 4] = ["backlog", "todo", "doing", "done"];

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
    /// batch of filesystem events is correlated into fine-grained domain
    /// events (`TaskMoved` / `TaskAdded` / `TaskRemoved` / `BCAppeared` /
    /// `BCDisappeared`) which are published onto the bus; a coarse
    /// `AgentheimChanged` is also published for the batch (skeleton-compat
    /// seam — see module docs).
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

        let agentheim_root = agentheim.clone();
        let mut debouncer = new_debouncer(DEBOUNCE_WINDOW, None, move |result: DebounceEventResult| {
            // The debouncer hands us either a coalesced batch of events or a
            // batch of errors. A watched directory vanishing shows up here as
            // an error batch; we do not crash on it (ADR-008's "survive folder
            // deletions"), we log and stop — the debouncer will have nothing
            // more to report.
            match result {
                Ok(events) if !events.is_empty() => {
                    // ADR-008/014: correlate the raw batch into fine-grained
                    // domain events.
                    let raw: Vec<Event> =
                        events.iter().map(|e| e.event.clone()).collect();
                    for event in correlate(project_id, &agentheim_root, &raw) {
                        bus.publish(event);
                    }
                    // Skeleton-compat seam: the coarse event still fires for
                    // every batch. `canvas-001` retires it.
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

/// What a single watched path resolved to, relative to `.agentheim/`.
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathKind {
    /// A task file: `contexts/<bc>/<state>/<task_id>.md`.
    Task {
        bc: String,
        state: String,
        task_id: String,
    },
    /// A bounded-context directory: `contexts/<bc>`.
    Bc { bc: String },
    /// Anything else under `.agentheim/` that does not change task placement
    /// (vision.md, INDEX.md, concept pages, the `contexts/` dir itself, …).
    Other,
}

/// Classify a watched absolute path against the `.agentheim/` root.
fn classify(agentheim_root: &Path, path: &Path) -> PathKind {
    let rel = match path.strip_prefix(agentheim_root) {
        Ok(rel) => rel,
        Err(_) => return PathKind::Other,
    };
    let parts: Vec<&str> = rel
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect();

    match parts.as_slice() {
        // contexts/<bc>
        ["contexts", bc] => PathKind::Bc {
            bc: (*bc).to_string(),
        },
        // contexts/<bc>/<state>/<file>.md
        ["contexts", bc, state, file]
            if TASK_STATES.contains(state) && file.ends_with(".md") =>
        {
            PathKind::Task {
                bc: (*bc).to_string(),
                state: (*state).to_string(),
                task_id: file.trim_end_matches(".md").to_string(),
            }
        }
        _ => PathKind::Other,
    }
}

/// Split one debounced `notify::Event` into the paths that *appeared* and the
/// paths that *were removed* by it. Content-only modifications (editor saves
/// inside a task file) carry no placement change and contribute nothing.
fn appeared_and_removed(event: &Event) -> (Vec<PathBuf>, Vec<PathBuf>) {
    match event.kind {
        EventKind::Create(_) => (event.paths.clone(), Vec::new()),
        EventKind::Remove(_) => (Vec::new(), event.paths.clone()),
        // The debouncer stitches rename pairs together. `Both` carries
        // `[from, to]`; `From` / `To` carry one side each.
        EventKind::Modify(ModifyKind::Name(mode)) => match mode {
            RenameMode::To => (event.paths.clone(), Vec::new()),
            RenameMode::From => (Vec::new(), event.paths.clone()),
            RenameMode::Both | RenameMode::Any => {
                // Convention: paths[0] is the source, paths[1] the target.
                match event.paths.as_slice() {
                    [from, to] => (vec![to.clone()], vec![from.clone()]),
                    [single] => (vec![single.clone()], vec![single.clone()]),
                    _ => (Vec::new(), Vec::new()),
                }
            }
            _ => (Vec::new(), Vec::new()),
        },
        // Content / metadata / access changes do not move a task.
        _ => (Vec::new(), Vec::new()),
    }
}

/// Correlate a debounced batch of raw filesystem events into fine-grained
/// domain events. Pure: no I/O, no clock — directly unit-testable.
///
/// - A removed task file and an appeared task file with the **same `task_id`**
///   pair into one `TaskMoved { from, to }`.
/// - Leftover appeared task files become `TaskAdded`; leftover removed task
///   files become `TaskRemoved`.
/// - A `contexts/<bc>` directory appearing / disappearing becomes
///   `BCAppeared` / `BCDisappeared`.
fn correlate(project_id: i64, agentheim_root: &Path, events: &[Event]) -> Vec<DomainEvent> {
    // Collected task-file movements, keyed by task_id so a create and a delete
    // of the same task in the same window can be paired.
    let mut appeared_tasks: HashMap<String, (String, String)> = HashMap::new(); // task_id -> (bc, state)
    let mut removed_tasks: HashMap<String, (String, String)> = HashMap::new();
    let mut appeared_bcs: Vec<String> = Vec::new();
    let mut removed_bcs: Vec<String> = Vec::new();

    for event in events {
        let (appeared, removed) = appeared_and_removed(event);
        for path in appeared {
            match classify(agentheim_root, &path) {
                PathKind::Task { bc, state, task_id } => {
                    appeared_tasks.insert(task_id, (bc, state));
                }
                PathKind::Bc { bc } => {
                    if !appeared_bcs.contains(&bc) {
                        appeared_bcs.push(bc);
                    }
                }
                PathKind::Other => {}
            }
        }
        for path in removed {
            match classify(agentheim_root, &path) {
                PathKind::Task { bc, state, task_id } => {
                    removed_tasks.insert(task_id, (bc, state));
                }
                PathKind::Bc { bc } => {
                    if !removed_bcs.contains(&bc) {
                        removed_bcs.push(bc);
                    }
                }
                PathKind::Other => {}
            }
        }
    }

    let mut out = Vec::new();

    // Pair same-`task_id` appear+remove into `TaskMoved`; the rest fall through
    // to `TaskAdded` / `TaskRemoved`. Iterating over removed and probing
    // appeared keeps the pairing strictly keyed on `task_id`.
    let paired: Vec<String> = removed_tasks
        .keys()
        .filter(|id| appeared_tasks.contains_key(*id))
        .cloned()
        .collect();

    for task_id in &paired {
        let (from_bc, from_state) = removed_tasks.remove(task_id).unwrap();
        let (to_bc, to_state) = appeared_tasks.remove(task_id).unwrap();
        // The bc should match for a move; if the same task_id somehow appears
        // under a different bc, the `to` side wins (that is where the file is
        // now). Equal in every realistic case.
        let _ = from_bc;
        out.push(DomainEvent::TaskMoved {
            project_id,
            bc: to_bc,
            from: from_state,
            to: to_state,
            task_id: task_id.clone(),
        });
    }

    for (task_id, (bc, state)) in appeared_tasks {
        out.push(DomainEvent::TaskAdded {
            project_id,
            bc,
            state,
            task_id,
        });
    }
    for (task_id, (bc, state)) in removed_tasks {
        out.push(DomainEvent::TaskRemoved {
            project_id,
            bc,
            state,
            task_id,
        });
    }
    for bc in appeared_bcs {
        out.push(DomainEvent::BCAppeared { project_id, bc });
    }
    for bc in removed_bcs {
        out.push(DomainEvent::BCDisappeared { project_id, bc });
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::EventBus;
    use notify::event::{CreateKind, RemoveKind};
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

    /// Build the absolute path of a task file under a fake `.agentheim/` root.
    fn task_path(root: &Path, bc: &str, state: &str, task_id: &str) -> PathBuf {
        root.join("contexts")
            .join(bc)
            .join(state)
            .join(format!("{task_id}.md"))
    }

    fn create(path: PathBuf) -> Event {
        Event {
            kind: EventKind::Create(CreateKind::File),
            paths: vec![path],
            attrs: Default::default(),
        }
    }

    fn remove(path: PathBuf) -> Event {
        Event {
            kind: EventKind::Remove(RemoveKind::File),
            paths: vec![path],
            attrs: Default::default(),
        }
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

    // --- correlation logic (ADR-008 requires these explicitly) -----------

    #[test]
    fn paired_create_and_delete_of_same_task_emits_one_task_moved() {
        let root = Path::new("/fake/.agentheim");
        let batch = vec![
            remove(task_path(root, "canvas", "backlog", "canvas-007")),
            create(task_path(root, "canvas", "doing", "canvas-007")),
        ];

        let events = correlate(42, root, &batch);

        assert_eq!(events.len(), 1, "expected exactly one event, got {events:?}");
        match &events[0] {
            DomainEvent::TaskMoved {
                project_id,
                bc,
                from,
                to,
                task_id,
            } => {
                assert_eq!(*project_id, 42);
                assert_eq!(bc, "canvas");
                assert_eq!(from, "backlog");
                assert_eq!(to, "doing");
                assert_eq!(task_id, "canvas-007");
            }
            other => panic!("expected TaskMoved, got {other:?}"),
        }
    }

    #[test]
    fn a_stitched_rename_event_also_emits_one_task_moved() {
        // The debouncer may hand us a single `Modify(Name(Both))` carrying
        // `[from, to]` instead of a separate create + remove.
        let root = Path::new("/fake/.agentheim");
        let batch = vec![Event {
            kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
            paths: vec![
                task_path(root, "canvas", "todo", "canvas-003"),
                task_path(root, "canvas", "done", "canvas-003"),
            ],
            attrs: Default::default(),
        }];

        let events = correlate(7, root, &batch);

        assert_eq!(events.len(), 1, "got {events:?}");
        assert!(matches!(
            &events[0],
            DomainEvent::TaskMoved { from, to, task_id, .. }
                if from == "todo" && to == "done" && task_id == "canvas-003"
        ));
    }

    #[test]
    fn an_unpaired_create_emits_task_added() {
        let root = Path::new("/fake/.agentheim");
        let batch = vec![create(task_path(root, "infrastructure", "backlog", "infrastructure-099"))];

        let events = correlate(1, root, &batch);

        assert_eq!(events.len(), 1, "got {events:?}");
        match &events[0] {
            DomainEvent::TaskAdded {
                bc, state, task_id, ..
            } => {
                assert_eq!(bc, "infrastructure");
                assert_eq!(state, "backlog");
                assert_eq!(task_id, "infrastructure-099");
            }
            other => panic!("expected TaskAdded, got {other:?}"),
        }
    }

    #[test]
    fn an_unpaired_delete_emits_task_removed() {
        let root = Path::new("/fake/.agentheim");
        let batch = vec![remove(task_path(root, "canvas", "done", "canvas-012"))];

        let events = correlate(1, root, &batch);

        assert_eq!(events.len(), 1, "got {events:?}");
        match &events[0] {
            DomainEvent::TaskRemoved {
                bc, state, task_id, ..
            } => {
                assert_eq!(bc, "canvas");
                assert_eq!(state, "done");
                assert_eq!(task_id, "canvas-012");
            }
            other => panic!("expected TaskRemoved, got {other:?}"),
        }
    }

    #[test]
    fn different_task_ids_in_one_window_do_not_pair_into_a_bogus_move() {
        // A create of one task and a delete of an *unrelated* task in the same
        // debounce window must stay separate — not be mistaken for a move.
        let root = Path::new("/fake/.agentheim");
        let batch = vec![
            create(task_path(root, "canvas", "todo", "canvas-100")),
            remove(task_path(root, "canvas", "doing", "canvas-200")),
        ];

        let events = correlate(5, root, &batch);

        assert_eq!(events.len(), 2, "expected two distinct events, got {events:?}");
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, DomainEvent::TaskMoved { .. })),
            "must not fabricate a TaskMoved: {events:?}"
        );
        assert!(events.iter().any(|e| matches!(
            e,
            DomainEvent::TaskAdded { task_id, state, .. }
                if task_id == "canvas-100" && state == "todo"
        )));
        assert!(events.iter().any(|e| matches!(
            e,
            DomainEvent::TaskRemoved { task_id, state, .. }
                if task_id == "canvas-200" && state == "doing"
        )));
    }

    #[test]
    fn a_new_bc_directory_emits_bc_appeared() {
        let root = Path::new("/fake/.agentheim");
        let batch = vec![Event {
            kind: EventKind::Create(CreateKind::Folder),
            paths: vec![root.join("contexts").join("voice")],
            attrs: Default::default(),
        }];

        let events = correlate(3, root, &batch);

        assert_eq!(events.len(), 1, "got {events:?}");
        assert!(matches!(
            &events[0],
            DomainEvent::BCAppeared { bc, .. } if bc == "voice"
        ));
    }

    #[test]
    fn a_removed_bc_directory_emits_bc_disappeared() {
        let root = Path::new("/fake/.agentheim");
        let batch = vec![Event {
            kind: EventKind::Remove(RemoveKind::Folder),
            paths: vec![root.join("contexts").join("voice")],
            attrs: Default::default(),
        }];

        let events = correlate(3, root, &batch);

        assert_eq!(events.len(), 1, "got {events:?}");
        assert!(matches!(
            &events[0],
            DomainEvent::BCDisappeared { bc, .. } if bc == "voice"
        ));
    }

    #[test]
    fn non_task_changes_under_agentheim_produce_no_fine_grained_events() {
        // An edit to vision.md or an INDEX.md is a real `.agentheim/` change
        // (so `AgentheimChanged` still fires from `start`), but it moves no
        // task and creates no BC — `correlate` yields nothing.
        let root = Path::new("/fake/.agentheim");
        let batch = vec![
            create(root.join("vision.md")),
            Event {
                kind: EventKind::Modify(ModifyKind::Any),
                paths: vec![root.join("contexts").join("canvas").join("INDEX.md")],
                attrs: Default::default(),
            },
        ];

        assert!(correlate(1, root, &batch).is_empty());
    }

    // --- the real debounced path (integration) ---------------------------

    #[tokio::test]
    async fn moving_a_task_file_emits_task_moved_and_still_emits_agentheim_changed() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Watch\n").unwrap();
        fs::write(
            dir.join(".agentheim/contexts/canvas/backlog/canvas-007.md"),
            "x",
        )
        .unwrap();

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let _watcher = AgentheimWatcher::start(42, &dir, bus).unwrap();

        // Give the OS watcher a moment to arm before mutating the tree.
        tokio::time::sleep(Duration::from_millis(200)).await;
        fs::rename(
            dir.join(".agentheim/contexts/canvas/backlog/canvas-007.md"),
            dir.join(".agentheim/contexts/canvas/doing/canvas-007.md"),
        )
        .unwrap();

        // Drain the batch's events (debounce window 250ms; generous slack for
        // CI/Windows). The batch yields the fine-grained event(s) followed by
        // the coarse `AgentheimChanged`.
        let mut saw_moved = false;
        let mut saw_changed = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !(saw_moved && saw_changed) {
            let event = tokio::time::timeout_at(deadline, rx.recv())
                .await
                .expect("events should arrive within the timeout")
                .expect("the bus should deliver the event");
            match event {
                DomainEvent::TaskMoved {
                    project_id,
                    ref bc,
                    ref from,
                    ref to,
                    ref task_id,
                } => {
                    assert_eq!(project_id, 42);
                    assert_eq!(bc, "canvas");
                    assert_eq!(from, "backlog");
                    assert_eq!(to, "doing");
                    assert_eq!(task_id, "canvas-007");
                    saw_moved = true;
                }
                DomainEvent::AgentheimChanged { project_id } => {
                    assert_eq!(project_id, 42);
                    saw_changed = true;
                }
                // Some platforms surface the rename as create+remove in the
                // same batch; either way a `TaskMoved` must result. An
                // unexpected `TaskAdded`/`TaskRemoved` would fail the
                // `saw_moved` assertion by timeout.
                other => panic!("unexpected event: {other:?}"),
            }
        }

        fs::remove_dir_all(&dir).ok();
    }
}
