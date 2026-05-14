---
id: canvas-001-targeted-canvas-updates
type: feature
status: done
completed: 2026-05-14
commit: 5fa7080
scope: bc
depends_on:
  - infrastructure-014-fine-grained-fs-events
  - design-system-001-styleguide
related_adrs:
  - ADR-008
  - ADR-009
related_research: []
prior_art: []
---

# Targeted canvas updates from fine-grained FS events

## Why

The walking skeleton's frontend reacts to *any* `.agentheim/` change by
re-fetching the whole `get_project` snapshot (the coarse `AgentheimChanged`
event). `infrastructure-014` (done — commit `1415d1e`) now makes the Rust core
emit the fine-grained `TaskMoved` / `TaskAdded` / `TaskRemoved` / `BCAppeared` /
`BCDisappeared` taxonomy alongside the coarse event. The canvas can stop
re-fetching and instead **patch its client-side model in place** — a tile's
task counts tick without a round-trip, a BC node appears/disappears without
redrawing everything.

This is the frontend half of the work split out of `infrastructure-014` during
refinement: the core emits the events, the canvas BC owns what the UI does with
them.

## What

Migrate the skeleton frontend (`src/lib/`) from the coarse re-fetch to targeted
updates driven by the fine-grained `guppi://event` variants, and **retire the
skeleton-compat `AgentheimChanged` event** — but *not* by deleting it outright.
`AgentheimChanged` currently has **two** jobs in the codebase:

1. **Normal-path skeleton event** — the watcher fires it for every debounced
   batch; `Canvas.svelte` re-fetches on it. *This role is removed by this task.*
2. **Lag-resync signal** — `lib.rs`'s frontend-bridge re-emits it when the
   Tokio broadcast channel reports `Lagged` (a consumer fell behind and lost
   events). This is ADR-009's documented "resync from the source of truth"
   strategy. The fine-grained events **cannot** replace this role — by
   definition the bridge has *lost* events and can't reconstruct them.

**Decision (refinement 2026-05-14, Marco):** keep an explicit resync event.
Rename `AgentheimChanged` → `ResyncRequired` (no `project_id` change — keep the
field). canvas-001 removes job (1) entirely; job (2) survives under the clearer
name. The watcher no longer emits it on the normal path; only `lib.rs`'s
`Lagged` arm emits it. The frontend full-refetches (`getProject()`) **only** on
`ResyncRequired`.

This is recorded as an **in-place amendment to ADR-009** (same pattern as
`infrastructure-014` reconciling ADR-008/ADR-009 in place — no new ADR). ADR-009
currently names `AgentheimChanged` as both the skeleton event and the resync
signal; the worker updates ADR-009's text to describe the split outcome:
fine-grained events for the normal path, `ResyncRequired` as the lag-only
escape hatch.

## Scope (in)

### Rust core
- Rename `DomainEvent::AgentheimChanged { project_id }` →
  `DomainEvent::ResyncRequired { project_id }` in `src-tauri/src/events.rs`.
- `src-tauri/src/watcher.rs`: stop publishing the event on every debounced
  batch (`start`'s closure). The watcher now publishes *only* the fine-grained
  events from `correlate()`.
- `src-tauri/src/lib.rs`: the frontend-bridge's `Lagged` arm emits
  `DomainEvent::ResyncRequired` instead of `AgentheimChanged`.
- Update the affected tests: `events.rs`'s
  `published_event_reaches_a_subscriber` (uses `AgentheimChanged`), and
  `watcher.rs`'s `moving_a_task_file_emits_task_moved_and_still_emits_agentheim_changed`
  — the watcher no longer emits the coarse event on the normal path, so that
  test's `saw_changed` expectation is dropped (rename the test accordingly;
  it now asserts *only* `TaskMoved` results from a move).

### Frontend
- `src/lib/types.ts`: extend the `DomainEvent` union to model the fine-grained
  variants the core actually emits — `task_moved`, `task_added`,
  `task_removed`, `bc_appeared`, `bc_disappeared` — and rename
  `agentheim_changed` → `resync_required`. The `kind` values are serde's
  `snake_case` tags of the Rust enum.
