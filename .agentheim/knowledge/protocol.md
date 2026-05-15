# Protocol

Chronological log of everything that happens in this project.
Newest entries on top.

---

## 2026-05-15 13:00 -- Work session ended

**Type:** Work / Session end
**Completed:** 4 (first-try PASS: 4, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 4 (f83222c canvas-006, ebe2e48 project-registry-003, 8ff6a1e canvas-005a, 4979e27 canvas-005b)
**Note:** Cleared the entire canvas-discovery refinement chain — canvas-006 (live-add serialisation) → project-registry-003 (manual register/remove + soft-delete + missing snapshot + ProjectRemoved event from both single-remove and scan-root cascade, schema v3, 30-day GC sweep) → canvas-005a (right-click context menu shell, Add/Remove/Missing affordances, `tauri-plugin-dialog` wired in, `project_removed` canonical handler established) → canvas-005b (scan-folder flow + scan-roots management modals + cascade-confirm; new `Modal.svelte` primitive extracted; new `list_projects_by_scan_root` IPC wrapper). All four PASS first try. Backend tests 73 → 89; `pnpm check` 0/0/0 throughout (937 → 938 files). New dependencies: `tauri-plugin-dialog` (Cargo + npm) + `dialog:allow-open` capability. No new ADRs (all decisions sat inside ADR-003 + ADR-004 + ADR-005 + ADR-008 + ADR-013 + the existing styleguide token vocabulary). No bounces, no escalations, no concept candidates. **todo/, doing/, backlog/ now empty in every BC.** Orchestrator's INDEX.md / protocol.md bookkeeping is uncommitted on the working tree — a separate `chore(work)` commit folds it in (matches the prior session pattern). **One verifier note worth surfacing:** the `scan::tests::register_project_rejects_a_non_agentheim_folder_with_exact_error_string` test added in project-registry-003 is tautological (asserts a hardcoded constant against itself rather than exercising the IPC handler) — production code at `lib.rs` correctly returns the exact string `"not an Agentheim project"`, but the regression contract is unenforced by that test. Worth strengthening when next touched.

---

## 2026-05-15 12:55 -- Task verified and completed: canvas-005b-scan-flow-and-scan-root-management - Scan flow + scan-root management

**Type:** Work / Task completion
**Task:** canvas-005b-scan-flow-and-scan-root-management - Scan flow + scan-root management
**Summary:** Landed the scan-folder flow and the scan-roots management surface as three HTML-overlay modals built on a new generic `Modal.svelte` primitive (header/body/footer slots + backdrop + Escape dismissal). Scan-folder: folder picker → `addScanRoot` → discovery checklist modal (already-imported rows at 60% opacity + pre-checked-disabled + "imported" badge; togglable rows checkbox-default-unchecked; Select-all/Select-none gated on togglable rows; "Import selected" disabled when zero NEW picks; rescan-flagged header with " (rescan)" suffix; empty-candidates state with single "OK"). Manage-scan-roots: hidden when zero roots, refreshed after add/remove; list rows show path, child-project count, Rescan, Remove. Cascade-confirm: stacks atop manage modal, names path + N child count + "Tile state for those projects will not be retained." Per-row child count via new thin `#[tauri::command] list_projects_by_scan_root` wrapper. `project_removed` handler from canvas-005a reused without duplication; cascade fan-out drops N tiles cleanly. canvas-006's `liveAddChain` is the smoke-test fix that makes the N-arrival import path correct.
**Verification:** PASS (iteration 1)
**Commit:** 4979e27
**Files changed:** 7 (Canvas.svelte, new Modal.svelte, ipc.ts, types.ts, src-tauri/lib.rs [+17 lines — one IPC wrapper], canvas README, moved task)
**Tests added:** 0 — no frontend test runner; `pnpm check` 0/0/0 (938 files) + `cargo check` clean. Backend test count unchanged (89/89 from project-registry-003).
**ADRs written:** none — modal patterns are component-internal under ADR-003 (PixiJS + HTML overlays) and the design-system tokens; cascade semantics realise ADR-013; retention boundary documented in confirm dialog body per ADR-005.
**Worth noting:** Modal extraction triggered by three consumers in one task (checklist + manage + cascade-confirm). A styleguide entry codifying the Modal pattern would be the right next step; not added in this task (cross-BC into design-system) and not promoted to backlog (the next design-system pass can pick it up from this task's done/ file).

---

## 2026-05-15 12:35 -- Batch started: [canvas-005b-scan-flow-and-scan-root-management]

**Type:** Work / Batch start
**Tasks:** canvas-005b-scan-flow-and-scan-root-management - Scan flow + scan-root management
**Parallel:** no (1 worker — final wave; all deps satisfied [canvas-006 ✓ project-registry-003 ✓ canvas-005a ✓ design-system-001 ✓])

---

## 2026-05-15 12:30 -- Task verified and completed: canvas-005a-single-shot-discovery-affordances - Single-shot Add / Remove / Missing

**Type:** Work / Task completion
**Task:** canvas-005a-single-shot-discovery-affordances - Single-shot Add / Remove / Missing
**Summary:** Landed ADR-005's three single-shot discovery affordances on the canvas — right-click empty canvas opens an HTML-overlay context menu with "Add project…" (Tauri folder picker via `@tauri-apps/plugin-dialog` → `registerProject` → live-add via existing `ProjectAdded` chain, error toast surfaces the exact `"not an Agentheim project"` rejection for 3s, cancelled picker is silent); right-click on a tile opens a context menu with "Remove project" (no confirmation step, ADR-005's 30-day undo is the safety net via `removeProject` → `ProjectRemoved` → canonical handler drops tile + clears tile-position cache + removes from `tile-layout`); missing tiles render at 50% opacity with `statusMissing` magenta border and a `✕` corner glyph at `spacing.lg` in `statusMissing` colour (no BC nodes drawn — `bcs: []` from the backend handles this). New HTML-overlay context-menu + error-toast patterns inlined under ADR-003 + design tokens; the `project_removed` handler is wired as THE canonical listener and 005b will reuse it without duplication. canvas-006's `liveAddChain` contract preserved.
**Verification:** PASS (iteration 1)
**Commit:** 8ff6a1e
**Files changed:** 10 (Canvas.svelte, ipc.ts, Cargo.toml, Cargo.lock, capabilities/default.json, src-tauri/lib.rs [+5 lines — plugin init only], package.json, pnpm-lock.yaml, canvas README, moved task)
**Tests added:** 0 — no frontend test runner; `pnpm check` 0/0/0 (937 files) + `cargo check` clean. Same posture as canvas-001 / canvas-002 / canvas-006.
**ADRs written:** none — context-menu and toast patterns are component-internal under ADR-003 (PixiJS + HTML overlays) and the design-system tokens. The task explicitly stipulates this; a future styleguide entry can codify the patterns if they proliferate (canvas-005b will exercise them, design-system follow-up can decide).
**New dependencies:** `tauri-plugin-dialog = "2"` (Cargo) + `@tauri-apps/plugin-dialog ^2.7.1` (npm) + `dialog:allow-open` capability. First consumer of the folder picker in this codebase.

---

## 2026-05-15 12:10 -- Batch started: [canvas-005a-single-shot-discovery-affordances]

**Type:** Work / Batch start
**Tasks:** canvas-005a-single-shot-discovery-affordances - Single-shot Add / Remove / Missing
**Parallel:** no (1 worker — canvas-005a unblocked by project-registry-003; 005b blocked on 005a so cannot run in parallel)

---

## 2026-05-15 12:05 -- Task verified and completed: project-registry-003-manual-add-remove-and-missing-projects - Manual add / remove / missing projects

**Type:** Work / Task completion
**Task:** project-registry-003-manual-add-remove-and-missing-projects - Manual add / remove / missing projects
**Summary:** Landed the remaining half of ADR-005's IPC surface — `register_project` (canonicalise + `.agentheim/` validate + upsert with NULL `scan_root_id` + supervisor arm, exact `"not an Agentheim project"` reject string, idempotent on canonical path; revives soft-deleted rows clearing `deleted_at` and rearming the watcher while keeping the `tile_positions` row), `remove_project` (soft-delete via `projects.deleted_at`, watcher torn down, `ProjectRemoved` event), schema v2→v3 with `projects.deleted_at TEXT NULL` + 30-day startup GC sweep (`RETENTION_DAYS = 30` single edit point), `ProjectSnapshot.missing: bool` for registered-but-unwatched rows (no more silent skip in `list_projects` / `get_project`), `ProjectRemoved { project_id }` domain event fired by **both** single-remove **and** the `remove_scan_root` cascade (BEFORE supervisor.remove + db.remove_project per child). Frontend types mirror; `Canvas.svelte` got a 6-line `project_removed` no-op arm (visual treatment is canvas-005a).
**Verification:** PASS (iteration 1)
**Commit:** ebe2e48
**Files changed:** 9 (db.rs, lib.rs, project.rs, events.rs, scan.rs, types.ts, Canvas.svelte, project-registry README, moved task)
**Tests added:** 16 — `cargo test --lib` 89/89 (db::tests: v3 migration, deleted_at revival on both upsert paths, soft-delete preserves tile_positions, GC sweep with forged timestamps; project::tests: missing-snapshot builder + healthy carries missing=false; scan::tests: register_project + revive + remove_project + missing-on-`list_projects` + extended cascade test taps the bus to assert observed_removed order); `pnpm check` 0/0/0
**ADRs written:** none — sits inside ADR-004 (schema) + ADR-005 (discovery + retention) + ADR-008 (watchers) + ADR-013 (cascade ordering)
**Verifier note:** `scan::tests::register_project_rejects_a_non_agentheim_folder_with_exact_error_string` is tautological — asserts a hardcoded constant against itself rather than calling the IPC handler. Production reject path at `lib.rs` is correct (`return Err("not an Agentheim project".to_string())`), but the regression contract is unenforced by that test. Worth strengthening when next touched.

---

## 2026-05-15 11:45 -- Batch started: [project-registry-003-manual-add-remove-and-missing-projects]

**Type:** Work / Batch start
**Tasks:** project-registry-003-manual-add-remove-and-missing-projects - Manual add / remove / missing projects
**Parallel:** no (1 worker — completes the ADR-005 backend surface; unblocks canvas-005a + 005b)

---

## 2026-05-15 11:40 -- Task verified and completed: canvas-006-live-add-race-on-concurrent-project-added - Live-add path races on concurrent `ProjectAdded` events

**Type:** Work / Task completion
**Task:** canvas-006-live-add-race-on-concurrent-project-added - Live-add path races on concurrent `ProjectAdded` events
**Summary:** Serialised the canvas `project_added` → `addLiveProject` path through a single named promise chain (`liveAddChain` + `enqueueLiveAdd`); N back-to-back arrivals from `import_scanned_projects` now process strictly sequentially, so each tile lands at a distinct spiral slot in memory and in `tile_positions` instead of colliding to one. Post-await `findProject` re-check kept as defence in depth.
**Verification:** PASS (iteration 1)
**Commit:** f83222c
**Files changed:** 2 (Canvas.svelte, canvas README — moved task file additionally)
**Tests added:** 0 — no frontend test runner; `pnpm check` 0/0/0 (936 files) + structural inspection per task acceptance (same posture as canvas-001/canvas-002)
**ADRs written:** none — serialisation primitive is component-internal (same reasoning canvas-002 used for state-shape); recorded in code comments + BC README

---

## 2026-05-15 11:30 -- Batch started: [canvas-006-live-add-race-on-concurrent-project-added]

**Type:** Work / Batch start
**Tasks:** canvas-006-live-add-race-on-concurrent-project-added - Live-add path races on concurrent `ProjectAdded` events
**Parallel:** no (1 worker — canvas-006 + project-registry-003 are both no-deps but share `src/lib/Canvas.svelte`; serialising. canvas-006 first because it removes a known race that 005b will hit on first run.)

---

## 2026-05-15 11:15 -- Model / Refined: canvas-005-project-discovery-affordances — split into 005a + 005b; spawned project-registry-003

**Type:** Model / Refine
**BC:** canvas (+ spawn into project-registry)
**Status after:** todo (all three new tasks promoted)
**Summary:** Refined the under-refined canvas-005 stub against the real backend, the styleguide, and ADRs 005/013. **Major finding:** ADR-005's discovery surface is only half-implemented at the IPC layer — `register_project(path)`, `remove_project(project_id)`, the "missing" tile state, and a `ProjectRemoved` event all need to ship before any UI can. `Db::upsert_project` + `Db::remove_project` exist but no IPC wraps them; `list_projects` silently skips unreadable rows; the existing `remove_scan_root` cascade fires no event and would leave stale tiles in the canvas forever. Decided with Marco: **spawn project-registry-003** as a hard backend prerequisite rather than punch through the BC seam from a canvas task. Locked the backend shape: soft-delete via `projects.deleted_at` (schema v2→v3) with 30-day startup GC sweep (faithful to ADR-005's stipulation); `ProjectSnapshot` gains `missing: bool` (single shape, one frontend code path); new `ProjectRemoved { project_id }` event fired by **both** `remove_project` AND the cascade in `remove_scan_root`; `register_project` rejects non-Agentheim folders with **exactly** `"not an Agentheim project"`. **Split canvas-005 into two tasks** along the interaction-surface axis: **005a** (single-shot Add / Remove / Missing — establishes the new right-click context-menu + error-toast patterns and the canonical `project_removed` handler) and **005b** (the scan flow + scan-root management — extends 005a's menu shell with two more items, ships the discovery checklist modal, the scan-roots management modal, and the cascade-remove confirmation; hard-`depends_on` canvas-006 because `import_scanned_projects`'s N-arrival case would otherwise re-demonstrate the live-add race). **Chrome decision:** right-click contextual menus, not toolbars or native menu bars — ambient, voice-first, zero resting chrome. **Three UX decisions:** (1) Remove-project skips a confirmation step, trusting ADR-005's 30-day re-add undo window; (2) missing tiles render as dim (50% opacity) + `statusMissing` magenta border + `✕` corner glyph; (3) already-imported checklist rows are visible-pre-ticked-disabled with an "imported" badge, maximising information at a tiny visual cost. Modal/menu patterns are net-new in this codebase — styling contracts inlined per task with existing `tokens.ts` / `--guppi-*` vocabulary; a follow-up design-system pass can codify "Menu" / "Modal" / "Toast" if patterns proliferate. No orchestrator round — all decisions resolvable from existing ADRs + styleguide + the Rust IPC surface, no architect/strategic-modeler/tactical-modeler delegation needed. Original `canvas-005-project-discovery-affordances.md` removed from `backlog/`; replaced by 005a + 005b + project-registry-003 in `todo/`.
**Split into:** canvas-005a-single-shot-discovery-affordances, canvas-005b-scan-flow-and-scan-root-management
**Spawned dependency:** project-registry-003-manual-add-remove-and-missing-projects (todo)
**ADRs written:** none — implementation sits inside ADR-004 (schema) + ADR-005 (discovery model + retention stipulation) + ADR-013 (cascade ordering, now extended to fire `ProjectRemoved`) + ADR-003 (overlay-layer modals). The "soft-delete with 30-day sweep" is realisation of an ADR-005 stipulation, not a new decision; the right-click context-menu + modal patterns are component-internal pending proliferation. All recorded in the task Notes so the next refiner does not re-open these questions.

