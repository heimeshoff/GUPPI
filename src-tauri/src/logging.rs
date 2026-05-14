//! Logging — `tracing` to rotating local files, no telemetry (ADR-010).
//!
//! The Rust core writes to `%APPDATA%\guppi\logs\guppi.log` with daily
//! rotation. `tracing-appender`'s rolling writer handles rotation but does not
//! prune old files itself, so [`sweep_retention`] runs once at startup (from
//! [`init`]) and deletes rotated `guppi.log.YYYY-MM-DD` files older than the
//! [`RETENTION_DAYS`] window. Frontend `console.*` forwarding via a
//! `log_from_frontend` command is wired in `lib.rs`.
//!
//! No Sentry, no telemetry, no error-reporting service — nothing leaves the
//! machine.

use std::path::Path;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// How many days of rotated log files to keep (ADR-010: "keeping the last 7
/// days"). ADR-010 notes the window "is a one-line change if it proves too
/// short" — this constant is that one line.
pub const RETENTION_DAYS: i64 = 7;

/// Prefix of a rotated log file: `tracing-appender`'s `rolling::daily` writer
/// produces `guppi.log.YYYY-MM-DD`.
const ROTATED_PREFIX: &str = "guppi.log.";

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

    // ADR-010 retention half: rotation is `tracing-appender`'s job, pruning is
    // ours. One-shot sweep at startup — GUPPI is a personal desktop app that
    // restarts regularly, so no background timer is needed.
    sweep_retention(log_dir, RETENTION_DAYS);

    Ok(guard)
}

/// Delete rotated log files in `log_dir` whose filename date is more than
/// `window_days` days before today.
///
/// A file's age is read by parsing the `YYYY-MM-DD` date out of its
/// `guppi.log.YYYY-MM-DD` filename — *not* from filesystem mtime — so the
/// behaviour is deterministic and immune to mtime drift. Any file in the
/// directory that does not match that exact pattern is left untouched: the
/// sweep never deletes anything it cannot positively date as a rotated GUPPI
/// log. A deletion that fails (locked file, permission error) logs a warning
/// and the sweep continues — it never panics or aborts startup.
pub fn sweep_retention(log_dir: &Path, window_days: i64) {
    let today = match today_ordinal() {
        Some(t) => t,
        None => {
            tracing::warn!("retention sweep skipped: could not read current date");
            return;
        }
    };

    let entries = match std::fs::read_dir(log_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(error = %e, "retention sweep skipped: could not read log directory");
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        // Only positively-dated rotated GUPPI logs are eligible for deletion.
        let date_str = match file_name.strip_prefix(ROTATED_PREFIX) {
            Some(d) => d,
            None => continue,
        };
        let file_ordinal = match parse_date_ordinal(date_str) {
            Some(o) => o,
            None => continue,
        };

        if today - file_ordinal > window_days {
            match std::fs::remove_file(&path) {
                Ok(()) => {
                    tracing::info!(file = %file_name, "deleted expired log file");
                }
                Err(e) => {
                    tracing::warn!(
                        error = %e,
                        file = %file_name,
                        "could not delete expired log file; continuing sweep"
                    );
                }
            }
        }
    }
}

