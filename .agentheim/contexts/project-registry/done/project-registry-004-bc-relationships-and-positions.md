---
id: project-registry-004-bc-relationships-and-positions
type: feature
status: done
completed: 2026-05-16
commit: b3727f5
scope: bc
depends_on: []
related_adrs:
  - ADR-004
  - ADR-008
  - ADR-009
  - ADR-014
related_research: []
prior_art:
  - project-registry-001-multi-project-snapshot-model
  - project-registry-003-manual-add-remove-and-missing-projects
---

# BC relationships + per-BC position storage

## Why

`canvas-007-project-as-frame` reshapes the canvas so each project renders as
a frame containing its BCs as bubbles, with edges between BCs drawn by
context-map relationship type (upstream/downstream/mutual/none). For that to
work, the registry must surface two new things the canvas does not have
today:

1. **BC↔BC relationships** — currently `ProjectSnapshot.bcs` is a flat list;
   no edges, no relationship metadata.
2. **Per-BC positions** — `tile_positions` stores per-project positions;
   per-BC positions inside a frame have nowhere to live.

This task ships both, end-to-end, plus the bootstrap data for GUPPI's own
seven BCs.

## What

### Data convention: per-BC README frontmatter

The relationship data source is the **YAML frontmatter of each
`contexts/<bc>/README.md`**. Every README grows a `relationships:` block:

```yaml
---
name: canvas
classification: core
relationships:
  - to: project-registry
    type: customer-supplier
    direction: upstream
  - to: agent-awareness
    type: customer-supplier
    direction: upstream
  - to: infrastructure
    type: shared-kernel
---
```

Schema (Rust types):

```rust
enum RelationshipType {
    CustomerSupplier,        // directional; pair with Direction
    SharedKernel,            // non-directional
    Partnership,             // non-directional
    AntiCorruptionLayer,     // directional with notch
    Conformist,              // directional
}
enum Direction { Upstream, Downstream }
struct Relationship {
    to: String,              // sibling BC name within the same project
    r#type: RelationshipType,
    direction: Option<Direction>,  // None for non-directional types
}
```

`to` is the bare BC directory name (e.g., `project-registry`), resolved
within the same project. Cross-project `to` references are explicitly
ignored at v1 (the canvas doesn't render them and the parser drops them with
a warning log line).

### Parser + surfacing

- Extend the BC enumeration (currently in `project.rs`'s
  `read_bounded_contexts`) to parse README frontmatter. Use `serde_yaml`
  (already a candidate; if not present, add it). Malformed frontmatter
  fails gracefully — BC still enumerates, `relationships: vec![]`, single
  warning logged per occurrence.
- Add `relationships: Vec<Relationship>` to `BoundedContext` (in
  `src/lib/types.ts` mirror as `Relationship[]`).
- `ProjectSnapshot` carries the populated relationships through
  `list_projects` and `get_project`.

### Fine-grained live event

- New `bc_relationships_changed { project_id, bc_name }` domain event
  (ADR-009). Fires when the watcher sees a `contexts/<bc>/README.md` write
  and the parsed `relationships:` block has changed (deep-equal compare
  against the in-memory cached previous value).
- Wire through the frontend bridge so `Canvas.svelte`'s `onDomainEvent`
  receives it and `snapshot-patch.ts` mutates the in-memory snapshot's
  `bcs[bc_name].relationships` in place. Canvas re-runs its
  intra-project edge layout for that project (canvas-007 owns the
  visual side).
- README writes that *don't* change the parsed relationships do NOT fire
  the event (avoid spurious relayouts on prose edits).

### Per-BC position schema + IPC

- Schema v3 → v4. New table:

```sql
CREATE TABLE bc_positions (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    bc_name    TEXT    NOT NULL,
    x          REAL    NOT NULL,
    y          REAL    NOT NULL,
    PRIMARY KEY (project_id, bc_name)
);
```

