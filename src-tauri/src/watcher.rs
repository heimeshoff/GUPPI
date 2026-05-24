//! Filesystem observation — `notify-debouncer-full`, scoped to `.agentheim/`
//! (ADR-008).
//!
//! The full ADR-008 design is a central `WatcherSupervisor` owning a
//! `project_id -> debounced watcher` map. This module is the **single-project**
//! form of that: one debounced watcher on one project's `.agentheim/`. The
//! multi-project supervisor lands with the project registry, not here
//! (`infrastructure-014` scope-out).
//!
//! What this module does (`infrastructure-014` / `canvas-001`): it correlates
//! each debounced batch of raw filesystem events into the fine-grained
//! ADR-008/ADR-009 domain events — `TaskMoved`, `TaskAdded`, `TaskRemoved`,
//! `BCAppeared`, `BCDisappeared` — and publishes *only* those onto the event
//! bus. `canvas-001` retired the coarse skeleton-compat event: the watcher no
//! longer publishes anything on the normal path beyond `correlate()`'s output.
//! The lag-only `ResyncRequired` signal is emitted by `lib.rs`'s frontend
//! bridge, not here.
//!
//! Correlation rule (ADR-008): a create and a delete of the **same `task_id`**
//! landing in the **same 250ms debounce window** is one `TaskMoved`. An
//! unpaired create is a `TaskAdded`; an unpaired delete is a `TaskRemoved`.
//! Creates and deletes of *different* `task_id`s in the same window are *not*
//! paired — they stay separate `TaskAdded` / `TaskRemoved`.

use crate::events::{DomainEvent, EventBus};
use crate::project::{parse_relationships, parse_task_file, Relationship, TaskColumn};
use notify::event::{ModifyKind, RenameMode};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
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

/// Per-project cache of the last parsed `relationships:` block for each BC.
/// `project-registry-004` consults this on every README write so the
/// `BcRelationshipsChanged` event only fires when the parsed set actually
/// differs from the cached value — prose-only edits do not trigger a relayout.
///
/// Lives inside the watcher closure (one cache per `AgentheimWatcher`) so the
/// state is naturally scoped to the project's lifetime and is dropped with the
/// watcher.
type RelationshipsCache = HashMap<String, Vec<Relationship>>;

