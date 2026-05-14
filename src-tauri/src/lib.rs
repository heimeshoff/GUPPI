//! GUPPI's Rust core — the walking skeleton (`infrastructure-012`).
//!
//! This module wires the pieces the eleven foundation ADRs settled into one
//! running Tauri 2 app:
//!
//! - **ADR-001** Tauri 2, Rust core / web frontend, IPC via `invoke`/`emit`.
//! - **ADR-004** SQLite state in the OS user-config dir (`db`).
//! - **ADR-005** the one hard-coded project is `upsert`ed into the registry.
//! - **ADR-008** one debounced `.agentheim/` watcher (`watcher`).
//! - **ADR-009** in-core `EventBus` + a single frontend-bridge task that is
//!   the *only* place Tauri's `emit` is called for domain events (`events`).
//! - **ADR-010** `tracing` to rotating local files (`logging`).
//!
//! Out of skeleton scope by the task contract: PTY (ADR-006), voice (ADR-007),
//! multi-project / registry UI, packaging mechanics (ADR-011 — `tauri build`
//! config lives in `tauri.conf.json`, exercised separately).

mod db;
mod events;
mod logging;
mod project;
mod watcher;

use db::Db;
use events::{DomainEvent, EventBus};
use project::ProjectSnapshot;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};
use watcher::AgentheimWatcher;

/// The Tauri event name the frontend listens on (ADR-009: a single event name
/// with a JSON payload).
const FRONTEND_EVENT: &str = "guppi://event";

/// The walking skeleton's one hard-coded project. The task says "Marco picks
/// one, e.g. `C:\src\heimeshoff\agentic\guppi`" — which is GUPPI's own repo,
/// the project this code lives in, so it is guaranteed to be a real Agentheim
/// project on this machine.
const HARDCODED_PROJECT_PATH: &str = r"C:\src\heimeshoff\agentic\guppi";

/// Application state shared with Tauri commands as managed state.
struct AppState {
    db: Arc<Db>,
    project_id: i64,
    project_path: PathBuf,
}

/// IPC command — ADR-005's `get_project`. Reads the hard-coded project off
/// disk into a `ProjectSnapshot` for the canvas.
#[tauri::command]
fn get_project(state: tauri::State<'_, AppState>) -> Result<ProjectSnapshot, String> {
    project::get_project(&state.project_path).map_err(|e| {
        tracing::error!(error = %e, "get_project failed");
        e.to_string()
    })
}

/// IPC command — persist the project tile's position on drag (ADR-004).
#[tauri::command]
fn save_tile_position(
    state: tauri::State<'_, AppState>,
    x: f64,
    y: f64,
) -> Result<(), String> {
    state
        .db
        .save_tile_position(state.project_id, x, y)
        .map_err(|e| {
            tracing::error!(error = %e, "save_tile_position failed");
            e.to_string()
        })
}

/// IPC command — read back the persisted tile position, if any.
#[tauri::command]
fn load_tile_position(
    state: tauri::State<'_, AppState>,
) -> Result<Option<(f64, f64)>, String> {
    state
        .db
        .tile_position(state.project_id)
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
            let project_path = PathBuf::from(HARDCODED_PROJECT_PATH);
            // Verify `.agentheim/` exists before going further — the skeleton
            // task's scope step 4 requires this check on startup.
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
            bus.publish(DomainEvent::ProjectAdded {
                project_id,
                path: project_path.to_string_lossy().into_owned(),
            });

            app.manage(AppState {
                db: db.clone(),
                project_id,
                project_path: project_path.clone(),
            });

            // --- ADR-009: the one frontend-bridge task -----------------
            // This is the ONLY place Tauri's `emit` is called for domain
            // events. It forwards the frontend-relevant subset to the WebView
            // under a single event name. A lagged receiver resyncs by simply
            // forwarding a change signal — the frontend re-fetches anyway.
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
                            // source of truth". Nudge the frontend to refetch.
                            tracing::warn!(skipped = n, "event bridge lagged; signalling resync");
                            let _ = app_handle.emit(
                                FRONTEND_EVENT,
                                &DomainEvent::AgentheimChanged { project_id },
                            );
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
            });

            // --- ADR-008: one debounced .agentheim watcher -------------
            match AgentheimWatcher::start(project_id, &project_path, bus.clone()) {
                // Keep the watcher alive for the process lifetime.
                Ok(w) => {
                    app.manage(w);
                }
                Err(e) => tracing::error!(error = %e, "could not start filesystem watcher"),
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_project,
            save_tile_position,
            load_tile_position,
            save_camera,
            load_camera,
            log_from_frontend,
        ])
        .run(tauri::generate_context!())
        .expect("error while running GUPPI");
}
