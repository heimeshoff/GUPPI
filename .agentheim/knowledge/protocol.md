# Protocol

Chronological log of everything that happens in this project.
Newest entries on top.

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
