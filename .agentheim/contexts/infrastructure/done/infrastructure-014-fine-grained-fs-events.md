---
id: infrastructure-014-fine-grained-fs-events
type: feature
status: done
completed: 2026-05-14
scope: global
depends_on:
  - infrastructure-012-walking-skeleton
blocks:
  - canvas-001-targeted-canvas-updates
related_adrs:
  - ADR-008
  - ADR-009
---

# Fine-grained filesystem domain events

## Why

The walking skeleton (`infrastructure-012`) ships a deliberately **crude**
watcher: any change inside a project's `.agentheim/` publishes a single coarse
`DomainEvent::AgentheimChanged { project_id }`, and the frontend reacts by
re-fetching the whole `get_project` snapshot. The skeleton's scope explicitly
sanctioned this ("crude — real event-bus mapping comes after the skeleton").

ADR-008 and ADR-009 specify a richer taxonomy that the skeleton did **not**
implement. This task makes the **Rust core** emit that taxonomy. The frontend's
*reaction* to it — targeted canvas updates and the eventual retirement of
`AgentheimChanged` — is split out to `canvas-001-targeted-canvas-updates`.

## What

Refactor the skeleton's existing **single-project** watcher
(`src-tauri/src/watcher.rs`) so it correlates raw debounced FS events into
fine-grained domain events, and extend the `DomainEvent` enum
(`src-tauri/src/events.rs`) accordingly.

### Final event taxonomy (decided during refinement)

```rust
TaskMoved   { project_id, bc, from, to, task_id }   // paired create + delete, same task_id, same debounce window
TaskAdded   { project_id, bc, state, task_id }       // unpaired create
TaskRemoved { project_id, bc, state, task_id }       // unpaired delete
BCAppeared  { project_id, bc }                       // new contexts/<bc>/ directory
BCDisappeared { project_id, bc }                     // removed contexts/<bc>/ directory
```

Two refinement decisions are baked in here:

1. **Unpaired create/delete get first-class variants.** ADR-008 flagged that an
   unpaired create or delete "needs a sensible fallback" but left it open. The
   fallback is now decided: a brand-new task file (which `model` writes straight
   into `backlog/`/`todo/` all the time) emits `TaskAdded`; an outright deletion
   emits `TaskRemoved`. No silent drop, no coarse re-fetch fallback for these.
2. **`TaskMoved` field names: `from` / `to`.** ADR-008's draft said
   `from_state` / `to_state`; ADR-009's enum said `from` / `to`. They are
   reconciled to ADR-009's shorter `from` / `to`. Both ADRs are updated in place
   as part of this task (no separate decision task — the reconciliation is
   purely naming).

## Scope (in)

- Implement create/delete correlation in the **existing single-project
  watcher** (in place — see Scope (out) on the supervisor) to emit `TaskMoved`
  when a create and a delete of the **same `task_id`** land in the **same 250ms
  debounce window**.
- Emit `TaskAdded` for an unpaired create and `TaskRemoved` for an unpaired
  delete within the window.
- Emit `BCAppeared` / `BCDisappeared` for `contexts/<bc>/` directory
  create/remove.
- Extend the `DomainEvent` enum with the five variants above; have the
  **frontend bridge** forward all five to the WebView under `guppi://event`.
- **Reconcile the ADRs in place:** update ADR-008's draft taxonomy to the
  implemented field names, and update ADR-009's enum block to add `TaskAdded`
  and `TaskRemoved`. Add a one-line reconciliation note to each.
- Keep `AgentheimChanged` **firing unchanged** alongside the new events, so the
  existing skeleton frontend keeps working with no change. Its retirement is
  `canvas-001`'s job — this keeps a clean incremental seam (no broken
  intermediate state).
- Cover the correlation logic with tests (ADR-008 requires this explicitly).

## Scope (out)

- **The multi-project `WatcherSupervisor`.** This task stays **single-project,
  refactoring the skeleton's existing watcher in place**. The
  `project_id -> watcher` supervisor lands with the project registry, not here.
- **Frontend reaction.** Targeted canvas updates (patching the client-side
  model instead of a full `get_project` re-fetch) and the **removal of
  `AgentheimChanged`** are owned by `canvas-001-targeted-canvas-updates`.

