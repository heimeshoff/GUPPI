//! GUPPI's Rust core — the walking skeleton (`infrastructure-012`), generalised
//! to the multi-project model by `project-registry-001`.
//!
//! This module wires the pieces the foundation ADRs settled into one running
//! Tauri 2 app:
//!
//! - **ADR-001** Tauri 2, Rust core / web frontend, IPC via `invoke`/`emit`.
//! - **ADR-004** SQLite state in the OS user-config dir (`db`).
//! - **ADR-005** every project is a row in the `projects` table; the
//!   hard-coded skeleton project is seeded on startup so the canvas has
//!   something to draw before `project-registry-002` lands.
//! - **ADR-008** one debounced `.agentheim/` watcher *per project*, owned by
//!   the central `WatcherSupervisor` (`supervisor`). The single-project
//!   primitive lives in `watcher`.
//! - **ADR-009** in-core `EventBus` + a single frontend-bridge task that is
//!   the *only* place Tauri's `emit` is called for domain events (`events`).
//! - **ADR-010** `tracing` to rotating local files (`logging`).
//!
//! `AppState` no longer carries a single `project_id`/`project_path`: those
//! were the walking-skeleton's hard-coded shape. The multi-project IPC commands
//! take `project_id` explicitly and resolve the path through the registry
//! (`Db::project_path`).

mod db;
mod events;
mod logging;
mod project;
mod pty;
mod scan;
mod supervisor;
mod watcher;

use db::{Db, ScanRootRow, DEFAULT_SCAN_DEPTH_CAP};
use events::{DomainEvent, EventBus};
use project::ProjectSnapshot;
use pty::ClaudeSession;
use scan::ScanCandidate;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use supervisor::WatcherSupervisor;
use tauri::{Emitter, Manager};

/// The Tauri event name the frontend listens on (ADR-009: a single event name
/// with a JSON payload).
const FRONTEND_EVENT: &str = "guppi://event";

/// The walking skeleton's one hard-coded project. The task says "Marco picks
/// one, e.g. `C:\src\heimeshoff\agentic\guppi`" — which is GUPPI's own repo,
/// the project this code lives in, so it is guaranteed to be a real Agentheim
/// project on this machine.
const HARDCODED_PROJECT_PATH: &str = r"C:\src\heimeshoff\agentic\guppi";

/// Application state shared with Tauri commands as managed state.
///
/// `project-registry-001` reshaped this: the skeleton's `project_id` /
/// `project_path` fields were the one-project assumption made flesh. They are
/// gone. Per-project IPC commands now take `project_id` explicitly and resolve
/// the path through the registry (`Db::project_path`).
struct AppState {
    db: Arc<Db>,
    /// The central per-project watcher orchestrator (ADR-008). Cheap to clone;
    /// every clone shares the same map. Held to keep the supervisor (and every
    /// project's watcher) alive for the process lifetime, and called by the
    /// `002b` cascade IPC (`import_scanned_projects.supervisor.add` per pick,
    /// `remove_scan_root.supervisor.remove` per cascaded child).
    supervisor: WatcherSupervisor,
    /// ADR-009 event bus, shared so spike commands can hand it to a
    /// `ClaudeSession` actor's read loop.
    bus: EventBus,
    /// The PTY spike's single session slot (`infrastructure-013-pty-spike`).
    /// Multi-session orchestration / a real registry is later feature work
    /// (ADR-006 scope-out); the spike needs exactly one live session it can
    /// spawn, drive, and kill from IPC to exercise the ADR-006 stack
    /// end-to-end on real Windows hardware.
    claude_session: Mutex<Option<ClaudeSession>>,
}

