//! Logging — `tracing` to rotating local files, no telemetry (ADR-010).
//!
//! The Rust core writes to `%APPDATA%\guppi\logs\guppi.log` with daily
//! rotation. `tracing-appender`'s rolling writer handles rotation; the 7-day
//! retention sweep is a deliberate follow-up (see the infrastructure backlog)
//! — `tracing-appender` does not prune old files itself, so the skeleton logs
//! and rotates but does not yet delete. Frontend `console.*` forwarding via a
//! `log_from_frontend` command is wired in `lib.rs`.
//!
//! No Sentry, no telemetry, no error-reporting service — nothing leaves the
//! machine.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// Initialise the global tracing subscriber, writing daily-rotated logs into
/// `log_dir`. The returned [`WorkerGuard`] must be kept alive for the lifetime
/// of the process — dropping it flushes and stops the non-blocking writer.
///
/// `log_dir` is created if it does not exist. The caller resolves it from
/// Tauri's `path` API (`%APPDATA%\guppi\logs`) rather than hard-coding it.
pub fn init(log_dir: &Path) -> std::io::Result<WorkerGuard> {
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::daily(log_dir, "guppi.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Default to `info` for GUPPI's own crates; `RUST_LOG` overrides it.
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,guppi_lib=debug"));

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .init();

    tracing::info!("GUPPI logging initialised");
    Ok(guard)
}
