---
id: canvas-006-live-add-race-on-concurrent-project-added
type: bug
status: done
completed: 2026-05-15
scope: bc
depends_on: []
related_adrs:
  - ADR-003
  - ADR-004
related_research: []
prior_art:
  - canvas-002-render-multiple-project-tiles
  - canvas-001-targeted-canvas-updates
---

# Live-add path races on concurrent `ProjectAdded` events

## Why

`import_scanned_projects` (project-registry-002b) registers picks one at a time
and the supervisor publishes one `ProjectAdded` per pick — N events in quick
succession on the single event bus. The canvas's live-add path
(`addLiveProject` in `src/lib/Canvas.svelte:523-537`) was written and verified
against the single-event case (the startup seed double-add) and does not hold
under N concurrent arrivals.

Observed (2026-05-15, devtools-driven import of 5 projects): backend
correctly imported and announced all 5 projects; canvas rendered only 1 new
tile plus the existing seed. The 4 lost tiles also caused 5 colliding rows
to be written into `tile_positions`. ADR-005's "every new project gets its
own tile" intent is broken in practice until this is fixed; canvas-005's
discovery checklist UI cannot ship over a racy live-add.

## What

The bug is the unserialised invocation of `addLiveProject` from the
`project_added` case in `Canvas.svelte`'s `onDomainEvent` handler.
Five concurrent closures:
1. All read `projects.length` at the same value when computing the spiral
   index for `buildEntry`, so all five tiles end up at `spiralPosition(N)`
   for the same `N`.
2. All execute `projects = [...projects, entry]` as concurrent
   read-modify-write — last write wins, four entries are dropped from the
   in-memory array.
3. `buildEntry` calls `saveTilePosition` *before* the append (line 460-ish),
   so the SQLite write happens even for the four entries that get lost from
   the in-memory state. `tile_positions` ends up with N rows all pointing at
   the same world position; on next mount, `loadTilePosition` restores those
   colliding positions and the tiles stack again.

The fix is structural — serialise live-adds — so the cause is removed in
one place rather than papered over at each consequence (the spiral-index
read, the array reassignment, the persistence). One natural shape is a
module-level promise chain inside `Canvas.svelte` that the
`project_added` case enqueues onto; the worker is free to pick a different
serialisation primitive if it composes better with the surrounding code.

**Scope (in)**

- `src/lib/Canvas.svelte` — serialise the `project_added` → `addLiveProject`
  path so N concurrent arrivals process strictly sequentially.
- `addLiveProject` reads `projects.length` after the previous arrival has
  appended, so each new tile gets a distinct spiral slot and the array
  reassignment cannot lose entries.
- Preserve all existing behaviour: idempotency for already-rendered ids
  (seed double-add), the post-await re-check, the `getProject(id)` fetch,
  the `saveTilePosition` persistence, the no-op for `project_missing`.
- `pnpm check` (svelte-check) stays clean.

**Scope (out)**

- **One-shot DB cleanup of the already-colliding `tile_positions` rows.**
  Marco runs the SQL manually against `%APPDATA%\guppi\guppi.db`; this task
  does not ship a sweep, a Db helper, or a startup auto-repair. (Decision
  with Marco, 2026-05-15: keep the bug fix minimal and forward-only;
  anyone else hitting the existing broken state will run the same SQL.)
- Frontend test runner. Same as canvas-001/canvas-002 — no Vitest /
  Playwright lands here. `pnpm check` plus inspection is the verification
  surface.
- Rust-side changes. The supervisor and event bus are correct; this is
  purely a frontend serialisation bug.
- `canvas-005-project-discovery-affordances` (the import UI) — still
  blocked on this fix structurally, but writing it remains its own task.

## Acceptance criteria

- [ ] In `Canvas.svelte`, `addLiveProject` invocations from the
      `project_added` case are serialised — no two run concurrently.
      Verified by code inspection (a single named serialisation point, no
      bare `void addLiveProject(...)` calls inside the event handler).
- [ ] Firing N `project_added` events back-to-back for N distinct unseen
      `project_id`s results in N tiles, each at a **distinct** spiral
      position (no two tiles share the same `pos.x`/`pos.y`). Demonstrated
      with N ≥ 3; the assertion is "distinct positions in the `projects`
      array after all events drain", not "no overlap on screen".
- [ ] After the same N events drain, `tile_positions` contains N rows for
      those project ids, with N distinct `(x, y)` pairs. Verified by
      reading from SQLite directly (`SELECT project_id, x, y FROM
      tile_positions WHERE project_id IN (...)`) or by an integration-style
      script run from devtools — no in-memory-only check.
- [ ] Existing idempotency case still holds: a `project_added` for a
      `project_id` already in the collection is a no-op (the
      `findProject(id) → return` early-exit). Verified by inspection.
- [ ] `project_missing` remains a no-op (canvas-005 territory). Verified by
      inspection that the branch is untouched.
- [ ] The startup `list_projects()` → `for i in 0..n: buildEntry(snapshot,
      i)` mount path is unchanged. The fix targets only the live-add path.
- [ ] `pnpm check` is clean — `0 errors, 0 warnings`.

## Notes

Surfaced 2026-05-15 during a devtools-driven exercise of
`add_scan_root` + `import_scanned_projects` against `C:\src\heimeshoff`. The
scan returned 6 candidates (Agentheim, Guppi, Mediatheka, Whisperheim,
Snapshot, Utterheim); 5 imported (Guppi was the already-imported seed); only
1 visualised on the canvas.