impl AgentheimWatcher {
    /// Begin watching `<project>/.agentheim/` recursively. Every debounced
    /// batch of filesystem events is correlated into fine-grained domain
    /// events (`TaskMoved` / `TaskAdded` / `TaskRemoved` / `BCAppeared` /
    /// `BCDisappeared`) which are published onto the bus. Those are the only
    /// events the watcher publishes — `canvas-001` retired the coarse
    /// skeleton-compat event (see module docs).
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
        // `project-registry-004`: per-project cache of the parsed
        // `relationships:` block, keyed by BC name. The closure mutates it on
        // every README write; deep-equal against the previous value decides
        // whether to fire `BcRelationshipsChanged`. Initialised lazily — empty
        // on first run, so a fresh-watcher's first README write fires the event
        // iff the BC actually declares any relationships. (Subsequent prose
        // edits don't.)
        let relationships_cache: Arc<Mutex<RelationshipsCache>> =
            Arc::new(Mutex::new(HashMap::new()));
        let mut debouncer = new_debouncer(DEBOUNCE_WINDOW, None, move |result: DebounceEventResult| {
            // The debouncer hands us either a coalesced batch of events or a
            // batch of errors. A watched directory vanishing shows up here as
            // an error batch; we do not crash on it (ADR-008's "survive folder
            // deletions"), we log and stop — the debouncer will have nothing
            // more to report.
            match result {
                Ok(events) if !events.is_empty() => {
                    // ADR-008/014: correlate the raw batch into fine-grained
                    // domain events — the only events the watcher publishes.
                    let raw: Vec<Event> =
                        events.iter().map(|e| e.event.clone()).collect();
                    let correlated = correlate(project_id, &agentheim_root, &raw);
                    // `project-registry-005`: `correlate` is pure and emits
                    // `TaskAdded` with empty metadata; enrich each from the
                    // just-created file's frontmatter before publishing so the
                    // canvas can draw the card without a resync. A task that
                    // both appeared AND only-changed in the same batch is a
                    // move/add, never a `TaskChanged` (those task_ids are
                    // tracked here so the in-place detector skips them).
                    let mut placement_changed: Vec<String> = Vec::new();
                    for event in correlated {
                        match &event {
                            DomainEvent::TaskAdded { bc, state, task_id, .. } => {
                                placement_changed.push(task_id.clone());
                                bus.publish(enrich_task_added(
                                    project_id,
                                    &agentheim_root,
                                    event.clone(),
                                    bc,
                                    state,
                                    task_id,
                                ));
                            }
                            DomainEvent::TaskMoved { task_id, .. }
                            | DomainEvent::TaskRemoved { task_id, .. } => {
                                placement_changed.push(task_id.clone());
                                bus.publish(event);
                            }
                            _ => {
                                bus.publish(event);
                            }
                        }
                    }
                    // `project-registry-005`: detect in-place writes to an
                    // existing task file (a `Modify(Data)`/`Modify(Any)` whose
                    // task_id did NOT also move/add/remove in this batch) and
                    // fire `TaskChanged` with the re-read frontmatter so the
                    // canvas patches the card's metadata in place.
                    for (path, column, task_id, bc) in
                        changed_task_files(&agentheim_root, &raw)
                    {
                        if placement_changed.contains(&task_id) {
                            continue;
                        }
                        let task = parse_task_file(&path, column);
                        bus.publish(DomainEvent::TaskChanged {
                            project_id,
                            bc,
                            task_id: task.id,
                            title: task.title,
                            type_: task.type_,
                            tags: task.tags,
                            blocked_question: task.blocked_question,
                        });
                    }
                    // `project-registry-004`: scan the same batch for BC
                    // README writes and fire `BcRelationshipsChanged` for each
                    // BC whose parsed relationships set actually changed.
                    let bcs_touched = readme_touched_bcs(&agentheim_root, &raw);
                    if !bcs_touched.is_empty() {
                        let mut cache = relationships_cache.lock().unwrap();
                        for bc in bcs_touched {
                            let parsed =
                                reparse_bc_relationships(&agentheim_root, &bc);
                            let changed = match cache.get(&bc) {
                                Some(prev) => prev != &parsed,
                                None => !parsed.is_empty(),
                            };
                            if changed {
                                cache.insert(bc.clone(), parsed);
                                bus.publish(DomainEvent::BcRelationshipsChanged {
                                    project_id,
                                    bc,
                                });
                            }
                        }
                    }
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
    /// A bounded-context README: `contexts/<bc>/README.md`. Surfaced separately
    /// from generic `Other` so `project-registry-004`'s relationship-change
    /// detector can spot README writes without re-classifying every path.
    BcReadme { bc: String },
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
        // contexts/<bc>/README.md — relationships frontmatter lives here
        // (`project-registry-004`).
        ["contexts", bc, "README.md"] => PathKind::BcReadme {
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

/// Scan a debounced batch for any `contexts/<bc>/README.md` writes (create,
/// modify, or rename). Returns the unique set of BC names whose README was
/// touched in this batch. Used by the watcher's relationship-change detector
/// (`project-registry-004`).
///
/// We include `Modify(Data)` and `Modify(Any)` here because the standard
/// editor-save path is a content change, not a create — and the task is
/// explicit that prose-only README edits should be cheap (the cache + parse
/// short-circuits) but README writes that change `relationships:` MUST fire
/// the event.
fn readme_touched_bcs(agentheim_root: &Path, events: &[Event]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for event in events {
        let touches_content = matches!(
            event.kind,
            EventKind::Create(_)
                | EventKind::Modify(_)
                | EventKind::Remove(_)
        );
        if !touches_content {
            continue;
        }
        for path in &event.paths {
            if let PathKind::BcReadme { bc } = classify(agentheim_root, path) {
                if !out.contains(&bc) {
                    out.push(bc);
                }
            }
        }
    }
    out
}

/// Re-parse one BC's `relationships:` frontmatter from disk. Wraps
/// `project::parse_relationships` with the watcher's sibling-name discovery
/// (the parser needs the list of sibling BCs in this project to drop
/// cross-project `to` references).
///
/// Errors during the sibling enumeration are swallowed: the watcher cannot
/// give up on a project just because `contexts/` momentarily disappears —
/// returning an empty vec degrades gracefully (the cache will simply see
/// "nothing changed" or "everything emptied", and the canvas will catch up on
/// the next legitimate change).
fn reparse_bc_relationships(agentheim_root: &Path, bc_name: &str) -> Vec<Relationship> {
    let contexts_dir = agentheim_root.join("contexts");
    let sibling_names: Vec<String> = match std::fs::read_dir(&contexts_dir) {
        Ok(read) => read
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect(),
        Err(_) => Vec::new(),
    };
    let bc_dir = contexts_dir.join(bc_name);
    parse_relationships(&bc_dir, bc_name, &sibling_names)
}

/// Re-read a just-created task file's frontmatter and fill the `TaskAdded`
/// event's metadata (`title`, `type_`, `tags`) so the canvas can draw the new
/// card in place (`project-registry-005`). The file is on disk by the time the
/// debounced batch is processed; a missing/malformed frontmatter degrades to
/// empty metadata via `parse_task_file` (the `task_id` is preserved from
/// `correlate`'s filename-stem-based id). Returns the input event unchanged for
/// any non-`TaskAdded` variant (defensive — the caller only passes `TaskAdded`).
fn enrich_task_added(
    project_id: i64,
    agentheim_root: &Path,
    event: DomainEvent,
    bc: &str,
    state: &str,
    task_id: &str,
) -> DomainEvent {
    let column = match TaskColumn::from_state(state) {
        Some(c) => c,
        None => return event,
    };
    let path = agentheim_root
        .join("contexts")
        .join(bc)
        .join(state)
        .join(format!("{task_id}.md"));
    let task = parse_task_file(&path, column);
    DomainEvent::TaskAdded {
        project_id,
        bc: bc.to_string(),
        state: state.to_string(),
        // Keep `correlate`'s task_id (filename-stem based) as the identity on
        // the event so a follow-up `TaskMoved`/`TaskRemoved` correlates; the
        // re-read frontmatter only fills the display metadata.
        task_id: task_id.to_string(),
        title: task.title,
        type_: task.type_,
        tags: task.tags,
    }
}

/// Scan a debounced batch for in-place content writes to existing task files —
/// a `Modify(Data)` / `Modify(Any)` on a `contexts/<bc>/<state>/<task_id>.md`
/// path (NOT a create, remove, or rename — those are handled by `correlate`).
/// Returns `(absolute_path, column, task_id, bc)` per unique touched task file
/// so the watcher can re-read frontmatter and fire `TaskChanged`
/// (`project-registry-005`). The caller filters out task_ids that also moved /
/// were added in the same batch.
fn changed_task_files(
    agentheim_root: &Path,
    events: &[Event],
) -> Vec<(PathBuf, TaskColumn, String, String)> {
    let mut out: Vec<(PathBuf, TaskColumn, String, String)> = Vec::new();
    for event in events {
        // Only content/metadata modifications — explicitly NOT rename
        // (`Modify(Name)`), which is a placement change handled by `correlate`.
        let is_content_modify = matches!(
            event.kind,
            EventKind::Modify(ModifyKind::Data(_))
                | EventKind::Modify(ModifyKind::Any)
                | EventKind::Modify(ModifyKind::Metadata(_))
        );
        if !is_content_modify {
            continue;
        }
        for path in &event.paths {
            if let PathKind::Task { bc, state, task_id } = classify(agentheim_root, path) {
                let column = match TaskColumn::from_state(&state) {
                    Some(c) => c,
                    None => continue,
                };
                if out.iter().any(|(_, _, id, _)| id == &task_id) {
                    continue;
                }
                out.push((path.clone(), column, task_id, bc));
            }
        }
    }
    out
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
                // README writes are handled by `readme_touched_bcs` +
                // relationship-change detection in the watcher closure, not
                // by `correlate` — they do not move tasks or add/remove BCs.
                PathKind::BcReadme { .. } | PathKind::Other => {}
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
                PathKind::BcReadme { .. } | PathKind::Other => {}
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
        // `correlate` is pure (no I/O): it emits `TaskAdded` with empty
        // metadata. The watcher closure enriches it from disk via
        // `enrich_task_added` before publishing (`project-registry-005`).
        out.push(DomainEvent::TaskAdded {
            project_id,
            bc,
            state,
            task_id,
            title: String::new(),
            type_: String::new(),
            tags: Vec::new(),
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
        // An edit to vision.md or an INDEX.md is a real `.agentheim/` change,
        // but it moves no task and creates no BC — `correlate` yields nothing,
        // and since `canvas-001` the watcher publishes nothing for such a
        // batch (no coarse event remains).
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
    async fn moving_a_task_file_emits_only_task_moved() {
        // Since `canvas-001` the watcher emits *only* the fine-grained events
        // from `correlate()` — no coarse event on the normal path. A move
        // therefore yields exactly one `TaskMoved` and nothing else.
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

        // The batch yields the fine-grained event(s). Drain until `TaskMoved`
        // is seen (debounce window 250ms; generous slack for CI/Windows).
        let mut saw_moved = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !saw_moved {
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
                // Some platforms surface the rename as create+remove in the
                // same batch; either way a `TaskMoved` must result. An
                // unexpected `TaskAdded`/`TaskRemoved` would fail the
                // `saw_moved` assertion by timeout. A coarse event arriving
                // here would also fail — the watcher must not emit one.
                other => panic!("unexpected event: {other:?}"),
            }
        }

        fs::remove_dir_all(&dir).ok();
    }

    // --- `project-registry-004`: relationships change-detection cache ----

    /// Make a Modify(Data) event for one path — the most common
    /// editor-save shape.
    fn modify_data(path: PathBuf) -> Event {
        Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(
                notify::event::DataChange::Content,
            )),
            paths: vec![path],
            attrs: Default::default(),
        }
    }

    // --- `project-registry-005`: in-place TaskChanged detection -----------

    #[test]
    fn changed_task_files_picks_up_a_content_modify_on_a_task_file() {
        let root = Path::new("/fake/.agentheim");
        let path = task_path(root, "canvas", "doing", "canvas-020");
        let events = vec![modify_data(path.clone())];
        let out = changed_task_files(root, &events);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, path);
        assert_eq!(out[0].1, TaskColumn::Doing);
        assert_eq!(out[0].2, "canvas-020");
        assert_eq!(out[0].3, "canvas");
    }

    #[test]
    fn changed_task_files_ignores_create_remove_and_rename() {
        // Creates/removes/renames are placement changes handled by `correlate`,
        // not in-place edits — they must not surface as TaskChanged candidates.
        let root = Path::new("/fake/.agentheim");
        let p = task_path(root, "canvas", "doing", "canvas-020");
        let events = vec![
            create(p.clone()),
            remove(p.clone()),
            Event {
                kind: EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                paths: vec![
                    task_path(root, "canvas", "todo", "canvas-020"),
                    task_path(root, "canvas", "doing", "canvas-020"),
                ],
                attrs: Default::default(),
            },
        ];
        assert!(changed_task_files(root, &events).is_empty());
    }

    #[test]
    fn changed_task_files_ignores_non_task_modifies() {
        let root = Path::new("/fake/.agentheim");
        let readme = root.join("contexts").join("canvas").join("README.md");
        let vision = root.join("vision.md");
        let events = vec![modify_data(readme), modify_data(vision)];
        assert!(changed_task_files(root, &events).is_empty());
    }

    #[test]
    fn enrich_task_added_fills_metadata_from_the_files_frontmatter() {
        let dir = scratch_project();
        let agentheim = dir.join(".agentheim");
        let task = agentheim
            .join("contexts")
            .join("canvas")
            .join("doing")
            .join("canvas-020.md");
        fs::write(
            &task,
            "---\nid: canvas-020\ntitle: Kanban interior\ntype: feature\ntags: [canvas]\n---\n",
        )
        .unwrap();

        let base = DomainEvent::TaskAdded {
            project_id: 7,
            bc: "canvas".to_string(),
            state: "doing".to_string(),
            task_id: "canvas-020".to_string(),
            title: String::new(),
            type_: String::new(),
            tags: Vec::new(),
        };
        let enriched = enrich_task_added(7, &agentheim, base, "canvas", "doing", "canvas-020");
        match enriched {
            DomainEvent::TaskAdded { title, type_, tags, task_id, .. } => {
                assert_eq!(task_id, "canvas-020");
                assert_eq!(title, "Kanban interior");
                assert_eq!(type_, "feature");
                assert_eq!(tags, vec!["canvas".to_string()]);
            }
            other => panic!("expected TaskAdded, got {other:?}"),
        }
        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn in_place_task_frontmatter_write_fires_task_changed() {
        // Acceptance: an in-place edit of a task file's frontmatter (no column
        // move) fires `TaskChanged` carrying the re-read metadata.
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Changed\n").unwrap();
        let task = dir
            .join(".agentheim/contexts/canvas/doing/canvas-020.md");
        fs::write(
            &task,
            "---\nid: canvas-020\ntitle: Original\ntype: feature\n---\n",
        )
        .unwrap();

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let _watcher = AgentheimWatcher::start(7, &dir, bus).unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        // In-place rewrite: same path, changed title + a blocked question.
        fs::write(
            &task,
            "---\nid: canvas-020\ntitle: Renamed\ntype: feature\nblocked_question: \"Dock left or right?\"\n---\n",
        )
        .unwrap();

        let mut saw_changed = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while !saw_changed {
            let event = tokio::time::timeout_at(deadline, rx.recv())
                .await
                .expect("events should arrive within the timeout")
                .expect("the bus should deliver the event");
            match event {
                DomainEvent::TaskChanged {
                    project_id,
                    ref bc,
                    ref task_id,
                    ref title,
                    ref blocked_question,
                    ..
                } => {
                    assert_eq!(project_id, 7);
                    assert_eq!(bc, "canvas");
                    assert_eq!(task_id, "canvas-020");
                    assert_eq!(title, "Renamed");
                    assert_eq!(
                        blocked_question.as_deref(),
                        Some("Dock left or right?")
                    );
                    saw_changed = true;
                }
                // A spurious TaskAdded/TaskMoved here would mean the platform
                // reported the write as a create/rename; tolerate by continuing
                // — only a timeout (no TaskChanged at all) fails the test.
                _ => continue,
            }
        }

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn readme_touched_bcs_returns_bc_for_create_on_bc_readme() {
        let root = Path::new("/fake/.agentheim");
        let path = root.join("contexts").join("canvas").join("README.md");
        let events = vec![create(path)];
        assert_eq!(readme_touched_bcs(root, &events), vec!["canvas".to_string()]);
    }

    #[test]
    fn readme_touched_bcs_returns_bc_for_modify_on_bc_readme() {
        // The standard editor-save path is a content modification.
        let root = Path::new("/fake/.agentheim");
        let path = root.join("contexts").join("voice").join("README.md");
        let events = vec![modify_data(path)];
        assert_eq!(readme_touched_bcs(root, &events), vec!["voice".to_string()]);
    }

    #[test]
    fn readme_touched_bcs_dedupes_multiple_events_for_the_same_bc() {
        // A burst of modify events on the same README (editor save + atomic
        // rename combo) yields exactly one BC in the result.
        let root = Path::new("/fake/.agentheim");
        let path = root.join("contexts").join("canvas").join("README.md");
        let events = vec![modify_data(path.clone()), modify_data(path)];
        assert_eq!(readme_touched_bcs(root, &events), vec!["canvas".to_string()]);
    }

    #[test]
    fn readme_touched_bcs_ignores_non_readme_writes() {
        // A task-file write, a vision.md edit, an INDEX.md edit — none of these
        // are README writes, so none surface here.
        let root = Path::new("/fake/.agentheim");
        let task = root.join("contexts").join("canvas").join("backlog").join("c-1.md");
        let vision = root.join("vision.md");
        let index = root.join("contexts").join("canvas").join("INDEX.md");
        let events = vec![
            modify_data(task),
            modify_data(vision),
            modify_data(index),
        ];
        assert!(
            readme_touched_bcs(root, &events).is_empty(),
            "non-README writes must not be picked up"
        );
    }

    #[test]
    fn reparse_bc_relationships_returns_empty_for_missing_readme() {
        // No README on disk → empty Vec, no error.
        let dir = scratch_project();
        // contexts/canvas exists but has no README.md.
        let agentheim = dir.join(".agentheim");
        assert!(reparse_bc_relationships(&agentheim, "canvas").is_empty());
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reparse_bc_relationships_picks_up_a_sibling_bc_reference() {
        // Lay out two BCs in the project so the sibling-name discovery in
        // `reparse_bc_relationships` finds the `to:` target.
        let dir = scratch_project();
        let agentheim = dir.join(".agentheim");
        fs::create_dir_all(agentheim.join("contexts").join("project-registry")).unwrap();
        let canvas_readme = agentheim.join("contexts").join("canvas").join("README.md");
        fs::write(
            &canvas_readme,
            "---\n\
             relationships:\n  - to: project-registry\n    type: customer-supplier\n    direction: upstream\n\
             ---\n# canvas\n",
        )
        .unwrap();

        let parsed = reparse_bc_relationships(&agentheim, "canvas");
        assert_eq!(parsed.len(), 1, "got {parsed:?}");
        assert_eq!(parsed[0].to, "project-registry");

        fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn relationship_change_fires_event_only_when_parsed_set_actually_changes() {
        // The load-bearing acceptance criterion: writes that change
        // `relationships:` MUST fire `BcRelationshipsChanged`; writes that
        // don't (prose-only edits) MUST NOT.
        let dir = scratch_project();
        let agentheim = dir.join(".agentheim");
        // Two BCs so the parser has a sibling target.
        fs::create_dir_all(agentheim.join("contexts").join("project-registry")).unwrap();

        let canvas_readme = agentheim.join("contexts").join("canvas").join("README.md");
        // Seed the README BEFORE the watcher arms so the cache starts populated
        // with whatever's on disk — actually no: the cache starts empty by
        // construction, and the first write should fire iff the parsed set
        // is non-empty. To keep this test pure to the "change-detection"
        // behaviour, we seed the README, arm the watcher, then make a
        // prose-only edit (no fire), then a relationship-changing edit (fires).
        fs::write(
            &canvas_readme,
            "---\nrelationships:\n  - to: project-registry\n    type: customer-supplier\n    direction: upstream\n---\n# canvas\n",
        )
        .unwrap();

        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let _watcher = AgentheimWatcher::start(7, &dir, bus).unwrap();

        // Let the watcher arm.
        tokio::time::sleep(Duration::from_millis(200)).await;

        // First write: rewrite the SAME file with the SAME relationships set
        // (just touching the prose body). The cache starts empty, so the
        // first event MAY fire (the parser produces a non-empty set, the
        // cache says "previous was empty/missing => changed"). To make the
        // test deterministic, treat the first event as "establish the cache"
        // and the second prose-only edit as the real prose-only assertion.

        // We re-write with the SAME relationships block + slightly different
        // prose. The watcher correlates this as a Modify on the README.
        fs::write(
            &canvas_readme,
            "---\nrelationships:\n  - to: project-registry\n    type: customer-supplier\n    direction: upstream\n---\n# canvas\n\nPROSE A.\n",
        )
        .unwrap();

        // Drain whatever the first edit produces (we expect at most one
        // `BcRelationshipsChanged` — the cache priming).
        let mut saw_initial_change = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Ok(DomainEvent::BcRelationshipsChanged { project_id, bc })) => {
                    assert_eq!(project_id, 7);
                    assert_eq!(bc, "canvas");
                    saw_initial_change = true;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        // The initial write should have populated the cache. (We don't fail
        // hard if it didn't fire — depending on platform timing the seed
        // might have been treated as identical to "no previous"; the next
        // assertion is the load-bearing one.)
        let _ = saw_initial_change;

        // Now do a PURE PROSE edit — same `relationships:` block, different
        // body. This MUST NOT fire `BcRelationshipsChanged`.
        fs::write(
            &canvas_readme,
            "---\nrelationships:\n  - to: project-registry\n    type: customer-supplier\n    direction: upstream\n---\n# canvas\n\nPROSE B (totally different).\n",
        )
        .unwrap();

        // Wait through the debounce window plus generous slack. Any
        // `BcRelationshipsChanged` here would be a test failure.
        let prose_deadline = tokio::time::Instant::now() + Duration::from_millis(900);
        loop {
            match tokio::time::timeout_at(prose_deadline, rx.recv()).await {
                Ok(Ok(DomainEvent::BcRelationshipsChanged { .. })) => {
                    panic!("prose-only README edit must not fire BcRelationshipsChanged");
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }

        // Now do a real relationship change — replace the to: target with a
        // different one. (We don't need a real second sibling; the parser
        // simply drops the unknown target with a warning, yielding an empty
        // set — which IS a change from the previous non-empty set.) This
        // MUST fire `BcRelationshipsChanged`.
        fs::write(
            &canvas_readme,
            "---\nrelationships: []\n---\n# canvas\n",
        )
        .unwrap();

        let mut saw_real_change = false;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Ok(Ok(DomainEvent::BcRelationshipsChanged { project_id, bc })) => {
                    assert_eq!(project_id, 7);
                    assert_eq!(bc, "canvas");
                    saw_real_change = true;
                    break;
                }
                Ok(Ok(_)) => continue,
                _ => break,
            }
        }
        assert!(
            saw_real_change,
            "a real change in the parsed relationships must fire BcRelationshipsChanged"
        );

        fs::remove_dir_all(&dir).ok();
    }
}
