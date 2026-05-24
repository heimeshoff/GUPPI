//! Reading an Agentheim project off disk into a `ProjectSnapshot`.
//!
//! The Agentheim layout this understands:
//!
//! ```text
//! <project>/.agentheim/
//!   vision.md                         first line -> project name
//!   contexts/<bc>/README.md           YAML frontmatter -> `relationships:`
//!   contexts/<bc>/{backlog,todo,doing,done}/*.md   -> task counts
//! ```
//!
//! Project discovery (ADR-005) is an explicit-registry concern; the walking
//! skeleton has exactly one hard-coded project, so this module only needs the
//! "read one known project" half — listing `contexts/*`, counting task files,
//! and parsing each BC's README frontmatter for its `relationships:` block.
//! The registry/scan affordances are out of skeleton scope.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The four task-state directories every Agentheim bounded context has.
const TASK_STATES: [&str; 4] = ["backlog", "todo", "doing", "done"];

#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
    #[error("no .agentheim directory found at {0}")]
    NotAnAgentheimProject(PathBuf),
    #[error("io error reading project: {0}")]
    Io(#[from] std::io::Error),
}

/// Task-file counts for one bounded context, keyed by Agentheim task state.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct TaskCounts {
    pub backlog: u32,
    pub todo: u32,
    pub doing: u32,
    pub done: u32,
}

/// The DDD relationship classification between two bounded contexts inside the
/// same project — surfaced via each BC README's YAML frontmatter and consumed
/// by the canvas to render intra-project edges (`canvas-007`). The set is
/// closed at v1; cross-project edges are explicitly out of scope.
///
/// Directional types (`customer-supplier`, `anticorruption-layer`,
/// `conformist`) require a paired `Direction`; non-directional types
/// (`shared-kernel`, `partnership`) carry `direction: None`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RelationshipType {
    /// Directional — pair with `Direction`.
    CustomerSupplier,
    /// Non-directional.
    SharedKernel,
    /// Non-directional.
    Partnership,
    /// Directional with a notch — pair with `Direction`.
    AntiCorruptionLayer,
    /// Directional — pair with `Direction`.
    Conformist,
}

impl RelationshipType {
    /// Whether this relationship type expects a `direction:` field in the
    /// frontmatter (and on the wire). Used by the parser to validate the
    /// shape — directional types without a direction, or non-directional
    /// types with one, are dropped with a warning rather than silently
    /// accepted.
    pub fn is_directional(self) -> bool {
        matches!(
            self,
            RelationshipType::CustomerSupplier
                | RelationshipType::AntiCorruptionLayer
                | RelationshipType::Conformist
        )
    }
}

/// Direction of a directional relationship — `upstream` (the **target** BC is
/// upstream of the **source** BC) or `downstream`. Encoded on the source side
/// only: each BC declares its relationships in its own README; the other side
/// declares the mirror in its README. Brainstorm/model will eventually keep
/// the two sides in sync (Agentheim follow-up); the parser does not enforce
/// symmetry at v1.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Upstream,
    Downstream,
}

/// One BC↔BC relationship, as declared in a BC's README frontmatter. `to` is
/// the bare directory name of a sibling BC inside the same project (e.g.
/// `project-registry`). Cross-project references are dropped at parse time
/// (single warning log line per occurrence — see `parse_relationships`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Relationship {
    /// Sibling BC name within the same project.
    pub to: String,
    /// DDD classification of the relationship.
    #[serde(rename = "type")]
    pub r#type: RelationshipType,
    /// `Some(Direction)` for directional types; `None` for non-directional
    /// types. The parser drops malformed combinations rather than fabricating
    /// a default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub direction: Option<Direction>,
}

/// Which of the four Agentheim task-state columns a task currently lives in —
/// derived from the subdirectory the task file sits under
/// (`backlog`/`todo`/`doing`/`done`). Serialised lowercase to match the
/// task-state vocabulary already used on the wire (`project-registry-005`).
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TaskColumn {
    Backlog,
    Todo,
    Doing,
    Done,
}

impl TaskColumn {
    /// The state-directory name → column mapping. Returns `None` for any other
    /// directory name (the caller only ever asks about the four task states).
    pub(crate) fn from_state(state: &str) -> Option<TaskColumn> {
        match state {
            "backlog" => Some(TaskColumn::Backlog),
            "todo" => Some(TaskColumn::Todo),
            "doing" => Some(TaskColumn::Doing),
            "done" => Some(TaskColumn::Done),
            _ => None,
        }
    }

    /// Stable ordering rank for sorting `tasks` within a BC (backlog → todo →
    /// doing → done), matching the left-to-right kanban column order.
    fn rank(self) -> u8 {
        match self {
            TaskColumn::Backlog => 0,
            TaskColumn::Todo => 1,
            TaskColumn::Doing => 2,
            TaskColumn::Done => 3,
        }
    }
}

