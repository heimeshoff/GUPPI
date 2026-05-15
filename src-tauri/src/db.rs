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

/// The schema version this build expects. Bump it and add a migration step in
/// `migrate` whenever the schema changes.
pub const CURRENT_SCHEMA_VERSION: i64 = 1;

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
}
