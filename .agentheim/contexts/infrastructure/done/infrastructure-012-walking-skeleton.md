---
id: infrastructure-012-walking-skeleton
type: spike
status: done
completed: 2026-05-14
commit: 1f37659
scope: global
depends_on:
  - infrastructure-001-desktop-runtime
  - infrastructure-002-frontend-framework
  - infrastructure-003-canvas-rendering
  - infrastructure-004-persistence
  - infrastructure-005-project-discovery
  - infrastructure-006-claude-pty
  - infrastructure-007-voice-integration
  - infrastructure-008-filesystem-observation
  - infrastructure-009-event-bus
  - infrastructure-010-logging
  - infrastructure-011-packaging
---

# Spike: walking skeleton

The thinnest end-to-end slice through the chosen stack. **Architecture-thick, feature-thin.** This is GUPPI's first prototype — the moment code first appears.

## Goal

Prove the spine of the stack end-to-end, with one hard-coded project rendered on a real canvas backed by a real `.agentheim/` directory. After this lands, all eleven foundation ADRs are validated by execution, not by argument.

## Scope (in)

1. The chosen desktop runtime boots a native window on Windows 11 titled "GUPPI".
2. The chosen frontend framework mounts a canvas (rendering library per ADR-003) filling the window.
3. Canvas supports pan (drag) and zoom (mouse wheel / pinch). Camera state in a frontend store.
4. Core has a hardcoded project path (Marco picks one, e.g. `C:\src\heimeshoff\agentic\guppi`). On startup it:
   - Verifies `.agentheim/` exists.
   - Reads `.agentheim/vision.md`'s first line as the project name.
   - Lists `.agentheim/contexts/*` (may be empty — that's fine).
5. A core command `get_project(path) -> ProjectSnapshot` returns `{ name, path, bcs: [{ name, task_counts: { backlog, todo, doing, done } }] }`.
6. Frontend calls `get_project` once on mount, renders one tile centred at origin with the project name + BC nodes connected to it, with task counts. Greybox visuals — the styleguide hasn't been signed off yet.
7. SQLite (or chosen persistence) initialised in the OS user-config dir with the ADR-004 schema. Hardcoded project inserted into `projects`. Tile position (0, 0) persisted on drag. Reopening the app restores camera and tile position.
8. Filesystem watcher (`notify-debouncer-full` or equivalent) watches the hardcoded project's `.agentheim/` and emits an event on any change. Frontend re-fetches `get_project` on the event. (Crude — real event-bus mapping comes after the skeleton.)

## Scope (out)

- PTY / `claude` spawning. **ADR-006 spike runs separately** before any v1.x feature depends on it.
- Voice. ADR-007 contract designed but not implemented in the skeleton.
- Multiple projects, registry UI, scanning.
- Markdown rendering, terminal panel.
- Auth, telemetry, updater.
- Styling / styleguide. Plain rectangles and lines are fine.

## Definition of done (acceptance criteria)

- [ ] `pnpm tauri dev` (or equivalent) opens a window showing one tile and its BC children, with live task counts.
- [ ] Move the tile with the mouse, close the app, reopen — the tile is where you left it, camera too.
- [ ] Manually move a file from `contexts/foo/backlog/x.md` to `contexts/foo/doing/x.md` in another window — within ~1 second the canvas updates the count.
- [ ] Closing the app leaves no orphan processes (sanity check for later PTY work).
- [ ] All eleven foundation ADRs are committed before this task is closed (decisions must be locked in).

## Estimated effort

2–4 days for someone comfortable with the chosen stack. First-time learners: budget a week.

## Risks retired

- The chosen runtime's frontend ⇄ core IPC actually works the way the docs say on Windows.
- Canvas pan/zoom feels right *inside* the chosen runtime's WebView (vs a normal browser).
- Persistence + runtime path resolution.
- Filesystem watcher against a real `.agentheim/` and event delivery latency.

## Risks NOT retired by this spike

- **PTY** (ADR-006) — needs its own one-day spike before any feature depends on it.
- **Voice** (ADR-007) — needs the Whisperheim bridge to exist before GUPPI can integrate.

## Outcome

GUPPI's first code. The full stack is scaffolded and **compiles and bundles
end-to-end**: `cargo test` (14 passing), `pnpm check` (0 errors), `pnpm build`,
and a complete `pnpm tauri build` producing
`GUPPI_0.1.0_x64_en-US.msi` — which validates ADR-001 (Tauri 2 core+frontend),
ADR-002 (Svelte 5 + adapter-static), ADR-011 (Tauri bundler, unsigned MSI) by
execution.

### Code-complete and compiling (all eleven ADRs honored in code)

- **Native window titled "GUPPI"** — `tauri.conf.json`, ADR-001.
- **Svelte 5 + SvelteKit static SPA** mounting a **PixiJS v8** canvas filling
  the window — `src/`, ADR-002 / ADR-003.
- **Pan (drag) + zoom (wheel)** with camera state in a Svelte 5 rune store
  (`src/lib/camera.svelte.ts`), zoom anchored at the cursor — ADR-003.
- **`get_project(path) -> ProjectSnapshot`** IPC command — verifies
  `.agentheim/` exists, reads the project name from `vision.md`'s first line,
  lists `contexts/*`, counts task `.md` files per state. `src-tauri/src/project.rs`.
- **Frontend renders** one project tile at world origin with BC nodes radiating
  from it, connecting edges, and live task counts (greybox visuals). Calls
  `get_project` on mount. `src/lib/Canvas.svelte`.
- **SQLite state** at `%APPDATA%\guppi\guppi.db` with the ADR-004 schema
  (`projects`, `tile_positions`, `clusters`, `app_state`, `schema_version`),
  versioned migrations, the hardcoded project upserted on startup, tile
  position + camera persisted/restored. `src-tauri/src/db.rs`, ADR-004.
- **`notify-debouncer-full` watcher** on the project's `.agentheim/`, 250ms
  debounce, publishing onto the **Tokio broadcast `EventBus`** (cap 1024, typed
  `DomainEvent`); a single **frontend-bridge task** forwards events to the
  WebView under `guppi://event`; the frontend re-fetches on the signal.
  `src-tauri/src/watcher.rs`, `events.rs`, `lib.rs` — ADR-008 / ADR-009.
- **`tracing` logging** to daily-rotated `%APPDATA%\guppi\logs\guppi.log`,
  frontend `console.*` forwarded via `log_from_frontend`. `src-tauri/src/logging.rs`,
  ADR-010.
- All eleven foundation ADRs (ADR-001 … ADR-011) are committed and Accepted.

### Needs Marco's hands-on confirmation (GUI interaction — not verifiable by an agent)

These are **ready to exercise** — the code paths exist and compile — but a
live GUI session is required to confirm them:

- Run `pnpm tauri dev`, confirm the window opens showing the tile + BC children
  with live counts.
- Move the tile with the mouse, close and reopen — confirm the tile and camera
  are restored (persistence round-trips are unit-tested; the *drag-to-persist*
  wiring is not).
- In another window, move a task file between `backlog/` and `doing/` — confirm
  the canvas count updates within ~1s (the watcher→bus→emit→re-fetch chain is
  unit-tested end to end in Rust; the frontend's reaction is not).
- Close the app, confirm no orphan processes (no PTY in the skeleton, so this
  is the trivial case — sanity check for later PTY work).

### Decisions / deviations

- **Repo layout** — Tauri-standard: SvelteKit frontend at the repo root,
  `src-tauri/` for the Rust core. Conventional; no ADR.
- **Coarse `AgentheimChanged` event** instead of ADR-008/009's fine-grained
  `TaskMoved` / `BCAppeared` / `BCDisappeared`. This is **sanctioned by this
  task's own scope** ("crude — real event-bus mapping comes after the
  skeleton"), not a new decision. Follow-up: `infrastructure-014`.
- **Log rotation wired, retention sweep deferred** — `tracing-appender` rotates
  daily but does not prune; the ADR-010 7-day retention is follow-up
  `infrastructure-015`.
- The `dialog` plugin was scaffolded then removed — ADR-005's folder pickers
  are explicitly canvas-BC future work, so no plugin is needed in the skeleton.

### Key files

- Rust core: `src-tauri/src/{lib,main,db,project,watcher,events,logging}.rs`,
  `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`,
  `src-tauri/capabilities/default.json`, `src-tauri/build.rs`.
- Frontend: `src/lib/{ipc.ts,types.ts,camera.svelte.ts,Canvas.svelte}`,
  `src/routes/{+page.svelte,+layout.ts}`, `src/app.html`,
  `package.json`, `svelte.config.js`, `vite.config.ts`, `tsconfig.json`.