/// One individual task record, as the canvas needs to draw a kanban card
/// (`project-registry-005`). Read from a `contexts/<bc>/<column>/<file>.md`
/// task file's YAML frontmatter at enumeration time. A malformed or absent
/// frontmatter degrades gracefully: `id` falls back to the filename stem and
/// the metadata fields are empty (logged once), so a single broken task file
/// never aborts the snapshot.
///
/// Coordination with `agent-awareness-002`: the registry owns the **static**
/// on-disk record (incl. a `blocked_question` if the Agentheim plugin wrote
/// one into frontmatter). The **live** per-task agent state and the
/// "AGENT NEEDS AN ANSWER" callout are agent-awareness's surface — the
/// registry supplies `blocked_question` only as a disk-artifact fallback.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Task {
    /// The task file's `id:` frontmatter, with a filename-stem fallback.
    pub id: String,
    /// The task's `title:` frontmatter; empty string if absent/unparseable.
    pub title: String,
    /// Which kanban column the task is in (derived from its subdirectory).
    pub column: TaskColumn,
    /// The task's `type:` frontmatter (`feature`/`bug`/`spike`/`decision`);
    /// empty string if absent. Serialised as `type_` to dodge the JS reserved
    /// word on the frontend mirror.
    #[serde(rename = "type_")]
    pub type_: String,
    /// The task's `tags:` frontmatter; empty if absent.
    pub tags: Vec<String>,
    /// A `blocked_question:` frontmatter field, if the task carries one on
    /// disk. `None` is the common case — most tasks are not blocked-on-question
    /// and the live callout text comes from `agent-awareness-002`, not here.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_question: Option<String>,
}

/// One bounded context as the canvas needs to draw it. The `relationships`
/// vector is parsed from the BC's `README.md` YAML frontmatter at enumeration
/// time; malformed frontmatter degrades to an empty vector with a single
/// warning log (the BC still enumerates).
///
/// `tasks` (`project-registry-005`) carries the individual task records the
/// kanban-accordion canvas draws as cards, in a stable order (column then id).
/// `task_counts` is **derived** from `tasks` — kept so existing count-consuming
/// code paths (the counts pill / accordion "N tasks" row) need no second
/// source and no flag-day.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BoundedContext {
    pub name: String,
    pub task_counts: TaskCounts,
    /// Individual task records, ordered by column (backlog→done) then by id.
    pub tasks: Vec<Task>,
    /// BC↔BC relationships declared in this BC's README frontmatter. Empty if
    /// the README is absent, has no frontmatter, or fails to parse.
    pub relationships: Vec<Relationship>,
}

/// Everything the frontend needs to render a project tile and its BC children.
///
/// The `id` is the registry's project id (`projects.id` — ADR-005). Carrying
/// it on the snapshot is the load-bearing change for `canvas-002`: the
/// canvas needs the id to key its per-project state and to route fine-grained
/// domain events back to the right tile (`project-registry-001`).
///
/// `missing` is `true` for a registry row whose `.agentheim/` directory is
/// gone on disk — the ADR-005 **registered-but-unwatched** state
/// (`project-registry-003`). Such snapshots always carry `bcs: []`, the
/// `name` falls back to the folder name, and `path` is the canonical path the
/// row was registered under. The canvas renders these in its missing-tile
/// visual (canvas-005a) rather than dropping the tile.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub bcs: Vec<BoundedContext>,
    pub missing: bool,
}

/// Read the Agentheim project rooted at `project_path` into a snapshot.
///
/// Fails only if `.agentheim/` is absent — a missing `vision.md` or an empty
/// `contexts/` directory degrades gracefully (the project name falls back to
/// the folder name; `bcs` is simply empty).
///
/// `project_id` is supplied by the caller (resolved from the registry); the
/// pure reader does not consult the database, it just stamps the id on the
/// snapshot so it crosses IPC with the rest of the data.
pub fn get_project(project_id: i64, project_path: &Path) -> Result<ProjectSnapshot, ProjectError> {
    let agentheim = project_path.join(".agentheim");
    if !agentheim.is_dir() {
        return Err(ProjectError::NotAnAgentheimProject(project_path.to_path_buf()));
    }

    let name = read_project_name(&agentheim, project_path);
    let bcs = read_bounded_contexts(&agentheim)?;

    Ok(ProjectSnapshot {
        id: project_id,
        name,
        path: project_path.to_string_lossy().into_owned(),
        bcs,
        missing: false,
    })
}

/// Build a synthetic `ProjectSnapshot` for the ADR-005 "missing" state — a
/// registry row whose `.agentheim/` directory has been removed on disk. The
/// snapshot carries `missing: true`, `bcs: []`, and a `name` that falls back
/// to the folder name (vision.md cannot be read; no `.agentheim/` exists).
/// `path` is the canonical path the registry knows the project by.
///
/// Used by `list_projects` and `get_project` in `lib.rs` when
/// `project::get_project` returns `NotAnAgentheimProject`: rather than silently
/// skipping or surfacing an error, the IPC layer hands the canvas a missing
/// snapshot so the tile can render in its missing visual (canvas-005a).
pub fn missing_snapshot(project_id: i64, project_path: &Path) -> ProjectSnapshot {
    let name = project_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unnamed project".to_string());
    ProjectSnapshot {
        id: project_id,
        name,
        path: project_path.to_string_lossy().into_owned(),
        bcs: Vec::new(),
        missing: true,
    }
}

