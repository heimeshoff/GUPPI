---
id: infrastructure-012-walking-skeleton
type: spike
status: todo
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
