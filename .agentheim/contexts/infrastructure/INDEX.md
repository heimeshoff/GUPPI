# infrastructure — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet — written as decision tasks below get worked.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
- [infrastructure-013-pty-spike](backlog/infrastructure-013-pty-spike.md) — `type: spike`, depends on walking skeleton. Empirical proof that `portable-pty` + Job Object + `claude.exe` works on Windows. Deferred from ADR-006.
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
- [infrastructure-008-filesystem-observation](todo/infrastructure-008-filesystem-observation.md) — `type: decision`. `notify-debouncer-full`, one watcher per project.
- [infrastructure-009-event-bus](todo/infrastructure-009-event-bus.md) — `type: decision`. Tokio broadcast + Tauri events.
- [infrastructure-010-logging](todo/infrastructure-010-logging.md) — `type: decision`. `tracing` to local rotating files, no telemetry.
- [infrastructure-011-packaging](todo/infrastructure-011-packaging.md) — `type: decision`. Tauri MSI bundler, unsigned initially.
- [infrastructure-012-walking-skeleton](todo/infrastructure-012-walking-skeleton.md) — `type: spike`, depends on 001–011. GUPPI's first prototype.
<!-- todo-list:end -->

**Todo count:** 5

## Doing

<!-- doing-list:start -->
*(None yet.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [infrastructure-007-voice-integration](done/infrastructure-007-voice-integration.md) — `type: decision`. Whisperheim WebSocket bridge; `voice-bridge.md` contract specced. → ADR-007.
- [infrastructure-005-project-discovery](done/infrastructure-005-project-discovery.md) — `type: decision`. Explicit registry primary + user-triggered folder scan; no unprompted disk-walking. → ADR-005.
- [infrastructure-003-canvas-rendering](done/infrastructure-003-canvas-rendering.md) — `type: decision`. PixiJS v8 (WebGL) + HTML overlays at world coords. → ADR-003.
- [infrastructure-006-claude-pty](done/infrastructure-006-claude-pty.md) — `type: decision`. `portable-pty` actor-per-session, native `claude.exe`, Job Objects. Empirical spike deferred → infrastructure-013. → ADR-006.
- [infrastructure-004-persistence](done/infrastructure-004-persistence.md) — `type: decision`. SQLite (`guppi.db`) in the OS user-config dir. → ADR-004.
- [infrastructure-002-frontend-framework](done/infrastructure-002-frontend-framework.md) — `type: decision`. Svelte 5 + SvelteKit (static adapter). → ADR-002.
- [infrastructure-001-desktop-runtime](done/infrastructure-001-desktop-runtime.md) — `type: decision`. Tauri 2 chosen, Windows-only day one. → ADR-001.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
