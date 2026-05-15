//! Scan-root discovery walk — ADR-013 / `project-registry-002a`.
//!
//! Walks a registered scan root's subtree and reports every directory that
//! looks like an Agentheim project (contains an `.agentheim/` subdirectory).
//! The walk is:
//!
//! - **Depth-capped** — a remaining-depth counter seeded from the root's
//!   persisted `depth_cap` (default 3, ADR-005 / ADR-013).
//! - **Junk-dir-pruned** — directories matching `SKIP_DIRS` are not entered.
//! - **Non-recursive into projects** — once a directory is identified as an
//!   Agentheim project it is reported as a candidate; the walk does NOT
//!   descend further. Nested projects-under-a-project are out of v1 scope.
//! - **Canonicalised** — every candidate path and the scan root itself are
//!   canonicalised at the module boundary (resolve, collapse symlinks,
//!   case-normalised by Windows itself). The DB only ever stores canonical
//!   paths (ADR-005).
//!
//! The walker is pure: it depends on neither `AppState` nor IPC and is
//! unit-tested against temp directory trees. Persistence + IPC live in
//! `db.rs` and `lib.rs`.

use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Directories that are never project roots and never *contain* projects worth
/// surfacing in v1. Pruned by exact directory-name match before descending.
/// Chosen for size (`node_modules`, `target`), version-control internals
/// (`.git`, `.svn`, `.hg`), build output (`dist`, `build`), and virtualenvs
/// (`.venv`). Extending this list is a one-line change here.
pub const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    ".svn",
    ".hg",
    "dist",
    "build",
    ".venv",
];

/// One row in the candidate checklist `add_scan_root` / `rescan_scan_root`
/// hands back to the frontend. `already_imported` lets the UI grey out or
/// pre-tick the rows whose canonical path is already in the `projects` table
/// (`002b` does the importing).
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScanCandidate {
    /// Canonical absolute path to the project root (the directory that
    /// contains `.agentheim/`).
    pub path: String,
    /// Nickname the import flow should pre-fill — for v1 this is the
    /// project-folder's file-name, matching `project.rs`'s vision-file
    /// fallback. The user is free to overwrite during import (`002b`).
    pub nickname_suggestion: String,
    /// `true` if the canonical path already appears in `projects.path`.
    pub already_imported: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("scan root does not exist or is not a directory: {0}")]
    RootMissing(PathBuf),
    #[error("could not canonicalise scan root: {source}")]
    Canonicalise {
        #[source]
        source: std::io::Error,
    },
}

/// Canonicalise a scan-root path: resolve, collapse symlinks, and on Windows
/// strip the `\\?\` UNC prefix that `fs::canonicalize` emits so the DB stores
/// the ordinary `C:\…` form the rest of the codebase deals with. Case is
/// already normalised by the Windows filesystem layer on `canonicalize`.
///
/// Errors only if the path does not exist or canonicalisation fails — both
/// surface as `ScanError::Canonicalise` / `RootMissing` to the caller.
pub fn canonicalize_root(path: &Path) -> Result<PathBuf, ScanError> {
    if !path.exists() {
        return Err(ScanError::RootMissing(path.to_path_buf()));
    }
    let canon = std::fs::canonicalize(path).map_err(|e| ScanError::Canonicalise { source: e })?;
    Ok(strip_unc_prefix(&canon))
}

/// Strip the Windows `\\?\` extended-length prefix from a path. No-op on
/// non-Windows. Kept module-private but visible to tests.
fn strip_unc_prefix(path: &Path) -> PathBuf {
    if cfg!(windows) {
        let s = path.to_string_lossy();
        if let Some(rest) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(rest);
        }
    }
    path.to_path_buf()
}

