# infrastructure — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet — written as decision tasks below get worked.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
- [infrastructure-015-log-retention-sweep](backlog/infrastructure-015-log-retention-sweep.md) — `type: feature`, depends on 012. Add the ADR-010 7-day retention sweep of rotated log files (rotation is wired; pruning is not).
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
*(None.)*
<!-- todo-list:end -->

**Todo count:** 0

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [infrastructure-014-fine-grained-fs-events](done/infrastructure-014-fine-grained-fs-events.md) — `type: feature`. Single-project watcher correlates each debounced batch into the fine-grained taxonomy (`TaskMoved`/`TaskAdded`/`TaskRemoved`/`BCAppeared`/`BCDisappeared`); ADR-008↔ADR-009 reconciled in place; `AgentheimChanged` kept alive as a compatibility seam. Frontend reaction → `canvas-001`.
- [infrastructure-013-pty-spike](done/infrastructure-013-pty-spike.md) — `type: spike`. ADR-006 PTY spike PASSED — `ClaudeSession` actor (`portable-pty` + Windows Job Object + cwd-per-spawn) in the `pty` module, mechanics proven by `cargo test` (18/18); real-`claude.exe` hands-on items exercisable via `pty_*` IPC. → ADR-006, ADR-012.
- [infrastructure-012-walking-skeleton](done/infrastructure-012-walking-skeleton.md) — `type: spike`. GUPPI's first code — Tauri 2 + Svelte 5 + PixiJS walking skeleton; all eleven foundation ADRs validated by execution (14 Rust tests, `pnpm check`, MSI build).
- [infrastructure-011-packaging](done/infrastructure-011-packaging.md) — `type: decision`. Tauri bundler, unsigned MSI, per-user install, GitHub Release updater feed. → ADR-011.
- [infrastructure-010-logging](done/infrastructure-010-logging.md) — `type: decision`. `tracing` to rotating local logs, 7-day retention, no telemetry. → ADR-010.
- [infrastructure-009-event-bus](done/infrastructure-009-event-bus.md) — `type: decision`. Tokio broadcast (cap 1024) + frontend-bridge to Tauri emit. → ADR-009.
- [infrastructure-008-filesystem-observation](done/infrastructure-008-filesystem-observation.md) — `type: decision`. `notify-debouncer-full`, one 250ms watcher per project, `WatcherSupervisor`. → ADR-008.
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