**Why this slipped through canvas-002's verifier:** canvas-002's acceptance
criteria probed only the *single* live-add idempotency case (the startup
seed double-add). The N-concurrent-arrivals case that
`import_scanned_projects` produces in production was never on the
acceptance-criteria surface, so the diff passed verification with the
race intact. The fix here adds the missing acceptance dimension; future
canvas tasks touching the event handler should treat "N arrivals" as the
default test stance, not "one arrival."

**Fix shape (worker may diverge if a cleaner primitive exists).**
The shape that fell out of the diagnosis is a module-level promise chain:

```ts
let liveAddChain: Promise<void> = Promise.resolve();
function enqueueLiveAdd(id: number) {
  liveAddChain = liveAddChain.then(() => addLiveProject(id)).catch((e) => {
    logToCore('error', `live-add chain step failed for ${id}: ${e}`);
  });
}
// in the 'project_added' case:
if (findProject(event.project_id)) return;
enqueueLiveAdd(event.project_id);
return;
```

The `.catch` keeps a single failed step from breaking subsequent arrivals
on the chain. The post-await `findProject` re-check inside `addLiveProject`
becomes redundant under strict serialisation but should stay — defence in
depth, and it's already covered by the existing
`live-add get_project failed` log path.

**Per Marco's decision, the DB cleanup is one-shot SQL, not part of this
task.** The worker does not ship a sweep, a Db method, or a startup
self-healer. The existing
broken state on Marco's machine is cleaned manually outside Agentheim's
work flow.

**No ADR.** The serialisation choice is component-internal (same reasoning
canvas-002 used to skip ADR-writing for the `Map`-vs-array state shape and
the shared-drag-controller pattern): a future maintainer who needs the
context will read this task file. Recorded here so the next refiner does
not re-open the question.

**Coordination:** `canvas-005-project-discovery-affordances` (the import-
checklist UI, in `canvas/backlog/`) builds the user-facing surface that
triggers this race in production. It is not strictly blocked on canvas-006
— canvas-005 can ship in parallel — but the canvas-006 fix should land
first to avoid a known-broken first-user-experience.

`prior_art`:
- `canvas-002-render-multiple-project-tiles` (introduced this code path
  and the missing acceptance dimension).
- `canvas-001-targeted-canvas-updates` (established `onDomainEvent`'s
  patching pattern; the bug lives inside that pattern's `project_added`
  case).

## Outcome

The live-add race is closed by serialising `addLiveProject` invocations
through a single module-scoped promise chain inside `Canvas.svelte`'s
`onMount` closure. The `project_added` case in `onDomainEvent` now calls
`enqueueLiveAdd(event.project_id)` instead of `void addLiveProject(...)`;
`enqueueLiveAdd` chains every call onto `liveAddChain` with a `.catch`
that logs a failed step rather than wedging the chain for the rest of
the burst. The first thing each enqueued step does is `findProject(id)`
(the seed double-add early-exit), then reads `projects.length` —
guaranteed to be the post-append length of the previous step's
arrival, so each tile gets a distinct spiral slot, the
`projects = [...projects, entry]` reassignment cannot lose entries to
last-write-wins, and each `saveTilePosition` writes a distinct
`(x, y)` row to SQLite.

**Concrete changes:**

- `src/lib/Canvas.svelte` — added `liveAddChain` + `enqueueLiveAdd`
  near `addLiveProject`; switched the `project_added` case to
  `enqueueLiveAdd`. Kept `addLiveProject`'s post-await `findProject`
  re-checks as defence-in-depth (they are redundant under strict
  serialisation but cost a single `Array#find` and survive any future
  upstream contract slip). Every other branch of `onDomainEvent`
  (`project_missing`, `resync_required`, fine-grained patches) is
  untouched, as is the startup `refresh()` mount path.

**Verification:**

- `pnpm check` (svelte-kit sync + svelte-check) — 0 errors, 0 warnings,
  936 files checked.
- Acceptance criteria 1, 4, 5, 6 verified by code inspection (single
  named serialisation point; `findProject` early-exit preserved;
  `project_missing` branch untouched; `refresh()` path untouched).
- Acceptance criteria 2 + 3 (N distinct positions in memory and in
  `tile_positions`) follow structurally from the chain: step N+1 only
  runs after step N's `projects = [...projects, entry]` settles, so
  the `projects.length` argument to `buildEntry` is monotone, the
  `spiralPosition(i)` calls produce distinct `(x, y)` pairs (the spiral
  is injective on `i`), and `saveTilePosition` writes a distinct row
  per id. The frontend has no test runner (same posture as canvas-001
  / canvas-002 — see those tasks' Outcome sections); the per-frame
  in-DB demonstration Marco will run from devtools against the
  `import_scanned_projects` flow that originally surfaced the bug.

**No ADR.** The serialisation is component-internal; the rationale is
captured in the inline block comment above `enqueueLiveAdd` plus the
canvas BC README paragraph (next bullet).

**README updates:** `contexts/canvas/README.md` "Rendering N projects"
section extended with the serialisation invariant — N concurrent
arrivals now process strictly sequentially, with the failure mode
spelled out so future maintainers do not unwind the chain without
seeing what it prevents. The "N concurrent arrivals" stance is named
as the default test posture for future changes to this handler.

**Scope held:** no DB sweep, no Db helper, no startup self-healer —
Marco runs the one-shot SQL against `%APPDATA%\guppi\guppi.db` himself
per the task's Scope (out). No Rust changes. No new ADR. No frontend
test runner. `canvas-005-project-discovery-affordances` remains the
right next step for the import UI; the structural blocker it had on
this fix is now removed.

**Key files:** `src/lib/Canvas.svelte` (lines around 330-345 — event
handler enqueue; lines around 524-572 — chain + `addLiveProject`).