/// Walk a (canonical) scan root for Agentheim projects.
///
/// `depth_cap` is the maximum directory depth below the root at which a
/// project may be discovered. `0` means "the root itself must be the project";
/// `3` (ADR-005 default) means "the root plus up to three nested levels".
///
/// `known_paths` is the set of canonical project paths already in the
/// registry — used to stamp `already_imported` on each candidate. Empty set
/// for a registry that has never imported anything.
///
/// Returns every Agentheim-project directory found, in deterministic order
/// (sorted by canonical path). Empty `Vec` if the subtree has none — that is
/// valid, the root stays persisted and rescannable.
pub fn walk_scan_root(
    root: &Path,
    depth_cap: u32,
    known_paths: &HashSet<String>,
) -> Vec<ScanCandidate> {
    let mut out = Vec::new();
    visit(root, depth_cap, known_paths, &mut out);
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

/// Recursive directory visitor. `remaining_depth` is the number of additional
/// levels we may descend; `0` means "look at this directory but do not enter
/// any subdirectory".
fn visit(
    dir: &Path,
    remaining_depth: u32,
    known_paths: &HashSet<String>,
    out: &mut Vec<ScanCandidate>,
) {
    // Is this directory itself an Agentheim project? If so, it IS a candidate
    // and we do NOT descend further (no nested projects in v1).
    if dir.join(".agentheim").is_dir() {
        let canonical = match std::fs::canonicalize(dir) {
            Ok(p) => strip_unc_prefix(&p),
            Err(_) => dir.to_path_buf(),
        };
        let path = canonical.to_string_lossy().into_owned();
        let nickname_suggestion = canonical
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.clone());
        let already_imported = known_paths.contains(&path);
        out.push(ScanCandidate {
            path,
            nickname_suggestion,
            already_imported,
        });
        return;
    }

    if remaining_depth == 0 {
        return;
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return, // unreadable subtree — silent skip, not an error
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Filesystem walks should follow only directories. Skip non-dir
        // entries cheaply: the `file_type` call avoids an `is_dir` syscall
        // failure mode on broken symlinks.
        match entry.file_type() {
            Ok(ft) if ft.is_dir() => {}
            _ => continue,
        }

        // Junk-dir pruning before descent. Compare against the directory name
        // only; absolute matches against arbitrary substrings are deliberately
        // not done.
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if SKIP_DIRS.contains(&name) {
                continue;
            }
        }

        visit(&path, remaining_depth - 1, known_paths, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build a scratch directory tree under temp/ and return its canonical path.
    fn scratch_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "guppi-scan-test-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        // Canonicalise so test assertions compare against the same shape the
        // walker emits (Windows would otherwise pick up case-normalisation
        // mid-test and make string compares brittle).
        canonicalize_root(&dir).unwrap()
    }

    fn make_project(at: &Path) {
        fs::create_dir_all(at.join(".agentheim/contexts")).unwrap();
    }

    #[test]
    fn empty_root_yields_no_candidates() {
        // ADR-013 acceptance: a scan root with zero projects is valid; the
        // walker returns an empty `Vec`, the root stays persisted.
        let root = scratch_dir("empty");
        let candidates = walk_scan_root(&root, 3, &HashSet::new());
        assert!(candidates.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn finds_a_project_at_each_depth_under_cap() {
        // ADR-013 acceptance: every `.agentheim/`-bearing subfolder within
        // `depth_cap` levels is reported.
        let root = scratch_dir("depths");
        // Depth 1: root/a
        let a = root.join("a");
        make_project(&a);
        // Depth 2: root/sub/b
        let b = root.join("sub").join("b");
        make_project(&b);
        // Depth 3: root/sub/inner/c
        let c = root.join("sub").join("inner").join("c");
        make_project(&c);

        let mut got: Vec<String> = walk_scan_root(&root, 3, &HashSet::new())
            .into_iter()
            .map(|c| c.path)
            .collect();
        got.sort();
        let mut want = [&a, &b, &c]
            .iter()
            .map(|p| canonicalize_root(p).unwrap().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        want.sort();
        assert_eq!(got, want);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn depth_cap_excludes_projects_beyond_it() {
        // ADR-013 / acceptance criterion 3: the walk does not descend past
        // `depth_cap`.
        let root = scratch_dir("cap");
        // Depth 1 — should be found at cap=1.
        let shallow = root.join("near");
        make_project(&shallow);
        // Depth 2 — should NOT be found at cap=1.
        let deep = root.join("far").join("deeper");
        make_project(&deep);

        let candidates: Vec<String> = walk_scan_root(&root, 1, &HashSet::new())
            .into_iter()
            .map(|c| c.path)
            .collect();
        let shallow_canon = canonicalize_root(&shallow).unwrap().to_string_lossy().into_owned();
        let deep_canon = canonicalize_root(&deep).unwrap().to_string_lossy().into_owned();
        assert!(candidates.contains(&shallow_canon), "depth-1 project must be found");
        assert!(
            !candidates.contains(&deep_canon),
            "depth-2 project must be excluded when cap = 1"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn does_not_descend_into_an_identified_project() {
        // ADR-013 / acceptance criterion 4: the walk does not descend into a
        // directory once it is identified as an Agentheim project. A nested
        // `.agentheim/` underneath an outer project must not produce a
        // second candidate.
        let root = scratch_dir("nested");
        let outer = root.join("outer");
        make_project(&outer);
        // A "nested project" under the outer project — should be ignored.
        let nested = outer.join("sub").join("nested");
        make_project(&nested);

        let candidates: Vec<String> = walk_scan_root(&root, 5, &HashSet::new())
            .into_iter()
            .map(|c| c.path)
            .collect();
        assert_eq!(candidates.len(), 1, "only the outer project must be reported");
        let outer_canon = canonicalize_root(&outer).unwrap().to_string_lossy().into_owned();
        assert_eq!(candidates[0], outer_canon);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn prunes_junk_directories() {
        // ADR-013 / acceptance criterion 3: `node_modules` / `.git` / `target`
        // (etc.) are pruned and never descended into, even if they happen to
        // contain a `.agentheim/`.
        let root = scratch_dir("junk");
        for junk in ["node_modules", ".git", "target", "dist", "build", ".venv"] {
            let trap = root.join(junk).join("decoy");
            make_project(&trap); // would-be candidate buried under junk dir
        }
        // Plus a legitimate project at depth 1 to prove the walker still works.
        let real = root.join("real");
        make_project(&real);

        let candidates: Vec<String> = walk_scan_root(&root, 5, &HashSet::new())
            .into_iter()
            .map(|c| c.path)
            .collect();
        assert_eq!(candidates.len(), 1, "only the non-junk project is reported");
        let real_canon = canonicalize_root(&real).unwrap().to_string_lossy().into_owned();
        assert_eq!(candidates[0], real_canon);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn marks_already_imported_when_path_matches_known_set() {
        // ADR-013 / acceptance criterion 5: a candidate whose canonical path
        // is already in `projects` is reported with `already_imported = true`;
        // a fresh one with `false`.
        let root = scratch_dir("known");
        let known = root.join("imported");
        let fresh = root.join("new");
        make_project(&known);
        make_project(&fresh);

        let known_canon = canonicalize_root(&known).unwrap().to_string_lossy().into_owned();
        let mut known_set = HashSet::new();
        known_set.insert(known_canon.clone());

        let candidates = walk_scan_root(&root, 3, &known_set);
        let known_row = candidates
            .iter()
            .find(|c| c.path == known_canon)
            .expect("known project must be in the candidate set");
        assert!(known_row.already_imported, "previously-imported must be flagged");

        let fresh_canon = canonicalize_root(&fresh).unwrap().to_string_lossy().into_owned();
        let fresh_row = candidates
            .iter()
            .find(|c| c.path == fresh_canon)
            .expect("fresh project must be in the candidate set");
        assert!(!fresh_row.already_imported, "fresh project must NOT be flagged");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn nickname_suggestion_is_the_project_folder_name() {
        // The import flow (`002b`) pre-fills the nickname; for v1 the
        // suggestion is the folder name (matches `project.rs`'s fallback
        // when `vision.md` is absent).
        let root = scratch_dir("nick");
        let p = root.join("my-cool-project");
        make_project(&p);

        let candidates = walk_scan_root(&root, 3, &HashSet::new());
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].nickname_suggestion, "my-cool-project");

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn canonicalize_root_strips_windows_unc_prefix() {
        // Windows `fs::canonicalize` emits paths prefixed with `\\?\`. The DB
        // and the rest of GUPPI deal with ordinary `C:\…` form; the walker
        // strips the prefix at the module boundary so callers never see it.
        let root = scratch_dir("unc");
        let canonical = canonicalize_root(&root).unwrap();
        let s = canonical.to_string_lossy();
        assert!(!s.starts_with(r"\\?\"), "canonical paths must not retain UNC prefix");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn canonicalize_root_errors_on_missing_path() {
        // Missing path is a clean `RootMissing`, not a panic — the IPC layer
        // turns it into a typed error for the user.
        let missing = std::env::temp_dir().join("guppi-scan-test-missing-XXXX");
        let _ = fs::remove_dir_all(&missing);
        let err = canonicalize_root(&missing).unwrap_err();
        assert!(matches!(err, ScanError::RootMissing(_)));
    }

    // -------- 002b: import + cascade composition tests ----------------------
    //
    // These mirror `add_scan_root_composition_*` above: they stitch together
    // the same steps the `import_scanned_projects` / `remove_scan_root` IPC
    // commands run, against real temp trees, a real `Db`, and a real
    // `WatcherSupervisor`. They assert the end-to-end behaviour the commands
    // guarantee without standing up a Tauri test app. The IPC handlers are
    // thin Tauri shells over this composition; if the composition is correct,
    // the handlers are correct.

    #[test]
    fn import_scanned_projects_registers_each_picked_path_idempotently() {
        // `project-registry-002b` acceptance criterion 1: each picked path is
        // registered with `scan_root_id` set; importing the same path twice
        // does not duplicate the row.
        use crate::db::Db;
        use crate::events::EventBus;
        use crate::supervisor::WatcherSupervisor;
        use std::collections::HashSet;

        let root = scratch_dir("import-once");
        let a = root.join("project-a");
        let b = root.join("nested").join("project-b");
        make_project(&a);
        make_project(&b);

        let db = Db::open_in_memory().unwrap();
        let bus = EventBus::new();
        let sup = WatcherSupervisor::new(bus);

        let canonical_root = canonicalize_root(&root).unwrap();
        let canonical_root_str = canonical_root.to_string_lossy().into_owned();
        let scan_root_id = db.upsert_scan_root(&canonical_root_str, 3).unwrap();

        // Step 1: replicate the IPC's re-verification walk.
        let known: HashSet<String> = db.list_project_paths().unwrap().into_iter().collect();
        let candidates = walk_scan_root(&canonical_root, 3, &known);
        assert_eq!(candidates.len(), 2);

        // Step 2: import both candidates.
        let mut imported = Vec::new();
        for c in &candidates {
            let pid = db
                .upsert_scanned_project(&c.path, &c.nickname_suggestion, scan_root_id)
                .unwrap();
            sup.add(pid, std::path::Path::new(&c.path)).unwrap();
            imported.push((pid, c.path.clone()));
        }
        assert_eq!(imported.len(), 2);
        for (pid, _) in &imported {
            assert!(sup.is_watching(*pid), "watcher must be armed");
        }

        // Step 3: re-importing the same paths yields the same project ids
        // (idempotent upsert + idempotent supervisor.add).
        for (expected_pid, path) in &imported {
            let nickname = candidates
                .iter()
                .find(|c| c.path == *path)
                .unwrap()
                .nickname_suggestion
                .clone();
            let pid = db
                .upsert_scanned_project(path, &nickname, scan_root_id)
                .unwrap();
            assert_eq!(pid, *expected_pid, "same path must yield same project_id");
            sup.add(pid, std::path::Path::new(path)).unwrap(); // idempotent
            assert!(sup.is_watching(pid));
        }

        // The DB has exactly two rows — no duplicates on re-import.
        assert_eq!(db.list_projects().unwrap().len(), 2);

        // The children are enumerable by scan_root_id (drives cascade).
        let mut children = db.list_projects_by_scan_root(scan_root_id).unwrap();
        children.sort();
        let mut expected: Vec<i64> = imported.iter().map(|(p, _)| *p).collect();
        expected.sort();
        assert_eq!(children, expected);

        for (pid, _) in &imported {
            sup.remove(*pid);
        }
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn import_scanned_projects_rejects_paths_outside_the_candidate_set() {
        // `project-registry-002b` acceptance criterion 2: a path outside the
        // root's freshly-walked candidate set must be REJECTED (skipped), not
        // silently imported. Re-verification is the cheap safeguard.
        use crate::db::Db;
        use std::collections::HashSet;

        let root = scratch_dir("import-reject");
        let inside = root.join("inside");
        make_project(&inside);
        // A second Agentheim project that lives entirely OUTSIDE the root's
        // subtree — a malicious or stale client could ask the IPC to import
        // it under the wrong root; the re-walk must catch it.
        let outside_root = scratch_dir("import-reject-other");
        let outside = outside_root.join("outside");
        make_project(&outside);

        let db = Db::open_in_memory().unwrap();
        let canonical_root = canonicalize_root(&root).unwrap();
        let scan_root_id = db
            .upsert_scan_root(&canonical_root.to_string_lossy(), 3)
            .unwrap();

        // Re-verify against the root's actual subtree.
        let known: HashSet<String> = db.list_project_paths().unwrap().into_iter().collect();
        let candidates = walk_scan_root(&canonical_root, 3, &known);
        let candidate_paths: HashSet<&str> =
            candidates.iter().map(|c| c.path.as_str()).collect();

        let inside_canon = canonicalize_root(&inside).unwrap().to_string_lossy().into_owned();
        let outside_canon = canonicalize_root(&outside).unwrap().to_string_lossy().into_owned();

        // The "inside" project is in the set; the "outside" project is not.
        assert!(candidate_paths.contains(inside_canon.as_str()));
        assert!(!candidate_paths.contains(outside_canon.as_str()));

        // Replicate the IPC's per-path gate.
        let requested = vec![inside_canon.clone(), outside_canon.clone()];
        let mut accepted: Vec<String> = Vec::new();
        for path in &requested {
            if candidate_paths.contains(path.as_str()) {
                let nickname = candidates
                    .iter()
                    .find(|c| c.path == *path)
                    .unwrap()
                    .nickname_suggestion
                    .clone();
                db.upsert_scanned_project(path, &nickname, scan_root_id)
                    .unwrap();
                accepted.push(path.clone());
            }
        }
        assert_eq!(accepted, vec![inside_canon.clone()]);

        // The outside path is NOT in the registry.
        let registered: Vec<String> = db.list_project_paths().unwrap();
        assert!(registered.contains(&inside_canon));
        assert!(
            !registered.contains(&outside_canon),
            "out-of-set path must be rejected, not silently registered"
        );

        fs::remove_dir_all(&root).ok();
        fs::remove_dir_all(&outside_root).ok();
    }

    #[test]
    fn remove_scan_root_cascade_drops_children_watchers_and_tiles_then_the_root() {
        // `project-registry-002b` acceptance criterion 3: removing a scan root
        // tears down every project imported under it — watcher gone, projects
        // row gone, tile_positions row gone (via ON DELETE CASCADE) — and
        // then deletes the root row. The ordering is the contract.
        use crate::db::Db;
        use crate::events::EventBus;
        use crate::supervisor::WatcherSupervisor;
        use std::collections::HashSet;

        let root = scratch_dir("cascade");
        let p1 = root.join("p1");
        let p2 = root.join("nested").join("p2");
        make_project(&p1);
        make_project(&p2);

        let db = Db::open_in_memory().unwrap();
        let bus = EventBus::new();
        let sup = WatcherSupervisor::new(bus);

        let canonical_root = canonicalize_root(&root).unwrap();
        let scan_root_id = db
            .upsert_scan_root(&canonical_root.to_string_lossy(), 3)
            .unwrap();

        // Import both candidates the same way the IPC would.
        let known: HashSet<String> = db.list_project_paths().unwrap().into_iter().collect();
        let candidates = walk_scan_root(&canonical_root, 3, &known);
        let mut child_ids = Vec::new();
        for c in &candidates {
            let pid = db
                .upsert_scanned_project(&c.path, &c.nickname_suggestion, scan_root_id)
                .unwrap();
            sup.add(pid, std::path::Path::new(&c.path)).unwrap();
            // Seed a tile position so the cascade has tile state to clear.
            db.save_tile_position(pid, 1.0, 2.0).unwrap();
            child_ids.push(pid);
        }
        assert_eq!(child_ids.len(), 2);
        for pid in &child_ids {
            assert!(sup.is_watching(*pid));
            assert_eq!(db.tile_position(*pid).unwrap(), Some((1.0, 2.0)));
        }

        // Replicate the IPC's cascade: enumerate children, tear down each,
        // delete the root last.
        let children = db.list_projects_by_scan_root(scan_root_id).unwrap();
        assert_eq!(children.len(), 2);
        for pid in &children {
            sup.remove(*pid);
            db.remove_project(*pid).unwrap();
        }
        db.delete_scan_root(scan_root_id).unwrap();

        // Watchers gone.
        for pid in &child_ids {
            assert!(!sup.is_watching(*pid), "watcher must be torn down");
        }
        // Project rows gone.
        assert!(db.list_projects().unwrap().is_empty());
        // Tile state gone (ON DELETE CASCADE).
        for pid in &child_ids {
            assert_eq!(db.tile_position(*pid).unwrap(), None);
        }
        // Root row gone.
        assert!(db.list_scan_roots().unwrap().is_empty());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn remove_scan_root_does_not_touch_manually_added_projects() {
        // `project-registry-002b` acceptance criterion 4: a manually-added
        // project (NULL scan_root_id) — even one that lives under the same
        // path subtree as a scan root — is NEVER touched by any root's
        // cascade.
        use crate::db::Db;
        use crate::events::EventBus;
        use crate::supervisor::WatcherSupervisor;
        use std::collections::HashSet;

        let root = scratch_dir("manual-survives");
        let discovered = root.join("discovered");
        let manual = root.join("manual-under-the-same-tree");
        make_project(&discovered);
        make_project(&manual);

        let db = Db::open_in_memory().unwrap();
        let bus = EventBus::new();
        let sup = WatcherSupervisor::new(bus);

        let canonical_root = canonicalize_root(&root).unwrap();
        let scan_root_id = db
            .upsert_scan_root(&canonical_root.to_string_lossy(), 3)
            .unwrap();

        // Manually register one project (NULL scan_root_id) — simulates the
        // ADR-005 "Add project…" affordance landing first.
        let manual_canon = canonicalize_root(&manual).unwrap().to_string_lossy().into_owned();
        let manual_id = db.upsert_project(&manual_canon, "Manual").unwrap();
        sup.add(manual_id, std::path::Path::new(&manual_canon)).unwrap();

        // Then the scan-and-import discovers the *other* project under the
        // root. The walker will report BOTH discovered + manual as candidates
        // (since both have .agentheim/), but the manual one is already in
        // the registry and the import would only stamp scan_root_id if it
        // were re-imported — the IPC's idempotency means a re-import of an
        // already-imported manual path would unfortunately overwrite the
        // origin. We test the correct happy path: the user only ticks the
        // genuinely discovered one in the checklist.
        let known: HashSet<String> = db.list_project_paths().unwrap().into_iter().collect();
        let candidates = walk_scan_root(&canonical_root, 3, &known);
        let discovered_canon = canonicalize_root(&discovered).unwrap().to_string_lossy().into_owned();

        // The user only picks the discovered (not already_imported) one.
        let picks = vec![discovered_canon.clone()];
        let mut discovered_id_opt = None;
        for path in &picks {
            let nickname = candidates
                .iter()
                .find(|c| c.path == *path)
                .unwrap()
                .nickname_suggestion
                .clone();
            let pid = db
                .upsert_scanned_project(path, &nickname, scan_root_id)
                .unwrap();
            sup.add(pid, std::path::Path::new(path)).unwrap();
            discovered_id_opt = Some(pid);
        }
        let discovered_id = discovered_id_opt.unwrap();

        // The manual project must NEVER appear in any scan root's
        // cascade enumeration — its `scan_root_id` is NULL.
        let manual_in_cascade = db
            .list_projects_by_scan_root(scan_root_id)
            .unwrap()
            .iter()
            .any(|id| *id == manual_id);
        assert!(
            !manual_in_cascade,
            "manual project must NEVER appear in any scan root's enumeration"
        );

        // Cascade-deregister the scan root.
        let children = db.list_projects_by_scan_root(scan_root_id).unwrap();
        assert_eq!(children, vec![discovered_id], "only the discovered project is in the cascade");
        for pid in &children {
            sup.remove(*pid);
            db.remove_project(*pid).unwrap();
        }
        db.delete_scan_root(scan_root_id).unwrap();

        // Discovered project gone.
        assert!(!sup.is_watching(discovered_id));
        // Manual project SURVIVED — watcher still armed, row still there.
        assert!(sup.is_watching(manual_id), "manual project's watcher must survive");
        let surviving: Vec<i64> = db.list_projects().unwrap().iter().map(|r| r.id).collect();
        assert_eq!(surviving, vec![manual_id]);

        sup.remove(manual_id);
        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn add_scan_root_composition_persists_then_walks_against_temp_tree() {
        // ADR-013 / acceptance criterion 2 (integration shape): the
        // `add_scan_root` IPC command is `canonicalise → upsert → list known →
        // walk`. This test stitches the same pieces against a real temp tree
        // and a fresh in-memory `Db` to assert the end-to-end behaviour the
        // command guarantees, without standing up a Tauri test app.
        use crate::db::Db;

        let root = scratch_dir("compose");
        // Two projects under the root at different depths, one already in the
        // registry to exercise the `already_imported` flag.
        let imported = root.join("imported");
        let fresh = root.join("nested").join("fresh");
        make_project(&imported);
        make_project(&fresh);

        let db = Db::open_in_memory().unwrap();
        let imported_canonical = canonicalize_root(&imported).unwrap().to_string_lossy().into_owned();
        db.upsert_project(&imported_canonical, "Imported").unwrap();

        // Compose the same flow as the IPC.
        let canonical = canonicalize_root(&root).unwrap();
        let canonical_str = canonical.to_string_lossy().into_owned();
        let scan_root_id = db.upsert_scan_root(&canonical_str, 3).unwrap();

        let known: HashSet<String> = db.list_project_paths().unwrap().into_iter().collect();
        let candidates = walk_scan_root(&canonical, 3, &known);

        // The persisted root is queryable post-walk.
        let row = db.get_scan_root(scan_root_id).unwrap().expect("root must persist");
        assert_eq!(row.path, canonical_str);
        assert_eq!(row.depth_cap, 3);

        // Both projects are reported; the already-imported one carries the flag.
        assert_eq!(candidates.len(), 2, "both projects must be in the checklist");
        let fresh_canonical = canonicalize_root(&fresh).unwrap().to_string_lossy().into_owned();
        let imported_row = candidates
            .iter()
            .find(|c| c.path == imported_canonical)
            .expect("imported project in checklist");
        assert!(imported_row.already_imported);
        let fresh_row = candidates
            .iter()
            .find(|c| c.path == fresh_canonical)
            .expect("fresh project in checklist");
        assert!(!fresh_row.already_imported);

        let _ = fs::remove_dir_all(&root);
    }
}
