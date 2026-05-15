//! GUPPI's own state — SQLite in the OS user-config directory (ADR-004).
//!
//! This is *Marco's view of the world* — registered projects, tile positions,
//! camera — never project content. The DB file lives at
//! `%APPDATA%\guppi\guppi.db` on Windows; the path is resolved at runtime by
//! the caller via Tauri's `path` API, not hard-coded here, so the same code
//! works on every target if macOS/Linux are ever validated.
//!
//! Migrations are ordered, versioned steps applied at startup, tracked by the
//! single-row `schema_version` table.

use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// One row of the `projects` table — the registry's view of a project
/// (ADR-005). The shape the multi-project snapshot model
/// (`project-registry-001`) reads back when serving `list_projects()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: i64,
    pub path: String,
    pub nickname: String,
}

/// One row of the `scan_roots` table — a folder the user has registered as a
/// rescannable parent for project discovery (ADR-013). `Serialize` so the
/// row crosses IPC unchanged for the `list_scan_roots` command.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ScanRootRow {
    pub id: i64,
    pub path: String,
    pub depth_cap: u32,
    pub added_at: String,
}

/// ADR-013 / ADR-005 default depth cap when none is supplied by the caller.
pub const DEFAULT_SCAN_DEPTH_CAP: u32 = 3;

/// The schema version this build expects. Bump it and add a migration step in
/// `migrate` whenever the schema changes.
///
/// v2 (ADR-013, `project-registry-002a`): adds the `scan_roots` table and a
/// nullable `projects.scan_root_id` column referencing it with
/// `ON DELETE RESTRICT` — the cascade-deregister must be app-driven
/// (`002b`), and `RESTRICT` makes that ordering a checked invariant.
pub const CURRENT_SCHEMA_VERSION: i64 = 2;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
}

