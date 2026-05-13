# infrastructure — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet — written as decision tasks below get worked.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
*(None yet.)*
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
- [infrastructure-001-desktop-runtime](todo/infrastructure-001-desktop-runtime.md) — `type: decision`. Tauri 2 (architect's recommendation). Open Q: Tauri vs Electron.
- [infrastructure-002-frontend-framework](todo/infrastructure-002-frontend-framework.md) — `type: decision`. Svelte 5. Open Q: Svelte vs React vs Solid.
- [infrastructure-003-canvas-rendering](todo/infrastructure-003-canvas-rendering.md) — `type: decision`. PixiJS v8 + HTML overlays.
- [infrastructure-004-persistence](todo/infrastructure-004-persistence.md) — `type: decision`. SQLite in OS user-config dir.
- [infrastructure-005-project-discovery](todo/infrastructure-005-project-discovery.md) — `type: decision`. Explicit registry + manual scan.
- [infrastructure-006-claude-pty](todo/infrastructure-006-claude-pty.md) — `type: decision`. `portable-pty` + Job Objects on Windows. Riskiest — needs a real spike. Open Qs: WSL vs native, multi-OS scope.
- [infrastructure-007-voice-integration](todo/infrastructure-007-voice-integration.md) — `type: decision`. Extend Whisperheim with local WebSocket bridge. Open Q: Marco's willingness to touch Whisperheim.
- [infrastructure-008-filesystem-observation](todo/infrastructure-008-filesystem-observation.md) — `type: decision`. `notify-debouncer-full`, one watcher per project.
- [infrastructure-009-event-bus](todo/infrastructure-009-event-bus.md) — `type: decision`. Tokio broadcast + Tauri events.
- [infrastructure-010-logging](todo/infrastructure-010-logging.md) — `type: decision`. `tracing` to local rotating files, no telemetry.
- [infrastructure-011-packaging](todo/infrastructure-011-packaging.md) — `type: decision`. Tauri MSI bundler, unsigned initially.
- [infrastructure-012-walking-skeleton](todo/infrastructure-012-walking-skeleton.md) — `type: spike`, depends on 001–011. GUPPI's first prototype.
<!-- todo-list:end -->

**Todo count:** 12

## Doing

<!-- doing-list:start -->
*(None yet.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
*(None yet.)*
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