- `ON DELETE CASCADE` so per-BC positions vanish with the project (matches
  `tile_positions` semantics for the cascade path; for ADR-005 soft-delete,
  follow the same "preserve through window, drop on hard-delete sweep"
  pattern as `tile_positions`).
- Two new IPC commands (mirroring `save_tile_position` /
  `load_tile_position` from project-registry-001):
  - `save_bc_position(project_id, bc_name, x, y) -> Result<(), String>`
  - `load_bc_position(project_id, bc_name) -> Option<{ x: f64, y: f64 }>`
- A third helper for batch load on project paint:
  - `load_bc_positions(project_id) -> HashMap<String, (f64, f64)>`

### Bootstrap

GUPPI's own seven BC READMEs (`canvas`, `project-registry`,
`claude-runner`, `agent-awareness`, `voice`, `design-system`,
`infrastructure`) get hand-curated `relationships:` frontmatter, translated
directly from `.agentheim/context-map.md`'s "Relationships" section. Done
in-task as a single edit pass.

## Acceptance criteria

- [ ] `serde_yaml` parses `relationships:` frontmatter from every
      `contexts/<bc>/README.md`; malformed frontmatter logs a warning and
      yields `relationships: []` without breaking BC enumeration.
- [ ] `BoundedContext` struct in `src-tauri/.../project.rs` and the
      `Relationship` / `RelationshipType` / `Direction` types are defined
      and serialised through `ProjectSnapshot`.
- [ ] `src/lib/types.ts` mirrors the new types exactly.
- [ ] `bc_relationships_changed { project_id, bc_name }` event fires on
      README writes that change the parsed relationships set, and does NOT
      fire on writes that don't.
- [ ] Schema v3→v4 migration creates `bc_positions` table with the cascade
      rule; existing data preserved.
- [ ] `save_bc_position` / `load_bc_position` / `load_bc_positions` IPC
      commands round-trip correctly; concurrent saves don't race.
- [ ] GUPPI's seven BC READMEs carry the `relationships:` frontmatter,
      reflecting `context-map.md`'s "Relationships" section accurately.
- [ ] `cargo test --lib` passes; new tests cover the parser, the
      change-detection in the event emitter, the migration, and the IPC
      round-trip.
- [ ] `pnpm check` 0/0/0.

## Scope (in)

- `src-tauri/src/project.rs` — frontmatter parsing in `read_bounded_contexts`.
- `src-tauri/src/types.rs` (or wherever BoundedContext lives) — new types.
- `src-tauri/src/db.rs` — schema v4, `bc_positions` CRUD.
- `src-tauri/src/events.rs` — new event variant + change-detection cache.
- `src-tauri/src/lib.rs` — three new IPC commands.
- `src/lib/types.ts`, `src/lib/ipc.ts` — TS mirrors + IPC wrappers.
- `.agentheim/contexts/*/README.md` — hand-curated frontmatter for GUPPI's
  own seven BCs.

## Scope (out)

- Brainstorm/model writing the frontmatter automatically going forward —
  captured separately in the Agentheim repo.
- Visual rendering of edges or BCs-inside-frame — `canvas-007` / `design-
  system-002`.
- Cross-project relationships. v2+.

## Notes

- **Why per-BC README frontmatter** (not `context-map.md` parsing, not a
  separate `relationships.yaml`): the README is where the BC explains
  itself; relationships are part of self-description. `context-map.md`
  stays as the human-readable narrative; brainstorm/model will eventually
  keep both in sync (Agentheim follow-up). A separate top-level file
  would put truth one hop away from the prose that explains it.
- **Watcher integration:** the existing ADR-008 watcher already fires on
  any file change inside `.agentheim/`. The `contexts/<bc>/README.md`
  case routes through the existing `agentheim_changed` -> targeted-event
  pathway; this task adds the `bc_relationships_changed` variant and the
  in-memory cache that decides whether to emit it.