/// Parse a `YYYY-MM-DD` string into a day ordinal (days since 0000-03-01,
/// proleptic Gregorian). The epoch is arbitrary — only *differences* between
/// ordinals are used — but the algorithm handles month and year boundaries
/// correctly. Returns `None` for any string that is not a valid date.
fn parse_date_ordinal(date_str: &str) -> Option<i64> {
    let mut parts = date_str.split('-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Reject impossible days (e.g. 2026-02-30) by round-tripping is overkill
    // here; the day-count algorithm below tolerates 1..=31 and the filenames
    // are machine-generated, so a basic range check is enough.

    // Days-since-epoch via the standard "shift March-based year" algorithm.
    let (y, m) = if month <= 2 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400; // year of era, 0..=399
    let doy = (153 * (m - 3) + 2) / 5 + day - 1; // day of year, 0..=365
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // day of era, 0..=146096
    Some(era * 146097 + doe)
}

/// Today's date as a day ordinal, derived from the system clock. Returns
/// `None` if the clock is set before the Unix epoch.
fn today_ordinal() -> Option<i64> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    // 1970-01-01 expressed in the same proleptic-Gregorian ordinal as
    // `parse_date_ordinal` (719468 days from the 0000-03-01 epoch).
    const UNIX_EPOCH_ORDINAL: i64 = 719_468;
    Some(UNIX_EPOCH_ORDINAL + secs / 86_400)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// `parse_date_ordinal` agrees with `today_ordinal`'s epoch: 1970-01-01
    /// parses to the same constant `today_ordinal` adds.
    #[test]
    fn unix_epoch_ordinal_is_consistent() {
        assert_eq!(parse_date_ordinal("1970-01-01"), Some(719_468));
    }

    /// Ordinal differences are real day counts, including across a month
    /// boundary.
    #[test]
    fn ordinal_differences_count_days() {
        let a = parse_date_ordinal("2026-05-14").unwrap();
        let b = parse_date_ordinal("2026-05-07").unwrap();
        assert_eq!(a - b, 7);
        // Across a month boundary: 2026-03-01 minus 2026-02-28 is 1 day.
        let m1 = parse_date_ordinal("2026-03-01").unwrap();
        let m0 = parse_date_ordinal("2026-02-28").unwrap();
        assert_eq!(m1 - m0, 1);
    }

    /// Malformed date strings are rejected, not silently accepted.
    #[test]
    fn malformed_dates_are_rejected() {
        assert_eq!(parse_date_ordinal("not-a-date"), None);
        assert_eq!(parse_date_ordinal("2026-13-01"), None);
        assert_eq!(parse_date_ordinal("2026-05"), None);
        assert_eq!(parse_date_ordinal("2026-05-14-extra"), None);
        assert_eq!(parse_date_ordinal(""), None);
    }

    /// The core acceptance test: against a temp directory of fixture files,
    /// files dated within the window survive, files dated outside it are
    /// removed, and a non-matching file is left alone.
    #[test]
    fn sweep_removes_only_expired_rotated_logs() {
        let dir = std::env::temp_dir().join(format!(
            "guppi-retention-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();

        // Anchor the fixtures off `today` so the test is not time-bombed.
        let today = today_ordinal().unwrap();
        let date_at = |offset_days: i64| -> String {
            // Walk back day-by-day from today to a YYYY-MM-DD string by brute
            // search — small offsets, runs once, keeps the test crate-free.
            let target = today - offset_days;
            for y in 1970..3000 {
                for m in 1..=12 {
                    for d in 1..=31 {
                        let s = format!("{y:04}-{m:02}-{d:02}");
                        if parse_date_ordinal(&s) == Some(target) {
                            return s;
                        }
                    }
                }
            }
            panic!("could not build date string for offset {offset_days}");
        };

        let fresh = format!("guppi.log.{}", date_at(0)); // today — survives
        let edge = format!("guppi.log.{}", date_at(RETENTION_DAYS)); // exactly window — survives
        let expired = format!("guppi.log.{}", date_at(RETENTION_DAYS + 1)); // over window — deleted
        let active = "guppi.log"; // the live file — not dated, untouched
        let stranger = "notes.txt"; // unrelated file — untouched

        for name in [fresh.as_str(), edge.as_str(), expired.as_str(), active, stranger] {
            fs::write(dir.join(name), b"x").unwrap();
        }

        sweep_retention(&dir, RETENTION_DAYS);

        assert!(dir.join(&fresh).exists(), "today's log must survive");
        assert!(dir.join(&edge).exists(), "log exactly at the window edge must survive");
        assert!(!dir.join(&expired).exists(), "log past the window must be deleted");
        assert!(dir.join(active).exists(), "the active undated log must be untouched");
        assert!(dir.join(stranger).exists(), "a non-matching file must be untouched");

        fs::remove_dir_all(&dir).ok();
    }
}
