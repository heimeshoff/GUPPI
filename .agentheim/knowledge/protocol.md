# Protocol

Chronological log of everything that happens in this project.
Newest entries on top.

---

## 2026-05-19 17:00 -- Task verified and completed: canvas-012 - Drag state can stick after pointerup — subsequent mouse moves pan the canvas

**Type:** Work / Task completion
**Task:** canvas-012-drag-state-stickiness - Drag state can stick after pointerup — subsequent mouse moves pan the canvas
**Summary:** Canvas drag controller extracted into a pure `src/lib/drag-controller.ts` module (verification-surface peer of `tile-layout.ts` / `bc-layout.ts` / `snapshot-patch.ts`, zero Svelte + zero Pixi imports), replacing the four-variable drag-state spread (`dragProjectId` / `dragBcName` / `dragOriginX/Y` at module scope + `panning` at function scope) with a single discriminated-union `DragState` (`idle` | `panning` | `frame` | `bc`) and a `Target` descriptor (`empty` | `frameHeader` | `bcBubble`); five transition fns (`onPointerDown` / `onPointerMove` / `onPointerUp` / `onPointerCancel` / `onPointerLeave`). The canonical stuck-pan repro (clicking a frame body, releasing, then moving the mouse) is closed by the state machine alone — `onPointerUp` lands in IDLE for every state kind, unconditionally; the old `window.pointerup` clear inside `if (panning)` after two early-return guards is gone by construction. New `window` `pointercancel` + `pointerleave` listeners wired as terminal events (both unconditional IDLE). `button === 2` short-circuit keeps right-clicks from entering a drag. Frame `hitArea` (header-bar-only) preserved — the frame-body pass-through that lets empty regions inside a frame pan the camera (per canvas-007) is unchanged. Pointer Capture API migration deliberately OUT — the audit confirmed the bug is a one-line unconditional-clear + two missing terminal listeners, not a structural failure of the window-listener model; the SM extraction makes a future PC migration cheap (one module's `onPointerUp` reroute, not three sprawling handlers) if an overlay-intercept failure mode ever surfaces. canvas/README.md gains drag-controller vocabulary entries.
**Verification:** PASS (iteration 1)
**Commit:** a2af868
**Files changed:** 3 (src/lib/drag-controller.ts NEW; src/lib/Canvas.svelte; .agentheim/contexts/canvas/README.md)
**Tests added:** 0 — legitimate TDD-skip per the doctrine's "UI tasks where the project has no UI test infrastructure" category, with the same-batch `infrastructure-017` adding the test runner. `drag-controller.ts` is the canonical pure-module shape that vitest tests will target in a future backfill task. `pnpm check` 0/0 (990 files); `cargo test --lib` 122/122.
**ADRs written:** none — the drag-controller extraction is component-internal; the discriminated-union shape + the no-imports invariant + the five-fn surface are documented in canvas/README.md and the module's own header. ADR candidate from the refinement pass deferred (write trigger: if the controller grows a second consumer or the persistence boundary shifts).
**New backlog items:** none.
**Note:** Final v1 design-refresh blocker triad item lands — canvas-014 (perf spike, 2026-05-18) + canvas-013 (rendering invariants, 2026-05-19) + canvas-012 (this commit) now all done. The verifier audit ran `pnpm check` + `cargo test --lib` end-to-end on the working tree and confirmed both green. canvas-015 (substantive renderer rewrite) is the next batch — it inherits the post-extraction `Canvas.svelte` shape, which means the drag-handler call sites are 3–5 lines each calling into the controller, instead of the prior sprawling per-handler closures.

---

## 2026-05-19 17:00 -- Task verified and completed: infrastructure-017 - Frontend test infrastructure (Vitest for the pure modules)

**Type:** Work / Task completion
**Task:** infrastructure-017-frontend-test-infrastructure - Frontend test infrastructure — Vitest for the pure modules
**Summary:** Vitest 2 stood up as GUPPI's frontend test runner for the pure module surface; 29 characterisation tests landed across `snapshot-patch.ts` (10), `tile-layout.ts` (9), `bc-layout.ts` (10) codifying the load-bearing invariants captured during canvas-001 / canvas-002 / canvas-007 — lazy zero-count BC creation, count clamp at 0, idempotent `bc_appeared`, Rust-matching BC sort order, `bc_relationships_changed` node-ensuring behaviour; spiral origin + leg pattern + adjacency + injectivity + purity + defensive fallback for NaN/Infinity; deterministic mulberry32-seeded BC layout, sticky pins, finite-positions-around-pins, related-BC-cluster edge-length metric, frame auto-fit, empty-input floor. `vitest.config.ts` deliberately skips the SvelteKit pipeline (node env, no `@sveltejs/kit/vite` plugin) per ADR-002's lean-runtime stance — the pure modules have zero Svelte + zero Pixi imports by design and don't need the framework wrapper. `package.json` gains `"test": "vitest run"`; `pnpm-lock.yaml` picks up vitest + transitive deps (`@vitest/*`, `chai`, `tinybench`, `tinypool`, `tinyspy`, `loupe`, `pathe`). README adds two new vocab entries (Pure module, Characterisation test). `pnpm test` green 29/29 (~811ms); `pnpm check` 0/0 (990 files).
**Verification:** PASS (iteration 1)
**Commit:** 8e9383e
**Files changed:** 7 (package.json; pnpm-lock.yaml; vitest.config.ts NEW; src/lib/snapshot-patch.test.ts NEW; src/lib/tile-layout.test.ts NEW; src/lib/bc-layout.test.ts NEW; .agentheim/contexts/infrastructure/README.md)
**Tests added:** 29 — `snapshot-patch.test.ts` 10, `tile-layout.test.ts` 9, `bc-layout.test.ts` 10. Worker authored characterisation tests against existing pure modules (not red-green-refactor — the modules already shipped in production); each test codifies an invariant from the originating canvas-task's Outcome section. Verifier counted ≥ 3 per file (AC #2) and confirmed each invariant is named in the corresponding `test(` description.
**ADRs written:** none — the choice not to wire `@sveltejs/kit/vite` is documented in `vitest.config.ts`'s header comment + the BC README vocabulary entry; no separate ADR (ADR-002 already covers the framework stance).
**New backlog items:** none. **Cross-task significance:** unblocks a future backfill of `src/lib/drag-controller.ts` tests (canvas-012's pure extract, also this batch). The drag-controller is now the canonical pure-module shape that vitest can target without infra friction.
**Note:** Disjoint-file parallel landing with canvas-012 (this batch) — neither worker touched the other's files; the lockfile + `package.json` script were the only shared-surface concerns and were owned cleanly by this worker. The verifier ran `pnpm test` + `pnpm check` end-to-end on the working tree (with canvas-012's diff also present) and confirmed both green, demonstrating the two diffs compose.

---

## 2026-05-19 16:50 -- Batch started: [canvas-012, infrastructure-017]

**Type:** Work / Batch start
**Tasks:** canvas-012-drag-state-stickiness - Drag state can stick after pointerup — subsequent mouse moves pan the canvas, infrastructure-017-frontend-test-infrastructure - Frontend test infrastructure — Vitest for the pure modules
**Parallel:** yes (2 workers — canvas-015 demoted to next batch, conflicts with canvas-012 on Canvas.svelte)

---

## 2026-05-19 14:55 -- Model / Promoted: canvas-015 - Persistent scene graph + camera as stage transform (replace tear-down/rebuild render)

**Type:** Model / Promote
**BC:** canvas
**From → To:** backlog → todo
**Summary:** Substantive renderer rework follow-up to the `canvas-perf-2026-05-17` spike (canvas-014, done). Replaces `renderScene`'s tear-down-and-rebuild model (`world.removeChildren()` + 200–500 fresh Pixi `Graphics` / `Text` / `Container` allocations per call) with persistent display objects per project frame + BC bubble + intra-project edge, keyed by `entry.id`; pan/zoom drives `world.position` / `world.scale` so the GPU handles the camera transform, instead of pre-projecting every Graphics to screen coords in JS. Readiness check passed without further refinement: 11 concrete ACs (no `world.removeChildren`; pan = `world.position` update only; wheel-zoom = `world.scale` + `world.position`; drag persistence unchanged; theme flip via `.clear()` + repaint; hover focus ring `.visible` toggle; missing-tile via property updates; frame-time ≤ 8 ms with 10+ frames + sustained 60 FPS during continuous pan; no regressions across canvas-002 / 005a / 005b / 006 / 007 / 008 + design-system-004 theme path; `pnpm check` + `cargo test --lib` clean), deps satisfied (canvas-014 done), staged-landing recommendation already baked into the task body (frames-only → BC bubbles → edges → camera-transform switch — each stage independently `pnpm check`-clean and smoke-testable). 1–3 day estimate; touches `Canvas.svelte` extensively. Coordinates with canvas-013 (now done — the project-title HTML overlay was reverted, so the worker on canvas-015 inherits project titles as Pixi `Text` in this refactor's surface).