- `src/lib/Canvas.svelte`: replace the `onDomainEvent` handler (currently
  `if (event.kind === 'agentheim_changed') void refresh()`) with targeted
  patching of the `snapshot` `$state`:
  - `task_moved` → decrement the `from` count, increment the `to` count on the
    affected BC.
  - `task_added` → increment the `state` count on the affected BC.
  - `task_removed` → decrement the `state` count on the affected BC.
  - `bc_appeared` → add a `BcSnapshot` node with zero counts (idempotent — no-op
    if a node with that name already exists).
  - `bc_disappeared` → remove the BC node with that name.
  - `resync_required` → the one remaining full `refresh()` / `getProject()`.
- Re-render: `Canvas.svelte` already calls `renderScene()` every ticker frame,
  and `snapshot` is Svelte 5 `$state` (deeply reactive). Mutating
  `snapshot.bcs[i].task_counts` is picked up on the next frame — **no new
  reactivity wiring, no explicit re-render call needed.** Do not add an
  animation system; per the count-tick decision below the number simply
  changes in place.

### Robustness (resolved during refinement — implement as specified)
- **Event-vs-BC ordering.** `correlate()` in `watcher.rs` pushes BC events
  *after* task events in its output `Vec`, so a brand-new BC created with task
  files in it yields `TaskAdded` *before* `BCAppeared` in the same batch. The
  frontend must tolerate any order: a `task_*` event for a BC not in the
  current model **lazily creates** a zero-count BC node, then applies the
  delta. `bc_appeared` is therefore idempotent (the node may already exist).
- **Count clamping.** If a `task_moved` / `task_removed` would drive a count
  below zero, the client model has drifted from disk. Clamp at 0 and
  `logToCore('warn', …)` — never render a negative count, never throw.
- **`project_id` filtering.** Every event carries `project_id`. The skeleton
  has one hard-coded project; the frontend ignores any event whose
  `project_id` does not match the loaded project's id. (The frontend learns
  its `project_id` from the `project_added` event it already receives.)

### Styleguide
- The only visual change is the BC node's count subtitle text changing in
  place. **Count-tick decision (Marco, refinement 2026-05-14): silent number
  update — no animated tick.** No tween, no per-node animation state. The
  styleguide criterion reduces to "introduce no jarring motion" — which a
  silent text swap satisfies for free. No `design-system` token work is
  needed beyond what `Canvas.svelte` already consumes.

## Scope (out)

- The `DomainEvent` taxonomy itself and the watcher's correlation logic —
  delivered by `infrastructure-014` (done).
- `agent-awareness`-driven status badges (running / idle / blocked) — a
  separate concern, not part of task-count plumbing. (Note: `deriveBcStatus()`
  in `Canvas.svelte` derives a badge *from counts* today; it keeps working
  unchanged because the patched counts feed it the same as a re-fetch did.)
- Animated count transitions — explicitly deferred (see styleguide decision).
- Multi-project handling — the skeleton is single-project; `project_id`
  filtering is implemented but only one project exists to filter for.

## Acceptance criteria

- [ ] A task file moved between states updates exactly the affected BC node's
      counts (`from` -1, `to` +1), with no `getProject` re-fetch.
- [ ] A new task file appearing updates the affected BC's count via
      `task_added`; a deleted one decrements via `task_removed` — no re-fetch.
- [ ] A new `contexts/<bc>/` directory adds a zero-count BC node via
      `bc_appeared`; a removed one removes the node via `bc_disappeared` —
      unaffected tiles are not redrawn from a fresh snapshot.
- [ ] A `task_added` / `task_moved` / `task_removed` for a BC not yet in the
      client model lazily creates a zero-count node and applies the delta;
      a subsequent `bc_appeared` for that BC is a no-op.
- [ ] A delta that would push a count below zero clamps at 0 and logs a
      warning — no negative count is ever rendered, nothing throws.
- [ ] `AgentheimChanged` no longer exists. `ResyncRequired` exists, is emitted
      **only** by `lib.rs`'s `Lagged` arm (never by the watcher's normal path),
      and is the **only** event that triggers a full `getProject()` re-fetch in
      the frontend.
