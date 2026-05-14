---
id: infrastructure-010-logging
type: decision
status: done
scope: global
depends_on: [infrastructure-001-desktop-runtime]
related_adrs: [ADR-010]
completed: 2026-05-14
commit: 0c64059
---

# Decision: Logging and error reporting

## Context

Personal tool, no cloud, no team. Need enough observability to debug a crash yourself, not to satisfy a support team.

## Architect's recommendation

**`tracing` + `tracing-subscriber` + `tracing-appender`** writing to `%APPDATA%\guppi\logs\guppi.log` with daily rotation, keep 7 days. Frontend logs forwarded through a Tauri command into the same file. **No Sentry, no telemetry.** Crash dialog with "Open log folder" button.

## Acceptance criteria

- [x] ADR committed at `.agentheim/knowledge/decisions/ADR-010-logging.md`
- [x] Log retention window confirmed (default 7 days)

## Outcome

Logging decision recorded as **ADR-010** (Status: Accepted) at
`.agentheim/knowledge/decisions/ADR-010-logging.md`. Choice: `tracing` +
`tracing-subscriber` + `tracing-appender` writing to
`%APPDATA%\guppi\logs\guppi.log` with daily rotation, 7-day retention;
frontend logs forwarded via a `log_from_frontend` Tauri command into the same
file; no Sentry/telemetry; crash dialog with "Open log folder" button. The
7-day retention default was confirmed and accepted. No application code in
this task — implementation is left for a future build task.

## Notes — architect's ADR draft

### ADR-010: Logging and error reporting — `tracing` to rotating local files, no telemetry

**Status:** Proposed
**Scope:** global

**Context.** Personal tool, no cloud, no team. Need enough observability to debug a crash *yourself*, not to satisfy a support team.

**Decision.**
- **Rust core:** `tracing` + `tracing-subscriber` + `tracing-appender` writing to `%APPDATA%\guppi\logs\guppi.log` with daily rotation, keep 7 days.
- **Frontend:** `console.log/warn/error` forwarded through a Tauri command `log_from_frontend(level, message)` so frontend logs land in the same file.
- **No Sentry, no telemetry, no error reporting service.** If a crash needs investigating, you read the log.
- **Crash dialog:** On unhandled panic, write to log, show a native dialog with "Open log folder" button, exit cleanly.

**Consequences.**
- (+) Zero external dependencies, zero privacy concerns.
- (+) Easy to inspect, grep, share if you ever need help.
- (–) No aggregation across runs. Acceptable.

**Reversibility.** Trivial.