---

## 2026-05-19 14:55 -- Model / Promoted: infrastructure-017 - Frontend test infrastructure (Vitest for the pure modules)

**Type:** Model / Promote
**BC:** infrastructure
**From → To:** backlog → todo
**Summary:** Long-standing test-surface gap closes: the three pure Svelte/Pixi-free modules under `src/lib/` (`snapshot-patch.ts` from canvas-001, `tile-layout.ts` from canvas-002, `bc-layout.ts` from canvas-007) were each extracted specifically so they could be unit-tested in isolation when test infra lands — each canvas task's done note has flagged the absence of that infra as the reason no `*.test.ts` files were added. Readiness check passed: 4 concrete ACs (`pnpm test` green from clean clone; ≥ 3 tests per pure module covering load-bearing invariants; `pnpm check` stays 0/0/0; CI integration when CI exists), explicit Scope-In (vitest config, devDependency + script, `*.test.ts` files in `src/lib/`) and Scope-Out (Svelte component tests, E2E / WebDriver) boundaries, no unmet deps (`depends_on: []`). **Cross-task significance:** unblocks the eventual unit-test surface for `src/lib/drag-controller.ts` (canvas-012's extraction target) — once both ship, the drag-controller becomes the first new pure module to land WITH tests, instead of waiting for a future backfill.

---

## 2026-05-19 14:35 -- Model / Promoted: canvas-012 - Drag state can stick after pointerup — subsequent mouse moves pan the canvas

**Type:** Model / Promote
**BC:** canvas
**From → To:** backlog → todo
**Summary:** Same-turn promotion following the REFINE pass above. Readiness check passed without another refinement: 9 concrete acceptance criteria (canonical-repro fixed, 3 drag-kind coverage, `pointercancel` + `pointerleave` wirings, right-click-during-drag, `src/lib/drag-controller.ts` extraction with no Svelte/Pixi imports, reproducer documented, `pnpm check` + `cargo test --lib` clean), no unmet deps (`depends_on: []`), source-pinned diagnosis (line numbers in `Canvas.svelte` for every claim site + the leak line). Last v1 blocker from the 2026-05-17 hands-on verification triad enters todo — canvas-014 (perf spike) + canvas-013 (rendering invariants) already done, canvas-012 (this) now ready for `/agentheim:work`.

---

## 2026-05-19 14:30 -- Model / Refined: canvas-012 - Drag state can stick after pointerup — subsequent mouse moves pan the canvas

**Type:** Model / Refine
**BC:** canvas
**Mode:** Interrogator (REFINE default)
**Status after:** backlog (promotion-ready, awaiting user sign-off)
**Summary:** Bug task refined against Marco's hands-on canonical reproducer ("click on a project frame body, release the mouse, then move — the screen pans"). Two scope locks made before delegation: (1) **fix shape = extract a small state machine** alongside the fix (pure module, no frontend test infra in scope — `infrastructure-017` stays in its own backlog); (2) **PC API migration permitted IF the audit reveals it's cleaner**. The orchestrator routed-to-self (read `Canvas.svelte` directly rather than delegating to architect; ranked the audit conclusive enough to skip the specialist hop). Diagnosis pinned to source line numbers: empty-canvas `panning` is a function-scope variable set at ~L1123 and cleared at ~L1213 only inside an `if (panning) { … }` branch that the BC-drag and frame-drag guards (~L1187 / ~L1204) can early-return past. Four drag-state variables spread across two scopes (`dragProjectId` / `dragBcName` / `dragOriginX/Y` at module scope, `panning` at function scope) is the structural root cause — invisible to single-function scanning. `pointercancel` and `pointerleave` are NOT currently wired at all; both are required by the existing ACs (new defensive coverage, not "tighten existing handlers"). PC API migration is **OUT** for this task — the bug is a one-line unconditional-clear fix plus two missing handlers, not a structural failure of the window-listener model; PC's cost (rewiring three claim sites against the underlying DOM canvas since PixiJS v8 events are synthetic) doesn't pay for itself. Future trigger named: if an overlay-intercept failure mode surfaces post-fix (Modal / context menu swallows pointerup-during-pan), file a follow-up PC migration task — the SM extraction makes that future migration cheap (one module's `onPointerUp` reroute, not three sprawling handlers). Refined `What` body documents the canonical repro, the source-pinned diagnosis, the structural fix (extraction to `src/lib/drag-controller.ts` with discriminated-union `DragState` + `Target` types + 5 pure transition functions: `onPointerDown` / `onPointerMove` / `onPointerUp` / `onPointerCancel` / `onPointerLeave`), and the PC-out call with rationale. Acceptance criteria expanded from 6 → 9: canonical-repro fixed (new AC #1, codifies Marco's repro), the original 5 ACs retained + sharpened with the `window`-level placement requirement for the new `pointercancel` / `pointerleave` listeners, and AC #7 captures the extraction (`src/lib/drag-controller.ts`, no Svelte/Pixi imports, no test infra in this task). Notes section pins every claim site and leak site by line number for the worker. **ADR candidate (scope `bc: canvas`) flagged for write-AFTER-the-diff, not before** — the module's public API stabilises during implementation; speculative ADRs rot fast.
**Split into:** none — task stayed whole.
**ADRs written:** none. The drag-controller ADR is the worker's output, scoped to canvas BC (local), written after the diff.

---

## 2026-05-19 13:30 -- Work session ended