/// The project name is the first line of `.agentheim/vision.md`, with a
/// leading markdown heading marker (`# `) stripped. If the file is missing or
/// empty, fall back to the project folder's name.
fn read_project_name(agentheim: &Path, project_path: &Path) -> String {
    let vision = agentheim.join("vision.md");
    if let Ok(contents) = std::fs::read_to_string(&vision) {
        if let Some(first_line) = contents.lines().next() {
            let trimmed = first_line.trim_start_matches('#').trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    project_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Unnamed project".to_string())
}

/// List `.agentheim/contexts/*` and count task files in each. An absent or
/// empty `contexts/` directory yields an empty list — that is valid.
fn read_bounded_contexts(agentheim: &Path) -> Result<Vec<BoundedContext>, ProjectError> {
    let contexts_dir = agentheim.join("contexts");
    if !contexts_dir.is_dir() {
        return Ok(Vec::new());
    }

    // The set of sibling BC names — used by `parse_relationships` to drop
    // cross-project / unknown-BC references with a warning rather than
    // silently letting them flow to the canvas where they'd dangle.
    let entries: Vec<_> = std::fs::read_dir(&contexts_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    let sibling_names: Vec<String> = entries
        .iter()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    let mut bcs = Vec::with_capacity(entries.len());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let bc_dir = entry.path();
        let tasks = read_tasks(&bc_dir);
        let task_counts = counts_from_tasks(&tasks);
        let relationships = parse_relationships(&bc_dir, &name, &sibling_names);
        bcs.push(BoundedContext {
            name,
            task_counts,
            tasks,
            relationships,
        });
    }

    // Stable ordering so the canvas does not reshuffle BC nodes between fetches.
    bcs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(bcs)
}

/// Internal mirror of the YAML frontmatter's `relationships:` block. Used as
/// the `serde_yaml` deserialisation target so the `Relationship` public type
/// stays free of YAML-specific concerns.
#[derive(Debug, Deserialize)]
struct FrontmatterRoot {
    #[serde(default)]
    relationships: Vec<Relationship>,
}

/// Parse the `relationships:` block from a BC's `README.md` YAML frontmatter.
///
/// Returns an empty `Vec` for any of the graceful-failure cases:
/// - README is missing
/// - README has no YAML frontmatter delimiters
/// - Frontmatter does not parse as YAML
/// - Frontmatter has no `relationships:` key
///
/// Each failure is logged at `warn` level **once per occurrence** (the watcher
/// re-invokes this on every README write, so a malformed frontmatter does not
/// spam the log on prose edits — the change-detection cache in the watcher
/// short-circuits on the deep-equal result).
///
/// Cross-project / unknown-BC `to` references are dropped from the returned
/// vector with their own warning log. `sibling_names` is the set of BC dirs in
/// the same project — relationships whose `to` is not in this set are
/// considered out of scope at v1.
///
/// **Validation:** directional relationship types (`customer-supplier`,
/// `anticorruption-layer`, `conformist`) require a `direction:` field;
/// non-directional types (`shared-kernel`, `partnership`) must not carry one.
/// Mismatched entries are dropped with a warning rather than silently
/// accepted, so a typo doesn't yield a half-broken edge on the canvas.
pub(crate) fn parse_relationships(
    bc_dir: &Path,
    bc_name: &str,
    sibling_names: &[String],
) -> Vec<Relationship> {
    let readme = bc_dir.join("README.md");
    let contents = match std::fs::read_to_string(&readme) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let frontmatter = match extract_frontmatter(&contents) {
        Some(fm) => fm,
        None => return Vec::new(),
    };

    let parsed: FrontmatterRoot = match serde_yaml::from_str(frontmatter) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                bc = bc_name,
                error = %e,
                path = %readme.display(),
                "parse_relationships: malformed frontmatter; treating as empty"
            );
            return Vec::new();
        }
    };

    let mut out = Vec::with_capacity(parsed.relationships.len());
    for rel in parsed.relationships {
        // Validate directionality shape — drop loud rather than silently
        // accept a mismatch.
        let directional = rel.r#type.is_directional();
        match (directional, rel.direction.is_some()) {
            (true, false) => {
                tracing::warn!(
                    bc = bc_name,
                    to = %rel.to,
                    "parse_relationships: directional type without direction; dropped"
                );
                continue;
            }
            (false, true) => {
                tracing::warn!(
                    bc = bc_name,
                    to = %rel.to,
                    "parse_relationships: non-directional type with a direction; dropped"
                );
                continue;
            }
            _ => {}
        }

        // Drop cross-project / unknown-BC references with a warning. At v1 the
        // canvas only draws intra-project edges (`canvas-007`).
        if !sibling_names.iter().any(|s| s == &rel.to) {
            tracing::warn!(
                bc = bc_name,
                to = %rel.to,
                "parse_relationships: `to` is not a sibling BC in this project; dropped"
            );
            continue;
        }

        out.push(rel);
    }

    out
}

