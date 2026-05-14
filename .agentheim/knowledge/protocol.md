# Protocol

Chronological log of everything that happens in this project.
Newest entries on top.

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