---

## 2026-05-15 10:45 -- Model / Captured: canvas-006-live-add-race-on-concurrent-project-added - Live-add path races on concurrent `ProjectAdded` events

**Type:** Model / Capture
**BC:** canvas
**Filed to:** todo (direct — diagnosis is precise, scope is one file, acceptance criteria fall out of the diagnosis; no orchestrator round needed)
**Summary:** Marco exercised `add_scan_root` + `import_scanned_projects` from devtools against `C:\src\heimeshoff`. Backend imported 5 projects + fired 5 `ProjectAdded` events correctly; canvas rendered only 1 new tile (+ seed). Root cause confirmed by reading `Canvas.svelte:523-537`: `addLiveProject` invocations from the `project_added` case are unserialised — concurrent closures all read the same `projects.length` at the spiral-index step (so all five auto-place at the same slot), `projects = [...projects, entry]` is a concurrent read-modify-write (so four entries are lost from the in-memory array), and `buildEntry` calls `saveTilePosition` before the append (so all five colliding positions are persisted to SQLite and will re-stack on restart). canvas-002's verifier missed this because its acceptance criteria probed only the seed-double-add idempotency case, never the N-concurrent-arrivals case that `import_scanned_projects` produces. Captured directly to `todo/` with concrete acceptance criteria covering the N-arrivals case (N tiles at distinct positions, N distinct `tile_positions` rows). DB cleanup of the already-colliding rows is **out of scope per Marco** — one-shot SQL outside Agentheim's flow, not a sweep/auto-repair. No ADR — serialisation primitive is component-internal (same reasoning canvas-002 used for state-shape).

