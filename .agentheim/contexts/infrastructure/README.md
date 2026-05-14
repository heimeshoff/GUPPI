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

## Codebase

The walking skeleton (`infrastructure-012`) is GUPPI's first code. Layout:

- **`src-tauri/`** — the Rust **Core**. Modules: `db` (ADR-004 SQLite state),
  `project` (`get_project` → `ProjectSnapshot`), `watcher` (ADR-008 debounced
  `.agentheim/` observer), `events` (ADR-009 `EventBus` + typed `DomainEvent`),
  `logging` (ADR-010 `tracing` to rotating files), `lib.rs` (Tauri wiring,
  IPC commands, the single ADR-009 frontend-bridge task).
- **`src/`** — the SvelteKit **Frontend** (ADR-002). `lib/ipc.ts` is the thin
  IPC abstraction over Tauri `invoke`/`listen` (ADR-001); `lib/camera.svelte.ts`
  is the camera rune store (ADR-003); `lib/Canvas.svelte` is the PixiJS v8
  canvas.

Run command: `pnpm tauri dev`. Release + MSI: `pnpm tauri build`.

## Ubiquitous language (skeleton additions)

- **ProjectSnapshot** — the Core's read-model of one Agentheim project for the
  canvas: `{ name, path, bcs: [{ name, task_counts }] }`. Produced by the
  `get_project` IPC command.
- **TaskCounts** — per-bounded-context tallies keyed by Agentheim task state
  (`backlog`, `todo`, `doing`, `done`), derived by counting `.md` files.
- **Frontend bridge** — the single Core task that subscribes to the `EventBus`
  and forwards frontend-relevant `DomainEvent`s to the WebView under the one
  Tauri event name `guppi://event`. The *only* place Tauri's `emit` is called
  for domain events (ADR-009).
- **AgentheimChanged** — the skeleton's deliberately coarse domain event: any
  change under a project's `.agentheim/` triggers a frontend re-fetch. The
  fine-grained `TaskMoved` / `BCAppeared` / `BCDisappeared` taxonomy from
  ADR-008/009 is deferred to `infrastructure-014`.

## Open questions

- All eleven foundation decisions are settled (ADR-001 … ADR-011) and now
  validated by the walking skeleton compiling and running.
- The PTY empirical spike (`infrastructure-013`) is still pending — ADR-006's
  decision is committed but not yet proven on hardware.