/// Extract the body of a leading YAML frontmatter block from a markdown file.
/// The frontmatter convention: the file starts with a line containing only
/// `---`, followed by YAML, terminated by another line containing only `---`.
/// Returns `None` for any file that does not match this shape.
fn extract_frontmatter(contents: &str) -> Option<&str> {
    let mut lines = contents.lines();
    let first = lines.next()?;
    if first.trim() != "---" {
        return None;
    }
    let start = first.len() + contents[first.len()..].find('\n')? + 1;

    // Find the matching closing `---`. We scan the remainder line by line to
    // locate the byte offset.
    let rest = &contents[start..];
    let mut cursor = 0usize;
    for line in rest.lines() {
        if line.trim() == "---" {
            return Some(&rest[..cursor]);
        }
        // +1 for the newline we just consumed. The final line in the file
        // may not have one, but we'd not return through this branch for it.
        cursor += line.len();
        if cursor < rest.len() && rest.as_bytes()[cursor] == b'\n' {
            cursor += 1;
        }
    }
    None
}

/// Internal `serde_yaml` target for a task file's frontmatter. Every field is
/// optional so a partial or empty frontmatter still deserialises (graceful
/// degradation — `project-registry-005`); unknown keys (e.g. `status`,
/// `created`, `depends_on`, `related_adrs`) are ignored.
#[derive(Debug, Default, Deserialize)]
struct TaskFrontmatter {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default, rename = "type")]
    type_: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    blocked_question: Option<String>,
}

/// Read every `.md` task file in a bounded context's four state folders into
/// `Task` records (`project-registry-005`). Missing state folders contribute
/// nothing — a BC need not have all four. The returned vector is ordered by
/// column (backlog→done) then by id so the canvas does not reshuffle cards
/// between fetches.
///
/// Cost is O(tasks) frontmatter parses per snapshot; acceptable because
/// snapshots are produced on mount + resync only — the canvas patches in place
/// from fine-grained events on the hot path (`canvas-001`).
fn read_tasks(bc_dir: &Path) -> Vec<Task> {
    let mut tasks = Vec::new();
    for state in TASK_STATES {
        let column = match TaskColumn::from_state(state) {
            Some(c) => c,
            None => continue,
        };
        let dir = bc_dir.join(state);
        let read = match std::fs::read_dir(&dir) {
            Ok(r) => r,
            Err(_) => continue,
        };
        for entry in read.filter_map(Result::ok) {
            let path = entry.path();
            let is_md = path
                .extension()
                .map(|ext| ext.eq_ignore_ascii_case("md"))
                .unwrap_or(false);
            if !is_md {
                continue;
            }
            tasks.push(parse_task_file(&path, column));
        }
    }
    tasks.sort_by(|a, b| a.column.rank().cmp(&b.column.rank()).then_with(|| a.id.cmp(&b.id)));
    tasks
}

/// Parse one task file at `path` (already known to be in `column`) into a
/// `Task`. Graceful degradation (`project-registry-005`): a missing file, an
/// absent/unparseable frontmatter, or a frontmatter without an `id:` all
/// degrade to the filename stem as `id` with empty metadata — logged once at
/// `warn`, never panics. Reused by the watcher to fill `TaskAdded`/`TaskChanged`
/// payloads.
pub(crate) fn parse_task_file(path: &Path, column: TaskColumn) -> Task {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let fm = read_task_frontmatter(path);
    match fm {
        Some(fm) => Task {
            id: fm.id.filter(|s| !s.is_empty()).unwrap_or_else(|| stem.clone()),
            title: fm.title.unwrap_or_default(),
            column,
            type_: fm.type_.unwrap_or_default(),
            tags: fm.tags,
            blocked_question: fm.blocked_question.filter(|s| !s.is_empty()),
        },
        None => Task {
            id: stem,
            title: String::new(),
            column,
            type_: String::new(),
            tags: Vec::new(),
            blocked_question: None,
        },
    }
}

/// Read + parse a task file's YAML frontmatter. Returns `None` (logged once at
/// `warn`) for a missing file, a file without a leading `---` frontmatter
/// block, or frontmatter that does not parse as YAML — the caller falls back to
/// the filename stem in every such case.
fn read_task_frontmatter(path: &Path) -> Option<TaskFrontmatter> {
    let contents = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "parse_task_file: cannot read task file; degrading to filename stem");
            return None;
        }
    };
    let frontmatter = match extract_frontmatter(&contents) {
        Some(fm) => fm,
        None => {
            tracing::warn!(path = %path.display(), "parse_task_file: no YAML frontmatter; degrading to filename stem");
            return None;
        }
    };
    match serde_yaml::from_str::<TaskFrontmatter>(frontmatter) {
        Ok(fm) => Some(fm),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "parse_task_file: malformed frontmatter; degrading to filename stem");
            None
        }
    }
}

