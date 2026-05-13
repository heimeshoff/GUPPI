# GUPPI

**General Unit Primary Peripheral Interface.**

A nod to Dennis E. Taylor's *Bobiverse*: GUPPI is Bob's non-sentient on-board
assistant — it runs the ship, handles the peripherals, and frees Bob to think.
Here, GUPPI is the non-sentient OS / orchestrator layer that sits *underneath*
the Agentic Developer (Claude Code + the Agentheim plugin) and gives it hands,
eyes, ears, and a voice. The wake word is, naturally, "Bob".

GUPPI does not think about the work. It runs the body so the agentic developer
can run the mind.

## What GUPPI is

A personal, single-machine mission-control surface for managing multiple
Agentheim projects at once. Concretely, GUPPI provides:

- **A canvas** — a Miro-like infinite surface where every Agentheim project on
  disk shows up as a tile, with its bounded contexts as connected child nodes.
  One ambient view across all projects, instead of N terminals.
- **A claude runner** — spawns `claude` processes *inside each target project's
  folder* so each project's own `CLAUDE.md`, skills, hooks, and settings apply.
  GUPPI is a controller, never a host.
- **Agent-awareness** — per-bounded-context indicators for running / idle /
  blocked-on-question, surfaced on the canvas at the BC's location.
- **A voice channel** — wake-word "Bob", Whisperheim for STT, Utterheim for
  TTS. Voice is a first-class input modality, not a gimmick.
- **Project discovery and creation** — find every folder containing
  `.agentheim/`, count tasks per state, and kick off new projects with
  `brainstorm` already running inside them.

## What GUPPI is not

- **Not sentient.** No agency, no judgment, no opinions about your code. The
  agentic developer is upstairs; GUPPI is downstairs.
- **Not a host for `claude`.** GUPPI never runs `claude` in its own folder to
  act on another project. Always spawns into the target project's cwd.
- **Not a replacement** for Claude Code, for Agentheim, or for the terminal.
  The terminal stays as an escape hatch.
- **Not a team tool.** Single user (Marco), single machine, local-only.
- **Not a code editor or a generic project manager.** It only knows the
  Agentheim shape: vision -> bounded contexts -> tasks.

## Where the actual specs live

- [`.agentheim/vision.md`](.agentheim/vision.md) — vision, problem,
  v1 scope, non-goals, ubiquitous language.
- [`.agentheim/context-map.md`](.agentheim/context-map.md) — bounded contexts
  and their relationships.
- [`.agentheim/contexts/`](.agentheim/contexts/) — each BC's `README.md`,
  backlog, and todo list.

## Status

Pre-v1. The domain has been modeled; foundation tasks (desktop runtime,
canvas rendering, claude PTY, persistence, project discovery, walking
skeleton) are in `infrastructure/todo/`. No application code exists yet.

v1 is deliberately tiny: a read-only canvas that shows every project and its
BCs with task counts. Voice, command execution, and live agent observation
come after.
