---
id: ADR-010
title: Logging and error reporting — `tracing` to rotating local files, no telemetry
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-010-logging]
---

# ADR-010: Logging and error reporting — `tracing` to rotating local files, no telemetry

**Status:** Accepted
**Scope:** global

## Context

GUPPI is a personal tool: no cloud, no team, no support organisation. The
only observability requirement is being able to debug a crash *yourself*
after the fact — not to satisfy a support desk or aggregate metrics across a
fleet.

The runtime is **Tauri 2** with a Rust core and a web-tech frontend
(ADR-001). That split means logs originate in two places — the Rust core
(PTY, filesystem watching, sqlite, voice IPC) and the frontend (canvas, tile
UI) — and a useful log needs both streams in one place, in order.

Anything heavier than local files (Sentry, a telemetry service, crash-report
upload) adds an external dependency and a privacy surface for zero benefit on
a single-user machine.

## Decision

Use **`tracing` + `tracing-subscriber` + `tracing-appender`** for all
logging.

- **Rust core:** `tracing-appender` writes to
  `%APPDATA%\guppi\logs\guppi.log` with **daily rotation**, keeping the last
  **7 days** (older files are deleted). `tracing-subscriber` formats entries
  with timestamp, level, and target.
- **Frontend:** `console.log` / `console.warn` / `console.error` are
  forwarded through a Tauri command `log_from_frontend(level, message)` so
  frontend logs land in the **same file** as core logs, interleaved by time.
- **No Sentry, no telemetry, no error-reporting service.** If a crash needs
  investigating, you read the log.
- **Crash dialog:** on an unhandled panic, write the panic to the log, show a
  native dialog with an **"Open log folder"** button, and exit cleanly.

## Consequences

- (+) Zero external dependencies and zero privacy concerns — nothing leaves
  the machine.
- (+) A single plain-text file is trivial to inspect, `grep`, and share if
  you ever do want help debugging.
- (+) `tracing` spans give structured context (which PTY session, which
  project) without bespoke logging plumbing.
- (–) No aggregation or trend analysis across runs. Accepted — there is one
  user on one machine.
- (–) The 7-day retention window means a bug reproduced only occasionally may
  age out of the logs before it is investigated. Accepted as the default;
  the window is a one-line change if it proves too short.

## Reversibility

Trivial. Adding a telemetry sink later is an additive `tracing` layer; the
retention window and log path are configuration. Nothing about this decision
is load-bearing for other components.