- [ ] Events whose `project_id` does not match the loaded project are ignored.
- [ ] ADR-009's text is amended in place to describe the split: fine-grained
      events for the normal path, `ResyncRequired` as the lag-only resync
      signal. No new ADR file.
- [ ] `cargo test --lib` and the frontend type-check pass; the renamed/updated
      Rust tests reflect that the watcher no longer emits a coarse event on the
      normal path.

## Notes

Split out of `infrastructure-014-fine-grained-fs-events` during refinement on
2026-05-14. Refined 2026-05-14: grounded against the actual code
(`events.rs`, `watcher.rs`, `lib.rs`, `Canvas.svelte`, `types.ts`,
`project.rs`); two decisions taken with Marco (`ResyncRequired` rename;
silent count update); three robustness gaps resolved (event ordering, count
clamping, `project_id` filtering).

**`infrastructure-014` is done** (commit `1415d1e`) — the events exist.

**Frontend gate — cleared.** Both dependencies are satisfied:
`infrastructure-014` is `done`, and `design-system-001-styleguide` is `done`
*and* signed off in person by Marco on 2026-05-14 (protocol entry that date).
Promoted backlog → todo on 2026-05-14.

## Outcome

Completed 2026-05-14.

The canvas now patches its client-side `ProjectSnapshot` in place from the
fine-grained filesystem events instead of re-fetching the whole snapshot on
every `.agentheim/` change. The coarse `AgentheimChanged` event is retired:
renamed `ResyncRequired`, stripped of its normal-path role, kept only as the
lag-only resync signal.

**Rust core:**
- `src-tauri/src/events.rs` — `DomainEvent::AgentheimChanged` →
  `DomainEvent::ResyncRequired`; the `published_event_reaches_a_subscriber`
  test updated.
- `src-tauri/src/watcher.rs` — the debounced-batch closure no longer publishes
  a coarse event; `correlate()`'s fine-grained output is all the watcher emits.
  The integration test renamed to `moving_a_task_file_emits_only_task_moved`,
  `saw_changed` expectation dropped.
- `src-tauri/src/lib.rs` — the frontend bridge's `Lagged` arm emits
  `ResyncRequired`.
- `cargo test --lib` 26/26; `cargo build` clean.

**Frontend:**
- `src/lib/types.ts` — `DomainEvent` union extended with `task_moved` /
  `task_added` / `task_removed` / `bc_appeared` / `bc_disappeared`;
  `agentheim_changed` → `resync_required`; added the `AgentheimState` type.
- `src/lib/snapshot-patch.ts` — new pure module: `applyDomainEvent()` patches a
  `ProjectSnapshot` in place. Lazy zero-count BC creation (tolerates `task_*`
  arriving before `bc_appeared`), count clamping at 0 with a warn on drift,
  idempotent `bc_appeared`.
- `src/lib/Canvas.svelte` — the `onDomainEvent` handler routes events:
  `project_added` learns `projectId`; fine-grained events are `project_id`-
  filtered then patched in place (silent count update, no animation); only
  `resync_required` triggers a full `refresh()`.
- `pnpm check` — 0 errors.

**Decisions / ADRs:** ADR-009 amended in place (reconciliation note in the
infrastructure-014 style) describing the `AgentheimChanged` → `ResyncRequired`
split. No new ADR.

**Surfaced:** `infrastructure-016-readme-resync-required-rename` created in the
infrastructure BC backlog — that BC's README still documents `AgentheimChanged`
as live (cross-BC, could not be edited from here).

**No frontend test infrastructure** exists in this project (no vitest, no
`.test.ts` files); the patching logic was extracted into the pure
`snapshot-patch.ts` module to keep it reviewable and isolated, and the frontend
contract is verified by `pnpm check` (svelte-check) per the task's acceptance
criteria. Adding frontend unit-test infra is a pre-existing gap, not opened as
a task here since no canvas frontend logic before this needed it either.

**Key files:** `src-tauri/src/events.rs`, `src-tauri/src/watcher.rs`,
`src-tauri/src/lib.rs`, `src/lib/types.ts`, `src/lib/snapshot-patch.ts`,
`src/lib/Canvas.svelte`, `.agentheim/knowledge/decisions/ADR-009-event-bus.md`.