- **Frontend impact carved out:** the canvas does **not** render relationship
  data in this task — that's `canvas-007`. This task ships only the data
  pipeline; canvas-007 consumes it. A worker on this task should not touch
  `Canvas.svelte` beyond verifying the new types serialise cleanly.

## Outcome

Shipped end-to-end. The registry now surfaces BC↔BC relationships parsed from
each BC's README YAML frontmatter, fires a fine-grained
`BcRelationshipsChanged` event only when the parsed set actually changes,
persists per-BC bubble positions in a new `bc_positions` table (schema v4),
and exposes both as IPC. GUPPI's seven BC READMEs carry the bootstrap
frontmatter translated from `context-map.md`'s "Relationships" section.

**Key files:**

- `src-tauri/src/project.rs` — renamed `BcSnapshot` → `BoundedContext`
  (per task spec), added `Relationship` / `RelationshipType` / `Direction`
  types, `parse_relationships()` with graceful degradation + cross-project
  drop logic + directionality validation.
- `src-tauri/src/db.rs` — schema v3→v4 migration creating `bc_positions`
  with `ON DELETE CASCADE` on `project_id`; `save_bc_position` /
  `bc_position` / `bc_positions` CRUD; `CURRENT_SCHEMA_VERSION = 4`.
- `src-tauri/src/events.rs` — new `BcRelationshipsChanged` variant on the
  ADR-009 enum.
- `src-tauri/src/watcher.rs` — per-project `RelationshipsCache` inside
  `AgentheimWatcher`; deep-equal change detection on README writes;
  `BcReadme` path classification.
- `src-tauri/src/lib.rs` — three new IPC commands (`save_bc_position`,
  `load_bc_position`, `load_bc_positions`).
- `src/lib/types.ts` — mirrored types (`Relationship`, `RelationshipType`,
  `Direction`, `BoundedContext`); `BcSnapshot` retained as a type alias for
  `BoundedContext` so `Canvas.svelte` continues to compile without an
  invasive rename (the "do not touch Canvas.svelte beyond verifying types
  serialise" rule honoured).
- `src/lib/ipc.ts` — `saveBcPosition`, `loadBcPosition`, `loadBcPositions`
  wrappers.
- `src/lib/snapshot-patch.ts` — `bcNode` lazy-create initialises
  `relationships: []`.
- All seven BC READMEs — bootstrap `relationships:` frontmatter from
  `.agentheim/context-map.md`.
- `.agentheim/knowledge/decisions/ADR-014-bc-relationships-frontmatter.md` —
  records the "frontmatter, not a side file" decision.

**Deviation from task spec:** the task said either rename `BcSnapshot` to
`BoundedContext` (preferred) OR add `relationships` to `BcSnapshot`. Took the
rename path in Rust (per the task's preference); on the TS side kept
`BcSnapshot` as a type alias for `BoundedContext` so `Canvas.svelte` does not
need to change. The frontend rename to `BoundedContext` is naturally a
`canvas-007` follow-up — that's the consumer.

**Tests:** 116→117 cargo tests passing (+27 new across `project::tests`,
`db::tests`, `watcher::tests`). `pnpm check` 0/0/0. Coverage:

- `project::tests` (+8): parser graceful-fail cases, directional and
  non-directional happy paths, malformed-shape drops, cross-project drop,
  multi-relationship mix, end-to-end `get_project` surfacing,
  frontmatter-extractor edge cases.
- `db::tests` (+8): v3→v4 migration with data preservation,
  `bc_position` round-trip, multi-BC and multi-project isolation, hard-delete
  cascade, soft-delete preservation, concurrent-saves race-freedom,
  fresh-DB at v4.
- `watcher::tests` (+7): `readme_touched_bcs` happy and ignore paths,
  `reparse_bc_relationships` happy and missing-README path, end-to-end live
  watcher firing `BcRelationshipsChanged` on real-change and **not** firing
  on prose-only edits.