/// A thread-safe handle to the GUPPI state database. Tauri shares this as
/// managed state; the `Mutex` is fine for GUPPI's workload (one user, low
/// write rate) and keeps `rusqlite`'s single-connection model honest.
pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    /// Open (creating if absent) the database at `path`, applying any pending
    /// migrations. The parent directory must already exist.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        Self::from_connection(conn)
    }

    /// Open an in-memory database — used by tests.
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self, DbError> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, DbError> {
        conn.pragma_update(None, "foreign_keys", true)?;
        migrate(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// The schema version currently recorded in the database. Exercised by
    /// the migration tests; kept on the public API as the migration-discipline
    /// check a future maintainer will reach for.
    #[allow(dead_code)]
    pub fn schema_version(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        let version: i64 =
            conn.query_row("SELECT version FROM schema_version", [], |row| row.get(0))?;
        Ok(version)
    }

    /// Insert a project if its path is not already registered, returning the
    /// project's row id either way. Idempotent — the hardcoded skeleton project
    /// can call this on every startup.
    pub fn upsert_project(&self, path: &str, nickname: &str) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (path, nickname, added_at, last_seen_at)
             VALUES (?1, ?2, datetime('now'), datetime('now'))
             ON CONFLICT(path) DO UPDATE SET last_seen_at = datetime('now')",
            (path, nickname),
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM projects WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Every registered project, ordered by `id` so the canvas sees a stable
    /// order across calls (`project-registry-001`). Empty list if nothing has
    /// been registered yet — not an error.
    pub fn list_projects(&self) -> Result<Vec<ProjectRow>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, nickname FROM projects ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProjectRow {
                id: row.get(0)?,
                path: row.get(1)?,
                nickname: row.get(2)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// The filesystem path registered for a project, by id. `Ok(None)` if the
    /// id is unknown — callers turn that into a clean IPC error rather than a
    /// panic (`project-registry-001` acceptance criterion).
    pub fn project_path(&self, project_id: i64) -> Result<Option<PathBuf>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM projects WHERE id = ?1")?;
        let mut rows = stmt.query([project_id])?;
        match rows.next()? {
            Some(row) => {
                let path: String = row.get(0)?;
                Ok(Some(PathBuf::from(path)))
            }
            None => Ok(None),
        }
    }

    /// Persist a tile's position for a project. There is at most one tile row
    /// per project in the skeleton, so this upserts on `project_id`.
    pub fn save_tile_position(
        &self,
        project_id: i64,
        x: f64,
        y: f64,
    ) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO tile_positions (project_id, x, y, width, height)
             VALUES (?1, ?2, ?3, 220, 120)
             ON CONFLICT(project_id) DO UPDATE SET x = ?2, y = ?3",
            (project_id, x, y),
        )?;
        Ok(())
    }

    /// Read back a tile's position, if one was ever persisted.
    pub fn tile_position(&self, project_id: i64) -> Result<Option<(f64, f64)>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT x, y FROM tile_positions WHERE project_id = ?1")?;
        let mut rows = stmt.query([project_id])?;
        match rows.next()? {
            Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
            None => Ok(None),
        }
    }

    /// Store a small UI preference (camera pan/zoom, last focus, …) in the
    /// `app_state` key/value table.
    pub fn set_app_state(&self, key: &str, value: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = ?2",
            (key, value),
        )?;
        Ok(())
    }

    /// Read a previously-stored UI preference.
    pub fn app_state(&self, key: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT value FROM app_state WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    // -------- scan-root CRUD (ADR-013, `project-registry-002a`) -----------
    //
    // The scan-root surface lets `scan.rs` and the IPC layer treat scan roots
    // as first-class persisted entities. Adding a row is idempotent on the
    // canonical path — a re-add of the same folder returns the existing id,
    // mirroring the registry's `upsert_project` shape.

    /// Insert a scan root (or return the existing row's id if the canonical
    /// path is already registered). The caller is responsible for handing in a
    /// *canonical* path (`scan::canonicalize_root`); the DB stores whatever it
    /// is given.
    pub fn upsert_scan_root(&self, path: &str, depth_cap: u32) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO scan_roots (path, depth_cap, added_at)
             VALUES (?1, ?2, datetime('now'))
             ON CONFLICT(path) DO UPDATE SET depth_cap = ?2",
            (path, depth_cap),
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM scan_roots WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Every registered scan root, ordered by `id` for stable UI listing.
    /// Empty list if nothing has been registered yet — not an error.
    pub fn list_scan_roots(&self) -> Result<Vec<ScanRootRow>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, depth_cap, added_at FROM scan_roots ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ScanRootRow {
                id: row.get(0)?,
                path: row.get(1)?,
                depth_cap: row.get::<_, i64>(2)? as u32,
                added_at: row.get(3)?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Look up one scan root by id. `Ok(None)` if the id is unknown — callers
    /// turn that into a clean IPC error.
    pub fn get_scan_root(&self, scan_root_id: i64) -> Result<Option<ScanRootRow>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, depth_cap, added_at FROM scan_roots WHERE id = ?1",
        )?;
        let mut rows = stmt.query([scan_root_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(ScanRootRow {
                id: row.get(0)?,
                path: row.get(1)?,
                depth_cap: row.get::<_, i64>(2)? as u32,
                added_at: row.get(3)?,
            })),
            None => Ok(None),
        }
    }

    /// The set of canonical project paths already in the registry. Used by
    /// the scan walker to mark candidates as `already_imported`. Returned as a
    /// `Vec<String>` (the caller wraps it in a `HashSet`); SQLite ordering is
    /// not relied on.
    pub fn list_project_paths(&self) -> Result<Vec<String>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT path FROM projects")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }
}