/// Derive a `TaskCounts` from the per-task records — `task_counts` is kept as a
/// convenience for count-only consumers but is no longer a second source of
/// truth (`project-registry-005`).
fn counts_from_tasks(tasks: &[Task]) -> TaskCounts {
    let mut counts = TaskCounts {
        backlog: 0,
        todo: 0,
        doing: 0,
        done: 0,
    };
    for task in tasks {
        match task.column {
            TaskColumn::Backlog => counts.backlog += 1,
            TaskColumn::Todo => counts.todo += 1,
            TaskColumn::Doing => counts.doing += 1,
            TaskColumn::Done => counts.done += 1,
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a throwaway Agentheim project tree in a unique temp directory.
    fn scratch_project() -> PathBuf {
        let mut dir = std::env::temp_dir();
        let unique = format!(
            "guppi-project-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        dir.push(unique);
        fs::create_dir_all(dir.join(".agentheim/contexts")).unwrap();
        dir
    }

    #[test]
    fn rejects_a_folder_without_dot_agentheim() {
        let dir = std::env::temp_dir().join(format!("guppi-not-a-project-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let err = get_project(1, &dir).unwrap_err();
        assert!(matches!(err, ProjectError::NotAnAgentheimProject(_)));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn reads_project_name_from_first_line_of_vision() {
        let dir = scratch_project();
        fs::write(
            dir.join(".agentheim/vision.md"),
            "# Vision: GUPPI\n\nThe rest of the vision.\n",
        )
        .unwrap();

        let snap = get_project(1, &dir).unwrap();
        assert_eq!(snap.name, "Vision: GUPPI");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_contexts_directory_is_valid() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Empty\n").unwrap();

        let snap = get_project(1, &dir).unwrap();
        assert!(snap.bcs.is_empty(), "no BCs is a valid snapshot");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn snapshot_carries_the_project_id_supplied_by_the_caller() {
        // `project-registry-001` / `canvas-002` coordination: the snapshot
        // must stamp the supplied id so it can flow with the data to the
        // frontend (the canvas keys per-project state on it).
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Stamping\n").unwrap();

        let snap = get_project(42, &dir).unwrap();
        assert_eq!(snap.id, 42);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn counts_task_files_per_state_per_bc() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Counting\n").unwrap();

        let bc = dir.join(".agentheim/contexts/infrastructure");
        for state in ["backlog", "todo", "doing", "done"] {
            fs::create_dir_all(bc.join(state)).unwrap();
        }
        fs::write(bc.join("backlog/a.md"), "x").unwrap();
        fs::write(bc.join("backlog/b.md"), "x").unwrap();
        fs::write(bc.join("doing/c.md"), "x").unwrap();
        fs::write(bc.join("done/d.md"), "x").unwrap();
        // A non-md file must not be counted.
        fs::write(bc.join("done/notes.txt"), "x").unwrap();

        let snap = get_project(1, &dir).unwrap();
        assert_eq!(snap.bcs.len(), 1);
        assert_eq!(
            snap.bcs[0].task_counts,
            TaskCounts {
                backlog: 2,
                todo: 0,
                doing: 1,
                done: 1,
            }
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn healthy_get_project_carries_missing_false() {
        // `project-registry-003`: every present-project snapshot must set
        // `missing: false` so the canvas's missing-tile visual (canvas-005a)
        // does not fire spuriously.
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Healthy\n").unwrap();
        let snap = get_project(1, &dir).unwrap();
        assert!(!snap.missing, "healthy project must have missing = false");
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn missing_snapshot_builds_a_missing_true_snapshot_for_a_registered_path() {
        // `project-registry-003`: when `.agentheim/` is gone the IPC layer
        // hands the canvas a synthetic snapshot instead of skipping the row.
        // The shape: missing = true, bcs empty, name = folder name, path
        // preserved.
        let dir = std::env::temp_dir().join(format!(
            "guppi-missing-snap-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // The folder need not exist for the helper to work — it operates on
        // the path string + registry id alone.
        let snap = missing_snapshot(123, &dir);
        assert!(snap.missing, "missing snapshot must have missing = true");
        assert_eq!(snap.id, 123);
        assert!(snap.bcs.is_empty(), "missing snapshot must have no BCs");
        // The folder name is reflected in `name`; the full path in `path`.
        let folder = dir.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(snap.name, folder);
        assert_eq!(snap.path, dir.to_string_lossy());
    }

    // -------- `project-registry-005`: per-task records ----------------------

    #[test]
    fn reads_individual_task_records_with_frontmatter_metadata() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Tasks\n").unwrap();
        let bc = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(bc.join("doing")).unwrap();
        fs::write(
            bc.join("doing/canvas-020-kanban.md"),
            "---\nid: canvas-020\ntitle: Kanban accordion interior\ntype: feature\ntags: [canvas, kanban]\nstatus: doing\n---\n# body\n",
        )
        .unwrap();

        let snap = get_project(1, &dir).unwrap();
        let canvas = snap.bcs.iter().find(|b| b.name == "canvas").unwrap();
        assert_eq!(canvas.tasks.len(), 1);
        let t = &canvas.tasks[0];
        assert_eq!(t.id, "canvas-020");
        assert_eq!(t.title, "Kanban accordion interior");
        assert_eq!(t.column, TaskColumn::Doing);
        assert_eq!(t.type_, "feature");
        assert_eq!(t.tags, vec!["canvas".to_string(), "kanban".to_string()]);
        assert_eq!(t.blocked_question, None);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn malformed_task_file_degrades_to_filename_stem_without_aborting_snapshot() {
        // Acceptance: ≥1 well-formed + ≥1 malformed task file in one BC. The
        // malformed file must yield a stem-id Task with empty metadata, and the
        // snapshot as a whole must still succeed.
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Degrade\n").unwrap();
        let bc = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(bc.join("backlog")).unwrap();
        // Well-formed.
        fs::write(
            bc.join("backlog/canvas-021-good.md"),
            "---\nid: canvas-021\ntitle: Good\ntype: feature\n---\nbody\n",
        )
        .unwrap();
        // Malformed YAML in the frontmatter block.
        fs::write(
            bc.join("backlog/canvas-022-broken.md"),
            "---\ntitle: [unterminated: : :\n---\nbody\n",
        )
        .unwrap();
        // No frontmatter at all.
        fs::write(bc.join("backlog/canvas-023-noyaml.md"), "just prose, no frontmatter\n").unwrap();

        let snap = get_project(1, &dir).unwrap();
        let canvas = snap.bcs.iter().find(|b| b.name == "canvas").unwrap();
        assert_eq!(canvas.tasks.len(), 3, "all three task files enumerate: {:?}", canvas.tasks);

        let broken = canvas.tasks.iter().find(|t| t.id == "canvas-022-broken").unwrap();
        assert_eq!(broken.title, "", "malformed frontmatter => empty title");
        assert_eq!(broken.type_, "");
        assert!(broken.tags.is_empty());

        let noyaml = canvas.tasks.iter().find(|t| t.id == "canvas-023-noyaml").unwrap();
        assert_eq!(noyaml.title, "");

        // The well-formed one still carries its id from frontmatter (not stem).
        assert!(canvas.tasks.iter().any(|t| t.id == "canvas-021" && t.title == "Good"));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn task_counts_stay_correct_when_derived_from_tasks() {
        // Regression: `task_counts` is now derived from `tasks` — it must still
        // equal the per-column file count, including a non-md file being
        // ignored.
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Counts\n").unwrap();
        let bc = dir.join(".agentheim/contexts/infrastructure");
        for state in ["backlog", "todo", "doing", "done"] {
            fs::create_dir_all(bc.join(state)).unwrap();
        }
        fs::write(bc.join("backlog/a.md"), "---\nid: a\n---\n").unwrap();
        fs::write(bc.join("backlog/b.md"), "---\nid: b\n---\n").unwrap();
        fs::write(bc.join("doing/c.md"), "---\nid: c\n---\n").unwrap();
        fs::write(bc.join("done/d.md"), "---\nid: d\n---\n").unwrap();
        fs::write(bc.join("done/notes.txt"), "x").unwrap();

        let snap = get_project(1, &dir).unwrap();
        let infra = snap.bcs.iter().find(|b| b.name == "infrastructure").unwrap();
        assert_eq!(
            infra.task_counts,
            TaskCounts { backlog: 2, todo: 0, doing: 1, done: 1 }
        );
        // And the derived count matches `tasks.len()`.
        assert_eq!(infra.tasks.len(), 4);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn tasks_are_ordered_by_column_then_id() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Order\n").unwrap();
        let bc = dir.join(".agentheim/contexts/canvas");
        for state in ["backlog", "doing"] {
            fs::create_dir_all(bc.join(state)).unwrap();
        }
        fs::write(bc.join("doing/z.md"), "---\nid: z\n---\n").unwrap();
        fs::write(bc.join("backlog/m.md"), "---\nid: m\n---\n").unwrap();
        fs::write(bc.join("backlog/a.md"), "---\nid: a\n---\n").unwrap();

        let snap = get_project(1, &dir).unwrap();
        let canvas = snap.bcs.iter().find(|b| b.name == "canvas").unwrap();
        let ids: Vec<&str> = canvas.tasks.iter().map(|t| t.id.as_str()).collect();
        // backlog (a, m) before doing (z).
        assert_eq!(ids, vec!["a", "m", "z"]);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn task_blocked_question_is_read_from_frontmatter_when_present() {
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Blocked\n").unwrap();
        let bc = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(bc.join("doing")).unwrap();
        fs::write(
            bc.join("doing/canvas-099.md"),
            "---\nid: canvas-099\ntitle: Blocked one\nblocked_question: \"Should the panel dock left or right?\"\n---\n",
        )
        .unwrap();

        let snap = get_project(1, &dir).unwrap();
        let canvas = snap.bcs.iter().find(|b| b.name == "canvas").unwrap();
        assert_eq!(
            canvas.tasks[0].blocked_question.as_deref(),
            Some("Should the panel dock left or right?")
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn moving_a_task_file_changes_the_count() {
        // This is the watcher's promise expressed as a pure-function test:
        // re-reading after a file move yields updated counts.
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# Move\n").unwrap();
        let bc = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(bc.join("backlog")).unwrap();
        fs::create_dir_all(bc.join("doing")).unwrap();
        fs::write(bc.join("backlog/x.md"), "x").unwrap();

        let before = get_project(1, &dir).unwrap();
        assert_eq!(before.bcs[0].task_counts.backlog, 1);
        assert_eq!(before.bcs[0].task_counts.doing, 0);

        fs::rename(bc.join("backlog/x.md"), bc.join("doing/x.md")).unwrap();

        let after = get_project(1, &dir).unwrap();
        assert_eq!(after.bcs[0].task_counts.backlog, 0);
        assert_eq!(after.bcs[0].task_counts.doing, 1);

        fs::remove_dir_all(&dir).ok();
    }

    // -------- `project-registry-004`: relationships frontmatter parser ------

    /// Helper: lay down a BC dir with a README containing the given frontmatter
    /// body. The body must not include the `---` delimiters; they are wrapped
    /// here for convenience.
    fn write_bc_readme(bc_dir: &Path, frontmatter_body: &str, prose: &str) {
        fs::create_dir_all(bc_dir).unwrap();
        let contents = format!("---\n{}\n---\n{}\n", frontmatter_body, prose);
        fs::write(bc_dir.join("README.md"), contents).unwrap();
    }

    #[test]
    fn parser_returns_empty_for_a_missing_readme() {
        // Graceful degradation case 1: no README at all => empty Vec, no panic.
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(&bc_dir).unwrap();
        // No README written.

        let rels = parse_relationships(&bc_dir, "canvas", &["project-registry".to_string()]);
        assert!(rels.is_empty(), "missing README must yield empty: {rels:?}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_returns_empty_for_a_readme_without_frontmatter() {
        // Graceful degradation case 2: README exists but has no `---` block.
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(&bc_dir).unwrap();
        fs::write(bc_dir.join("README.md"), "# canvas\n\nJust prose.\n").unwrap();

        let rels = parse_relationships(&bc_dir, "canvas", &[]);
        assert!(rels.is_empty(), "no frontmatter => empty: {rels:?}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_returns_empty_for_malformed_yaml() {
        // Graceful degradation case 3: frontmatter present but YAML is broken.
        // BC still enumerates (`get_project` succeeds), just with empty rels.
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        fs::create_dir_all(&bc_dir).unwrap();
        fs::write(
            bc_dir.join("README.md"),
            "---\nrelationships: [this is: : broken :]\n---\n# canvas\n",
        )
        .unwrap();

        let rels = parse_relationships(&bc_dir, "canvas", &["project-registry".to_string()]);
        assert!(rels.is_empty(), "malformed YAML => empty: {rels:?}");

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_extracts_directional_customer_supplier_relationship() {
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        write_bc_readme(
            &bc_dir,
            "name: canvas\nrelationships:\n  - to: project-registry\n    type: customer-supplier\n    direction: upstream",
            "# canvas\n",
        );

        let rels = parse_relationships(
            &bc_dir,
            "canvas",
            &["project-registry".to_string(), "canvas".to_string()],
        );
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].to, "project-registry");
        assert_eq!(rels[0].r#type, RelationshipType::CustomerSupplier);
        assert_eq!(rels[0].direction, Some(Direction::Upstream));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_extracts_non_directional_shared_kernel_relationship() {
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        write_bc_readme(
            &bc_dir,
            "relationships:\n  - to: infrastructure\n    type: shared-kernel",
            "# canvas\n",
        );

        let rels = parse_relationships(
            &bc_dir,
            "canvas",
            &["infrastructure".to_string(), "canvas".to_string()],
        );
        assert_eq!(rels.len(), 1);
        assert_eq!(rels[0].to, "infrastructure");
        assert_eq!(rels[0].r#type, RelationshipType::SharedKernel);
        assert_eq!(rels[0].direction, None);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_drops_directional_relationship_missing_a_direction() {
        // A `customer-supplier` without `direction:` is malformed — drop it
        // with a warning, do not silently invent a default.
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        write_bc_readme(
            &bc_dir,
            "relationships:\n  - to: project-registry\n    type: customer-supplier",
            "# canvas\n",
        );

        let rels = parse_relationships(
            &bc_dir,
            "canvas",
            &["project-registry".to_string(), "canvas".to_string()],
        );
        assert!(
            rels.is_empty(),
            "directional type without direction must be dropped: {rels:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_drops_non_directional_relationship_with_a_direction() {
        // A `shared-kernel` with `direction:` is malformed — drop with warning.
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        write_bc_readme(
            &bc_dir,
            "relationships:\n  - to: infrastructure\n    type: shared-kernel\n    direction: upstream",
            "# canvas\n",
        );

        let rels = parse_relationships(
            &bc_dir,
            "canvas",
            &["infrastructure".to_string(), "canvas".to_string()],
        );
        assert!(
            rels.is_empty(),
            "non-directional type with direction must be dropped: {rels:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_drops_cross_project_to_references() {
        // The `to` value must name a sibling BC in the same project. References
        // to outside-of-project things (e.g. external services like
        // "Whisperheim") are dropped at v1 — `canvas-007` only renders
        // intra-project edges.
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/voice");
        write_bc_readme(
            &bc_dir,
            "relationships:\n  - to: Whisperheim\n    type: anticorruption-layer\n    direction: upstream",
            "# voice\n",
        );

        // sibling_names does NOT include "Whisperheim".
        let rels = parse_relationships(&bc_dir, "voice", &["voice".to_string()]);
        assert!(
            rels.is_empty(),
            "cross-project `to` must be dropped at v1: {rels:?}"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn parser_handles_multiple_mixed_relationships() {
        let dir = scratch_project();
        let bc_dir = dir.join(".agentheim/contexts/canvas");
        write_bc_readme(
            &bc_dir,
            "name: canvas\n\
             classification: core\n\
             relationships:\n\
             \x20 - to: project-registry\n\
             \x20   type: customer-supplier\n\
             \x20   direction: upstream\n\
             \x20 - to: agent-awareness\n\
             \x20   type: customer-supplier\n\
             \x20   direction: upstream\n\
             \x20 - to: infrastructure\n\
             \x20   type: shared-kernel",
            "# canvas\n",
        );

        let siblings = vec![
            "canvas".to_string(),
            "project-registry".to_string(),
            "agent-awareness".to_string(),
            "infrastructure".to_string(),
        ];
        let rels = parse_relationships(&bc_dir, "canvas", &siblings);
        assert_eq!(rels.len(), 3, "expected three relationships: {rels:?}");
        assert!(rels.iter().any(|r| r.to == "project-registry"
            && r.r#type == RelationshipType::CustomerSupplier
            && r.direction == Some(Direction::Upstream)));
        assert!(rels.iter().any(|r| r.to == "agent-awareness"
            && r.r#type == RelationshipType::CustomerSupplier
            && r.direction == Some(Direction::Upstream)));
        assert!(rels.iter().any(
            |r| r.to == "infrastructure" && r.r#type == RelationshipType::SharedKernel
        ));

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn get_project_surfaces_relationships_on_each_bc() {
        // End-to-end through the public `get_project`: parsing fires at
        // enumeration, and the snapshot's `bcs[…].relationships` is populated.
        let dir = scratch_project();
        fs::write(dir.join(".agentheim/vision.md"), "# E2E\n").unwrap();

        let canvas_dir = dir.join(".agentheim/contexts/canvas");
        write_bc_readme(
            &canvas_dir,
            "relationships:\n  - to: project-registry\n    type: customer-supplier\n    direction: upstream",
            "# canvas\n",
        );
        // A second BC with no relationships.
        let registry_dir = dir.join(".agentheim/contexts/project-registry");
        fs::create_dir_all(&registry_dir).unwrap();
        fs::write(registry_dir.join("README.md"), "# project-registry\n").unwrap();

        let snap = get_project(1, &dir).unwrap();
        let canvas_bc = snap.bcs.iter().find(|b| b.name == "canvas").unwrap();
        assert_eq!(canvas_bc.relationships.len(), 1);
        assert_eq!(canvas_bc.relationships[0].to, "project-registry");
        let registry_bc = snap
            .bcs
            .iter()
            .find(|b| b.name == "project-registry")
            .unwrap();
        assert!(
            registry_bc.relationships.is_empty(),
            "no frontmatter => empty"
        );

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn frontmatter_extractor_handles_no_leading_dashes() {
        // The frontmatter must start at the very first line. A README that
        // begins with prose and contains `---` later is not a frontmatter block.
        assert_eq!(extract_frontmatter("# heading\nrelationships:\n---\n"), None);
    }

    #[test]
    fn frontmatter_extractor_handles_unterminated_block() {
        // Opening `---` without a matching closing `---` is malformed; we yield
        // None rather than parsing everything-to-EOF as YAML.
        assert_eq!(extract_frontmatter("---\nrelationships:\n"), None);
    }
}
