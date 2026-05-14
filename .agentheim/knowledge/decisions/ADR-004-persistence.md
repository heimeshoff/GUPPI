---
id: ADR-004
title: Persistence — SQLite in the OS user-config directory
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-004-persistence]
related_adrs: [ADR-001]
---

# ADR-004: Persistence — SQLite in the OS user-config directory

**Status:** Accepted
**Scope:** global

## Context

GUPPI owns state that is separate from any project: known projects (paths +
nicknames + last-seen), tile positions per project, cluster groupings, the
last camera (pan + zoom), and UI preferences. None of this belongs inside any
project's `.agentheim/` directory — it is **Marco's view of the world**, not
project content, and it must survive independently of any single project.

The runtime is **Tauri 2** with a Rust core (ADR-001), so persistence lives on
the Rust side and is reached over IPC. Day one targets **Windows 11 only**;
macOS and Linux are kept architecturally possible but unvalidated, so the
storage choice must not assume a single OS path layout.

The choice came down to three options:

1. **Flat JSON file** — trivial and human-editable, but concurrent writes and
   schema migration become painful as the state model grows.
2. **SQLite** — battle-tested, atomic writes, easy migrations, queryable, a
   single file on disk. Slightly more ceremony than JSON.
3. **Embedded key-value store (`sled` / `redb`)** — Rust-native and fast, but
   no ad-hoc query support and a weaker migration story.

## Decision

Use **SQLite** for GUPPI's own state, accessed from the Rust core via
`rusqlite` (or `sqlx` if async plus compile-time-checked queries become
desirable later — both are acceptable; the team picks at implementation time).

The database is a **single file** stored in the OS user-config directory:

- Windows: `%APPDATA%\guppi\guppi.db`
  (i.e. `C:\Users\Marco\AppData\Roaming\guppi\guppi.db`)
- macOS: `~/Library/Application Support/guppi/guppi.db`
- Linux: `~/.config/guppi/guppi.db`

The path is resolved at runtime via Tauri's `path` API (`app_config_dir()`)
rather than hard-coded, so the same code works on every target as cross-platform
support is exercised. A `schema_version` table tracks migrations, applied in
ordered steps at startup.

### Initial schema sketch

Accepted as drafted by the architect:

- `projects(id, path, nickname, added_at, last_seen_at)`
- `tile_positions(project_id, x, y, width, height, cluster_id NULL)`
- `clusters(id, name, color)`
- `app_state(key, value)` — camera (`pan_x`, `pan_y`, `zoom`), last focus, and
  other small UI preferences
- `schema_version(version)` — single-row migration marker

This is a sketch, not a frozen contract: it establishes the shape of the
storage. Column types and constraints are settled when the persistence layer
is built.

## Consequences

- (+) Atomic, durable, and queryable. Backing up GUPPI's state is copying one
  file.
- (+) Migrations are a well-trodden path with SQLite; the `schema_version`
  table gives a clear discipline.
- (+) Easy future extension — for example a `sessions` table tracking spawned
  `claude` PIDs and their state — without reworking the storage layer.
- (+) Path resolution via Tauri's `path` API keeps the storage location correct
  on every OS if and when macOS/Linux are validated.
- (–) Slightly more upfront ceremony than a flat JSON file; the team must keep
  migration discipline (every schema change is an ordered, versioned step).

## Reversibility

High. SQLite is a single file with a stable, well-understood format. Exporting
to JSON — or moving to another store entirely — is a one-script job if the
decision is ever revisited.
