# infrastructure

## Purpose

Owns the **globally-true** technical foundation of GUPPI: desktop runtime, frontend framework, canvas rendering library, persistence, project discovery convention, PTY/session ownership, voice bridge contracts, filesystem watching, event bus, logging, and packaging. The standing home for cross-cutting tech concerns.

BC-local infrastructure (an adapter that only one BC uses, a repository implementation specific to one BC, a queue handler scoped to one BC) does **not** belong here — it stays inside the originating BC. The test: *"if any single BC didn't exist, would this concern still need to exist?"* If yes, it lives here.

## Classification

**Generic.** Solved-problem plumbing. None of what lives here is GUPPI's reason to exist; it's the table on which the core BCs (canvas, claude-runner, agent-awareness) do their work.

## Ubiquitous language (seed)

Generic ops/runtime vocabulary, not project-specific:

- **Runtime** — the desktop process host (Tauri / Electron / similar).
- **Frontend** — the UI layer running in the runtime's WebView.
- **Core** — the non-UI side of the runtime (Rust in Tauri's case), owning filesystem/PTY/persistence.
- **IPC** — the communication channel between core and frontend.
- **Event bus** — the in-core typed pub/sub for domain events.
- **State store** — GUPPI's own persistent state (tile positions, registry, preferences); lives in OS user-config dir, NOT in any managed project's `.agentheim/`.
- **Watcher** — a debounced filesystem observer for a project's `.agentheim/` subtree.
- **Bridge** — a local IPC contract with an external tool (Whisperheim, Utterheim).
- **PTY** — pseudo-terminal used to host a managed project's `claude` process.
- **Job Object** (Windows-specific) — OS-level container that ensures child processes die with the parent.

## Upstream dependencies

None inside GUPPI (this BC sits at the bottom of the stack). External: the OS, the user's installed `claude.exe`, Whisperheim, Utterheim.

## Downstream consumers

Every other BC consumes something here:
- `canvas` — frontend framework, rendering library, state-store API for layout persistence.
- `project-registry` — state-store, filesystem watcher.
- `claude-runner` — PTY library, Job Object on Windows, event bus.
- `agent-awareness` — filesystem watcher, event bus.
- `voice` — voice-bridge contracts (Whisperheim STT, Utterheim TTS).
- `design-system` — frontend framework (the styleguide must be expressible in whatever frontend stack is chosen).

## Open questions

- All foundation decisions are in `todo/` as `type: decision` tasks. See INDEX.md.
- The walking-skeleton task in this BC's `todo/` is the project's first prototype — feature-thin, architecture-thick.