**Type:** Work / Session end
**Completed:** 1 (first-try PASS: 1, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 1 (3315a10 canvas-013) + 1 fix-up

---

## 2026-05-19 13:30 -- Task verified and completed: canvas-013 - Crisp rendering at any zoom + Miro-style constant-size project titles

**Type:** Work / Task completion
**Task:** canvas-013-crisp-rendering-constant-size-project-titles - Crisp rendering at any zoom + Miro-style constant-size project titles
**Summary:** Three canvas-rendering invariants landed as an **extension to ADR-003** (scope `global`, unchanged): (1) **Crispness** — `app.init({ resolution: window.devicePixelRatio, autoDensity: true, … })` at the single Pixi boot site (Canvas.svelte L534-553), the load-bearing AC #1 fix; every Text and Graphics now rasterises at the framebuffer's real resolution rather than the previous default-1×-and-GPU-upscale path that was producing bilinear-smear blur halos at zoom-out. (2) **Constant-screen-size project titles (Miro-style)** — project frame titles render as HTML overlays in the existing ADR-003 overlay container at `var(--guppi-size-title)` constant CSS pixel size at every zoom; end-truncates via CSS `text-overflow: ellipsis` against the current header width (no per-frame JS measurement); z-index 5 (above canvas, below context-menu 10 / error-toast 11 / modal-backdrop 20); `pointer-events: none` so the overlay never swallows pan/drag/right-click/wheel-zoom; subscribes to camera + projects + theme runes the same way the Pixi scene does. The Pixi `Text` allocation that used to draw the title in `drawProjectFrame` is removed — one per-project per-render rasterisation shed. (3) **Screen-space stroke widths for the borders category** — frame border, BC bubble border, header divider, focus ring (project + BC), and all four intra-project edge variants (incl. arrowheads `headLen`/`headW` + ACL notch size) drop the `* z` multiplier; widths are constant CSS px (autoDensity handles device-px conversion). Caveats baked in: focus-ring **inset** stays world-space (proportional to the shape); arrowhead `pullBack` stays world-space because the bubble it pulls back from is drawn at `bcInsideWidth * z` in screen space. BC bubble title + task-count pill + frame header counts label stay in Pixi `Text` and scale with the camera — crispness comes from the DPR fix; BitmapText deferred (no backlog item filed — trigger is "BitmapText shows up as a top-3 hotspot in a future profile", not "we have time"). BC-text-floor invariant ("every BC always shows its name, if you squint") formally documented in the ADR. canvas/README.md gains two new vocabulary entries (Project-title overlay, Crispness invariant) and annotates Missing tile with the title-overlay 50%-opacity link. ADR-003 extension dated `## Extension 2026-05-19 — Crispness invariant + constant-size project titles` with full Context/Decision/Consequences/Reversibility/Implementation pointers; the second-source-of-truth risk (title overlay alongside the Pixi scene) is acknowledged + mitigated (both keyed by `entry.id`, both read `entry.snapshot.name`, no intermediate caching).
**Verification:** PASS (iteration 1)
**Commit:** 3315a10
**Files changed:** 4 (src/lib/Canvas.svelte; .agentheim/knowledge/decisions/ADR-003-canvas-rendering.md; .agentheim/contexts/canvas/README.md; .agentheim/contexts/canvas/done/canvas-013-…md NEW)
**Tests added:** 0 — `type: feature` UI task, legitimate TDD-skip per the doctrine's "UI tasks where the project has no UI test infrastructure" category (canvas-007 / canvas-008 precedent; `infrastructure-017-frontend-test-infrastructure` already in backlog tracks the gap). Test-surface artefacts: none — the new code lives entirely inside `Canvas.svelte` (Svelte component + Pixi imperative paths) and `.frame-title*` CSS. `pnpm check` clean (940 files, 0 errors, 0 warnings); `cargo test --lib` 122/122.
**ADRs written:** ADR-003 extended (no new ADR file — extension to existing global ADR; bidirectional backlink added: ADR-003 `related_tasks` gains canvas-013, canvas-013's `related_adrs` already included ADR-003).
**New backlog items:** none.
**Note:** Sharp first-try PASS on a six-AC, four-files-changed task with one load-bearing DPR fix + a structural new rendering path (HTML overlay) + a screen-space-stroke-widths sweep across 9 `.stroke({ width: ... })` call sites. The pre-loaded prior art (canvas-002, canvas-007, canvas-008) and the verbatim ADR-003 paste in the worker prompt paid off — the worker authored the ADR extension as an **extension** (not a new ADR) per the task's explicit instruction, with correct status/scope/date frontmatter, and the screen-space-stroke-widths grep was clean on the verifier's audit pass. The canvas-007 "tokens defined but not consumed" lever continued to apply — the new `.frame-title` CSS reads tokens (`--guppi-font-family`, `--guppi-size-title`, `--guppi-weight-bold`, `--guppi-frame-title-text`) instead of inline values, and the verifier confirmed all four tokens exist in `tokens.css` with both dark + light definitions where applicable. The canvas-014 perf-spike companion held — the report explicitly noted that HTML-overlay-for-titles is strengthened by the hotspot ranking, and canvas-013's strategy adds zero new per-frame cost (CSS truncation is browser-internal on style change; the overlay rides camera-event updates the same way the Pixi scene does, not per-tick from `renderScene`). **v1 design-refresh blocker triad now 2/3 done** — canvas-014 (perf spike, 2026-05-18) + canvas-013 (this commit) shipped; canvas-012 (drag-state-stickiness bug) remains in backlog. **Outstanding human gate:** Marco's in-person `pnpm tauri dev` sign-off of the new rendering invariants — the AC's eyeball + measurement steps (38%/100%/110% title-size cross-check, long-name ellipsis at zoom 38%, hairline crispness at zoom-out) need a side-by-side against the current production canvas before this counts as visually verified in addition to type-checked.

---

## 2026-05-19 13:14 -- Batch started: [canvas-013-crisp-rendering-constant-size-project-titles]

**Type:** Work / Batch start
**Tasks:** canvas-013-crisp-rendering-constant-size-project-titles - Crisp rendering at any zoom + Miro-style constant-size project titles
**Parallel:** no (1 worker — sole ready task; v1 blocker; both deps (design-system-001, canvas-014) in done/)

---

## 2026-05-18 -- Work session ended

**Type:** Work / Session end
**Completed:** 1 (first-try PASS: 1, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 1 (eea3d09 canvas-014)

---

## 2026-05-18 -- Task verified and completed: canvas-014 - Investigate canvas pan/zoom sluggishness — find the per-frame cost

**Type:** Work / Task completion
**Task:** canvas-014-investigate-pan-zoom-performance - Investigate canvas pan/zoom sluggishness — find the per-frame cost
**Summary:** Spike identified the dominant per-frame cost — `app.ticker.add(() => renderScene())` was rebuilding the entire scene graph (`world.removeChildren()` + ~200–500 fresh Pixi `Graphics` / `Text` / `Container` allocations for N≈7 frames × ~5 BCs) ~60 Hz, idle AND during gestures (the interactive paths already called `renderScene()` themselves, so the ticker was pure double-work). Trivial-win patch landed: ticker guarded by `cameraTarget` (steady-state per-frame cost drops to zero), explicit `window` `resize` listener replaces the incidental coverage the unconditional rebuild provided, explicit `renderScene()` added to the targeted-event default branch as a load-bearing correctness fix (without it `task_*`/`bc_appeared`/`bc_disappeared` events would silently stop visually updating the canvas). Research report at `.agentheim/knowledge/research/canvas-perf-2026-05-17/README.md` ranks the three remaining hotspots — (1) full scene-graph rebuild per `renderScene` call → canvas-015 (persistent scene graph + camera-as-stage-transform pan, 1–3 days), (2) screen-space overlays allocate per render → canvas-016 (cache voice indicator + future agent-awareness badges, ½ day), (3) no broad-phase hit rejection → canvas-017 (`world.eventMode = 'passive'` + `world.hitArea` of scene bounds, ½ day). canvas-018 captures a dev-only diagnostic seam follow-up (`window.__guppi` behind a DEV guard) surfaced by the reproducer-protocol friction. canvas-013 unblocked — its strategy hypothesis (HTML overlay for project titles + BitmapText deferred) is unchanged by these findings; the report is explicit that HTML-overlay-for-titles is **strengthened** by the hotspot ranking. canvas-013's AC #1 (DPR fix at `Canvas.svelte` L533) deliberately untouched per the spike scope boundary. **Operator confirmation outstanding** (recorded in the report): Marco needs to (a) paste actual `app.renderer.type` / `.resolution` / `devicePixelRatio` / `ticker.FPS` / GPU-string values into `console-snapshot.md` next to the report, and (b) hands-on smoke-test that a `task_*` event still visually updates a BC's counts post-fix.
**Verification:** PASS (iteration 1)
**Commit:** eea3d09
**Files changed:** 8 (src/lib/Canvas.svelte; .agentheim/contexts/canvas/README.md; .agentheim/knowledge/research/canvas-perf-2026-05-17/README.md NEW; canvas/backlog/canvas-{015,016,017,018}-*.md NEW; canvas/done/canvas-014-*.md NEW)
**Tests added:** 0 — type: spike; AC explicitly asks for `pnpm check` clean + `cargo test --lib` unchanged, both verified (940 files 0/0/0; 122/122).
**ADRs written:** none — ADR-003 (PixiJS v8 + HTML overlays) and ADR-015 (one-shot BC layout) both hold unchanged; the substantive renderer rework (which may warrant an ADR-003 extension) is properly deferred to canvas-015.
**New backlog items:** canvas-015 (persistent scene graph + stage-transform pan, depends on canvas-014, v1-perf), canvas-016 (cache screen-space overlays, depends on canvas-014), canvas-017 (broad-phase hit rejection, depends on canvas-014), canvas-018 (dev-only diagnostic seam, no deps).
**Unblocks:** canvas-013 (the perf report did not change the rendering-strategy hypothesis — promote-without-another-refinement-pass posture from 2026-05-17 REFINE holds; canvas-013 is now ready).
**Note:** Sharp first-try PASS on a hard-to-bound spike. The "trivial wins land in-spike, non-trivial fixes spin out" discipline worked exactly as designed — one load-bearing ticker fix (with the accompanying targeted-event renderScene compensation) shipped, the substantive renderer rework spun out as canvas-015 with proper depends_on, and the analytical cost model gives canvas-015's worker a starting point that does not require re-deriving the problem. The verifier's audit specifically flagged "are there OTHER dispatch paths that previously relied on the next-tick rebuild?" — the worker's source-only audit was deep enough to find the targeted-event branch as the one missing-dispatch site and patch it in the same commit, which is what kept this a first-try PASS instead of an iteration-2.

---

## 2026-05-18 -- Batch started: [canvas-014-investigate-pan-zoom-performance]

**Type:** Work / Batch start
**Tasks:** canvas-014-investigate-pan-zoom-performance - Investigate canvas pan/zoom sluggishness — find the per-frame cost
**Parallel:** no (1 worker — sole ready task; v1 blocker that gates canvas-013)

---

## 2026-05-18 -- Model / Promoted: canvas-014 - Investigate canvas pan/zoom sluggishness — find the per-frame cost

**Type:** Model / Promote
**BC:** canvas
**From → To:** backlog → todo
**Summary:** Sole promotion this turn. Marco's session-open question ("what's next?") landed on the v1-blocker triad surfaced 2026-05-17 (canvas-012 / 013 / 014). canvas-013 is hard-gated on canvas-014's perf report per the 2026-05-17 REFINE, so 014 is the unblocker. canvas-012 (drag stickiness) and canvas-003 (focus-zoom) held in backlog by choice — 014's findings may reshape both before they enter todo (012's drag-controller suspects overlap with overlay reflow / hit-test suspects from 014; 003's keyboard scheme still wants a refinement pass). Readiness check passed without REFINE — 6 concrete acceptance criteria, no unmet deps, measurement protocol + frame-time targets already spelled out in the body.

---

## 2026-05-17 -- Model / Refined: canvas-013 - Crisp rendering at any zoom + Miro-style constant-size project titles

**Type:** Model / Refine
**BC:** canvas
**Mode:** Interrogator (REFINE default)
**Status after:** backlog (gated on canvas-014; near-ready)
**Summary:** Three Marco locks made during this refinement: (1) project title overflow at constant size → **truncate with end-ellipsis at the current frame header width**; (2) ordering vs. canvas-014 → **canvas-014 runs first; canvas-013 gains a hard `depends_on: [canvas-014]`** so the rendering-strategy ADR is written with real per-frame cost numbers in hand; (3) BC text visibility floor → **none. Always render BC text, no matter how small.** Orchestrator pass routed to architect-style in-line analysis (one specialist, narrow scope) which verified every hypothesis against `Canvas.svelte` line-by-line and surfaced one load-bearing finding the original task missed: `app.init({ resizeTo, background, antialias })` at L533 sets neither `resolution` nor `autoDensity`, so PixiJS defaults to `resolution = 1` regardless of `devicePixelRatio` — every Text and Graphics is rasterized at 1× and the GPU upscales to the display's actual DPR. That DPR fix is now AC #1 (load-bearing). Other architect recommendations baked into the refined task body: ADR is an **extension to ADR-003**, not a new ADR (crispness + overlay-positioning refine the existing PixiJS-v8 + HTML-overlays decision rather than contradicting it); intra-project edges (arrowheads + ACL notches) belong to the "borders" category and get constant screen-space stroke widths together with the frame and bubble borders, not their own scheme; `BitmapText` migration is deferred as a canvas-014 follow-up if per-frame Pixi `Text` rebuild shows up as a top-3 hotspot, NOT a 013 commitment; project-title HTML overlay piggybacks on the existing ADR-003 overlay container (where modals / toasts / menus already live) in its own z-band below interactive overlays. Acceptance criteria expanded from 9 → 10 entries, all three Marco locks baked in. Promotion stance: **promote-without-another-refinement-pass after canvas-014 ships** — the strategy hypothesis is now sharp enough for the worker to author the ADR extension and the implementation directly; only a 014 finding that materially changes the strategy (e.g. `BitmapText` becoming a 013 AC instead of a 014 follow-up) would trigger another REFINE pass.
**Split into:** none — task stayed whole.
**ADRs written:** none. The crispness/constant-size strategy ADR is the worker's output (extension to ADR-003).

---

## 2026-05-17 -- Model / Captured: canvas-012, canvas-013, canvas-014 — front-end refinements from hands-on verification

**Type:** Model / Capture
**BC:** canvas
**Mode:** Facilitator
**Filed to:** backlog (×3)
**Summary:** Marco ran the freshly-shipped v1 design-refresh trio (003 → 004 → 008) hands-on and surfaced three front-end issues that need to land before the existing backlog continues. canvas-012 (bug): drag state sometimes sticks after `pointerup`; subsequent bare mouse-moves continue panning the canvas — audit the shared window-level drag controller's pointerup/pointercancel/pointerleave paths across all three drag kinds (frame / BC / empty-canvas pan). canvas-013 (feature): two rendering invariants — (1) crispness universal (borders + all text at display resolution at every zoom, no bilinear blur, no disappearing hairlines), (2) project frame titles at constant on-screen pixel size regardless of camera zoom (Miro-style); BC titles + task counts deliberately scale with zoom but stay crisp; strategy mix proposed in notes (HTML overlay for project titles, Pixi re-rasterization for BC text, inverse-zoom-scaled line widths for borders); decision → ADR. canvas-014 (spike): pan / zoom feel sluggish; cause unknown; output is a research report at `.agentheim/knowledge/research/canvas-perf-2026-05-17/` with active renderer + per-frame cost + top-3 hotspots + proposed fixes, trivial wins land in-spike, non-trivial fixes become follow-up tasks. All three flagged as v1 blockers per Marco's "before we continue with all the other tickets" prioritization. Prior art linked: canvas-002 (shared drag controller origin), canvas-007 (frame/BC drag-claim split, project-as-frame rendering), canvas-008 (current token consumption). Related ADRs: ADR-003 (PixiJS v8 + HTML overlays — already permits the overlay strategy canvas-013 needs); ADR-015 (deterministic BC layout, relevant to canvas-014's perf baseline).

---

## 2026-05-16 -- Work session ended

**Type:** Work / Session end
**Completed:** 3 (first-try PASS: 3, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 3 (ab441d0 design-system-003, 6ce724e design-system-004, bbe3704 canvas-008)
**ADRs written:** 0 new; ADR-009 amended (new `DomainEvent::PreferenceChanged` variant + dated 2026-05-16 reconciliation note for the generic key/value preferences design).
**New backlog items:** design-system-005-document-modal-backdrop-token — surfaced by canvas-008's light-mode audit catch (modal scrim token added but not yet documented in STYLEGUIDE.md).
**Note:** The v1 design-refresh trio shipped sequentially in a single clean session — design-system-003 (brand orange + brand blue + revised status palette + hairlineStrong + four glow RGBAs + dimensional bumps 188×60 / 36px), design-system-004 (dual palettes + SQLite-persisted theme via new v5 `preferences` table + `PreferenceChanged` event + top-right toggle UI), canvas-008 (verification + audit pass against the revised tokens in both themes). **Three first-try PASS verdicts in a row** — a useful contrast against the iteration-1 FAIL we saw on canvas-007 last session. The "tokens defined but not consumed" lever still proved its worth here: canvas-008's verifier specifically grepped for inline hex + dimensional literals in the four rendering modules, and the audit caught one structural light-mode gap (`Modal.svelte`'s hardcoded backdrop) the type-checker is blind to. **Pattern reinforced:** when the §3.x acceptance criteria explicitly enumerate token names, the verifier turns that enumeration into a checklist. **todo/, doing/, backlog/ status post-session:** todo/ empty in every BC; doing/ empty in every BC; design-system backlog +1 (design-system-005, ready for the next refine/work pass). Marco's in-person `pnpm tauri dev` sign-off remains the outstanding human gate — actionable now against HEAD with the full revised palette + working theme toggle visible end-to-end. **Concept candidates:** none.

---

## 2026-05-16 -- Task verified and completed: canvas-008 - Apply revised tokens — visual re-validation against the 2026-05-16 design

**Type:** Work / Task completion
**Task:** canvas-008-apply-revised-tokens - Apply revised tokens — visual re-validation against the 2026-05-16 design
**Summary:** Token-application audit pass against the 2026-05-16 design. `Canvas.svelte` + `bc-layout.ts` + `tile-layout.ts` + `snapshot-patch.ts` all consume tokens cleanly (zero inline hex, zero inline dimensional literals); the new 188×60 BC + 36px header values flow through correctly. Theme flip verified end-to-end — `initTheme()` runs pre-Pixi-init so first paint is in the persisted palette; `onThemeChange()` calls `renderScene()` + resets `renderer.background.color`. Audit catch: `Modal.svelte`'s inline `rgb(22 22 28 / 70%)` backdrop was a stale dark-theme assumption — fixed with new theme-invariant `--guppi-modal-backdrop` / `modalBackdrop` token (`rgba(10, 10, 14, 0.62)` per design §5).
**Verification:** PASS (iteration 1)
**Commit:** bbe3704
**Files changed:** 3 (Modal.svelte + tokens.ts + tokens.css)
**Tests added:** 0 — audit-pass task per AC #7 ("no new tests mandatory here unless the audit surfaces a logic regression"); the one catch was a stale CSS literal, not a logic regression. `pnpm check` 0/0/0 (940); `cargo test --lib` 122/122.
**ADRs written:** none — no structural decision required.
**New backlog items:** design-system-005-document-modal-backdrop-token (design-system BC; the new modal-backdrop token exists in both contract files but `STYLEGUIDE.md` doesn't yet document it; documentation-only follow-up).
**Closes:** the v1 design-refresh trio (design-system-003 → design-system-004 → canvas-008).
**Note:** canvas-007's "tokens defined but not consumed" lever continued to pay off — the verifier explicitly grepped `Canvas.svelte` / `bc-layout.ts` / `tile-layout.ts` / `snapshot-patch.ts` for inline hex + raw dimensional literals (188 / 160 / 56 / 24 / 36) and zero matches confirmed the consumer side flows through tokens cleanly. The Modal.svelte catch was the audit's structural value-add: a light-mode regression the type-checker could not see.

---

## 2026-05-16 -- Batch started: [canvas-008-apply-revised-tokens]

**Type:** Work / Batch start
**Tasks:** canvas-008-apply-revised-tokens - Apply revised tokens — visual re-validation against the 2026-05-16 design
**Parallel:** no (1 worker — last task in the v1 design-refresh trio)

---

## 2026-05-16 -- Task verified and completed: design-system-004 - Optional light theme — toggle persisted in SQLite

**Type:** Work / Task completion
**Task:** design-system-004-light-theme - Optional light theme — toggle persisted in SQLite
**Summary:** Optional light theme landed end-to-end — dual palettes (`colorDark` + `colorLight`) in `tokens.ts`; `[data-theme="light"]` block in `tokens.css`; theme persisted via new v5 SQLite `preferences (key, value)` table; new `get_preference` / `set_preference` IPC commands + new `DomainEvent::PreferenceChanged` variant. Top-right toggle in `Canvas.svelte` flips both HTML overlay (`data-theme` on `<html>`) and PixiJS scene (`renderScene()` + `app.renderer.background.color` re-set).
**Verification:** PASS (iteration 1)
**Commit:** 6ce724e
**Files changed:** 11 (db.rs + events.rs + lib.rs + tokens.ts + tokens.css + theme.svelte.ts new + ipc.ts + types.ts + Canvas.svelte + ADR-009 amended + BC README)
**Tests added:** 5 cargo tests (`fresh_db_is_at_schema_version_five`, `fresh_db_seeds_default_theme_preference_as_dark`, `preference_round_trips`, `v4_db_migrates_to_v5_without_data_loss`, `preference_changed_event_reaches_a_subscriber`); existing `fresh_db_is_at_schema_version_four` relaxed to `>= 4` per the v2/v3 contract-test idiom. `pnpm check` 0/0/0 (940); `cargo test --lib` 122/122.
**ADRs written:** ADR-009-event-bus.md (amended) — new `PreferenceChanged { key, value }` variant added to the enum sketch; dated 2026-05-16 reconciliation note explaining the generic key/value design and `theme` as the first inhabitant; `related_tasks` extended to include `design-system-004-light-theme`.
**Unblocks:** `canvas-008-apply-revised-tokens` — both upstream deps (003 + 004) now done; canvas-008 is now ready. End of the v1-relevant design refresh trio.
**Note:** Larger task than typical — spans Rust (schema migration v4→v5, IPC, event taxonomy), TypeScript (new `theme.svelte.ts` rune, `ipc.ts` wrappers, `types.ts` event union), and Svelte/PixiJS (toggle UI in `Canvas.svelte`, scene re-draw on theme change). The `initTheme()` load-before-`Application.init()` ordering means first paint already lands in the persisted palette — no flash-of-dark on restart. The new `theme.svelte.ts` rune is a clean piece of design-system BC scaffolding; canvas-008 will consume it without further plumbing changes.

---

## 2026-05-16 -- Batch started: [design-system-004-light-theme]

**Type:** Work / Batch start
**Tasks:** design-system-004-light-theme - Optional light theme — toggle persisted in SQLite
**Parallel:** no (1 worker — sole ready task; canvas-008 blocked on this)

---

## 2026-05-16 -- Task verified and completed: design-system-003 - Brand colors + status palette + v1 dimensional refinement

**Type:** Work / Task completion
**Task:** design-system-003-brand-colors-and-status-revision - Brand colors + status palette + v1 dimensional refinement
**Summary:** Brand palette (orange `#ff8b00` warm anchor + blue `#25abfe` cool secondary), revised status palette (grey/blue/red/orange + ○▶◆✕ glyphs), new `hairlineStrong` `#3a3b46` token + four status-glow RGBA strings, and v1 dimensional refinements (`frameHeaderHeight` 24→36, `bcInsideWidth×Height` 160×56→188×60) landed across `tokens.ts`, `tokens.css`, and `STYLEGUIDE.md`. `Canvas.svelte` untouched — visual re-validation is canvas-008's job.
**Verification:** PASS (iteration 1)
**Commit:** ab441d0
**Files changed:** 4 (tokens.ts + tokens.css + STYLEGUIDE.md + BC README)
**Tests added:** 0 — pure token + doc updates; `pnpm check` 0/0/0 (939 files) is the gate. canvas-008's visual re-validation will be the integration gate.
**ADRs written:** none — Q9 STYLEGUIDE.md §5 entry records the brand-hue decision per the design-system-002 Q4–Q8 pattern (acceptance-criterion #7 explicitly says "no ADR if no conflict with a prior styleguide ADR").
**Unblocks:** `design-system-004-light-theme` (was blocked on 003; depends_on satisfied → moves to ready next batch). `canvas-008-apply-revised-tokens` remains blocked pending 004.
**Note:** Verifier explicitly cross-checked every row of the task's `## What` token table against `tokens.ts` + `tokens.css` mirror, plus every STYLEGUIDE.md example hex against `tokens.ts`. The post-canvas-007 verification pattern ("type-checker can't see token consistency") proved its worth in the reverse direction — instead of a consumer that didn't read tokens, this was a producer that had to fan every value out across three files; no orphans found.

---

## 2026-05-16 -- Batch started: [design-system-003-brand-colors-and-status-revision]

**Type:** Work / Batch start
**Tasks:** design-system-003-brand-colors-and-status-revision - Brand colors + status palette + v1 dimensional refinement
**Parallel:** no (1 worker — sole ready task; 004 and canvas-008 blocked on this)

---

## 2026-05-16 -- Model / Captured + refined: 2026-05-16 design ingestion

**Type:** Model / Capture + Refine
**BC:** design-system + canvas + voice + agent-awareness
**Mode:** Facilitator → Suggestor
**Summary:** Marco generated a full visual design at claude.ai/design and pointed the model skill at the Anthropic-API export URL (`api.anthropic.com/v1/design/h/EEOPCxE8gLGPIpNeTv4eHA?open_file=GUPPI.html`). WebFetch returned a 51.7KB gzip tarball (the "handoff bundle" — README, chat transcript, GUPPI.html, 8 JSX modules, guppi-tokens.css). Extracted, read the bundle's `README.md` (its instructions: read the chat first, then GUPPI.html, then imports), then read `chats/chat1.md` + `GUPPI.html` + `guppi-tokens.css`. The chat captured two business decisions that override the orbit-baseline placeholder accents: (1) brand colours `#ff8b00` (warm, frame border + focus ring + `missing` status) and `#25abfe` (cool, BC border + `running` status); (2) revised status palette grey ○ / blue ▶ / red ◆ / orange ✕ (was grey-blue / blue / amber / magenta-pink). The design also pins v1 dimensional refinements (BC bubble 188×60 was 160×56; header bar 36px was 24; edges on a distinct `hairlineStrong` `#3a3b46` rather than the existing `fgMuted` `#6e6e80`) and ships a full optional light theme (`[data-theme="light"]` block in the design's tokens.css). The design also pins six v2+ component specs (project detail panel §5, terminal panel §6, command palette ⌘K §8, voice-state indicator §4, blocked-question callout §3, zoom transition §7).

**Filed:** design bundle saved to `.agentheim/contexts/design-system/references/claude-design-2026-05-16/` (12 files: README, chat, GUPPI.html, 8 JSX modules, guppi-tokens.css). Source of truth for every captured task.

**New tasks (todo, v1-relevant, ready-to-work):**
- `design-system-003-brand-colors-and-status-revision` — `tokens.ts` + `tokens.css` + STYLEGUIDE.md updates; new `hairlineStrong` token; ~12 token value changes; 3 dimensional changes.
- `design-system-004-light-theme` — light palette + theme-switchable token resolution + SQLite preference table (schema v4→v5) + `get_preference`/`set_preference` IPC + new `PreferenceChanged` event (amends ADR-009) + toggle UI top-right. Depends on `003`.
- `canvas-008-apply-revised-tokens` — visual re-validation of `Canvas.svelte` against revised tokens in both themes; token-application audit (no inline hex); cross-check against design's `GUPPI.html` artboards. Depends on `003` + `004`. Supersedes `canvas-004`.

**New tasks (backlog, v2+ specs — design-pinned, do NOT promote without upstream ready):**
- `canvas-009-project-detail-panel-spec` — 520px right slide-in markdown reader; v1.5+.
- `canvas-010-terminal-panel-spec` — orchestrator + sub-agents + blocked-card; v2+; hard upstream is a structured stream taxonomy from `claude-runner`.
- `canvas-011-command-palette-spec` — ⌘K, voice-vocabulary parity; v2+; hard upstream is the intent-to-executor router.
- `voice-001-voice-state-indicator-spec` — screen-space pill, 4 states; v2+; hard upstream is Whisperheim/Utterheim bridge.
- `agent-awareness-001-blocked-question-callout-spec` — 240px tethered callout, pulse-synced; v2+; hard upstream is structured blocked-state.

**Refined (light pass):**
- `canvas-003-focus-zoom` — appended "Design pins (2026-05-16)" section: overview 38% / focused 110% / 320ms / `cubic-bezier(.16,.84,.36,1)` / `transform-origin: center of target` / focus-ring 1.5px brand orange. Resolves two open questions (camera-state-on-focus = transient; ESC = inverse transition); two stay open (keyboard scheme, BC focus targets). Structurally unblocked by `canvas-007` (commit `e2296c2`) — promotable once the two remaining questions answered in a Suggestor pass.

**Closed (subsumed):**
- `canvas-004-styleguide-visuals` — moved to `done/` with a `subsumed_by: canvas-008` frontmatter marker and a closure note in the body. No work done, no commit; the residual scope is now `canvas-008`'s. Per Marco's 2026-05-16 sign-off.

**Sign-offs given by Marco (2026-05-16) before capture:**
- Light theme persistence: **SQLite** per ADR-004 (not localStorage).
- `canvas-004`: **close as subsumed** by `canvas-008` (not refine, not keep).
- `design-system-003` scope: **bundle** colors + status + dimensions in one task (not split).

**No orchestrator round, no ADRs written** (the §5 entry in STYLEGUIDE.md is the resolution path for the brand-color decision per the `design-system-002` pattern; `design-system-004` will amend ADR-009 with the new event variant during its worker run, not at capture time).

**INDEXes updated:** canvas, design-system, voice, agent-awareness. Backlog counts after this pass: canvas 4 (canvas-003 unblocked + 009/010/011 v2+); design-system 0 in backlog, 2 in todo; voice 1; agent-awareness 1; infrastructure 1 (infrastructure-017, unchanged). Todo counts: design-system 2, canvas 1.

**Next:** Marco's call. The v1-relevant trio (003 → 004 → 008) is ready for `/agentheim:work`. Suggested batch order: 003 first (no dependencies), 004 second (depends on 003), 008 third (depends on both). The v2+ specs stay frozen in backlog until their respective upstreams (claude-runner stream, Whisperheim bridge, intent router) land.

---

## 2026-05-16 -- Work session ended

**Type:** Work / Session end
**Completed:** 1 (first-try PASS: 0, re-dispatched: 1, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 1 (e2296c2 canvas-007-project-as-frame)
**ADRs written:** ADR-015 (scope: bc, bc: canvas)
**New backlog items:** infrastructure-017-frontend-test-infrastructure
**Note:** Solo task this session — `canvas-007-project-as-frame`, the project-as-frame integration redesign atop yesterday's design-system-002 + project-registry-004 wins. **One surprising thing worth flagging:** the iteration-1 verifier caught a tight §3.7 contract gap (`bcInsidePill*` tokens defined in `tokens.ts` but not consumed in `Canvas.svelte`'s `makeBcBubble`; counts were rendered as a plain `Text` glyph rather than a rounded-rect pill; secondary `weightBold` vs. specified `weightMedium`). `pnpm check` cannot see styleguide compliance — only type compliance — so this would have shipped wrong without the verification gate. Iteration 2 closed it with a +22-line scoped delta inside `makeBcBubble`; no regression elsewhere. Pattern worth remembering: the §3.6/§3.7/§3.8 token enumeration in the acceptance criteria text is the verifier's lever for catching "styleguide tokens defined but not consumed" gaps that the type-checker is blind to. **todo/, doing/, backlog/ status:** todo/ empty in every BC; canvas-003-focus-zoom + canvas-004-styleguide-visuals still in canvas backlog (both candidates for next REFINE-against-frames pass); infrastructure-017 newly captured in infrastructure backlog. **canvas-003 is now structurally unblocked** — its open questions (keyboard scheme, BC-as-focus-target, ESC behaviour) can be re-asked against the frames-and-bubbles shape this session shipped. Marco's deferred in-person `STYLEGUIDE.md` §5.Q4–Q8 sign-off is also actionable now via `pnpm tauri dev` against commit e2296c2.

---

## 2026-05-16 -- Task verified and completed: canvas-007-project-as-frame - Project-as-frame — BCs inside, edges between BCs by relationship type

**Type:** Work / Task completion
**Task:** canvas-007-project-as-frame - Project-as-frame — BCs inside, edges between BCs by relationship type
**Summary:** Project tiles now render as bounded frames whose header bars carry project name + task counts and whose interior holds BCs as denser bubbles connected by intra-project relationship edges. Four §3.8 edge variants on the locked single-neutral `fgMuted` palette (geometry distinguishes customer-supplier / shared-kernel / ACL / conformist). New pure deterministic spring-electrical BC layout module (`src/lib/bc-layout.ts` — mulberry32 seeded by `project_id`, one-shot on input change, no rAF loop) places bubbles inside each frame; sticky per-BC drag positions persisted via `project-registry-004`'s `saveBcPosition` / `loadBcPositions` IPC; re-layout fires only on `bc_appeared` / `bc_disappeared` / `bc_relationships_changed`. `snapshot-patch.ts` gains a `bc_relationships_changed` case that signals `refreshOne(project_id)` in `Canvas.svelte` followed by a one-shot recompute — `canvas-001` targeted-update invariant preserved (no full `list_projects` re-fetch). `BcSnapshot` TS alias retired in favour of `BoundedContext` across all consumers. Frame body is pass-through so the empty-canvas right-click menu still opens inside an empty frame region; frame header bar owns the drag handle + tile-right-click menu wiring (canvas-005a). canvas-005b cascade, canvas-006 `liveAddChain`, missing-tile visual all rewired against the new shape. Voice BC renders edge-less inside its frame as expected (Whisperheim/Utterheim ACLs cross-project, dropped by the registry parser at v1).
**Verification:** PASS (iteration 2 — iteration 1 caught a §3.7 pill-token gap; the four `bcInsidePill*` tokens were not consumed in `makeBcBubble` and BC name used `weightBold` instead of `weightMedium`; iteration 2 closed it with a `Graphics.roundRect.fill(bcInsidePillFill)` element sized to `bcInsidePillHeight × max(bcInsidePillMinWidth, textWidth+pad)` with `bcInsidePillRadius` corner radius, right-aligned in the bubble + counts `Text` centred over it.)
**Commit:** e2296c2
**Files changed:** 8 (Canvas.svelte + snapshot-patch.ts + types.ts + bc-layout.ts new + canvas README + ADR-015 new + infrastructure-017 new + moved task file)
**Tests added:** 0 — project still has no frontend test runner; `bc-layout.ts` built pure (no Svelte/Pixi imports) against that future surface. `pnpm check` 0/0/0 (939 files); `pnpm build` passes; `cargo test --lib` 117/117 (no Rust changes — task was frontend-only as scoped).
**ADRs written:** ADR-015 (scope: bc, bc: canvas) — BC layout inside a project frame: deterministic one-shot spring-electrical with sticky pins. Pure module `src/lib/bc-layout.ts` alongside `tile-layout.ts`; mulberry32 RNG seeded by `project_id` (deterministic across restarts); pins applied before the simulation runs so dragged BCs stay put.
**New backlog items:** infrastructure-017-frontend-test-infrastructure — add vitest + first test set for the three pure modules (`tile-layout.ts`, `snapshot-patch.ts`, `bc-layout.ts`). Surfaced from canvas-007; canvas-001 / canvas-002 / canvas-006 also called this out at completion time.
**Unblocks:** `canvas-003-focus-zoom` — its open questions (keyboard scheme, BC-as-focus-target, ESC behaviour) can now be re-asked against frames-and-bubbles. Marco's deferred in-person `STYLEGUIDE.md` §5.Q4–Q8 sign-off is also actionable now via `pnpm tauri dev` against this commit.

---

## 2026-05-16 -- Verification failed: canvas-007-project-as-frame

**Type:** Work / Verification failure
**Task:** canvas-007-project-as-frame - Project-as-frame — BCs inside, edges between BCs by relationship type
**Iteration:** 1 of 3
**Reasons:** §3.7 `bcInsidePill*` tokens not consumed (`bcInsidePillFill`, `bcInsidePillHeight`, `bcInsidePillMinWidth`, `bcInsidePillRadius`); counts rendered as plain `Text` glyph at bottom-left of bubble rather than as the right-aligned rounded-rect pill the styleguide §3.7 mandates. Secondary: BC name uses `weightBold` where §3.7 specifies `weightMedium`.
**Iteration hint:** likely-fixable
**Next:** re-dispatched worker

---

## 2026-05-16 11:01 -- Batch started: [canvas-007-project-as-frame]

**Type:** Work / Batch start
**Tasks:** canvas-007-project-as-frame - Project-as-frame — BCs inside, edges between BCs by relationship type
**Parallel:** no (1 worker — sole ready task)

---

## 2026-05-16 -- Model / Refined + Promoted: canvas-007-project-as-frame

**Type:** Model / Refine + Promote
**BC:** canvas
**Status after:** todo (was backlog)
**Mode:** Interrogator (defaulted; quick verification pass, no questions needed — every wave-hand had a concrete contract waiting)
**Summary:** REFINE pass confirmed every "what design-system-002 ships" and "how project-registry-004 shapes the snapshot" wave-hand has collapsed to a named contract from the upstream done notes, so the task was promoted to `todo/` in the same pass. **Wave-hands collapsed verbatim:** (1) Frame tokens nailed to §3.6 names (`frameFill`, `frameBorder`, `frameHeaderFill`, `frameHeaderDivider`, `radiusFrame`, `borderWidthFrame`, `frameHeaderHeight`, `framePadding`, `frameMin*`). (2) Interior bubble tokens nailed to §3.7 names (`radiusBcInside` 8, `bcInside{Width,Height}`, `bcInsidePill*`). (3) Four edge variants nailed to §3.8 on the locked single-neutral `fgMuted` palette: customer-supplier line + arrowhead (`arrowheadLength` 10, `arrowheadWidth` 8), shared-kernel/partnership line no decoration, anti-corruption-layer line + filled-triangle notch at midpoint pointing upstream (`aclNotchSize` 10), conformist `edgeWeightConformist` (lighter than `edgeWeight`). (4) Data shape nailed: `BoundedContext.relationships: Relationship[]` (Rust struct `BoundedContext` from project-registry-004 + TS still has `BcSnapshot` as a back-compat alias that canvas-007 explicitly retires per project-registry-004's done note); cross-project `to` references already dropped by the registry parser (canvas does not double-handle). (5) Event nailed: `BcRelationshipsChanged { project_id, bc_name }` from ADR-009's enum; `snapshot-patch.ts` extends to handle the variant (and the existing `bcNode` lazy-create already seeds `relationships: []` — no work needed there). (6) IPC nailed: `saveBcPosition` / `loadBcPosition` / `loadBcPositions` wrappers exist; canvas wires the calls. (7) Voice BC's no-edges-in-v1 state explicitly added as an expected acceptance criterion (its Whisperheim/Utterheim ACLs are cross-project, locked out at v1; the registry-004 bootstrap reflects this — voice will render edge-less inside its frame, not a bug). (8) Marco's deferred in-person `STYLEGUIDE.md` §5.Q4–Q8 sign-off becomes actionable *after* canvas-007 ships — canvas-007 is the `pnpm tauri dev` visual confirmation point. **Frontmatter additions:** `related_adrs` gained ADR-014 (the frontmatter-as-relationship-data-source decision canvas-007 consumes). **No orchestrator round** — every decision was already locked from the 2026-05-15 13:45 Suggestor refinement + the two prereq done notes (commits 38f48ab + b3727f5); no architect / strategic-modeler / tactical-modeler delegation needed. **No ADRs written.** **No new dependencies.** No new task captured; canvas-003 stays in backlog with its `depends_on canvas-007` chain intact. Backlog after this pass: canvas-003, canvas-004 (still backlog; canvas-004 is the next candidate for a REFINE-against-frames pass once canvas-007 lands or sooner if Marco wants).

---

## 2026-05-16 -- Work session ended

**Type:** Work / Session end
**Completed:** 2 (first-try PASS: 2, re-dispatched: 0, skipped: 0)
**Bounced:** 0
**Failed:** 0
**Escalated after verification:** 0
**Commits:** 2 (38f48ab design-system-002, b3727f5 project-registry-004)
**Note:** Cleared both upstream tasks gating `canvas-007-project-as-frame` — the project-as-frame redesign is now ready to be promoted from backlog to todo (a `model` REFINE pass will likely confirm this trivially, since both prerequisites landed verbatim against the locked decisions from the 2026-05-15 13:45 Suggestor refinement). The two tasks were serialised (not parallelised) because both touched the design-system BC README — design-system-002 updated its own README with the new ubiquitous-language entries; project-registry-004 wrote the bootstrap `relationships:` frontmatter across all seven BC READMEs (a scope explicitly granted to the worker via the orchestrator's "exception" override of the standard worker rule). The serial-not-parallel cost was small (no real parallel speedup since project-registry-004 is the substantial task — Rust types, schema migration, IPC, watcher event, frontmatter bootstrap, 27 new tests). Backend tests 89 → 117. `pnpm check` stable at 938/0/0 throughout. New ADR: **ADR-014 (scope: bc, bc: project-registry)** — per-BC README frontmatter chosen as the relationship-data home (over parsing context-map.md prose, over a separate relationships.yaml). New dependency: `serde_yaml` (Cargo). No bounces, no escalations, no concept candidates. **todo/, doing/, backlog/ now empty in every BC.** Orchestrator's INDEX.md / protocol.md bookkeeping + the commit-SHA frontmatter back-fills are uncommitted on the working tree — a separate `chore(work)` commit folds them in (matches the prior session pattern). **One surprising thing worth flagging:** the worker handled the cross-BC frontmatter bootstrap cleanly under the orchestrator's "exception" override — all 6 non-project-registry BC READMEs received only YAML frontmatter prepended above original prose, with zero prose edits (verifier confirmed). The exception mechanism worked; this pattern is reusable for future "one task touches many BCs at the structural layer" cases (e.g., a future task that adds a shared field to every BC's frontmatter would use the same shape).

---

## 2026-05-16 -- Task verified and completed: project-registry-004-bc-relationships-and-positions - BC relationships + per-BC position storage

**Type:** Work / Task completion
**Task:** project-registry-004-bc-relationships-and-positions - BC relationships + per-BC position storage
**Summary:** Landed the two structural prerequisites for `canvas-007-project-as-frame`. (1) BC↔BC relationships now flow from per-BC `README.md` YAML frontmatter (`relationships:` block carrying `to` / `type` / `direction`) into `BoundedContext.relationships: Vec<Relationship>` on `ProjectSnapshot` via `serde_yaml`; `BcSnapshot` renamed to `BoundedContext` across `project.rs`, `lib.rs`, the TS mirror, and all call sites. Malformed frontmatter degrades gracefully (single warning, empty relationships, BC still enumerates); cross-project `to` references dropped at v1 with a warning. (2) New fine-grained `BcRelationshipsChanged { project_id, bc_name }` domain event fires from `AgentheimWatcher` only when the parsed relationship set actually changes — deep-equal change-detection cache keeps prose-only README edits from triggering spurious relayouts (canvas-007 consumes this directly). (3) Schema v3→v4 adds `bc_positions(project_id, bc_name, x, y)` with `ON DELETE CASCADE` on `projects(id)`; mirrors `tile_positions`' soft-delete preservation semantics. (4) Three new IPC commands — `save_bc_position` / `load_bc_position` / `load_bc_positions` — round-trip via `INSERT … ON CONFLICT(project_id, bc_name) DO UPDATE` atomic upsert (race-free under concurrent saves). (5) GUPPI's seven BC READMEs carry hand-curated `relationships:` frontmatter translated from `.agentheim/context-map.md`'s Relationships section: canvas → project-registry + agent-awareness + infrastructure; agent-awareness → claude-runner (CS-upstream) + project-registry (conformist) + infrastructure; voice → infrastructure only (Whisperheim/Utterheim ACL is cross-project, omitted at v1); claude-runner / design-system / project-registry → infrastructure (shared-kernel); infrastructure stays edge-free.
**Verification:** PASS (iteration 1)
**Commit:** b3727f5
**Files changed:** 19 (Cargo.toml + Cargo.lock + project.rs + db.rs + events.rs + watcher.rs + lib.rs + types.ts + ipc.ts + snapshot-patch.ts + 7 BC READMEs + ADR-014 + moved task file)
**Tests added:** 27 — `cargo test --lib` 117/117 (was 89). Coverage: parser (`relationships:` parses; missing frontmatter / malformed YAML degrade gracefully); change-detection (event fires on real change, NOT on prose-only edit — both asserted in one e2e); migration (fresh DB at v4; v3→v4 upgrade preserves rows); IPC round-trip + concurrency (atomic upsert under interleaved writes); CASCADE on hard-delete + preservation through soft-delete. `pnpm check` 0/0/0 (938 files, unchanged from baseline).
**ADRs written:** ADR-014 (scope: bc, bc: project-registry) — per-BC README frontmatter chosen over parsing `context-map.md` prose (brittle) and separate `relationships.yaml` (truth too far from prose); `context-map.md` stays as the human-readable narrative.
**Carry-forward note (Agentheim follow-up tracked elsewhere):** brainstorm/model eventually need to write/maintain `relationships:` frontmatter automatically — captured at `agentheim/.agentheim/backlog/brainstorm-model-maintain-bc-relationships-frontmatter.md` from the 2026-05-15 refinement.

---

## 2026-05-16 -- Batch started: [project-registry-004-bc-relationships-and-positions]

**Type:** Work / Batch start
**Tasks:** project-registry-004-bc-relationships-and-positions - BC relationships + per-BC position storage
**Parallel:** no (1 worker — sole ready task after design-system-002 completed; touches all 7 BC READMEs for the relationship-frontmatter bootstrap, so a parallel partner would conflict regardless.)

---

## 2026-05-16 -- Task verified and completed: design-system-002-project-frame-vocabulary - Project-as-frame visual vocabulary

**Type:** Work / Task completion
**Task:** design-system-002-project-frame-vocabulary - Project-as-frame visual vocabulary
**Summary:** Landed the project-as-frame visual vocabulary in `STYLEGUIDE.md` — §3.6 Project frame (frameFill/frameBorder/frameHeaderFill/frameHeaderDivider tokens, header-bar drag/right-click target, empty-frame placeholder, PixiJS pseudocode); §3.7 BC bubble (inside frame) — denser variant with title + counts pill in one row at default zoom, status badge slot retained; §3.8 Intra-project edges — four variants (customer-supplier directional w/ arrowhead, mutual/shared-kernel/partnership non-directional, ACL directional w/ midpoint triangle notch, conformist lighter weight); single-neutral `fgMuted` palette, geometry distinguishes types. 17 new colour tokens + 17 new shape tokens mirrored name-for-name across `src/lib/design/tokens.ts` (canonical, PixiJS-ready) and `src/lib/design/tokens.css` (CSS custom properties). `references/project-frame-sketch.md` added as an ASCII layout reference for Marco's sign-off conversation. Q4–Q8 defaults documented with override paths per the styleguide-001 pattern; Marco's in-person sign-off remains a deferred human gate.
**Verification:** PASS (iteration 1)
**Commit:** 38f48ab
**Files changed:** 5 (STYLEGUIDE.md, README.md, references/project-frame-sketch.md new, tokens.ts, tokens.css) + moved task file
**Tests added:** 0 — pure styleguide + tokens + documentation task; `pnpm check` 0/0/0 (938 files) confirms tokens compile cleanly. No `Canvas.svelte` change (canvas-007 is the consumer).
**ADRs written:** none — every decision sits inside ADR-002 (Svelte 5 + SvelteKit) + ADR-003 (PixiJS v8 + HTML overlay split, which the new pseudocode honours); aesthetic defaults documented as override paths in `STYLEGUIDE.md` §5 rather than ADRs (the §5 table answers "why this, not the obvious alternative?" directly).

---

## 2026-05-16 -- Batch started: [design-system-002-project-frame-vocabulary]

**Type:** Work / Batch start
**Tasks:** design-system-002-project-frame-vocabulary - Project-as-frame visual vocabulary
**Parallel:** no (1 worker — design-system-002 + project-registry-004 both touch the design-system BC README; serialised. design-system-002 first, lower-numbered.)

---

## 2026-05-15 13:45 -- Model / Refined: canvas-007-project-as-frame — Suggestor pass landed full decomposition

**Type:** Model / Refine
**BC:** canvas (+ spawn into project-registry, design-system, + Agentheim repo)
**Status after:** canvas-007 backlog (the integration); project-registry-004 + design-system-002 promoted to todo
**Mode:** Suggestor (Marco switched mid-refinement)
**Summary:** Took the canvas-007 redesign from "lots of open questions" to "fully decomposed, two upstream tasks promoted to todo" in one Suggestor pass. **Locked decisions:** (1) **Data source = per-BC README YAML frontmatter** — each `contexts/<bc>/README.md` grows a `relationships:` block with `to` (sibling BC name), `type` (customer-supplier / shared-kernel / partnership / anti-corruption-layer / conformist), `direction` (upstream/downstream, only for directional types). Chosen over parsing `context-map.md` (brittle prose) and a separate `relationships.yaml` (truth too far from prose). `context-map.md` stays as the human-readable narrative; brainstorm/model eventually keep both in sync (captured as a follow-up in the Agentheim repo). (2) **Layout inside frame = deterministic force-directed initial layout** seeded by the relationship graph; per-BC manual drag overrides persisted via new `bc_positions` table; relayout fires only on add/remove/relationship-change (no continuous animation). (3) **Frame sizing = auto-fit content, no user-resize at v1**. (4) **Edge vocabulary = four types** (upstream/downstream directional w/ arrowhead, mutual/shared-kernel/partnership non-directional, ACL directional w/ notch glyph, conformist directional lighter weight); single neutral palette, geometry distinguishes; `design-system-002` finalises. (5) **Project header = frame's top edge** carrying project name, status badges, task counts; right-click on title bar opens existing tile context menu. (6) **Schema migration = v3→v4** with `bc_positions(project_id, bc_name, x, y)` table, `ON DELETE CASCADE`, mirroring `tile_positions` semantics. (7) **Cross-project edges = v2+, locked out of scope**. (8) **Migration path = hard cutover** (v1 unshipped, no users). (9) **Bootstrap data**: hand-curate GUPPI's seven BC READMEs in-task as part of `project-registry-004`; other Agentheim projects render zero edges in GUPPI's canvas until the upstream Agentheim work lands (graceful degradation). **Decomposition (three tasks):** **`project-registry-004`** (todo) — frontmatter parser via `serde_yaml`, `BoundedContext.relationships: Vec<Relationship>` on `ProjectSnapshot`, new `bc_relationships_changed { project_id, bc_name }` fine-grained event with deep-equal change-detection cache to avoid spurious relayouts on prose edits, schema v4, three new IPC commands (`save_bc_position` / `load_bc_position` / `load_bc_positions`), bootstrap 7 GUPPI READMEs. **`design-system-002`** (todo) — `STYLEGUIDE.md` gains §"Project frame" + §"BC bubble (inside frame)" + §"Intra-project edges"; new `frame.*` + `edge*` tokens in `tokens.ts` / `tokens.css`; aesthetic defaults proposed for Marco's in-person sign-off (1px solid `tileBorder` frame w/ integrated header bar at `spacing.xl`, rounded-rectangle BC bubble, ACL notch as midpoint triangle, single-neutral `fgMuted` edge palette letting geometry differentiate). **`canvas-007`** (stays backlog) — the integration; replace orbit rendering with frame + interior bubbles + intra-project edges, consume the upstream data, persist per-BC drag, keep canvas-001 targeted updates / canvas-005a-005b right-click affordances + missing-tile / canvas-006 live-add serialisation working; force-directed initial layout in a new pure module (test surface alongside `tile-layout.ts`); promotes to todo once both upstream tasks complete. **Agentheim-repo follow-up captured** at `C:\src\heimeshoff\agentic\agentheim\.agentheim\backlog\brainstorm-model-maintain-bc-relationships-frontmatter.md` — brainstorm writes `relationships:` frontmatter on BC creation; model updates it on capture/refine; schema + symmetry rule documented in a new `references/` doc. **Frontend gate:** canvas-007 + design-system-002 + project-registry-004 are coherent because `STYLEGUIDE.md` is the single source of truth design-system-002 extends and canvas-007 consumes. **No orchestrator round** — every decision resolvable from styleguide-001 + ADR-003 + ADR-004 + ADR-008 + ADR-009 + the existing Rust IPC surface; no architect / strategic-modeler / tactical-modeler delegation needed. **No ADRs written** — every decision sits inside existing ADRs (ADR-003 PixiJS rendering structure shifts inside, ADR-004 schema gets a v4 migration, ADR-008 watcher fires the new event variant, ADR-009 gains one event variant in the existing typed enum). `canvas-003-focus-zoom` continues to `depends_on` `canvas-007`.
**Split into:** canvas-007 (stays backlog), project-registry-004 (todo), design-system-002 (todo)
**Spawned external task:** `agentheim/.agentheim/backlog/brainstorm-model-maintain-bc-relationships-frontmatter.md`
**ADRs written:** none

---

## 2026-05-15 13:30 -- Model / Refined: canvas-003-focus-zoom — locked "B" + spawned canvas-007

**Type:** Model / Refine
**BC:** canvas
**Status after:** backlog (both canvas-003 and canvas-007)
**Summary:** Refining canvas-003 surfaced a much bigger structural redesign. Locked the focus-zoom *frame target*: focusing a project frames the **project's bounded region including its BCs** (option B). That answer is stable across today's orbit model and the redesign captured below — in both, focus = "the union of project + its BCs", which is the same `Camera.fitTo(box)` machinery the styleguide already names. The styleguide ships `Camera.fitTo` + `Camera.lerpTo` + the 320ms `durationCamera` budget; the focus task is mostly wiring the trigger and answering the input-scheme / persistence / ESC questions. **More importantly:** Marco wants the canvas to abandon the "project = bubble with orbiting BCs" rendering altogether. New model: **project = surrounding frame, BCs = bubbles inside the frame, edges between BCs drawn by context-map relationship type** (upstream/downstream/mutual/none). That's a visual + data redesign, not a focus-zoom subset, so it gets its own task — `canvas-007-project-as-frame` — captured to `backlog/` with the open questions explicitly catalogued (data source for BC↔BC relationships, layout inside the frame, frame sizing, edge visual vocabulary, project header placement, schema migration for per-BC positions, cross-project edges, migration path). canvas-003 is now hard-`depends_on` canvas-007 — the redesign ships first so canvas-003's open questions (keyboard scheme, BC-as-focus-target) get answered against frames-and-bubbles rather than tiles-and-orbits. Likely task spawns during canvas-007 refinement: a `project-registry-NNN` for the data source and a `design-system-NNN` for the edge vocabulary; possibly an infrastructure schema-migration task. No ADRs written — every decision sits inside ADR-003 (PixiJS + HTML overlay, rendering structure changes inside the existing stance). Refinement continues on canvas-007 next turn.
**Split into:** *(none — canvas-003 itself not split; canvas-007 captured as a new sibling task surfaced by canvas-003's "frame what?" question.)*
**Spawned dependency:** canvas-007-project-as-frame (backlog)
**ADRs written:** none

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