## Acceptance criteria

- [ ] `DomainEvent` enum has `TaskMoved { project_id, bc, from, to, task_id }`,
      `TaskAdded { project_id, bc, state, task_id }`,
      `TaskRemoved { project_id, bc, state, task_id }`,
      `BCAppeared { project_id, bc }`, `BCDisappeared { project_id, bc }`.
- [ ] A create + delete of the **same** `task_id` across two
      `{backlog,todo,doing,done}/` dirs within one 250ms window emits exactly
      one `TaskMoved` with correct `from` / `to`.
- [ ] An unpaired create emits `TaskAdded`; an unpaired delete emits
      `TaskRemoved` — both with the correct `state`.
- [ ] A same-window create and delete of **different** `task_id`s does **not**
      pair into a bogus `TaskMoved` — they emit a separate `TaskAdded` and
      `TaskRemoved`.
- [ ] A new / removed `contexts/<bc>/` directory emits `BCAppeared` /
      `BCDisappeared`.
- [ ] The frontend bridge forwards all five variants under `guppi://event`.
- [ ] `AgentheimChanged` still fires for every `.agentheim/` change exactly as
      before — the skeleton frontend is untouched and still works.
- [ ] ADR-008's taxonomy section and ADR-009's enum block are updated in place
      to match the implemented shape, each with a reconciliation note.
- [ ] Tests cover: paired move, unpaired create, unpaired delete, the
      different-`task_id` non-pairing case, and BC appear/disappear.

## Notes

Surfaced during `infrastructure-012-walking-skeleton`. The skeleton's
`src-tauri/src/watcher.rs` and `events.rs` carry comments pointing here.

Refined 2026-05-14: four open decisions resolved (single-project in-place;
`TaskAdded`/`TaskRemoved` for unpaired events; ADR reconciliation folded in;
frontend split to `canvas-001`). The `AgentheimChanged`-kept-alive decision is
deliberate — it lets `infrastructure-014` and `canvas-001` land independently
without a broken intermediate state.

## Outcome

The single-project watcher (`src-tauri/src/watcher.rs`) now correlates each
debounced batch of raw `notify` events into the fine-grained ADR-008/ADR-009
taxonomy, and `DomainEvent` (`src-tauri/src/events.rs`) carries the five new
variants. Correlation is a pure, unit-testable function `correlate()`:

- It splits each `notify::Event` into "appeared" / "removed" paths (handling
  `Create`, `Remove`, and the debouncer's stitched `Modify(Name(..))` rename
  events), classifies each path as a task file
  (`contexts/<bc>/<state>/<task_id>.md`), a BC directory (`contexts/<bc>`), or
  `Other`.
- A removed and an appeared task file with the **same `task_id`** in one batch
  pair into `TaskMoved { from, to }`; leftovers become `TaskAdded` /
  `TaskRemoved`; different-`task_id` create+delete pairs are *not* fabricated
  into a bogus move.
- `contexts/<bc>/` directory create/remove → `BCAppeared` / `BCDisappeared`.

`AgentheimChanged` still fires for every batch (the deliberate seam) so the
skeleton frontend is untouched — its retirement is `canvas-001`. The frontend
bridge in `lib.rs` already forwards *every* `DomainEvent` under `guppi://event`
unfiltered, so the five new variants reach the WebView with no `lib.rs` change.

ADR-008's domain-event mapping and ADR-009's enum block were both updated in
place with reconciliation notes (`from`/`to` field names settled; `TaskAdded`/
`TaskRemoved` added as ADR-008's previously-open "sensible fallback").

Tests: 10 watcher tests (7 new pure-correlation tests covering paired move,
stitched-rename move, unpaired create, unpaired delete, the different-`task_id`
non-pairing case, BC appear, BC disappear, and a no-fine-grained-event case for
non-task changes; plus the existing integration test upgraded to assert
`TaskMoved` *and* `AgentheimChanged` arrive). Full `cargo test --lib`: 26/26
passing. `cargo build` clean.

Key files: `src-tauri/src/watcher.rs`, `src-tauri/src/events.rs`.
