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
///
/// v3 (ADR-005 retention realisation, `project-registry-003`): adds the
/// nullable `projects.deleted_at` column (ISO-8601 UTC). NULL = live row.
/// A non-NULL value soft-deletes the row: `list_projects` /
/// `list_projects_by_scan_root` filter it out, but `project_path` still
/// resolves it (so the GC sweep and per-id cleanup paths can look it up).
/// `Db::open` sweeps rows older than `RETENTION_DAYS` after the migration
/// runs.
pub const CURRENT_SCHEMA_VERSION: i64 = 3;

/// ADR-005's 30-day retention window for soft-deleted projects — `remove_project`
/// flags a row with `deleted_at`, the row stays for `RETENTION_DAYS` so a
/// re-add via `upsert_project` revives the tile in place, and `Db::open`'s
/// startup GC sweep hard-deletes anything older. Single edit point if the ADR's
/// stipulation ever shifts.
pub const RETENTION_DAYS: i64 = 30;

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
        // ADR-005 retention realisation: sweep any soft-deleted projects whose
        // 30-day window has elapsed. Cascading `tile_positions` rows go with
        // them via the existing ON DELETE CASCADE FK (schema v1). Logged as
        // `info` so the operational signal (ADR-010) is visible.
        sweep_expired_soft_deletes(&conn)?;
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
    ///
    /// **Soft-delete revival (ADR-005, `project-registry-003`):** the ON
    /// CONFLICT clause clears `deleted_at` (sets it to NULL). A re-add of a
    /// previously soft-deleted path therefore revives the row, and because
    /// `tile_positions` is preserved through soft-delete the tile reappears at
    /// its old spot. This is the load-bearing detail of the 30-day retention
    /// design — there is no admin "undelete" affordance; re-registering is the
    /// one and only restore path.
    pub fn upsert_project(&self, path: &str, nickname: &str) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (path, nickname, added_at, last_seen_at, deleted_at)
             VALUES (?1, ?2, datetime('now'), datetime('now'), NULL)
             ON CONFLICT(path) DO UPDATE SET
                 last_seen_at = datetime('now'),
                 deleted_at   = NULL",
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
    ///
    /// **Soft-deleted rows are invisible to enumeration** (ADR-005 retention,
    /// `project-registry-003`). The 30-day retention window is bookkeeping
    /// only — to the rest of the system a `deleted_at IS NOT NULL` row no
    /// longer exists. `project_path(id)` still resolves it (the GC sweep and
    /// cascade cleanup paths need per-id resolve).
    pub fn list_projects(&self) -> Result<Vec<ProjectRow>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, path, nickname FROM projects
             WHERE deleted_at IS NULL
             ORDER BY id ASC",
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

    /// Insert a discovered-under-a-scan-root project (or refresh `last_seen_at`
    /// if its canonical path is already in the registry), stamping the
    /// `scan_root_id` of the root that surfaced it. Like `upsert_project` but
    /// participates in **origin tracking** (ADR-013): the column stays NULL for
    /// manually-added projects and non-NULL for cascade-deregistration to find
    /// later.
    ///
    /// Idempotent on `path` — re-importing the same canonical path is a
    /// no-op-with-`last_seen_at`-bump and returns the existing row's id. The
    /// `scan_root_id` is set on insert and refreshed on conflict (a project
    /// re-discovered under a different root reflects its most recent
    /// discoverer; this is a corner that should not arise in normal use but is
    /// the least surprising of the available choices).
    pub fn upsert_scanned_project(
        &self,
        path: &str,
        nickname: &str,
        scan_root_id: i64,
    ) -> Result<i64, DbError> {
        let conn = self.conn.lock().unwrap();
        // ADR-005 retention realisation: clear `deleted_at` on insert/update so
        // re-importing a soft-deleted path revives the row alongside its
        // preserved tile-position (`project-registry-003`).
        conn.execute(
            "INSERT INTO projects (path, nickname, added_at, last_seen_at, scan_root_id, deleted_at)
             VALUES (?1, ?2, datetime('now'), datetime('now'), ?3, NULL)
             ON CONFLICT(path) DO UPDATE SET
                 last_seen_at = datetime('now'),
                 scan_root_id = ?3,
                 deleted_at   = NULL",
            (path, nickname, scan_root_id),
        )?;
        let id: i64 = conn.query_row(
            "SELECT id FROM projects WHERE path = ?1",
            [path],
            |row| row.get(0),
        )?;
        Ok(id)
    }

    /// Delete a single project row by id. `tile_positions` follows via the
    /// existing `ON DELETE CASCADE` (set in v1 of the schema). An unknown
    /// `project_id` is a clean no-op (zero rows affected) — never a panic — so
    /// the cascade-deregister loop in `remove_scan_root` can call this without
    /// pre-checking existence.
    ///
    /// **Hard-delete, no retention.** ADR-005's 30-day tile-state retention is
    /// scoped to the user-initiated single "Remove project" affordance
    /// (`canvas-005`), not to this method's bulk-cascade callers (ADR-013).
    /// Single-project removal will reuse this method but must pair it with the
    /// GC machinery that lands with `canvas-005`.
    pub fn remove_project(&self, project_id: i64) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        // ON DELETE CASCADE on tile_positions.project_id (schema v1) handles
        // the tile row; clusters.id-side never references a project, so this
        // single DELETE is the whole story.
        conn.execute("DELETE FROM projects WHERE id = ?1", [project_id])?;
        Ok(())
    }

    /// **Soft-delete a single project row by id** — the realisation of ADR-005's
    /// 30-day retention stipulation (`project-registry-003`). Sets
    /// `deleted_at = datetime('now')`; the row stays in the table and the
    /// matching `tile_positions` row is **not** touched, so a re-add via
    /// `upsert_project` (which clears `deleted_at`) restores the tile in place.
    /// `Db::open`'s startup GC sweep eventually hard-deletes anything still
    /// soft-deleted after `RETENTION_DAYS`.
    ///
    /// Unknown `project_id` is a clean no-op (zero rows affected) — the IPC
    /// layer turns "unknown id" into a user-facing error before getting here.
    /// Calling this on an already-soft-deleted row simply re-stamps the
    /// timestamp; the 30-day window restarts. Harmless in practice.
    pub fn soft_delete_project(&self, project_id: i64) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET deleted_at = datetime('now') WHERE id = ?1",
            [project_id],
        )?;
        Ok(())
    }

    /// Whether a project id is currently soft-deleted. Used by tests (and a
    /// future admin / introspection affordance — not yet wired into IPC,
    /// since the public surface is intentionally the upsert/list/soft-delete
    /// triad). Returns `Ok(None)` for an unknown id; `Ok(Some(None))` for a
    /// live row; `Ok(Some(Some(ts)))` for a soft-deleted row.
    #[allow(dead_code)]
    pub fn project_deleted_at(&self, project_id: i64) -> Result<Option<Option<String>>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT deleted_at FROM projects WHERE id = ?1")?;
        let mut rows = stmt.query([project_id])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get::<_, Option<String>>(0)?)),
            None => Ok(None),
        }
    }

    /// Every child project id of a scan root, in id order. Drives the
    /// app-driven cascade in `remove_scan_root` (ADR-013): per id, the IPC
    /// command calls `supervisor.remove(id)` and then `db.remove_project(id)`
    /// before finally dropping the root row itself. Manually-added projects
    /// (NULL `scan_root_id`) never appear in any cascade's enumeration.
    pub fn list_projects_by_scan_root(
        &self,
        scan_root_id: i64,
    ) -> Result<Vec<i64>, DbError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id FROM projects
             WHERE scan_root_id = ?1 AND deleted_at IS NULL
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([scan_root_id], |row| row.get::<_, i64>(0))?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Delete a scan root row by id — **never** call this before its child
    /// projects have been removed. The schema's `ON DELETE RESTRICT` (ADR-013)
    /// will reject the delete if any `projects.scan_root_id` still references
    /// it, which is the checked invariant that protects the app-driven
    /// cascade in `remove_scan_root` from getting the ordering wrong.
    ///
    /// Unknown `scan_root_id` is a clean no-op (zero rows affected).
    pub fn delete_scan_root(&self, scan_root_id: i64) -> Result<(), DbError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM scan_roots WHERE id = ?1", [scan_root_id])?;
        Ok(())
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

    if current < 3 {
        // Step 2 -> 3: ADR-005's 30-day tile-state retention realisation
        // (`project-registry-003`). Adds `projects.deleted_at` (ISO-8601 UTC,
        // NULL = live). The user-initiated single "Remove project" affordance
        // sets it; `Db::open` sweeps anything older than `RETENTION_DAYS` on
        // startup. `tile_positions` is preserved through the retention window
        // so a re-add restores the tile in place. Cascade-deregister
        // (`remove_scan_root`, ADR-013) is unaffected — it hard-deletes
        // directly, ignoring `deleted_at` entirely.
        //
        // `ALTER TABLE ADD COLUMN` is fine: the new column is NULL-able with
        // no non-NULL default, so existing rows pick up NULL ("live") and
        // remain valid without a backfill.
        conn.execute_batch(
            "ALTER TABLE projects ADD COLUMN deleted_at TEXT NULL;",
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

/// Hard-delete every project row whose `deleted_at` is older than the ADR-005
/// retention window. Cascading `tile_positions` rows go with each project via
/// the existing ON DELETE CASCADE FK (schema v1). Called by `Db::from_connection`
/// at the end of startup.
///
/// `tracing::info!`s the count of swept rows for the ADR-010 operational
/// signal — silent on an empty sweep (the common case).
fn sweep_expired_soft_deletes(conn: &Connection) -> Result<(), DbError> {
    // SQLite's `datetime('now', '-N days')` is a string-comparable ISO-8601
    // ordering against `deleted_at` (also a `datetime('now')` value), so a
    // straight `WHERE deleted_at < <threshold>` works without any time-crate
    // dependency.
    let cutoff_expr = format!("datetime('now', '-{} days')", RETENTION_DAYS);
    let sql = format!(
        "DELETE FROM projects
         WHERE deleted_at IS NOT NULL AND deleted_at < {cutoff_expr}"
    );
    let swept = conn.execute(&sql, [])?;
    if swept > 0 {
        tracing::info!(
            swept,
            retention_days = RETENTION_DAYS,
            "soft-delete GC sweep: hard-deleted expired projects"
        );
    }
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
        // ADR-013 acceptance: a fresh DB has the scan_roots table + scan_root_id
        // column. The version itself moved to v3 with `project-registry-003`
        // (added `deleted_at`), but the v2 surface this test cares about
        // remains intact.
        let db = Db::open_in_memory().unwrap();
        // The scan_roots table is queryable on a fresh DB.
        assert!(db.list_scan_roots().unwrap().is_empty());
        // And the scan_root_id column exists (a scanned-project upsert would
        // fail otherwise — exercised by the v1→v2 migration test).
        assert!(db.schema_version().unwrap() >= 2);
    }

    #[test]
    fn fresh_db_is_at_schema_version_three() {
        // `project-registry-003` acceptance: a fresh DB lands at v3 (adds
        // `projects.deleted_at` for ADR-005's 30-day retention realisation).
        let db = Db::open_in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), 3);
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

        // Open with migration applied — v1 leaps all the way to the current
        // version (the migration runner applies every step it has not yet
        // applied; `project-registry-003` adds v3).
        let db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), CURRENT_SCHEMA_VERSION);

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

    // -------- 002b: import + remove + cascade-deregister --------------------

    #[test]
    fn upsert_scanned_project_stamps_scan_root_id() {
        // `project-registry-002b` acceptance: importing a candidate under a
        // scan root persists `scan_root_id` so the cascade can find it later.
        let db = Db::open_in_memory().unwrap();
        let root_id = db.upsert_scan_root("C:/src", 3).unwrap();

        let project_id = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root_id)
            .unwrap();

        // The row carries scan_root_id = root_id (verified via direct query —
        // the public `ProjectRow` doesn't expose the column).
        let conn = db.conn.lock().unwrap();
        let stamped: Option<i64> = conn
            .query_row(
                "SELECT scan_root_id FROM projects WHERE id = ?1",
                [project_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stamped, Some(root_id), "import must stamp scan_root_id");
    }

    #[test]
    fn upsert_scanned_project_is_idempotent_on_path() {
        // `project-registry-002b` acceptance: re-importing the same candidate
        // does not duplicate the row; same project id comes back.
        let db = Db::open_in_memory().unwrap();
        let root_id = db.upsert_scan_root("C:/src", 3).unwrap();

        let first = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root_id)
            .unwrap();
        let second = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root_id)
            .unwrap();
        assert_eq!(first, second);
        assert_eq!(db.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn remove_project_deletes_row_and_cascades_tile_positions() {
        // `project-registry-002b` acceptance: `remove_project` clears the
        // projects row and (via ON DELETE CASCADE on tile_positions.project_id)
        // the associated tile state.
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        db.save_tile_position(id, 10.0, 20.0).unwrap();
        assert_eq!(db.tile_position(id).unwrap(), Some((10.0, 20.0)));

        db.remove_project(id).unwrap();

        assert!(db.list_projects().unwrap().is_empty(), "row must be gone");
        assert_eq!(
            db.tile_position(id).unwrap(),
            None,
            "tile_positions must cascade"
        );
    }

    #[test]
    fn remove_project_with_unknown_id_is_a_clean_no_op() {
        // `project-registry-002b` acceptance: unknown project_id must be a
        // clean no-op (not a panic, not an error). Mirrors `project_path` /
        // `supervisor.remove`'s shape so the cascade loop can call into it
        // without pre-checks.
        let db = Db::open_in_memory().unwrap();
        // No projects inserted — id 99_999 cannot match anything.
        db.remove_project(99_999).expect("must not panic or error");
        assert!(db.list_projects().unwrap().is_empty());
    }

    #[test]
    fn list_projects_by_scan_root_returns_only_that_roots_children() {
        // `project-registry-002b` acceptance: the cascade enumerates children
        // by scan_root_id. Manually-added projects (NULL) and projects under
        // *other* roots must not appear.
        let db = Db::open_in_memory().unwrap();
        let root_a = db.upsert_scan_root("C:/src", 3).unwrap();
        let root_b = db.upsert_scan_root("D:/work", 3).unwrap();

        let p_a1 = db
            .upsert_scanned_project("C:/src/p1", "P1", root_a)
            .unwrap();
        let p_a2 = db
            .upsert_scanned_project("C:/src/p2", "P2", root_a)
            .unwrap();
        let _p_b = db
            .upsert_scanned_project("D:/work/q", "Q", root_b)
            .unwrap();
        // Manually-added (NULL scan_root_id) — must NEVER appear in any
        // cascade's enumeration.
        let _p_manual = db.upsert_project("E:/loose/project", "Manual").unwrap();

        let children = db.list_projects_by_scan_root(root_a).unwrap();
        assert_eq!(children, vec![p_a1, p_a2]);
    }

    #[test]
    fn list_projects_by_scan_root_for_empty_root_is_empty_vec() {
        let db = Db::open_in_memory().unwrap();
        let root = db.upsert_scan_root("C:/empty", 3).unwrap();
        assert!(db.list_projects_by_scan_root(root).unwrap().is_empty());
    }

    #[test]
    fn delete_scan_root_succeeds_after_children_removed() {
        // `project-registry-002b` acceptance: the cascade ordering — remove
        // children first, then the root — clears the ON DELETE RESTRICT FK and
        // the scan_roots row goes away.
        let db = Db::open_in_memory().unwrap();
        let root = db.upsert_scan_root("C:/src", 3).unwrap();
        let child = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root)
            .unwrap();

        db.remove_project(child).unwrap();
        db.delete_scan_root(root).unwrap();

        assert!(db.list_scan_roots().unwrap().is_empty());
    }

    #[test]
    fn delete_scan_root_with_living_child_is_rejected_by_restrict() {
        // `project-registry-002b`: the ON DELETE RESTRICT FK (ADR-013) makes
        // the app-driven cascade ordering a checked invariant — a delete of
        // the root while a child still references it must fail loud rather
        // than orphan the child.
        let db = Db::open_in_memory().unwrap();
        let root = db.upsert_scan_root("C:/src", 3).unwrap();
        let _child = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root)
            .unwrap();

        let err = db.delete_scan_root(root).unwrap_err();
        // rusqlite reports this as a constraint failure; we only care that
        // we get an error variant (the type is opaque), and that the rows
        // are still there.
        assert!(matches!(err, DbError::Sqlite(_)), "must surface as DbError");
        assert_eq!(db.list_scan_roots().unwrap().len(), 1);
        assert_eq!(db.list_projects().unwrap().len(), 1);
    }

    #[test]
    fn delete_scan_root_with_unknown_id_is_a_clean_no_op() {
        let db = Db::open_in_memory().unwrap();
        db.delete_scan_root(99_999).expect("must not panic or error");
    }

    // -------- 003: soft-delete + 30-day GC sweep ---------------------------

    #[test]
    fn v2_db_migrates_to_v3_without_data_loss() {
        // `project-registry-003` acceptance: a v2 DB (no `deleted_at` column)
        // is migrated to v3 in place, gaining the column. Existing project
        // rows survive untouched, and the new column defaults to NULL for all
        // pre-existing rows.
        use rusqlite::Connection;

        let path = std::env::temp_dir().join(format!(
            "guppi-v2-v3-migration-{}-{:?}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);

        // Hand-roll a v2 database with one project row and one scan_roots row.
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "foreign_keys", true).unwrap();
            conn.execute_batch(
                "CREATE TABLE schema_version (version INTEGER NOT NULL);
                 INSERT INTO schema_version (version) VALUES (2);
                 CREATE TABLE clusters (
                     id    INTEGER PRIMARY KEY AUTOINCREMENT,
                     name  TEXT NOT NULL,
                     color TEXT NOT NULL
                 );
                 CREATE TABLE scan_roots (
                     id        INTEGER PRIMARY KEY AUTOINCREMENT,
                     path      TEXT NOT NULL UNIQUE,
                     depth_cap INTEGER NOT NULL DEFAULT 3,
                     added_at  TEXT NOT NULL
                 );
                 CREATE TABLE projects (
                     id           INTEGER PRIMARY KEY AUTOINCREMENT,
                     path         TEXT NOT NULL UNIQUE,
                     nickname     TEXT NOT NULL,
                     added_at     TEXT NOT NULL,
                     last_seen_at TEXT NOT NULL,
                     scan_root_id INTEGER NULL REFERENCES scan_roots(id) ON DELETE RESTRICT
                 );
                 CREATE TABLE tile_positions (
                     project_id INTEGER PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
                     x          REAL NOT NULL,
                     y          REAL NOT NULL,
                     width      REAL NOT NULL,
                     height     REAL NOT NULL,
                     cluster_id INTEGER NULL REFERENCES clusters(id) ON DELETE SET NULL
                 );
                 CREATE TABLE app_state (
                     key   TEXT PRIMARY KEY,
                     value TEXT NOT NULL
                 );
                 INSERT INTO projects (path, nickname, added_at, last_seen_at)
                 VALUES ('C:/src/guppi', 'GUPPI', datetime('now'), datetime('now'));
                 INSERT INTO scan_roots (path, depth_cap, added_at)
                 VALUES ('C:/src', 3, datetime('now'));",
            )
            .unwrap();
        }

        // Open with migration applied.
        let db = Db::open(&path).unwrap();
        assert_eq!(db.schema_version().unwrap(), 3);

        // Pre-existing project survived AND has deleted_at = NULL.
        let rows = db.list_projects().unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, "C:/src/guppi");

        let conn = rusqlite::Connection::open(&path).unwrap();
        let deleted_at: Option<String> = conn
            .query_row(
                "SELECT deleted_at FROM projects WHERE path = 'C:/src/guppi'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            deleted_at.is_none(),
            "pre-existing rows must default to deleted_at = NULL"
        );

        // The scan_roots row survived too.
        assert_eq!(db.list_scan_roots().unwrap().len(), 1);

        drop(db);
        drop(conn);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn soft_delete_project_sets_deleted_at_but_keeps_row_and_tile_position() {
        // `project-registry-003` acceptance: a single "Remove project" is a
        // soft-delete — the row stays, `deleted_at` is set, and the
        // `tile_positions` row is NOT touched (so a re-add restores in place).
        // `list_projects` no longer returns the soft-deleted row.
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        db.save_tile_position(id, 42.0, 17.0).unwrap();
        assert_eq!(db.tile_position(id).unwrap(), Some((42.0, 17.0)));

        db.soft_delete_project(id).unwrap();

        // Row still exists in the table — `project_path` resolves the id.
        assert_eq!(
            db.project_path(id).unwrap(),
            Some(PathBuf::from("C:/src/guppi")),
            "soft-deleted rows must still resolve via project_path"
        );
        // ...but `list_projects` filters it out.
        assert!(
            db.list_projects().unwrap().is_empty(),
            "list_projects must hide soft-deleted rows"
        );
        // Tile state survives.
        assert_eq!(
            db.tile_position(id).unwrap(),
            Some((42.0, 17.0)),
            "tile_positions must survive soft-delete"
        );
        // And deleted_at is set.
        let deleted_at = db.project_deleted_at(id).unwrap();
        assert!(
            deleted_at.as_ref().and_then(|o| o.as_ref()).is_some(),
            "deleted_at must be set after soft-delete: {deleted_at:?}"
        );
    }

    #[test]
    fn upsert_project_revives_a_soft_deleted_row_clearing_deleted_at() {
        // `project-registry-003` acceptance: re-registering a path whose row
        // was soft-deleted clears `deleted_at` and returns the SAME project id
        // (so the tile re-appears at its preserved position).
        let db = Db::open_in_memory().unwrap();
        let id = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        db.save_tile_position(id, 1.5, 2.5).unwrap();
        db.soft_delete_project(id).unwrap();
        assert!(db.list_projects().unwrap().is_empty());

        // Revive.
        let revived = db.upsert_project("C:/src/guppi", "GUPPI").unwrap();
        assert_eq!(revived, id, "same path must yield same id across revive");
        assert_eq!(db.list_projects().unwrap().len(), 1);
        let deleted_at = db.project_deleted_at(id).unwrap();
        assert_eq!(
            deleted_at.as_ref().and_then(|o| o.as_ref()),
            None,
            "deleted_at must be cleared on revive"
        );
        // Tile position preserved through the soft-delete + revive cycle.
        assert_eq!(db.tile_position(id).unwrap(), Some((1.5, 2.5)));
    }

    #[test]
    fn upsert_scanned_project_also_revives_a_soft_deleted_row() {
        // `project-registry-003`: the same NULL-on-conflict clause exists on
        // both upsert paths so importing a soft-deleted scanned project
        // revives it.
        let db = Db::open_in_memory().unwrap();
        let root = db.upsert_scan_root("C:/src", 3).unwrap();
        let id = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root)
            .unwrap();
        db.soft_delete_project(id).unwrap();
        assert!(db.list_projects().unwrap().is_empty());

        let revived = db
            .upsert_scanned_project("C:/src/guppi", "GUPPI", root)
            .unwrap();
        assert_eq!(revived, id);
        assert_eq!(db.list_projects().unwrap().len(), 1);
        let deleted_at = db.project_deleted_at(id).unwrap();
        assert!(deleted_at.as_ref().and_then(|o| o.as_ref()).is_none());
    }

    #[test]
    fn list_projects_by_scan_root_hides_soft_deleted_children() {
        // `project-registry-003`: the scan-root cascade enumeration must NOT
        // see soft-deleted children (they are already gone to the rest of the
        // system; the GC sweep will hard-delete them on a later restart).
        let db = Db::open_in_memory().unwrap();
        let root = db.upsert_scan_root("C:/src", 3).unwrap();
        let alive = db
            .upsert_scanned_project("C:/src/p1", "P1", root)
            .unwrap();
        let soft_deleted = db
            .upsert_scanned_project("C:/src/p2", "P2", root)
            .unwrap();
        db.soft_delete_project(soft_deleted).unwrap();

        let children = db.list_projects_by_scan_root(root).unwrap();
        assert_eq!(
            children,
            vec![alive],
            "soft-deleted children must not appear in cascade enumeration"
        );
    }

    #[test]
    fn project_deleted_at_returns_none_for_unknown_id() {
        let db = Db::open_in_memory().unwrap();
        assert!(db.project_deleted_at(99_999).unwrap().is_none());
    }

    #[test]
    fn soft_delete_project_with_unknown_id_is_a_clean_no_op() {
        let db = Db::open_in_memory().unwrap();
        db.soft_delete_project(99_999)
            .expect("must not panic or error");
    }

    #[test]
    fn startup_gc_sweep_hard_deletes_rows_older_than_retention_window() {
        // `project-registry-003` acceptance: rows with `deleted_at < now - 30d`
        // are deleted on `Db::open`; tile_positions for those rows cascade-
        // delete via the existing ON DELETE CASCADE FK. Live rows
        // (`deleted_at = NULL`) and recently-deleted rows
        // (`deleted_at >= now - 30d`) are untouched.
        use rusqlite::Connection;

        let path = std::env::temp_dir().join(format!(
            "guppi-gc-sweep-{}-{:?}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);

        // Build a v3 DB with three rows. Skip Db::open the first time so the
        // sweep does not run before we forge the timestamps; instead apply
        // migrations through a fresh Db, then mutate via a side connection.
        let live_id;
        let recent_id;
        let stale_id;
        {
            let db = Db::open(&path).unwrap();
            live_id = db.upsert_project("C:/live", "Live").unwrap();
            recent_id = db.upsert_project("C:/recent", "Recent").unwrap();
            stale_id = db.upsert_project("C:/stale", "Stale").unwrap();
            // Give the stale row a tile so we can verify the cascade.
            db.save_tile_position(stale_id, 9.0, 9.0).unwrap();
        }

        // Forge timestamps via a side connection: `recent` was deleted 1 day
        // ago (well within retention), `stale` was deleted 60 days ago (well
        // outside retention).
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "foreign_keys", true).unwrap();
            conn.execute(
                "UPDATE projects SET deleted_at = datetime('now', '-1 day') WHERE id = ?1",
                [recent_id],
            )
            .unwrap();
            conn.execute(
                "UPDATE projects SET deleted_at = datetime('now', '-60 days') WHERE id = ?1",
                [stale_id],
            )
            .unwrap();
        }

        // Re-open: GC sweep runs as part of `from_connection`.
        {
            let db = Db::open(&path).unwrap();
            // Stale row is gone entirely.
            assert!(
                db.project_path(stale_id).unwrap().is_none(),
                "stale soft-deleted row must be hard-deleted by GC sweep"
            );
            // Stale row's tile cascaded.
            assert_eq!(
                db.tile_position(stale_id).unwrap(),
                None,
                "tile_positions must cascade with the stale row"
            );
            // Recent (within retention) row is still soft-deleted but present.
            assert_eq!(
                db.project_path(recent_id).unwrap(),
                Some(PathBuf::from("C:/recent")),
                "recently soft-deleted row must survive the sweep"
            );
            let recent_deleted_at = db.project_deleted_at(recent_id).unwrap();
            assert!(
                recent_deleted_at.as_ref().and_then(|o| o.as_ref()).is_some(),
                "recent row stays soft-deleted, not revived"
            );
            // Live row is untouched.
            assert_eq!(
                db.project_path(live_id).unwrap(),
                Some(PathBuf::from("C:/live")),
                "live row must not be affected by the sweep"
            );
            // `list_projects` returns only the live row.
            let alive: Vec<i64> = db.list_projects().unwrap().iter().map(|r| r.id).collect();
            assert_eq!(alive, vec![live_id]);
        }

        let _ = std::fs::remove_file(&path);
    }
}
