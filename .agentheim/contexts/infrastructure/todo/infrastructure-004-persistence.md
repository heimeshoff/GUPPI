---
id: infrastructure-004-persistence
type: decision
status: todo
scope: global
depends_on: [infrastructure-001-desktop-runtime]
---

# Decision: Persistence for GUPPI's own state

## Context

GUPPI owns state separate from any project: known projects (paths + nicknames + last-seen), tile positions per project, cluster groupings, last camera (pan + zoom), UI preferences. None of this belongs inside any project's `.agentheim/` — it's Marco's *view of the world*, not project content.

## Architect's recommendation

**SQLite** via `rusqlite` (or `sqlx`) in the OS user-config dir:
- Windows: `%APPDATA%\guppi\guppi.db`
- macOS: `~/Library/Application Support/guppi/guppi.db`
- Linux: `~/.config/guppi/guppi.db`

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-004-persistence.md`
- [ ] Initial schema sketch (projects, tile_positions, clusters, app_state) reviewed and either accepted or amended

## Notes — architect's ADR draft

### ADR-004: Persistence — SQLite in the OS user-config directory

**Status:** Proposed
**Scope:** global

**Context.** GUPPI owns state separate from any project: known projects (paths + nicknames + last-seen), tile positions per project, cluster groupings, last camera (pan + zoom), UI preferences. None of this belongs inside any project's `.agentheim/` — it's *Marco's view of the world*, not project content. Choice: flat JSON file vs SQLite vs a key-value store (sled, redb).

**Options considered.**
1. **JSON file** — Trivial, human-editable, fine until concurrent writes or schema migration is painful.
2. **SQLite** — Battle-tested, atomic writes, easy migrations, queryable, single file. Slightly more ceremony.
3. **Embedded KV (sled / redb)** — Rust-native, fast, but no ad-hoc query, and migration story is weaker.

**Decision.** Use **SQLite** via `rusqlite` (or `sqlx` if you want async + compile-time checked queries). Database file lives at:

- Windows: `%APPDATA%\guppi\guppi.db` (i.e. `C:\Users\Marco\AppData\Roaming\guppi\guppi.db`)
- macOS: `~/Library/Application Support/guppi/guppi.db`
- Linux: `~/.config/guppi/guppi.db`

Resolved via Tauri's `path` API (`app_config_dir()`). A single `schema_version` table tracks migrations.

**Initial schema sketch.**
- `projects(id, path, nickname, added_at, last_seen_at)`
- `tile_positions(project_id, x, y, width, height, cluster_id NULL)`
- `clusters(id, name, color)`
- `app_state(key, value)` — for camera (pan_x, pan_y, zoom), last focus, etc.

**Consequences.**
- (+) Atomic, durable, queryable. Trivial to back up — one file.
- (+) Easy future addition: a `sessions` table for tracking spawned `claude` PIDs and their state.
- (–) Slightly more upfront than JSON; needs a migration discipline.

**Reversibility.** High. SQLite → JSON export is a one-script job if we ever change our minds.
