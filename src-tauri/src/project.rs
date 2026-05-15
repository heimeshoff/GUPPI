//! Reading an Agentheim project off disk into a `ProjectSnapshot`.
//!
//! The Agentheim layout this understands:
//!
//! ```text
//! <project>/.agentheim/
//!   vision.md                         first line -> project name
//!   contexts/<bc>/{backlog,todo,doing,done}/*.md   -> task counts
//! ```
//!
//! Project discovery (ADR-005) is an explicit-registry concern; the walking
//! skeleton has exactly one hard-coded project, so this module only needs the
//! "read one known project" half — listing `contexts/*` and counting task
//! files. The registry/scan affordances are out of skeleton scope.

use serde::Serialize;
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

/// One bounded context as the canvas needs to draw it.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BcSnapshot {
    pub name: String,
    pub task_counts: TaskCounts,
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
    pub bcs: Vec<BcSnapshot>,
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
fn read_bounded_contexts(agentheim: &Path) -> Result<Vec<BcSnapshot>, ProjectError> {
    let contexts_dir = agentheim.join("contexts");
    if !contexts_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut bcs = Vec::new();
    for entry in std::fs::read_dir(&contexts_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let task_counts = count_tasks(&entry.path());
        bcs.push(BcSnapshot { name, task_counts });
    }

    // Stable ordering so the canvas does not reshuffle BC nodes between fetches.
    bcs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(bcs)
}

/// Count `.md` task files in each of a bounded context's four state folders.
/// Missing state folders count as zero — a BC need not have all four.
fn count_tasks(bc_dir: &Path) -> TaskCounts {
    let count_in = |state: &str| -> u32 {
        let dir = bc_dir.join(state);
        match std::fs::read_dir(&dir) {
            Ok(entries) => entries
                .filter_map(Result::ok)
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext.eq_ignore_ascii_case("md"))
                        .unwrap_or(false)
                })
                .count() as u32,
            Err(_) => 0,
        }
    };

    TaskCounts {
        backlog: count_in(TASK_STATES[0]),
        todo: count_in(TASK_STATES[1]),
        doing: count_in(TASK_STATES[2]),
        done: count_in(TASK_STATES[3]),
    }
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
}
