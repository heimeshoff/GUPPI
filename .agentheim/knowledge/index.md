# Knowledge index

Top-level catalog for the GUPPI project. Tracks bounded contexts and global ADRs.

## Bounded contexts

<!-- bc-list:start -->
- [canvas](../contexts/canvas/INDEX.md) — Miro-like surface, the primary view. **Core.**
- [project-registry](../contexts/project-registry/INDEX.md) — discovers/lists/creates Agentheim projects on disk. **Supporting.**
- [claude-runner](../contexts/claude-runner/INDEX.md) — spawns `claude` inside each target project's folder; owns PTY/stdio. **Core.**
- [agent-awareness](../contexts/agent-awareness/INDEX.md) — "what's running / waiting / blocked"; drives status badges and question-at-BC overlays. **Core.**
- [voice](../contexts/voice/INDEX.md) — Whisperheim (STT) + Utterheim (TTS) bridge; wake-word "Bob". **Core.**
- [design-system](../contexts/design-system/INDEX.md) — visual tokens, components, patterns; gates every frontend task. **Supporting.**
- [infrastructure](../contexts/infrastructure/INDEX.md) — globally-true tech foundation (runtime, persistence, IPC, etc.). **Generic.**
<!-- bc-list:end -->

## Global ADRs

<!-- adr-global:start -->
- [ADR-007 — Voice integration: Whisperheim WebSocket bridge](decisions/ADR-007-voice-integration.md) — Accepted. Extend Whisperheim with a local WebSocket bridge; GUPPI subscribes to wake-word/transcript, emits speak. Contract: `contexts/infrastructure/voice-bridge.md`.
- [ADR-005 — Project discovery: explicit registry + manual scan](decisions/ADR-005-project-discovery.md) — Accepted. Explicit registry primary (the `projects` table), user-triggered folder scan, no unprompted disk-walking.
- [ADR-003 — Canvas rendering: PixiJS v8](decisions/ADR-003-canvas-rendering.md) — Accepted. PixiJS v8 (WebGL) for the canvas, HTML overlays positioned to world coordinates for rich interactive tiles.
- [ADR-006 — Claude session ownership & PTY](decisions/ADR-006-claude-pty.md) — Accepted. `portable-pty` actor-per-session, native `claude.exe` with cwd-per-project, Job Objects for cleanup. Empirical Windows spike deferred (infrastructure-013).
- [ADR-004 — Persistence: SQLite in OS user-config dir](decisions/ADR-004-persistence.md) — Accepted. Single `guppi.db` SQLite file for GUPPI's own view-state, resolved via Tauri's path API.
- [ADR-002 — Frontend framework: Svelte 5 + SvelteKit](decisions/ADR-002-frontend-framework.md) — Accepted. Svelte 5 + SvelteKit static adapter, SPA inside the Tauri bundle.
- [ADR-001 — Desktop runtime: Tauri 2](decisions/ADR-001-desktop-runtime.md) — Accepted. Tauri 2 (Rust core + web frontend), Windows-only validated day one.
<!-- adr-global:end -->

## Recent activity

See `protocol.md` for the chronological log.