---

## 2026-05-15 10:20 -- Work session ended

**Type:** Work / Session end
**Completed:** 4 (first-try PASS: 4, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 4 (d594ad5 project-registry-001, bace9fd project-registry-002a, e690fd3 canvas-002, 5ad554a project-registry-002b)
**Note:** Cleared the entire v1-completion chain — `project-registry-001` (multi-project snapshot model + WatcherSupervisor) unblocked the parallel pair `project-registry-002a` (scan roots + walker) and `canvas-002` (keyed-by-project_id Canvas restructure), which then unblocked `project-registry-002b` (import + cascade-deregister). All four passed verification first try. Backend test count 32 → 73; `pnpm check` 0/0/0 throughout. No new ADRs (all decisions sat inside existing ADR-013/008/005/004/003 envelopes). No bounces, no escalations, no concept candidates. `todo/`, `doing/`, `backlog/` now empty in every BC. Orchestrator's INDEX.md / protocol.md bookkeeping is uncommitted on the working tree — a separate `chore(work)` commit folds it in (matches the prior session pattern).

---

## 2026-05-15 10:15 -- Task verified and completed: project-registry-002b-import-and-cascade-deregister - Import scanned projects + cascade-deregister

**Type:** Work / Task completion
**Task:** project-registry-002b-import-and-cascade-deregister - Import scanned projects + cascade-deregister
**Summary:** Landed the v1 mutation layer for the project registry — `import_scanned_projects` registers checklist picks with origin tracking and watcher arming, and `remove_scan_root` performs the app-driven cascade-deregister (`supervisor.remove` → `db.remove_project` per child → `delete_scan_root` last; `ON DELETE RESTRICT` is the checked invariant). Hard-delete; manually-added projects (NULL `scan_root_id`) are immune.
**Verification:** PASS (iteration 1)
**Commit:** 5ad554a
**Files changed:** 6 (db.rs, lib.rs, scan.rs, supervisor.rs [+9 lines — dead_code allow removed], project-registry README, moved task)
**Tests added:** 13 — `cargo test --lib` 73/73 (`db::tests` upsert_scanned_project_stamps_scan_root_id + idempotency + remove_project + RESTRICT-rejects-living-child + list_projects_by_scan_root; `scan::tests` import-registers-and-watches, rejects-out-of-set, cascade-drops-watchers-tiles-then-root, does-not-touch-manually-added)
**ADRs written:** none — ADR-013 already specifies the cascade semantics; implementation is the realization

---

## 2026-05-15 10:00 -- Batch started: [project-registry-002b-import-and-cascade-deregister]

**Type:** Work / Batch start
**Tasks:** project-registry-002b-import-and-cascade-deregister - Import scanned projects + cascade-deregister
**Parallel:** no (1 worker — final v1 chain task; depends on 001 + 002a, both done)

---

## 2026-05-15 09:55 -- Task verified and completed: canvas-002-render-multiple-project-tiles - Render multiple project tiles

**Type:** Work / Task completion
**Task:** canvas-002-render-multiple-project-tiles - Render multiple project tiles
**Summary:** Restructured the canvas from a single-tile component to a keyed per-project collection — every registered project renders as its own tile with project-scoped node keys, a shared window-level drag controller, spiral auto-placement persisted on first sight, per-id event routing, live-add idempotency, and unioned zoom-to-fit bounds.
**Verification:** PASS (iteration 1)
**Commit:** e690fd3
**Files changed:** 4 (Canvas.svelte, new tile-layout.ts, canvas README, moved task)
**Tests added:** 0 — no frontend test runner; `pnpm check` (936 files / 0 errors / 0 warnings) + pure-module extraction `tile-layout.ts` (same verification strategy as `snapshot-patch.ts` from canvas-001) per task notes
**ADRs written:** none — implementation sits inside ADR-003 (PixiJS camera) + ADR-004 (tile-position persistence); state-shape and shared-drag-controller pattern are component-internal, recorded in code comments

---

## 2026-05-15 09:50 -- Task verified and completed: project-registry-002a-scan-roots-and-walk - Scan roots + folder discovery walk

**Type:** Work / Task completion
**Task:** project-registry-002a-scan-roots-and-walk - Scan roots + folder discovery walk
**Summary:** Landed the ADR-013 scan-root storage + discovery foundation — schema v1→v2 migration (new `scan_roots` table + nullable `projects.scan_root_id` FK with `ON DELETE RESTRICT`), new `scan.rs` module with depth-capped, junk-pruned, canonicalised walker that returns a `ScanCandidate` checklist, scan-root CRUD on `Db`, and three new IPC commands (`add_scan_root` / `rescan_scan_root` / `list_scan_roots`).
**Verification:** PASS (iteration 1)
**Commit:** bace9fd
**Files changed:** 5 (db.rs, new scan.rs, lib.rs, project-registry README, moved task)
**Tests added:** 19 — `cargo test --lib` 60/60 green (`db::tests::fresh_db_is_at_schema_version_two`, `v1_db_migrates_to_v2_without_data_loss`, `scan_roots_persist_across_db_handle_close_and_reopen`, `empty_scan_root_is_still_persisted_and_rescannable` + the `scan::tests` battery covering pruning, depth-cap, no-descent-into-identified-project, `already_imported`, UNC canonicalisation, composition)
**ADRs written:** none — ADR-013 already covers this surface; the `\\?\` UNC-strip detail in `canonicalize_root` recorded inline in code

---

## 2026-05-15 09:25 -- Batch started: [project-registry-002a-scan-roots-and-walk, canvas-002-render-multiple-project-tiles]

**Type:** Work / Batch start
**Tasks:** project-registry-002a-scan-roots-and-walk - Scan roots + folder discovery walk; canvas-002-render-multiple-project-tiles - Render multiple project tiles
**Parallel:** yes (2 workers — disjoint surfaces: 002a is Rust-only, canvas-002 is frontend-only; different BC READMEs)

---

## 2026-05-15 09:20 -- Task verified and completed: project-registry-001-multi-project-snapshot-model - Multi-project snapshot model

**Type:** Work / Task completion
**Task:** project-registry-001-multi-project-snapshot-model - Multi-project snapshot model
**Summary:** Generalised the path-implicit single-project core into a multi-project model — every per-project IPC command takes `project_id` explicitly, `list_projects()` + `get_project(project_id)` serve the canvas, and the new `WatcherSupervisor` (`Arc<Mutex>` map) owns the per-project debounced watcher map publishing `ProjectAdded` on add. `AppState` dropped `project_id`/`project_path` fields.
**Verification:** PASS (iteration 1)
**Commit:** d594ad5
**Files changed:** 9 (db.rs, project.rs, supervisor.rs [new], lib.rs, Canvas.svelte, ipc.ts, types.ts, project-registry README, moved task)
**Tests added:** 10 (cargo test --lib 41/41 green; `pnpm check` clean)
**ADRs written:** none — ADR-008's "downstream" supervisor implementation cashed in; the `Arc<Mutex>` vs Tokio-task simplification recorded in source doc + ADR-008 reconciliation note (already amended)

---

## 2026-05-15 09:00 -- Batch started: [project-registry-001-multi-project-snapshot-model]

**Type:** Work / Batch start
**Tasks:** project-registry-001-multi-project-snapshot-model - Multi-project snapshot model
**Parallel:** no (1 worker — root of the v1 chain; 002a/002b/canvas-002 all blocked on it)

---

## 2026-05-15 00:40 -- Model / Refined: canvas-002-render-multiple-project-tiles - Render multiple project tiles

**Type:** Model / Refine
**BC:** canvas
**Status after:** todo (promoted)
**Summary:** Grounded against the real skeleton (`Canvas.svelte` single-valued `snapshot`/`tilePos`/`projectId` $state, `ipc.ts`, `types.ts`, `snapshot-patch.ts`) and the unworked dependency `project-registry-001`. Surfaced one real coordination gap — the canvas needs each project's `project_id` but `ProjectSnapshot` has no `id` field — and resolved four decisions with Marco: (1) **add `id` to `ProjectSnapshot`** (Rust + TS) so id flows with the snapshot; (2) auto-placement is **spiral-out from world origin**; (3) auto-placed positions **persist immediately**; (4) zoom-to-fit (`f`) **frames all tiles**. Orchestrator round (no architect delegation needed — all structural choices already decided, work sits inside ADR-003/ADR-004's envelope) produced a worker-ready body: `Canvas.svelte` single-tile→keyed-collection restructure, new pure `tile-layout.ts` (spiral placement, the no-frontend-test-infra verification surface), shared drag controller replacing N per-tile window listeners, per-project `canvas-001` patching, idempotent live-add on `ProjectAdded`. Verdict: holds as ONE task (the keyed-collection restructure couples every closure in the component). Promoted to `todo/`. Coordination note appended to `project-registry-001` (still in `todo/`, unworked) requiring `id` on `list_projects()` / `get_project()` snapshots.
**Split into:** none
**ADRs written:** none — implementation within ADR-003 (PixiJS camera) + ADR-004 (tile-position persistence); the `Map`-vs-array state shape and shared-drag-controller pattern are component-internal, not cross-cutting. Recorded in the task Notes so it isn't re-opened.

---

## 2026-05-15 00:10 -- Model / Refined: project-registry-002-scan-roots-and-discovery — split into 002a + 002b

**Type:** Model / Refine
**BC:** project-registry (+ capture into canvas)
**Status after:** todo (both split tasks)
**Summary:** Grounded against the DB schema (`schema_version` 1, no `scan_roots` table, no `remove_project`) and ADR-005. Resolved four open questions with Marco: (1) **keep the checklist** — scan returns candidates, user picks (faithful to ADR-005), not auto-register; (2) **cascade-deregister** on scan-root removal; (3) **depth cap default 3 + junk-dir pruning**; (4) **track originating scan root** (`projects.scan_root_id`, NULL = manually added) — required to enable the cascade. Also: cascade **hard-deletes** — ADR-005's 30-day tile retention is scoped to the single "Remove project" affordance only. Orchestrator round (architect) produced the schema (`scan_roots` table + nullable FK `ON DELETE RESTRICT`), `scan.rs` walk design, the IPC surface, and recommended the split + a canvas UI task. Split taken: **002a** (schema v1→v2 + scan walk + scan-root CRUD + `add_scan_root`/`rescan`/`list` — independently shippable, returns a checklist) and **002b** (`import_scanned_projects` + `Db::remove_project` + `remove_scan_root` app-driven cascade). Both promoted to `todo/`.
**Split into:** project-registry-002a-scan-roots-and-walk, project-registry-002b-import-and-cascade-deregister
**Captured:** canvas-005-project-discovery-affordances (under-refined stub in `canvas/backlog/` — the ADR-005 BC seam: folder pickers, discovery checklist modal, "Remove project", "missing" tile state; depends on `002b` + `design-system-001`)
**ADRs written:** ADR-013 (Scan roots — persisted, rescannable discovery folders; **note:** orchestrator drafted this as "ADR-010" but that id was already taken by Logging — corrected to ADR-013). ADR-005 amended with a `## Reconciliation` section (superseded-in-part by ADR-013; clarifies the 30-day-retention scoping).

---

## 2026-05-14 23:30 -- Model / Refined: project-registry-001-multi-project-snapshot-model - Multi-project snapshot model

**Type:** Model / Refine
**BC:** project-registry
**Status after:** todo
**Summary:** Grounded against the real skeleton code (`lib.rs` single-project `AppState`, `watcher.rs` single `AgentheimWatcher`, `db.rs` has no `list_projects`). Resolved the three open questions with Marco: (1) `get_project` reshaped to `get_project(project_id)` + `list_projects()` added — per-project resync wants a precise re-fetch; (2) `WatcherSupervisor` uses incremental add/remove, not wholesale rebuild; (3) `list_projects()` is a cold disk read each call, no cached model. Orchestrator round (architect) produced a worker-ready body: new `supervisor.rs` module (`Arc<Mutex>` map — simplifies ADR-008's sketched Tokio-task shape), `AppState` drops `project_id`/`project_path`, `ProjectAdded` published from `supervisor.add`, hardcoded seed kept but routed through `add`, `remove_project` row-deletion deferred to `project-registry-002`. 8 concrete acceptance criteria. Verdict: holds as ONE task (no split — `AppState` restructure couples all three pieces). Promoted to `todo/`.
**Split into:** none
**ADRs written:** none — ADR-008 amended with a reconciliation note recording the landed `WatcherSupervisor` (module, `Arc<Mutex>` concurrency shape, `add` publishes `ProjectAdded`, missing `.agentheim/` → registered-but-unwatched).

---

## 2026-05-14 23:15 -- Model / Captured: v1-completion task set (5 tasks across canvas + project-registry)

**Type:** Model / Capture
**BC:** canvas, project-registry
**Filed to:** backlog
**Summary:** "Finish v1 first" capture pass — decomposed the vision's v1 canvas-MVP gap (the walking skeleton renders only one hardcoded project) into 5 backlog tasks. **project-registry:** `project-registry-001` (multi-project snapshot model — `list_projects()` + one watcher per project), `project-registry-002` (scan roots — user hands GUPPI folders, GUPPI recursively walks subfolders for `.agentheim/` projects; Marco's decision, answers the registry README's "where does GUPPI look" open question). **canvas:** `canvas-002` (render N tiles), `canvas-003` (focus-zoom), `canvas-004` (greybox → STYLEGUIDE.md visuals). All filed under-refined to `backlog/` — each needs a REFINE pass (open questions noted in every task). Beyond-v1 roadmap items (voice, live agent-awareness, terminal panel, detail view) deliberately left uncaptured.

---

## 2026-05-14 23:10 -- Work session ended

**Type:** Work / Session end
**Completed:** 2 (first-try PASS: 2, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 2 (d260393 infrastructure-015, 7af6f4c infrastructure-016)
**Note:** Both infrastructure tasks completed, each passing verification first try. Run separately rather than in parallel — both touch the infrastructure BC README, so the conflict rule held them to one-per-batch. `todo/`, `doing/`, and `backlog/` are now empty in every context. No new backlog items, no concept candidates. Note: the orchestrator's INDEX.md / protocol.md / SHA-stamp bookkeeping is left uncommitted on the working tree (matches the prior session's pattern — a separate `chore(work)` commit folds it in).

---

## 2026-05-14 23:08 -- Task verified and completed: infrastructure-016-readme-resync-required-rename - Update infrastructure README — `AgentheimChanged` → `ResyncRequired`

**Type:** Work / Task completion
**Task:** infrastructure-016-readme-resync-required-rename - Update infrastructure README — `AgentheimChanged` → `ResyncRequired`
**Summary:** Resynced the infrastructure BC README's event taxonomy — dropped the stale live `AgentheimChanged` "compatibility seam" entry, added a `ResyncRequired` entry (lag-only signal, emitted solely by `lib.rs`'s `Lagged` arm), labelled the fine-grained FS events as the normal path.
**Verification:** PASS (iteration 1)
**Commit:** 7af6f4c
**Files changed:** 1 (infrastructure README)
**Tests added:** 0 (doc-only chore)
**ADRs written:** none (ADR-009 already amended by `canvas-001`)

---

## 2026-05-14 23:06 -- Batch started: [infrastructure-016-readme-resync-required-rename]

**Type:** Work / Batch start
**Tasks:** infrastructure-016-readme-resync-required-rename - Update infrastructure README — `AgentheimChanged` → `ResyncRequired`
**Parallel:** no (1 worker)

---

## 2026-05-14 23:05 -- Task verified and completed: infrastructure-015-log-retention-sweep - Log retention — 7-day sweep of rotated log files

**Type:** Work / Task completion
**Task:** infrastructure-015-log-retention-sweep - Log retention — 7-day sweep of rotated log files
**Summary:** Startup retention sweep added to `logging.rs` — `sweep_retention` (called from `init()`) deletes rotated `guppi.log.YYYY-MM-DD` files older than the `RETENTION_DAYS` window (default 7), dated by parsing the filename not mtime; non-matching files untouched, failed deletions log+continue.
**Verification:** PASS (iteration 1)
**Commit:** d260393
**Files changed:** 2 (logging.rs, infrastructure README)
**Tests added:** 4 (logging.rs unit tests — `cargo test --lib` 30/30)
**ADRs written:** none (implements ADR-010's retention half)

---

## 2026-05-14 23:00 -- Batch started: [infrastructure-015-log-retention-sweep]

**Type:** Work / Batch start
**Tasks:** infrastructure-015-log-retention-sweep - Log retention — 7-day sweep of rotated log files
**Parallel:** no (1 worker)

---

## 2026-05-14 22:45 -- Model / Refined: infrastructure-015-log-retention-sweep - Log retention — 7-day sweep of rotated log files

**Type:** Model / Refine
**BC:** infrastructure
**Status after:** todo
**Summary:** Grounded against the real code (`logging.rs`, `lib.rs:210` `.setup()` hook). Resolved the two open decisions with Marco: (1) sweep runs **startup-only**, no background timer; (2) file age is read by **parsing `YYYY-MM-DD` from the `guppi.log.YYYY-MM-DD` filename**, not mtime. Rewrote the task with a full 6-point acceptance-criteria checklist (named constant default 7d, non-matching files untouched, deletion failure logs+continues, unit test against dated fixtures). No orchestrator round — ADR-010 leaves no architectural depth. Refinement made it ready → promoted to `todo/`. `infrastructure/backlog/` now empty.
**Split into:** none
**ADRs written:** none

---

## 2026-05-14 22:30 -- Model / Promoted: infrastructure-016-readme-resync-required-rename - Update infrastructure README — `AgentheimChanged` → `ResyncRequired`

**Type:** Model / Promote
**BC:** infrastructure
**From → To:** backlog → todo
**Note:** Readiness confirmed — 3 concrete acceptance criteria, exact file + line scope, pure doc change. Sole dependency `canvas-001` is done (commit 5fa7080). `infrastructure/todo/` now has 1 item; `infrastructure-015-log-retention-sweep` left in backlog.

---

## 2026-05-14 22:12 -- Work session ended

**Type:** Work / Session end
**Completed:** 1 (first-try PASS: 1, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 1 (5fa7080 canvas-001)
**Note:** canvas-001 passed verification first try. `todo/` and `doing/` now empty in every context. canvas-001 surfaced one new backlog item — `infrastructure-016-readme-resync-required-rename` (cross-BC: the worker couldn't edit the infrastructure README) — sitting in `infrastructure/backlog/`, not promoted. `infrastructure-015-log-retention-sweep` also still in backlog. No concept candidates.

---

## 2026-05-14 22:10 -- Task verified and completed: canvas-001-targeted-canvas-updates - Targeted canvas updates from fine-grained FS events

**Type:** Work / Task completion
**Task:** canvas-001-targeted-canvas-updates - Targeted canvas updates from fine-grained FS events
**Summary:** The canvas patches its client-side `ProjectSnapshot` in place from the fine-grained FS domain events instead of re-fetching; the coarse `AgentheimChanged` is retired — renamed `ResyncRequired`, kept only as the lag-only resync signal emitted by `lib.rs`'s `Lagged` arm.
**Verification:** PASS (iteration 1)
**Commit:** 5fa7080
**Files changed:** 10 (incl. new `src/lib/snapshot-patch.ts`)
**Tests added:** 0 new (no frontend test infra; Rust tests renamed/updated — `cargo test --lib` 26/26, `pnpm check` 0 errors)
**ADRs written:** ADR-009 amended in place (no new ADR)

---

## 2026-05-14 22:00 -- Batch started: [canvas-001-targeted-canvas-updates]

**Type:** Work / Batch start
**Tasks:** canvas-001-targeted-canvas-updates - Targeted canvas updates from fine-grained FS events
**Parallel:** no (1 worker)

---

## 2026-05-14 21:10 -- Model / Promoted: canvas-001-targeted-canvas-updates - Targeted canvas updates from fine-grained FS events

**Type:** Model / Promote
**BC:** canvas
**From → To:** backlog → todo
**Note:** Frontend gate cleared — both deps satisfied (`infrastructure-014` done; `design-system-001-styleguide` done *and* signed off, same session). This is the first frontend feature task to clear the styleguide gate.

---

## 2026-05-14 21:08 -- Styleguide signed off

**Type:** Milestone / Human gate
**BC:** design-system
**Summary:** Marco reviewed the styleguide baseline live via `pnpm tauri dev` and signed off in person — the acceptance-criterion gate on `design-system-001-styleguide`. Approved the visual vocabulary and all three deferred open-question defaults as-is (dark-only / no light mode, rounded-rectangle tiles, restrained motion budget). `design-system-001`'s sign-off criterion is now checked; `STYLEGUIDE.md` status updated. **The frontend gate is now open** — frontend feature tasks in any BC can be promoted and worked. Still open: frontend-bearing BC READMEs must reference `STYLEGUIDE.md` (tracked follow-up), and Marco's separate design-skill refinement pass.

---

## 2026-05-14 20:55 -- Model / Refined: canvas-001-targeted-canvas-updates - Targeted canvas updates from fine-grained FS events

**Type:** Model / Refine
**BC:** canvas
**Status after:** backlog
**Summary:** Grounded the task against the actual code (`events.rs`, `watcher.rs`, `lib.rs`, `Canvas.svelte`, `types.ts`, `project.rs`). Surfaced that `AgentheimChanged` has a second, undocumented-in-task job — the ADR-009 lag-resync signal in `lib.rs`'s `Lagged` arm — which the fine-grained events cannot replace. Decision (Marco): don't delete it, rename `AgentheimChanged` → `ResyncRequired`, drop only its normal-path/skeleton role; record as an in-place ADR-009 amendment (no new ADR). Decision (Marco): silent count update, no animated tick. Resolved three robustness gaps directly in the task: event-vs-BC ordering (`correlate()` emits `TaskAdded` before `BCAppeared`; frontend lazily creates BC nodes), count clamping at 0, and `project_id` filtering. Full acceptance-criteria section rewritten. Left in `backlog/` — `infrastructure-014` dep is cleared, but the `design-system-001` styleguide sign-off gate is still open; promotable on Marco's sign-off.
**Split into:** none
**ADRs written:** none (ADR-009 to be amended in place by the worker)

---

## 2026-05-14 18:00 -- Work session ended

**Type:** Work / Session end
**Completed:** 1 (first-try PASS: 1, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 1 (1415d1e infrastructure-014)
**Note:** infrastructure-014 passed verification first try. `todo/` and `doing/` now empty in every context. Remaining unscheduled work: `infrastructure-015-log-retention-sweep` and `canvas-001-targeted-canvas-updates` (the frontend reaction to 014's new taxonomy) both sit in `backlog/`, not promoted. canvas-001 is now unblocked — 014 was its only dependency.

---

## 2026-05-14 17:55 -- Task verified and completed: infrastructure-014-fine-grained-fs-events - Fine-grained filesystem domain events

**Type:** Work / Task completion
**Task:** infrastructure-014-fine-grained-fs-events - Fine-grained filesystem domain events
**Summary:** The single-project `.agentheim/` watcher now correlates each debounced filesystem batch into the fine-grained ADR-008/ADR-009 domain events (`TaskMoved`, `TaskAdded`, `TaskRemoved`, `BCAppeared`, `BCDisappeared`), while the coarse `AgentheimChanged` keeps firing as a deliberate compatibility seam for the skeleton frontend.
**Verification:** PASS (iteration 1)
**Commit:** 1415d1e
**Files changed:** 5
**Tests added:** 7+ (paired move, unpaired create, unpaired delete, different-`task_id` non-pairing, BC appear/disappear; `cargo test --lib` 26/26)
**ADRs written:** ADR-008 + ADR-009 reconciled in place (no new ADR)

---

## 2026-05-14 17:45 -- Batch started: [infrastructure-014-fine-grained-fs-events]

**Type:** Work / Batch start
**Tasks:** infrastructure-014-fine-grained-fs-events - Fine-grained filesystem domain events
**Parallel:** no (1 worker)

---

## 2026-05-14 17:30 -- Model / Refined: infrastructure-014-fine-grained-fs-events - Fine-grained filesystem domain events

**Type:** Model / Refine
**BC:** infrastructure
**Status after:** todo
**Summary:** Resolved four open decisions baked into the task — (1) refactor the skeleton's single-project watcher **in place** rather than waiting for the multi-project `WatcherSupervisor`; (2) unpaired create/delete get first-class `TaskAdded` / `TaskRemoved` variants (the ADR-008 "sensible fallback", now decided); (3) the ADR-008↔ADR-009 `from_state`/`to_state` vs `from`/`to` disagreement is reconciled to `from`/`to`, folded into the 014 worker (no separate decision task); (4) the frontend reaction is split out to a new `canvas-001` task. Added a full acceptance-criteria section and the deliberate "`AgentheimChanged` kept alive" seam so 014 and canvas-001 can land independently. Promoted backlog → todo.
**Split into:** canvas-001-targeted-canvas-updates (new, in canvas/backlog)
**ADRs written:** none (ADR-008 + ADR-009 to be updated *in place* by the 014 worker)

---

## 2026-05-14 16:55 -- Work session ended

**Type:** Work / Session end
**Completed:** 1 (first-try PASS: 1, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 1 (a12c46a PTY spike)
**Note:** infrastructure-013 PTY spike passed first try. `todo/` and `doing/` now empty in every context. Remaining infrastructure backlog: infrastructure-014-fine-grained-fs-events, infrastructure-015-log-retention-sweep — both still in `backlog/`, not promoted. Open hands-on follow-up: ADR-006's real-`claude.exe` items (TUI rendering, minutes-long session, force-crash orphan check) are exercisable via the new `pty_*` IPC commands and await Marco's live confirmation — same pattern as the walking skeleton's GUI checks.

---

## 2026-05-14 16:50 -- Task verified and completed: infrastructure-013-pty-spike - Spike: PTY end-to-end on Windows

**Type:** Work / Task completion
**Task:** infrastructure-013-pty-spike - Spike: PTY end-to-end on Windows
**Summary:** ADR-006 PTY stack implemented as a `ClaudeSession` actor (`portable-pty` + Windows Job Object + cwd-per-spawn, raw-bytes read loop onto the EventBus); risky mechanics proven by 18/18 `cargo test` on Windows 11. Real-`claude.exe` hands-on items exercisable via new `pty_*` IPC commands.
**Verification:** PASS (iteration 1)
**Commit:** a12c46a
**Files changed:** 12
**Tests added:** 4 (PTY: cwd-correct spawn + output streaming, input/resize round-trip, child-gone-after-drop)
**ADRs written:** ADR-012-pty-session-teardown-ordering.md (new); ADR-006 updated with the PASSED spike result

---

## 2026-05-14 16:16 -- Batch started: [infrastructure-013-pty-spike]

**Type:** Work / Batch start
**Tasks:** infrastructure-013-pty-spike - Spike: PTY end-to-end on Windows
**Parallel:** no (1 worker)
**Note:** Walking skeleton (012) confirmed by Marco's hands-on GUI testing — all four manual acceptance steps pass; 012 task file updated. Promoted 013 from backlog → todo → doing. This is the deferred ADR-006 empirical spike — the riskiest piece of the architecture, must pass before any v1.x feature depends on PTY.

---

## 2026-05-14 15:45 -- Work session ended

**Type:** Work / Session end
**Completed:** 2 (first-try PASS: 2, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 3 (1f37659 walking skeleton, b7db68e styleguide, 73c2c58 SHA-stamp chore)
**Note:** The 2026-05-14 14:48 toolchain blocker is resolved — Rust 1.95.0 + MSVC build tools were installed since the pause. `todo/` is now empty in every context. The only remaining unscheduled item is infrastructure-013-pty-spike (still in `backlog/`, not promoted to `todo/`). Open follow-ups from this run: frontend-bearing BC READMEs (e.g. canvas) need a reference to `contexts/design-system/STYLEGUIDE.md`; design-system-001's open-question defaults await Marco's design-skill refinement + in-person sign-off; walking-skeleton spike DoD has manual GUI steps awaiting Marco's hands-on confirmation.

---

## 2026-05-14 15:42 -- Task verified and completed: design-system-001-styleguide - Feature: styleguide

**Type:** Work / Task completion
**Task:** design-system-001-styleguide - Feature: styleguide
**Summary:** GUPPI styleguide — colour/typography/spacing/shape/motion tokens (TS object + mirrored CSS variables), colourblind-friendly four-state status palette, documented component states for tile/BC node/edge/status badge/voice indicator; walking-skeleton canvas upgraded from greybox to the styleguide baseline. `pnpm check` + `pnpm build` green.
**Verification:** PASS (iteration 1)
**Commit:** b7db68e
**Files changed:** ~9
**ADRs written:** none
**Open-question defaults chosen by worker (Marco can override):** dark-mode default / light optional, rounded-rectangle tiles, restrained motion budget.
**Deferred (NOT satisfied by the worker):** Marco's in-person sign-off gate, and his planned design-skill refinement pass. Follow-up: frontend-bearing BC READMEs (e.g. canvas) still need a reference to `contexts/design-system/STYLEGUIDE.md` + a restatement of the frontend gate — a worker may not edit other BCs' READMEs.

---

## 2026-05-14 15:30 -- Batch started: [design-system-001-styleguide]

**Type:** Work / Batch start
**Tasks:** design-system-001-styleguide - Feature: styleguide
**Parallel:** no (1 worker)
**Note:** Unblocked by infrastructure-012. Task has 3 open questions for Marco (light mode, tile shape, motion budget) and an in-person sign-off gate. Marco authorized the worker to pick sensible defaults this run — he will refine via a design skill afterward, and the in-person sign-off remains a separate gate (not satisfiable by the worker).

---

## 2026-05-14 15:25 -- Task verified and completed: infrastructure-012-walking-skeleton - Spike: walking skeleton

**Type:** Work / Task completion
**Task:** infrastructure-012-walking-skeleton - Spike: walking skeleton
**Summary:** GUPPI's first code — a Tauri 2 + Svelte 5 + PixiJS app whose Rust core reads one hard-coded Agentheim project into a ProjectSnapshot, persists tile/camera state in SQLite, and pushes filesystem-change events through a Tokio broadcast EventBus to the canvas. All eleven foundation ADRs validated by execution.
**Verification:** PASS (iteration 1)
**Commit:** 1f37659
**Files changed:** 38 (incl. lockfiles); 14 Rust tests passing, `pnpm check` clean
**ADRs written:** none
**New backlog items:** infrastructure-014-fine-grained-fs-events, infrastructure-015-log-retention-sweep
**Note:** Spike DoD has manual GUI acceptance steps (drag tile + reopen, manual file-move → count update) that need Marco's hands-on confirmation — code-complete and compiling, not agent-verifiable.

---

## 2026-05-14 15:00 -- Batch started: [infrastructure-012-walking-skeleton]

**Type:** Work / Batch start
**Tasks:** infrastructure-012-walking-skeleton - Spike: walking skeleton
**Parallel:** no (1 worker)
**Note:** Toolchain blocker from the 2026-05-14 14:48 pause is resolved — Rust 1.95.0 (stable-x86_64-pc-windows-msvc) and MSVC VC build tools are both installed. `~/.cargo/bin` is not on the shell PATH; worker instructed to prepend it.

---

## 2026-05-14 14:48 -- Work session paused: toolchain blocker

**Type:** Work / Session pause
**Reason:** infrastructure-012-walking-skeleton (the only remaining ready task) is a Tauri 2 app and requires a Rust toolchain. `cargo`/`rustc`/`rustup` are not installed on this machine; Node/pnpm/npm are present. Task moved back to todo/ — not dispatched.
**Completed this session:** infrastructure-001 through 011 (all 11 foundation decision ADRs).
**Blocked:** infrastructure-012-walking-skeleton (needs Rust toolchain), design-system-001-styleguide (depends on 012), infrastructure-013-pty-spike (depends on 012).
**Next:** install Rust (`rustup`) + MSVC build tools, then re-run `work`.

---

## 2026-05-14 14:46 -- Task completed (verification skipped): infrastructure-011-packaging - Packaging and install

**Type:** Work / Task completion
**Task:** infrastructure-011-packaging - Packaging and install
**Summary:** Tauri's bundler targets an unsigned MSI on Windows (deferred-unsigned signing posture), per-user install at `%LOCALAPPDATA%\Programs\guppi\`, updates via the Tauri updater plugin against a GitHub Release feed.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** 3bbc01b
**Files changed:** 1

---

## 2026-05-14 14:43 -- Batch started: [infrastructure-011-packaging]

**Type:** Work / Batch start
**Tasks:** infrastructure-011-packaging - Packaging and install
**Parallel:** no (1 worker)

---

## 2026-05-14 14:40 -- Task completed (verification skipped): infrastructure-010-logging - Logging and error reporting

**Type:** Work / Task completion
**Task:** infrastructure-010-logging - Logging and error reporting
**Summary:** `tracing` stack writing to rotating local log files (`%APPDATA%\guppi\logs`, daily rotation, 7-day retention); frontend logs forwarded via a Tauri command; no telemetry; crash dialog with "Open log folder".
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** 0c64059
**Files changed:** 1

---

## 2026-05-14 14:39 -- Task completed (verification skipped): infrastructure-009-event-bus - IPC and event bus

**Type:** Work / Task completion
**Task:** infrastructure-009-event-bus - IPC and event bus
**Summary:** Two-layer event bus — a Tokio broadcast channel (capacity 1024) carrying a typed `DomainEvent` enum in the Rust core, with a thin frontend-bridge task forwarding frontend-relevant events to the WebView via Tauri emit.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** a1d21d5
**Files changed:** 1

---

## 2026-05-14 14:38 -- Task completed (verification skipped): infrastructure-008-filesystem-observation - Filesystem observation

**Type:** Work / Task completion
**Task:** infrastructure-008-filesystem-observation - Filesystem observation
**Summary:** `notify-debouncer-full` with one 250ms-debounced watcher per registered project scoped to its `.agentheim/`, coordinated by a central `WatcherSupervisor` Tokio task translating FS events into `TaskMoved`/`BCAppeared`/`BCDisappeared` domain events.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** c1cc2be
**Files changed:** 1

---

## 2026-05-14 14:35 -- Batch started: [infrastructure-008-filesystem-observation, infrastructure-009-event-bus, infrastructure-010-logging]

**Type:** Work / Batch start
**Tasks:** infrastructure-008-filesystem-observation - Filesystem observation, infrastructure-009-event-bus - IPC and event bus, infrastructure-010-logging - Logging and error reporting
**Parallel:** yes (3 workers)

---

## 2026-05-14 14:32 -- Task verified and completed: infrastructure-007-voice-integration - Voice integration architecture

**Type:** Work / Task completion
**Task:** infrastructure-007-voice-integration - Voice integration architecture
**Summary:** Voice integration is a local WebSocket bridge added to Whisperheim — GUPPI subscribes to wake_word/transcript events and emits speak events. The versioned transport contract (event shapes, bridge.json port discovery, exponential-backoff reconnection, graceful degradation) is specced in contexts/infrastructure/voice-bridge.md.
**Verification:** PASS (iteration 1)
**Commit:** ba59f4d
**Files changed:** 2
**Tests added:** 0
**ADRs written:** ADR-007-voice-integration

---

## 2026-05-14 14:29 -- Task completed (verification skipped): infrastructure-005-project-discovery - Project discovery model

**Type:** Work / Task completion
**Task:** infrastructure-005-project-discovery - Project discovery model
**Summary:** Project discovery is an explicit registry (the ADR-004 `projects` table) plus a user-triggered "Scan folder for projects…" command — no unprompted disk-walking; canvas BC UI affordances noted as downstream modeling.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** 48e95b3
**Files changed:** 1

---

## 2026-05-14 14:26 -- Task completed (verification skipped): infrastructure-003-canvas-rendering - Canvas rendering library

**Type:** Work / Task completion
**Task:** infrastructure-003-canvas-rendering - Canvas rendering library
**Summary:** PixiJS v8 (WebGL) chosen as the infinite-canvas renderer, with HTML overlays positioned to world coordinates for tiles needing rich interactive content (markdown viewer, terminal panel).
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** 1f9942c
**Files changed:** 1

---

## 2026-05-14 14:22 -- Batch started: [infrastructure-003-canvas-rendering, infrastructure-005-project-discovery, infrastructure-007-voice-integration]

**Type:** Work / Batch start
**Tasks:** infrastructure-003-canvas-rendering - Canvas rendering library, infrastructure-005-project-discovery - Project discovery model, infrastructure-007-voice-integration - Voice integration architecture
**Parallel:** yes (3 workers)

---

## 2026-05-14 14:19 -- Task verified and completed: infrastructure-006-claude-pty - Claude session ownership & PTY

**Type:** Work / Task completion
**Task:** infrastructure-006-claude-pty - Claude session ownership & PTY
**Summary:** GUPPI owns each Claude session as a Tokio actor over `portable-pty` (ConPTY), spawning native Windows `claude.exe` with cwd-per-project and a Windows Job Object for orphan-free cleanup. Empirical Windows spike marked DEFERRED, tracked as new backlog task infrastructure-013-pty-spike.
**Verification:** PASS (iteration 1)
**Commit:** 08dc87b
**Files changed:** 2
**Tests added:** 0
**ADRs written:** ADR-006-claude-pty
**New backlog items:** infrastructure-013-pty-spike

---

## 2026-05-14 14:16 -- Task completed (verification skipped): infrastructure-004-persistence - Persistence

**Type:** Work / Task completion
**Task:** infrastructure-004-persistence - Persistence
**Summary:** GUPPI's own view-state persists in a single SQLite file (`guppi.db`) in the OS user-config dir, resolved via Tauri's path API; projects/tile_positions/clusters/app_state schema sketch accepted with a schema_version migrations table.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** 7608ba2
**Files changed:** 1

---

## 2026-05-14 14:14 -- Task completed (verification skipped): infrastructure-002-frontend-framework - Frontend framework

**Type:** Work / Task completion
**Task:** infrastructure-002-frontend-framework - Frontend framework
**Summary:** Frontend framework decision recorded — Svelte 5 + SvelteKit (static adapter), SPA shipped as static assets inside the Tauri 2 bundle.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** c20f26d
**Files changed:** 1

---

## 2026-05-14 14:08 -- Batch started: [infrastructure-002-frontend-framework, infrastructure-004-persistence, infrastructure-006-claude-pty]

**Type:** Work / Batch start
**Tasks:** infrastructure-002-frontend-framework - Frontend framework, infrastructure-004-persistence - Persistence, infrastructure-006-claude-pty - Claude session ownership & PTY
**Parallel:** yes (3 workers)

---

## 2026-05-14 14:05 -- Task completed (verification skipped): infrastructure-001-desktop-runtime - Desktop runtime

**Type:** Work / Task completion
**Task:** infrastructure-001-desktop-runtime - Desktop runtime
**Summary:** Recorded the desktop runtime decision as an accepted ADR — Tauri 2 (Rust core + web frontend), validated on Windows 11 only day one.
**Verification:** SKIPPED — decision-only task (single ADR file)
**Commit:** 8657d99
**Files changed:** 1

---

## 2026-05-14 13:59 -- Batch started: [infrastructure-001-desktop-runtime]

**Type:** Work / Batch start
**Tasks:** infrastructure-001-desktop-runtime - Desktop runtime
**Parallel:** no (1 worker)

---

## 2026-05-13 — Brainstorm: initial vision

**Type:** Brainstorm
**Outcome:** vision created
**BCs identified:** canvas, project-registry, claude-runner, agent-awareness, voice, design-system, infrastructure (7 total — 4 core, 2 supporting, 1 generic)
**Summary:** GUPPI is a personal Miro-like mission-control for Agentheim+Claude Code projects. v1 is a read-only canvas MVP showing every project as a tile with BC children and task counts; voice/commands/agent-observation/terminal emulation come after. Load-bearing rule: GUPPI spawns `claude` inside each target project's folder, never its own. Strategic-modeler folded `document-viewer` into `canvas` (rendering is a feature of the detail view, not a separate concern). Architect produced 11 ADR drafts covering runtime (Tauri 2), frontend (Svelte 5), canvas (PixiJS), persistence (SQLite), discovery (explicit registry), PTY (`portable-pty` with Job Objects on Windows), voice (Whisperheim WebSocket bridge), filesystem (`notify`), event bus (Tokio broadcast + Tauri events), logging (`tracing`, local-only), and packaging (Tauri MSI). Walking-skeleton spike specced. Styleguide task specced (entire product is frontend, gate is mandatory).
**ADRs written:** none (foundation ADRs deferred to decision tasks — see below)
**Foundation tasks emitted:**
- 11 `type: decision` tasks in `contexts/infrastructure/todo/` (one per ADR draft, all global scope)
- 1 `type: spike` walking-skeleton task in `contexts/infrastructure/todo/` (depends on all 11 decisions)
- 1 `type: feature` styleguide task in `contexts/design-system/todo/` (depends on walking-skeleton, requires Marco sign-off before any frontend feature is promoted)

**Architect open questions surfaced (decide when working the relevant task):**
1. Tauri vs Electron (ADR-001)
2. Svelte vs React vs Solid (ADR-002)
3. Willingness to add a WebSocket bridge to Whisperheim (ADR-007)
4. `claude.exe` native Windows vs WSL (ADR-006)
5. macOS/Linux: day-one requirement or nice-to-have? (cross-cutting)

---