/// IPC command — read every registered project into snapshots
/// (`project-registry-001`). A row whose `.agentheim/` is missing is skipped
/// and logged rather than aborting the call: a single broken project must not
/// strand the canvas. The frontend calls this on mount and on
/// `resync_required` (ADR-009 lag escape hatch — `canvas-001`).
#[tauri::command]
fn list_projects(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<ProjectSnapshot>, String> {
    let rows = state.db.list_projects().map_err(|e| {
        tracing::error!(error = %e, "list_projects: db query failed");
        e.to_string()
    })?;

    let mut snapshots = Vec::with_capacity(rows.len());
    for row in rows {
        let path = PathBuf::from(&row.path);
        match project::get_project(row.id, &path) {
            Ok(snapshot) => snapshots.push(snapshot),
            Err(e) => {
                // Skip-and-log: a missing .agentheim/ for one project must not
                // abort the whole list. Logged as `warn` so it shows up in the
                // tracing file (ADR-010) — the registered-but-unwatched
                // (ADR-005 "missing") state is the operational signal.
                tracing::warn!(
                    project_id = row.id,
                    path = %row.path,
                    error = %e,
                    "list_projects: skipping unreadable project"
                );
            }
        }
    }
    Ok(snapshots)
}

/// IPC command — read exactly one registered project's snapshot
/// (`project-registry-001`). The frontend invokes this for per-project resync
/// (a `ResyncRequired { project_id }` event re-fetches one project, not all).
/// An unknown `project_id` is a clean error, never a panic.
#[tauri::command]
fn get_project(
    state: tauri::State<'_, AppState>,
    project_id: i64,
) -> Result<ProjectSnapshot, String> {
    let path = state
        .db
        .project_path(project_id)
        .map_err(|e| {
            tracing::error!(error = %e, project_id, "get_project: db lookup failed");
            e.to_string()
        })?
        .ok_or_else(|| {
            tracing::warn!(project_id, "get_project: unknown project_id");
            format!("unknown project_id: {project_id}")
        })?;

    project::get_project(project_id, &path).map_err(|e| {
        tracing::error!(error = %e, project_id, "get_project failed");
        e.to_string()
    })
}

/// The payload `add_scan_root` returns: the persisted root's id plus the
/// candidate checklist from walking its subtree.
#[derive(Debug, serde::Serialize)]
struct AddScanRootResult {
    scan_root_id: i64,
    candidates: Vec<ScanCandidate>,
}

/// IPC command — register a folder as a scan root (ADR-013) and walk it for
/// candidate Agentheim projects. The root is canonicalised + persisted FIRST,
/// so an empty subtree still leaves a rescannable root behind. `depth_cap` is
/// optional; `None` uses the ADR-005 / ADR-013 default of 3.
///
/// Importing the picked candidates is `project-registry-002b`'s job; this
/// command never touches `projects`.
#[tauri::command]
fn add_scan_root(
    state: tauri::State<'_, AppState>,
    path: String,
    depth_cap: Option<u32>,
) -> Result<AddScanRootResult, String> {
    let depth = depth_cap.unwrap_or(DEFAULT_SCAN_DEPTH_CAP);

    let canonical = scan::canonicalize_root(std::path::Path::new(&path)).map_err(|e| {
        tracing::warn!(error = %e, path = %path, "add_scan_root: canonicalisation failed");
        e.to_string()
    })?;
    let canonical_str = canonical.to_string_lossy().into_owned();

    let scan_root_id = state
        .db
        .upsert_scan_root(&canonical_str, depth)
        .map_err(|e| {
            tracing::error!(error = %e, path = %canonical_str, "add_scan_root: db insert failed");
            e.to_string()
        })?;

    let known = state.db.list_project_paths().map_err(|e| {
        tracing::error!(error = %e, "add_scan_root: list_project_paths failed");
        e.to_string()
    })?;
    let known_set: HashSet<String> = known.into_iter().collect();

    let candidates = scan::walk_scan_root(&canonical, depth, &known_set);
    tracing::info!(
        scan_root_id,
        path = %canonical_str,
        candidate_count = candidates.len(),
        "add_scan_root: walk complete"
    );

    Ok(AddScanRootResult {
        scan_root_id,
        candidates,
    })
}

/// IPC command — re-walk an already-registered scan root (ADR-013). Returns a
/// fresh candidate checklist; previously-imported candidates are flagged via
/// `already_imported` so the UI can grey them out and surface only the new
/// arrivals.
#[tauri::command]
fn rescan_scan_root(
    state: tauri::State<'_, AppState>,
    scan_root_id: i64,
) -> Result<Vec<ScanCandidate>, String> {
    let row = state
        .db
        .get_scan_root(scan_root_id)
        .map_err(|e| {
            tracing::error!(error = %e, scan_root_id, "rescan_scan_root: db lookup failed");
            e.to_string()
        })?
        .ok_or_else(|| {
            tracing::warn!(scan_root_id, "rescan_scan_root: unknown scan_root_id");
            format!("unknown scan_root_id: {scan_root_id}")
        })?;

    let known = state.db.list_project_paths().map_err(|e| {
        tracing::error!(error = %e, "rescan_scan_root: list_project_paths failed");
        e.to_string()
    })?;
    let known_set: HashSet<String> = known.into_iter().collect();

    let candidates = scan::walk_scan_root(
        std::path::Path::new(&row.path),
        row.depth_cap,
        &known_set,
    );
    tracing::info!(
        scan_root_id,
        candidate_count = candidates.len(),
        "rescan_scan_root: walk complete"
    );
    Ok(candidates)
}

/// IPC command — list every registered scan root (ADR-013). Empty list is
/// valid; the UI shows the empty state.
#[tauri::command]
fn list_scan_roots(state: tauri::State<'_, AppState>) -> Result<Vec<ScanRootRow>, String> {
    state.db.list_scan_roots().map_err(|e| {
        tracing::error!(error = %e, "list_scan_roots: db query failed");
        e.to_string()
    })
}

/// IPC command — import the user's checklist picks from a scan root's walk
/// into the registry (`project-registry-002b`, ADR-013). For each picked path:
///
/// 1. The path is re-verified against a *fresh* walk of the root's current
///    subtree — the frontend's set is not trusted, since the filesystem may
///    have shifted between `add_scan_root` and the user's tick-and-confirm.
///    A path outside the freshly-computed candidate set is rejected (skipped
///    + logged); not silently registered.
/// 2. `Db::upsert_scanned_project` stamps the discovering `scan_root_id` so
///    the cascade-deregister (`remove_scan_root`) can find it later.
/// 3. `WatcherSupervisor::add` arms the `.agentheim/` watcher for the new
///    project_id (ADR-008). Missing `.agentheim/` is *not* fatal — the project
///    stays registered-but-unwatched per ADR-005 and the supervisor's existing
///    contract.
///
/// Returns the imported project ids in input order — minus any that were
/// rejected as out-of-set, so callers can diff against the request to see
/// what was skipped.
///
/// Importing the same path twice is harmless: `upsert_scanned_project` is
/// idempotent on the canonical path; the supervisor's `add` is idempotent on
/// the project id.
#[tauri::command]
fn import_scanned_projects(
    state: tauri::State<'_, AppState>,
    scan_root_id: i64,
    paths: Vec<String>,
) -> Result<Vec<i64>, String> {
    // Look up the root first — an unknown id is a clean IPC error, never a
    // panic; mirrors `rescan_scan_root`'s shape.
    let root = state
        .db
        .get_scan_root(scan_root_id)
        .map_err(|e| {
            tracing::error!(error = %e, scan_root_id, "import_scanned_projects: db lookup failed");
            e.to_string()
        })?
        .ok_or_else(|| {
            tracing::warn!(scan_root_id, "import_scanned_projects: unknown scan_root_id");
            format!("unknown scan_root_id: {scan_root_id}")
        })?;

    // Re-walk the root NOW. The frontend's set is advisory; the source of
    // truth is the live filesystem. Out-of-set paths are skipped, not
    // silently imported — the acceptance criterion is explicit about this.
    let known = state.db.list_project_paths().map_err(|e| {
        tracing::error!(error = %e, "import_scanned_projects: list_project_paths failed");
        e.to_string()
    })?;
    let known_set: HashSet<String> = known.into_iter().collect();
    let candidates = scan::walk_scan_root(
        std::path::Path::new(&root.path),
        root.depth_cap,
        &known_set,
    );
    let candidate_paths: HashSet<&str> =
        candidates.iter().map(|c| c.path.as_str()).collect();

    let mut imported = Vec::with_capacity(paths.len());
    for path in &paths {
        if !candidate_paths.contains(path.as_str()) {
            tracing::warn!(
                scan_root_id,
                path = %path,
                "import_scanned_projects: path not in candidate set; skipping"
            );
            continue;
        }

        // Nickname: the candidate's `nickname_suggestion`, which is the
        // folder name (matches the scan walker's contract).
        let nickname = candidates
            .iter()
            .find(|c| c.path == *path)
            .map(|c| c.nickname_suggestion.clone())
            .unwrap_or_else(|| path.clone());

        let project_id = state
            .db
            .upsert_scanned_project(path, &nickname, scan_root_id)
            .map_err(|e| {
                tracing::error!(
                    error = %e,
                    path = %path,
                    scan_root_id,
                    "import_scanned_projects: db upsert failed"
                );
                e.to_string()
            })?;

        // Arm the watcher. Missing `.agentheim/` is the registered-but-
        // unwatched state (ADR-005); log and continue rather than abort the
        // whole batch.
        if let Err(e) = state
            .supervisor
            .add(project_id, std::path::Path::new(path))
        {
            tracing::warn!(
                error = %e,
                project_id,
                path = %path,
                "import_scanned_projects: supervisor.add failed; project stays registered-but-unwatched"
            );
        }
        imported.push(project_id);
    }

    tracing::info!(
        scan_root_id,
        requested = paths.len(),
        imported = imported.len(),
        "import_scanned_projects: complete"
    );
    Ok(imported)
}

/// IPC command — remove a scan root, **cascade-deregistering** every project
/// discovered under it (`project-registry-002b`, ADR-013). The cascade is
/// app-driven, not DB-level, because tearing down each project's
/// `WatcherSupervisor` entry must happen in application code (the `notify`
/// watcher cannot be torn down by SQLite). The DB's `ON DELETE RESTRICT` FK
/// makes the ordering a checked invariant: if any child still references the
/// root, the final `delete_scan_root` will fail loud rather than orphan rows.
///
/// Order:
///   1. Enumerate child project ids (`scan_root_id = ?`).
///   2. Per child: `supervisor.remove(id)` then `db.remove_project(id)`.
///   3. Delete the `scan_roots` row last.
///
/// **Cascade hard-deletes** — ADR-005's 30-day tile-state retention is scoped
/// to the user-initiated single "Remove project" affordance (`canvas-005`),
/// *not* to scan-root cascade-deregister. Manually-added projects (NULL
/// `scan_root_id`) are NEVER touched.
///
/// Unknown `scan_root_id` is a clean error, never a panic.
#[tauri::command]
fn remove_scan_root(
    state: tauri::State<'_, AppState>,
    scan_root_id: i64,
) -> Result<(), String> {
    // Verify the root exists before tearing anything down — an unknown id is
    // a clean IPC error, not a silent no-op (the user picked it from the UI;
    // a non-existent id is a bug, not an idempotent re-remove).
    if state
        .db
        .get_scan_root(scan_root_id)
        .map_err(|e| {
            tracing::error!(error = %e, scan_root_id, "remove_scan_root: db lookup failed");
            e.to_string()
        })?
        .is_none()
    {
        tracing::warn!(scan_root_id, "remove_scan_root: unknown scan_root_id");
        return Err(format!("unknown scan_root_id: {scan_root_id}"));
    }

    let children = state
        .db
        .list_projects_by_scan_root(scan_root_id)
        .map_err(|e| {
            tracing::error!(error = %e, scan_root_id, "remove_scan_root: enumerate children failed");
            e.to_string()
        })?;

    // Tear down each child: supervisor first, then DB row. The supervisor's
    // `remove` is a silent no-op for unknown ids, so a child that was
    // registered-but-unwatched (ADR-005 "missing" state) is fine.
    for project_id in &children {
        state.supervisor.remove(*project_id);
        if let Err(e) = state.db.remove_project(*project_id) {
            tracing::error!(
                error = %e,
                project_id,
                scan_root_id,
                "remove_scan_root: child remove_project failed; aborting cascade"
            );
            return Err(e.to_string());
        }
    }

    // All children gone — the RESTRICT FK no longer blocks the root delete.
    state.db.delete_scan_root(scan_root_id).map_err(|e| {
        tracing::error!(
            error = %e,
            scan_root_id,
            "remove_scan_root: delete_scan_root failed after children cleared"
        );
        e.to_string()
    })?;

    tracing::info!(
        scan_root_id,
        cascaded = children.len(),
        "remove_scan_root: cascade-deregister complete"
    );
    Ok(())
}

/// IPC command — persist a project tile's position on drag (ADR-004). Takes
/// `project_id` explicitly: the registry no longer rides on `AppState`.
#[tauri::command]
fn save_tile_position(
    state: tauri::State<'_, AppState>,
    project_id: i64,
    x: f64,
    y: f64,
) -> Result<(), String> {
    state
        .db
        .save_tile_position(project_id, x, y)
        .map_err(|e| {
            tracing::error!(error = %e, project_id, "save_tile_position failed");
            e.to_string()
        })
}

/// IPC command — read back a project's persisted tile position, if any.
#[tauri::command]
fn load_tile_position(
    state: tauri::State<'_, AppState>,
    project_id: i64,
) -> Result<Option<(f64, f64)>, String> {
    state
        .db
        .tile_position(project_id)
        .map_err(|e| e.to_string())
}

/// IPC command — persist the camera (pan + zoom) as a JSON blob in `app_state`.
#[tauri::command]
fn save_camera(state: tauri::State<'_, AppState>, camera: String) -> Result<(), String> {
    state
        .db
        .set_app_state("camera", &camera)
        .map_err(|e| e.to_string())
}

/// IPC command — read back the persisted camera, if any.
#[tauri::command]
fn load_camera(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    state.db.app_state("camera").map_err(|e| e.to_string())
}

// --- PTY spike IPC (infrastructure-013-pty-spike, ADR-006) ---------------
//
// These commands let the empirical PTY spike be driven hands-on from a live
// `pnpm tauri dev` session — the part of the ADR-006 DoD (long-running
// stability across idle/active periods; orphan check after a force-crash)
// that cannot be exercised by `cargo test` alone. The automated mechanics
// are proven by `pty.rs`'s tests; these commands expose the same actor so
// Marco can confirm the hands-on criteria against the real `claude.exe`.

/// Spawn `claude.exe` in the given project's folder, wrapped in a Windows
/// Job Object, with its read loop streaming `SessionOutput` onto the bus.
/// Replaces any existing spike session. `project-registry-001`: the cwd is
/// resolved through the registry (`Db::project_path`) rather than being read
/// off `AppState`.
#[tauri::command]
fn pty_spawn_claude(
    state: tauri::State<'_, AppState>,
    project_id: i64,
) -> Result<i64, String> {
    let project_path = state
        .db
        .project_path(project_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown project_id: {project_id}"))?;
    let session_id = 1;
    let session = ClaudeSession::spawn(
        session_id,
        "claude.exe",
        &[],
        &project_path,
        state.bus.clone(),
    )
    .map_err(|e| {
        tracing::error!(error = %e, project_id, "pty_spawn_claude failed");
        e.to_string()
    })?;
    // Dropping the previous session (if any) runs its ADR-006 cleanup path.
    *state.claude_session.lock().unwrap() = Some(session);
    Ok(session_id)
}

/// Write input bytes to the live spike session.
#[tauri::command]
fn pty_write(state: tauri::State<'_, AppState>, input: String) -> Result<(), String> {
    let mut guard = state.claude_session.lock().unwrap();
    let session = guard.as_mut().ok_or("no live claude session")?;
    session.write(input.as_bytes()).map_err(|e| e.to_string())
}

/// Resize the live spike session's terminal.
#[tauri::command]
fn pty_resize(
    state: tauri::State<'_, AppState>,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let guard = state.claude_session.lock().unwrap();
    let session = guard.as_ref().ok_or("no live claude session")?;
    session.resize(rows, cols).map_err(|e| e.to_string())
}

/// Kill the live spike session (the explicit "end session" path). Dropping it
/// also closes the Job Object handle.
#[tauri::command]
fn pty_kill(state: tauri::State<'_, AppState>) -> Result<(), String> {
    // Taking the session out of the slot drops it -> ADR-006 cleanup runs.
    let session = state.claude_session.lock().unwrap().take();
    match session {
        Some(_) => Ok(()),
        None => Err("no live claude session".into()),
    }
}

/// Whether the spike session's child is still alive.
#[tauri::command]
fn pty_is_alive(state: tauri::State<'_, AppState>) -> bool {
    let mut guard = state.claude_session.lock().unwrap();
    guard.as_mut().map(|s| s.is_alive()).unwrap_or(false)
}

/// IPC command — ADR-010's frontend log forwarding. `console.*` in the WebView
/// is routed here so frontend logs land in the same file as core logs.
#[tauri::command]
fn log_from_frontend(level: String, message: String) {
    match level.as_str() {
        "error" => tracing::error!(target: "frontend", "{message}"),
        "warn" => tracing::warn!(target: "frontend", "{message}"),
        "debug" => tracing::debug!(target: "frontend", "{message}"),
        _ => tracing::info!(target: "frontend", "{message}"),
    }
}

/// The Tauri application entry point, called from `main.rs`.
pub fn run() {
    // Bus is created before the app so producers (the watcher) and the
    // frontend bridge can both be wired during `setup`.
    let bus = EventBus::new();

    tauri::Builder::default()
        .manage(bus.clone())
        .setup(move |app| {
            // --- ADR-010: logging to %APPDATA%\guppi\logs ---------------
            let log_dir = app
                .path()
                .app_config_dir()
                .map(|d| d.join("logs"))
                .unwrap_or_else(|_| PathBuf::from("logs"));
            // The guard must outlive the process; hand it to Tauri to own.
            match logging::init(&log_dir) {
                Ok(guard) => {
                    app.manage(guard);
                }
                Err(e) => eprintln!("WARNING: could not initialise file logging: {e}"),
            }

            // --- ADR-004: SQLite state in %APPDATA%\guppi -------------
            let config_dir = app.path().app_config_dir().map_err(|e| {
                format!("could not resolve app config dir: {e}")
            })?;
            std::fs::create_dir_all(&config_dir)?;
            let db_path = config_dir.join("guppi.db");
            let db = Arc::new(
                Db::open(&db_path)
                    .map_err(|e| format!("could not open state database: {e}"))?,
            );
            tracing::info!(db = %db_path.display(), "state database ready");

            // --- ADR-005: register the one hard-coded project ----------
            // `project-registry-001`: the seed stays so the app is not
            // stranded at zero projects before `project-registry-002` lands.
            // It is now routed through `WatcherSupervisor::add`, which also
            // publishes `ProjectAdded` — `setup()` no longer publishes it
            // itself.
            let project_path = PathBuf::from(HARDCODED_PROJECT_PATH);
            // Verify `.agentheim/` exists before going further — the
            // skeleton's startup check; the supervisor would also reject it
            // but a missing seed at startup is an outright bootstrap failure
            // worth surfacing early.
            if !project_path.join(".agentheim").is_dir() {
                return Err(format!(
                    "hard-coded project has no .agentheim directory: {}",
                    project_path.display()
                )
                .into());
            }
            let project_id = db
                .upsert_project(
                    &project_path.to_string_lossy(),
                    "GUPPI",
                )
                .map_err(|e| format!("could not register project: {e}"))?;

            let supervisor = WatcherSupervisor::new(bus.clone());

            app.manage(AppState {
                db: db.clone(),
                supervisor: supervisor.clone(),
                bus: bus.clone(),
                claude_session: Mutex::new(None),
            });

            // --- ADR-009: the one frontend-bridge task -----------------
            // This is the ONLY place Tauri's `emit` is called for domain
            // events. It forwards the frontend-relevant subset to the WebView
            // under a single event name. The fine-grained filesystem events
            // let the frontend patch its model in place; a lagged receiver has
            // *lost* events it cannot reconstruct, so the bridge emits
            // `ResyncRequired` — the one signal that makes the frontend
            // re-fetch the whole snapshot (ADR-009 lag-resync strategy).
            let app_handle = app.handle().clone();
            let mut rx = bus.subscribe();
            tauri::async_runtime::spawn(async move {
                loop {
                    match rx.recv().await {
                        Ok(event) => {
                            if let Err(e) = app_handle.emit(FRONTEND_EVENT, &event) {
                                tracing::warn!(error = %e, "failed to emit event to frontend");
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            // ADR-009: never block; treat as "resync from
                            // source of truth". The bridge has lost events it
                            // cannot reconstruct, so it emits `ResyncRequired`
                            // — the one event that makes the frontend re-fetch
                            // the whole `get_project` snapshot.
                            tracing::warn!(skipped = n, "event bridge lagged; signalling resync");
                            let _ = app_handle.emit(
                                FRONTEND_EVENT,
                                &DomainEvent::ResyncRequired { project_id },
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            // --- ADR-008: one debounced .agentheim watcher per project,
            // mediated by the `WatcherSupervisor` (project-registry-001) ----
            // The supervisor publishes `ProjectAdded` on a successful add — no
            // separate publish here.
            if let Err(e) = supervisor.add(project_id, &project_path) {
                tracing::error!(error = %e, "could not start filesystem watcher for seed project");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_projects,
            get_project,
            add_scan_root,
            rescan_scan_root,
            list_scan_roots,
            import_scanned_projects,
            remove_scan_root,
            save_tile_position,
            load_tile_position,
            save_camera,
            load_camera,
            log_from_frontend,
            pty_spawn_claude,
            pty_write,
            pty_resize,
            pty_kill,
            pty_is_alive,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GUPPI");
}
