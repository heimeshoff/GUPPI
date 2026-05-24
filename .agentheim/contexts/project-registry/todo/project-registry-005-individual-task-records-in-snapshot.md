---
id: project-registry-005
title: Individual task records in the project snapshot (counts → tasks)
status: backlog
type: feature
context: project-registry
created: 2026-05-24
completed:
commit:
depends_on: []
blocks: [canvas-020, canvas-021, canvas-022, agent-awareness-002]
tags: [data-contract, snapshot, watcher, events, tasks]
related_adrs: [ADR-008, ADR-009, ADR-004]
related_research: []
prior_art: [project-registry-001, project-registry-004, canvas-001]
---

## Why

The canvas pivot renders each BC as a kanban board of **individual task
cards** (id, title, column, type, tags, blocked-question text), not a
counts pill. Today `ProjectSnapshot.bcs[]` carries only
`BoundedContext { name, task_counts, relationships }` (project-registry-001
/ 004) — per-BC COUNTS, no individual task records. The canvas literally
cannot draw a card it has no data for. This task grows the read-model from
counts to per-task records, end-to-end through the ADR-008 watcher and the
ADR-009 event taxonomy.

The fine-grained FS events ALREADY carry `task_id`
(`TaskAdded/TaskMoved/TaskRemoved`, ADR-008/ADR-009) — but the watcher only
reads it for count deltas; it never parses the task file's frontmatter.
This task makes the watcher read task identity + metadata and the snapshot
expose it.

## What

### New read-model: per-task records

- New Rust struct `Task` (in `project.rs`, mirrored in `src/lib/types.ts`):
  `{ id: String, title: String, column: TaskColumn, type_: String,
  tags: Vec<String>, blocked_question: Option<String> }` where
  `TaskColumn` ∈ `backlog | todo | doing | done` (derived from the
  subdirectory, consistent with the existing task-state vocabulary).
  `id` is the task file's `id:` frontmatter (e.g. `canvas-019-...`) with a
  filename-stem fallback; `title`, `type_` (`feature|bug|spike|decision`),
  `tags` parsed from frontmatter; `blocked_question` is the captured
  question text when the task is blocked-on-question (source: the
  agent-awareness signal / a `blocked_question:` frontmatter field — confirm
  in coordination with agent-awareness-002; registry parses the file, it
  does NOT derive live agent state).
- `BoundedContext` gains `tasks: Vec<Task>` (stable order by column then
  id). KEEP `task_counts` for now (cheap, derivable, avoids a flag-day for
  any non-card consumer) OR derive it from `tasks.len()` per column —
  decide in refinement; lean toward keeping it derived so the canvas counts
  pill / accordion-row "N tasks" needs no second source.
- The pure `project::get_project(project_id, &path)` reader now reads each
  `{backlog,todo,doing,done}/*.md` file's frontmatter (it currently only
  counts files). Cost: O(tasks) frontmatter parses per snapshot; acceptable
  on mount + resync only (snapshots are not the hot path — canvas-001
  patches in place from events).

### Event taxonomy: per-task payload

- `TaskAdded` / `TaskMoved` / `TaskRemoved` already carry `task_id`. Extend
  the payloads so the canvas can patch a card in place WITHOUT a resync:
  `TaskAdded { project_id, bc, state, task_id, title, type_, tags }`,
  `TaskMoved { project_id, bc, from, to, task_id }` (no metadata change on
  move), `TaskRemoved` unchanged. A new variant
  `TaskChanged { project_id, bc, task_id, title, type_, tags,
  blocked_question }` fires when a task FILE's frontmatter changes in place
  (title edit, tag change, blocked-question set/cleared) without a column
  move — the watcher detects an in-place write to an existing task file.
  (ADR-009 enum is "expected to grow"; add the variant + a reconciliation
  note in ADR-009.)

### Watcher (ADR-008)

- The single-project `AgentheimWatcher` (`watcher.rs`) parses the task
  file's frontmatter when emitting `TaskAdded` (for the metadata payload)
  and detects in-place task-file writes → `TaskChanged`. The
  create/delete-within-window correlation that produces `TaskMoved` is
  unchanged (move = same `task_id` across two state dirs).

## Acceptance criteria

- [ ] `BoundedContext` carries `tasks: Vec<Task>`; `Task` has
      `{ id, title, column, type_, tags, blocked_question }`. Mirrored in
      `src/lib/types.ts`.
- [ ] `get_project` populates `tasks` by reading each task file's
      frontmatter; a malformed/missing-frontmatter task file degrades to a
      filename-stem id + empty metadata (logged), never aborts the snapshot.
      Unit tested with ≥1 well-formed + ≥1 malformed task file.
- [ ] `task_counts` stays correct (kept or derived from `tasks`); existing
      count-consuming code paths unaffected — regression covered.
- [ ] `TaskAdded` payload carries `title`, `type_`, `tags`; new
      `TaskChanged` variant fires on in-place task-file frontmatter change
      (title/tags/blocked-question) without a column move — unit tested
      against a real-FS in-place write.
- [ ] `TaskMoved` continues to fire on cross-state-dir move of the same
      `task_id` (regression).
- [ ] ADR-009 gains the `TaskChanged` variant + payload-extension
      reconciliation note; the frontend bridge forwards the new/extended
      variants under the existing `guppi://event` name.
- [ ] `cargo test --lib` green (new tests added); the canvas can read
      `snapshot.bcs[].tasks[]` (frontend `pnpm check` clean against the new
      `types.ts`).

## Notes

This is the data-contract spine of the pivot — canvas-020/021/022 and
agent-awareness-002 all depend on it. It is substrate-agnostic (does NOT
depend on canvas-019) so it can land in parallel with the rendering
decision.

Coordination with agent-awareness-002: the registry owns the STATIC task
record (what's on disk — incl. a `blocked_question` if the Agentheim plugin
writes one into frontmatter); agent-awareness owns the LIVE per-task agent
state ("orchestrator · waiting 2m 14s"). Confirm during refinement whether
the blocked-question text is a disk artifact (registry) or a live signal
(agent-awareness) or both — the reference image shows it inside the docked
panel's "AGENT NEEDS AN ANSWER" callout, which is live-state-driven, so
lean: agent-awareness supplies the callout text, registry supplies a
fallback `blocked_question` only if present on disk.

`blocked` count surfaced in the reference accordion header ("1 active ·
2 blocked · 1 idling") is a per-BC roll-up of live agent state →
agent-awareness-002's surface, NOT a registry count. Registry counts stay
column-based.
