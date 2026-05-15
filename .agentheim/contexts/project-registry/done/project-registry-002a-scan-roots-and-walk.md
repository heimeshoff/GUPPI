---
id: project-registry-002a-scan-roots-and-walk
type: feature
status: done
completed: 2026-05-15
commit: bace9fd
scope: bc
depends_on:
  - project-registry-001-multi-project-snapshot-model
related_adrs:
  - ADR-004
  - ADR-005
  - ADR-008
  - ADR-013
related_research: []
prior_art: []
---

# Scan roots + folder discovery walk

## Why

`project-registry-001` lets the registry *hold* N projects, but nothing
*populates* it beyond the one hardcoded seed. v1 needs a real way to get
projects in: the user hands GUPPI **scan roots** — folders GUPPI recursively
walks for `.agentheim/`-bearing subfolders.

This task is the **storage + discovery foundation**: the schema for persisted
scan roots, and the walk that turns a root into a checklist of candidate
projects. It is independently shippable — after this task you can add a scan
root and get a checklist back; *importing* picked candidates and removing roots
is the sibling task `project-registry-002b`.

Scan roots are a deliberate evolution beyond ADR-005's *one-shot* scan command
(which never persisted the picked folder), recorded in ADR-013; ADR-005 carries
a reconciliation note. The walk stays user-triggered — never unprompted
background disk-walking.

## What

**Schema (migration v1 → v2)** — extends ADR-004's sketch; DDL owned by ADR-013.
- New `scan_roots(id, path TEXT UNIQUE, depth_cap INTEGER NOT NULL DEFAULT 3,
  added_at TEXT NOT NULL)`. `path` is the canonical absolute path, normalised
  on insert.
