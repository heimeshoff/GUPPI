---
id: infrastructure-015-log-retention-sweep
type: feature
status: backlog
scope: global
depends_on:
  - infrastructure-012-walking-skeleton
related_adrs:
  - ADR-010
---

# Log retention — 7-day sweep of rotated log files

## Why

ADR-010 specifies `tracing-appender` writing daily-rotated logs to
`%APPDATA%\guppi\logs\guppi.log`, **keeping the last 7 days** (older files
deleted).

The walking skeleton (`infrastructure-012`) wired the **rotation** half:
`tracing-appender`'s `rolling::daily` writer rotates the file daily. It did
**not** wire the **retention** half — `tracing-appender` does not prune old
files on its own. Today the `logs/` directory grows unbounded.

## Scope (in)

- On startup (and/or on a low-frequency timer), delete `guppi.log.*` files in
  the log directory older than 7 days.
- Make the retention window a single named constant (ADR-010: "the window is a
  one-line change if it proves too short").
- Test the sweep against a directory of dated fixture files.

## Scope (out)

- Any change to the logging *format* or *destination* — those are settled by
  ADR-010 and implemented in the skeleton's `src-tauri/src/logging.rs`.

## Notes

Surfaced during `infrastructure-012-walking-skeleton`. The skeleton's
`src-tauri/src/logging.rs` module comment points here.