/// Apply ordered, versioned migration steps. Each step moves the schema from
/// version N to N+1; `schema_version` records where we are.
fn migrate(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)",
        [],
    )?;

    let current: i64 = conn
        .query_row("SELECT version FROM schema_version", [], |row| row.get(0))
        .unwrap_or(0);

    if current < 1 {
        // Step 0 -> 1: the ADR-004 initial schema sketch.
        conn.execute_batch(
            "CREATE TABLE projects (
                 id           INTEGER PRIMARY KEY AUTOINCREMENT,
                 path         TEXT NOT NULL UNIQUE,
                 nickname     TEXT NOT NULL,
                 added_at     TEXT NOT NULL,
                 last_seen_at TEXT NOT NULL
             );
             CREATE TABLE tile_positions (
                 project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                 x          REAL NOT NULL,
                 y          REAL NOT NULL,
                 width      REAL NOT NULL,
                 height     REAL NOT NULL,
                 cluster_id INTEGER NULL REFERENCES clusters(id) ON DELETE SET NULL
             );
             CREATE TABLE clusters (
                 id    INTEGER PRIMARY KEY AUTOINCREMENT,
                 name  TEXT NOT NULL,
                 color TEXT NOT NULL
             );
             CREATE TABLE app_state (
                 key   TEXT PRIMARY KEY,
                 value TEXT NOT NULL
             );",
        )?;
    }

    if current < 2 {
        // Step 1 -> 2: ADR-013 scan roots.
        //
        // - New `scan_roots` table (canonical path UNIQUE, per-root depth cap).
        // - `projects.scan_root_id` added as a NULL-able FK with
        //   `ON DELETE RESTRICT`: SQLite cannot tear down the per-project
        //   filesystem watchers (`WatcherSupervisor::remove`, ADR-008) on
        //   cascade, so the deregister has to be app-driven. `RESTRICT` turns
        //   the ordering ("remove watchers + drop child projects before the
        //   root row") into a checked invariant.
        //
        // `ALTER TABLE ADD COLUMN` is fine here because the column is NULL-able
        // with no non-NULL default, so existing rows pick up NULL and remain
        // valid (NULL = manually added, ADR-005 "Add project…").
        conn.execute_batch(
            "CREATE TABLE scan_roots (
                 id        INTEGER PRIMARY KEY AUTOINCREMENT,
                 path      TEXT NOT NULL UNIQUE,
                 depth_cap INTEGER NOT NULL DEFAULT 3,
                 added_at  TEXT NOT NULL
             );
             ALTER TABLE projects
                 ADD COLUMN scan_root_id INTEGER NULL
                 REFERENCES scan_roots(id) ON DELETE RESTRICT;",
        )?;
    }

    // Record the version we ended on (single-row table).
    conn.execute("DELETE FROM schema_version", [])?;
    conn.execute(
        "INSERT INTO schema_version (version) VALUES (?1)",
        [CURRENT_SCHEMA_VERSION],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_db_is_at_current_schema_version() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn upsert_project_is_idempotent_on_path() {
        let db = Db::open_in_memory().unwrap();
        let first = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        let second = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        assert_eq!(first, second, "same path must yield the same project id");
    }

    #[test]
    fn tile_position_round_trips() {
        let db = Db::open_in_memory().unwrap();
        let pid = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();

        assert_eq!(db.tile_position(pid).unwrap(), None);

        db.save_tile_position(pid, 12.5, -34.0).unwrap();
        assert_eq!(db.tile_position(pid).unwrap(), Some((12.5, -34.0)));

        // Dragging again overwrites in place.
        db.save_tile_position(pid, 99.0, 1.0).unwrap();
        assert_eq!(db.tile_position(pid).unwrap(), Some((99.0, 1.0)));
    }

    #[test]
    fn list_projects_returns_every_registered_row() {
        // `project-registry-001` acceptance criterion: with N >= 2 projects in
        // the table, `list_projects()` returns one row per project, in stable
        // (id) order.
        let db = Db::open_in_memory().unwrap();
        let id_a = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        let id_b = db.upsert_project("D:/work/other", "Other").unwrap();

        let rows = db.list_projects().unwrap();

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, id_a);
        assert_eq!(rows[0].path, "C:/src/guppi");
        assert_eq!(rows[0].nickname, "GUPPI");
        assert_eq!(rows[1].id, id_b);
        assert_eq!(rows[1].path, "D:/work/other");
        assert_eq!(rows[1].nickname, "Other");
    }

    #[test]
    fn list_projects_on_an_empty_registry_is_an_empty_vec_not_an_error() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.list_projects().unwrap().is_empty());
    }

    #[test]
    fn project_path_resolves_an_id_back_to_the_registered_path() {
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        assert_eq!(
            db.project_path(id).unwrap(),
            Some(PathBuf::from("C:/src/guppi"))
        );
    }

    #[test]
    fn project_path_for_an_unknown_id_is_none_not_an_error() {
        // Underpins the `get_project(project_id)` acceptance criterion:
        // an unknown id must produce a clean error, not a panic, so this
        // method must distinguish "not found" from "I/O failure".
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.project_path(99_999).unwrap(), None);
    }

    #[test]
    fn app_state_round_trips_camera() {
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.app_state("camera").unwrap(), None);

        db.set_app_state("camera", r#"{"pan_x":1,"pan_y":2,"zoom":1.5}"#)
            .unwrap();
        assert_eq!(
            db.app_state("camera").unwrap().as_deref(),
            Some(r#"{"pan_x":1,"pan_y":2,"zoom":1.5}"#)
        );
    }

    // -------- ADR-013 scan-root schema + CRUD (`project-registry-002a`) ----

    #[test]
    fn fresh_db_is_at_schema_version_two() {
        // ADR-013 acceptance: a fresh DB lands at v2 (scan_roots + scan_root_id).
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), 2);
    }

    #[test]
    fn v1_db_migrates_to_v2_without_data_loss() {
        // ADR-013 acceptance: a v1 DB (no scan_roots table, no scan_root_id
        // column) is migrated to v2 in place, gaining the table + column.
        // Existing project rows survive untouched.
        use rusqlite::Connection;

        let path = std::env::temp_dir().join(format!(
            "guppi-migration-test-{}-{:?}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        // Make sure no stale file from a prior crashed run is present.
        let _ = std::fs::remove_file(&path);

        // Hand-roll a v1 database with one project row.
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "foreign_keys", true).unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_version (version INTEGER NOT NULL);
                 INSERT INTO schema_version (version) VALUES (1);
                 CREATE TABLE projects (
                     id           INTEGER PRIMARY KEY AUTOINCREMENT,
                     path         TEXT NOT NULL UNIQUE,
                     nickname     TEXT NOT NULL,
                     added_at     TEXT NOT NULL,
                     last_seen_at TEXT NOT NULL
                 );
                 CREATE TABLE tile_positions (
                     project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                     x          REAL NOT NULL,
                     y          REAL NOT NULL,
                     width      REAL NOT NULL,
                     height     REAL NOT NULL,
                     cluster_id INTEGER NULL REFERENCES clusters(id) ON DELETE SET NULL
                 );
                 CREATE TABLE clusters (
                     id    INTEGER PRIMARY KEY AUTOINCREMENT,
                     name  TEXT NOT NULL,
                     color TEXT NOT NULL
                 );
                 CREATE TABLE app_state (
                     key   TEXT PRIMARY KEY,
                     value TEXT NOT NULL
                 );
                 INSERT INTO projects (path, nickname, added_at, last_seen_at)
                 VALUES ('C:/src/guppi', 'GUPPI', datetime('now'), datetime('now'));",
            )
            .unwrap();
        }

        // Open with migration applied.
        let db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 2);

        // Pre-existing project survived.
        let rows = db.list_projects().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "C:/src/guppi");
        assert_eq!(rows[0].nickname, "GUPPI");

        // The new column exists and is NULL for pre-existing rows.
        let conn = rusqlite::Connection::open(&path).unwrap();
        let scan_root_id: Option<i64> = conn
            .query_row(
                "SELECT scan_root_id FROM projects WHERE path = 'C:/src/guppi'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(scan_root_id.is_none(), "pre-existing rows must default to NULL");

        // The scan_roots table is empty and queryable.
        assert!(db.list_scan_roots().unwrap().is_empty());

        // Drop handles before cleanup so the file is unlocked on Windows.
        drop(db);
        drop(conn);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn upsert_scan_root_is_idempotent_on_canonical_path() {
        let db = Db::open_in_memory().unwrap();
        let a = db.upsert_scan_root("C:/src", 3).unwrap();
        let b = db.upsert_scan_root("C:/src", 3).unwrap();
        assert_eq!(a, b, "same path must yield the same scan_root_id");
    }

    #[test]
    fn list_scan_roots_returns_every_registered_row_in_id_order() {
        let db = Db::open_in_memory().unwrap();
        let id_a = db.upsert_scan_root("C:/src", 3).unwrap();
        let id_b = db.upsert_scan_root("D:/work", 5).unwrap();

        let rows = db.list_scan_roots().unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].id, id_a);
        assert_eq!(rows[0].path, "C:/src");
        assert_eq!(rows[0].depth_cap, 3);
        assert_eq!(rows[1].id, id_b);
        assert_eq!(rows[1].path, "D:/work");
        assert_eq!(rows[1].depth_cap, 5);
    }

    #[test]
    fn list_scan_roots_on_empty_db_is_empty_vec_not_error() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.list_scan_roots().unwrap().is_empty());
    }

    #[test]
    fn get_scan_root_returns_row_or_none() {
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_scan_root("C:/src", 4).unwrap();

        let row = db.get_scan_root(id).unwrap().expect("must exist");
        assert_eq!(row.id, id);
        assert_eq!(row.path, "C:/src");
        assert_eq!(row.depth_cap, 4);
        assert!(!row.added_at.is_empty());

        assert!(db.get_scan_root(99_999).unwrap().is_none());
    }

    #[test]
    fn list_project_paths_returns_every_registered_project_path() {
        let db = Db::open_in_memory().unwrap();
        db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        db.upsert_project("D:/work/other", "Other").unwrap();

        let mut paths = db.list_project_paths().unwrap();
        paths.sort();
        assert_eq!(paths, vec!["C:/src/guppi".to_string(), "D:/work/other".to_string()]);
    }

    #[test]
    fn scan_roots_persist_across_db_handle_close_and_reopen() {
        // ADR-013 / acceptance criterion 6: scan roots survive an app restart.
        // Modelled here as "close the `Db` handle, reopen at the same file,
        // and `list_scan_roots()` returns the same rows."
        let path = std::env::temp_dir().join(format!(
            "guppi-scan-roots-persist-{}-{:?}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);

        let id_a;
        let id_b;
        {
            let db = Db::open(&path).unwrap();
            id_a = db.upsert_scan_root("C:/src", 3).unwrap();
            id_b = db.upsert_scan_root("D:/work", 5).unwrap();
        }
        // Re-open at the same path — simulates an app restart.
        {
            let db = Db::open(&path).unwrap();
            assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);
            let rows = db.list_scan_roots().unwrap();
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0].id, id_a);
            assert_eq!(rows[0].path, "C:/src");
            assert_eq!(rows[0].depth_cap, 3);
            assert_eq!(rows[1].id, id_b);
            assert_eq!(rows[1].path, "D:/work");
            assert_eq!(rows[1].depth_cap, 5);
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn empty_scan_root_is_still_persisted_and_rescannable() {
        // ADR-013 / acceptance criterion 7: a scan root with zero candidates
        // is valid; it is persisted and `get_scan_root` round-trips it for a
        // later rescan.
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_scan_root("E:/empty/folder", 3).unwrap();
        // No projects were ever discovered under it — does not matter; the
        // row stands on its own and can be re-read.
        let row = db.get_scan_root(id).unwrap().expect("empty root must persist");
        assert_eq!(row.path, "E:/empty/folder");
        assert_eq!(row.depth_cap, 3);
    }
}