- New nullable column `projects.scan_root_id INTEGER REFERENCES
  scan_roots(id) ON DELETE RESTRICT`. NULL = manually added (ADR-005 "Add
  project…"); non-NULL = discovered under that root. `ON DELETE RESTRICT` makes
  the app-driven cascade ordering (in `002b`) a checked invariant.
- Migration: a `current < 2` step adds the table + column; bump
  `CURRENT_SCHEMA_VERSION` to `2`. (SQLite `ALTER TABLE ADD COLUMN` with a
  `REFERENCES` clause is fine for a NULL-able column with no non-NULL default.)

**Scan walk — new module `src-tauri/src/scan.rs`**
- Recursively walks a scan root's subtree; a directory containing `.agentheim/`
  is a candidate. The walk does **not** descend into a directory once it is
  identified as an Agentheim project (nested projects-under-a-project are out
  of scope for v1).
- **Depth cap** — a remaining-depth counter seeded from the root's `depth_cap`
  (default 3, persisted per root).
- **Junk-dir pruning** — a `const SKIP_DIRS` list (`node_modules`, `.git`,
  `target`, `.svn`, `.hg`, `dist`, `build`, `.venv`) pruned by directory name
  before descending.
- **Canonicalisation** — every candidate path and every scan-root path is
  canonicalised (resolve, collapse symlinks, case-normalise on Windows) at the
  module boundary; the DB only ever stores canonical paths (ADR-005).
- Returns `Vec<ScanCandidate { path, nickname_suggestion, already_imported }>`.
  `already_imported` is computed against `projects.path` so the checklist (in
  `002b`'s UI consumer) can grey out / pre-mark known candidates.
- `scan.rs` depends on neither `AppState` nor IPC — unit-testable against temp
  dirs.

**`Db` methods (scan-root CRUD + candidate support)**
- Insert a scan root (canonical path, depth_cap, added_at) → `scan_root_id`.
- `list_scan_roots() -> Vec<ScanRootRow>` (`id, path, depth_cap, added_at`).
- Read a single scan root's row.
- A lookup that, given candidate canonical paths, reports which already exist
  in `projects` (drives `ScanCandidate.already_imported`).

**IPC commands**
- `add_scan_root(path, depth_cap: Option<u32>) -> { scan_root_id,
  candidates }` — canonicalises + persists the root **first**, then walks and
  returns the checklist. An empty root (no candidates) is valid and stays
  persisted/rescannable.
- `rescan_scan_root(scan_root_id) -> Vec<ScanCandidate>` — re-walks on demand.
- `list_scan_roots() -> Vec<ScanRootRow>`.

## Acceptance criteria

- [ ] A fresh DB opens at `schema_version` 2; a v1 DB migrates to v2 gaining
      the `scan_roots` table and `projects.scan_root_id` column without data
      loss — unit tested (migration test).
- [ ] `add_scan_root` canonicalises and persists the root, walks the subtree,
      and returns a candidate for every `.agentheim/`-bearing subfolder within
      `depth_cap` levels — integration tested against a temp tree.
- [ ] The walk prunes `node_modules` / `.git` / `target` (etc.) and does not
      descend past `depth_cap` — unit tested.
- [ ] The walk does not descend into a directory once it is identified as an
      Agentheim project — unit tested.
- [ ] `ScanCandidate.already_imported` is `true` for a candidate whose
      canonical path is already in `projects`, `false` otherwise — unit tested.
- [ ] Scan roots persist across app restarts (`list_scan_roots` returns them).
- [ ] A scan root with zero candidates is still persisted and rescannable.

## Notes

Surfaced from the v1 "finish v1 first" capture pass (2026-05-14); split from
the original `project-registry-002` during refinement (2026-05-15).

Decisions made during refinement (2026-05-15, with Marco):
- **Checklist kept** — add/rescan returns candidates; the user picks which to
  import (faithful to ADR-005). Importing is `002b`.
- **Depth cap default 3, junk-dir pruning** — reuses ADR-005's stated default;
  `depth_cap` is persisted per root and user-configurable.
- **Origin tracking** — `projects.scan_root_id` records the discovering root
  (NULL = manually added). Enables `002b`'s cascade-deregister.

Consumes `project-registry-001`'s surface (`Db`, `AppState` shape). The
`scan_root_id` column is *added* here but only *written* in `002b`
(`upsert_scanned_project`); here it is always NULL on the existing seed path.

Sibling: `project-registry-002b-import-and-cascade-deregister` builds the
mutation layer (import picked candidates, `remove_scan_root` cascade,
`Db::remove_project`) on this foundation.

## Outcome

Landed the scan-root storage + discovery foundation per ADR-013.

**Schema (v1 → v2):**
- New `scan_roots(id, path UNIQUE, depth_cap, added_at)` table.
- New nullable `projects.scan_root_id` column with `REFERENCES scan_roots(id)
  ON DELETE RESTRICT`. NULL = manually-added (ADR-005); non-NULL = discovered
  under that root. `RESTRICT` makes the `002b` app-driven cascade ordering a
  checked invariant.
- `CURRENT_SCHEMA_VERSION` bumped to `2`. Migration applies `ALTER TABLE
  projects ADD COLUMN scan_root_id …` — additive over v1, no data loss.

**New module `src-tauri/src/scan.rs`:**
- `SKIP_DIRS` const list (`node_modules`, `.git`, `target`, `.svn`, `.hg`,
  `dist`, `build`, `.venv`) — pruned by directory-name match before descent.
- `canonicalize_root(path)` — resolves, collapses symlinks, strips the
  Windows `\\?\` UNC prefix at the module boundary so the DB only ever stores
  ordinary `C:\…` form (ADR-005 canonicalisation guarantee).
- `walk_scan_root(root, depth_cap, known_paths) -> Vec<ScanCandidate>` —
  remaining-depth counter, junk-dir pruning, does NOT descend into a
  directory once it is identified as an Agentheim project (no nested
  projects in v1). Sorted output for determinism.
- `ScanCandidate { path, nickname_suggestion, already_imported }` — `Serialize`
  so it crosses IPC; `already_imported` is `HashSet` membership against
  `projects.path`.

**`Db` methods:** `upsert_scan_root` (idempotent on canonical path),
`list_scan_roots`, `get_scan_root`, `list_project_paths` (drives
`already_imported`). `ScanRootRow` is `Serialize` for IPC.

**IPC commands (lib.rs):**
- `add_scan_root(path, depth_cap?) -> { scan_root_id, candidates }` —
  canonicalises and persists FIRST so an empty subtree still leaves a
  rescannable root, then walks.
- `rescan_scan_root(scan_root_id) -> Vec<ScanCandidate>`.
- `list_scan_roots() -> Vec<ScanRootRow>`.

**Tests:** 19 new tests (60/60 total cargo tests green):
- Db: `fresh_db_is_at_schema_version_two`, `v1_db_migrates_to_v2_without_data_loss`
  (hand-rolled v1 db round-trip), scan-root CRUD idempotency / listing /
  lookup, `list_project_paths`, cross-restart persistence, empty-root
  persistence.
- Scan: empty-root, finds-at-each-depth, depth-cap exclusion, no-nested-descent,
  junk-pruning, `already_imported` flag, nickname suggestion, UNC-prefix
  stripping, missing-path error, plus a `add_scan_root`-shaped integration
  test that composes canonicalise → upsert → list-known → walk against a real
  temp tree.

**Key files:**
- `src-tauri/src/db.rs` — schema v2 migration, scan-root CRUD, `ScanRootRow`,
  `DEFAULT_SCAN_DEPTH_CAP`.
- `src-tauri/src/scan.rs` — new module; the walker.
- `src-tauri/src/lib.rs` — three new IPC commands, registered in
  `invoke_handler`.
- `.agentheim/contexts/project-registry/README.md` — ubiquitous-language entries
  for *scan root*, *scan candidate*, *origin tracking*; resolved the
  "where does GUPPI look for projects?" open question.

**Frontend untouched** (canvas-002 owns `src/lib/*` in parallel). IPC
contracts are additive — no existing commands changed.

**`scan_root_id` column** is written by `002b`'s `upsert_scanned_project`;
this task only adds the column and leaves it NULL on the seed-project upsert
path. Clippy clean for new code (one pre-existing warning in
`supervisor.rs` left untouched, out of scope).
