---
id: infrastructure-015-log-retention-sweep
type: feature
status: done
completed: 2026-05-14
scope: global
depends_on:
  - infrastructure-012-walking-skeleton
related_adrs:
  - ADR-010
---

# Log retention — 7-day sweep of rotated log files

## Why

ADR-010 specifies `tracing-appender` writing daily-rotated logs to
`%APPDATA%\guppi\logs\`, **keeping the last 7 days** (older files deleted).

The walking skeleton (`infrastructure-012`) wired the **rotation** half:
`tracing-appender`'s `rolling::daily` writer rotates the file daily, producing
`guppi.log.YYYY-MM-DD` files. It did **not** wire the **retention** half —
`tracing-appender` does not prune old files on its own. Today the `logs/`
directory grows unbounded.

## What

Add a one-shot retention sweep that runs **at startup** and deletes rotated log
files older than the retention window.

- The sweep runs **once per process launch**, no background timer. GUPPI is a
  personal desktop app that gets restarted regularly, so unbounded growth
  within a single session is a non-issue (decision: Marco, 2026-05-14).
- A file's age is read by **parsing the `YYYY-MM-DD` date out of its filename**
  (`guppi.log.YYYY-MM-DD`), not from filesystem mtime — deterministic,
  testable with plain fixture files, immune to mtime drift (decision: Marco,
  2026-05-14).
- Natural home: a `sweep_retention(log_dir, window)` function in
  `src-tauri/src/logging.rs`, called from `logging::init()` (which already
  receives `log_dir` and runs in Tauri's `.setup()` hook — `lib.rs:210`).
  Calling it from inside `init()` keeps a single logging entry point.

## Acceptance criteria

- [ ] On startup, rotated log files in the log directory whose filename date is
      older than the retention window are deleted.
- [ ] The retention window is a single named constant (ADR-010: "the window is
      a one-line change if it proves too short"); default **7 days**.
- [ ] Age is determined by parsing `YYYY-MM-DD` from the `guppi.log.YYYY-MM-DD`
      filename — not from mtime.
- [ ] Files in the log directory that do **not** match the
      `guppi.log.YYYY-MM-DD` pattern are left untouched (the sweep never
      deletes anything it cannot positively date as a rotated GUPPI log).
- [ ] A deletion failure (locked file, permission error) logs a warning and the
      sweep continues — it never panics or aborts startup.
- [ ] A unit test exercises `sweep_retention` against a temp directory of
      fixture files: files dated within the window survive, files dated outside
      it are removed, and a non-matching file is left alone.

## Scope (out)

- Any change to the logging *format* or *destination* — those are settled by
  ADR-010 and implemented in the skeleton's `src-tauri/src/logging.rs`.
- A background timer / periodic re-sweep — explicitly decided against; startup
  only.

## Notes

Surfaced during `infrastructure-012-walking-skeleton`. The skeleton's
`src-tauri/src/logging.rs` module comment points here. Refined and promoted
2026-05-14 — the two open decisions (trigger timing, age source) were resolved
with Marco; no orchestrator round needed (ADR-010 leaves no architectural depth
to mine).

No ADR written: both design decisions (startup-only trigger, filename-date age
source) were settled with Marco during refinement and are recorded above, and
the retention policy itself is ADR-010. Nothing new to decide.

## Outcome

Added `sweep_retention(log_dir, window_days)` to `src-tauri/src/logging.rs`,
called once from `logging::init` after the subscriber is up. It reads each
file's age by parsing the `YYYY-MM-DD` date out of the `guppi.log.YYYY-MM-DD`
filename — not mtime — converting both the filename date and today's date to
proleptic-Gregorian day ordinals (no date crate added; the algorithm is
self-contained and handles month/year boundaries). Files older than the
`RETENTION_DAYS` named constant (default 7) are deleted; files that do not
match the rotated-log pattern — including the live `guppi.log` — are left
untouched; a deletion failure logs a warning and the sweep continues.

Four unit tests cover the epoch consistency between the two date paths, day
counting across a month boundary, rejection of malformed date strings, and the
end-to-end sweep against a temp directory of fixture files (within-window and
edge-of-window files survive, past-window file is deleted, the live undated log
and an unrelated file are untouched). Full lib suite: 30 passing.

Key file: `src-tauri/src/logging.rs`.
