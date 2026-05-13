# Vision: GUPPI

## Purpose
GUPPI is a personal mission-control for managing development projects that use the Agentheim plugin and Claude Code. It gives me a single Miro-like canvas where every Agentheim project is visible at once — projects as tiles, bounded contexts as connected child nodes, with live indicators of what's running and what needs attention. I drive it primarily by voice ("Bob, refine task 104 in image-gallery") with mouse and keyboard as fallbacks. It launches `claude` *inside each target project's folder* so the project's own skills, hooks, and rules apply — GUPPI is a controller, not a host.

## Users
Just me (Marco). Single-user, single-machine, native desktop tool. No multi-user, no team features, no remote access.

## The problem
Today, running Agentheim across multiple repos means:
- juggling lots of terminals and losing track of which is doing what
- no overview — I can't see across projects at a glance
- can't talk to my computer most of the time; to use voice I have to focus a specific terminal first
- "where am I" cognitive overhead every time I switch repos

(Having to `cd` into each repo is *not* a real pain — the filesystem is fine. The pain is the lack of a single ambient surface and the lack of ambient voice.)

## What success looks like

**v1 (irreducible core): Canvas-only MVP.** A read-only Miro-like surface showing every Agentheim-managed project as a tile, with its BCs as connected child nodes, with task counts (backlog / doing / done) per BC. Pan, zoom, drag to reposition, click-or-keyboard "zoom to focus". No interactions yet, no voice yet, no live agent observation yet — just the overview. When v1 lands, the "no overview" pain is gone and I can already feel relief.

**Beyond v1 (in rough roadmap order, captured here so we don't lose the picture):**
- Click/keyboard-driven commands: move task backlog→todo, trigger `model`, create new project (folder + `git init` + kick off `brainstorm` inside it)
- Voice input: "Bob, …" wake word via Whisperheim, fuzzy command parsing, all Agentheim actions accessible by voice; voice-driven focus ("zoom to image-gallery")
- Voice output via Utterheim narrator (notifications, "agent is asking you something")
- Live agent-awareness: per-BC indicators for running / idle / blocked-on-question, with the question rendered at the BC's location on the canvas
- Emulated terminal panel for interactive Claude sessions — shallow view of orchestrator + sub-agent comms, same as a real terminal would show
- Project detail view: rendered markdown for `vision.md`, `research/*.md`, ADRs, BC READMEs

## Non-goals
- **Not a multi-user or team tool.** Personal assistant for me.
- **Not a replacement for `claude` in the terminal.** Bare terminal stays as an escape hatch; GUPPI is the *primary* launcher, not the *exclusive* one. Sessions started in a bare terminal won't be fully observable by GUPPI, and that's accepted.
- **Not a host for `claude`.** GUPPI spawns `claude` *inside each target project's folder* so the project's CLAUDE.md, skills, hooks, and settings apply. GUPPI never runs its own `claude` in its own folder to act on another project. This is the load-bearing architectural rule.
- **Not a generic project management tool.** It only knows the Agentheim shape: vision → bounded contexts → tasks. Not a Jira/Linear/Trello replacement.
- **Not a code editor.** GUPPI renders markdown for viewing; the IDE owns editing.
- **Not a hosted service.** Local-only, native desktop, single machine.
- **Not an Agentheim replacement.** GUPPI *invokes* Agentheim's skills (`brainstorm`, `model`, `work`); it does not reimplement them.

## Ubiquitous language (seed)
- **Project** — a folder on disk that contains an `.agentheim/` directory; appears as a tile on the canvas.
- **Bounded context (BC)** — a `contexts/<name>/` directory inside a project; appears as a child node connected to its project's tile.
- **Tile / node** — the visual representation of a project (large) or a BC (small) on the canvas.
- **Canvas** — the Miro-like infinite surface; the primary view.
- **Tile state** — running / idle / blocked-on-question, derived from filesystem state and from claude-runner's observations.
- **Session** — a running `claude` process for a given project. Either owned by GUPPI (spawned inside the project folder, full visibility) or started independently in a bare terminal (observed by filesystem only).
- **Wake word** — "Bob", spoken to begin a voice command.
- **Narrator** — Utterheim; the TTS channel GUPPI uses to speak to me.
- **Transcriber** — Whisperheim; the STT pipeline (Silero VAD + Parakeet) GUPPI listens through.

## Open questions
- **Where does Whisperheim run, and where does the wake-word listener live?** Same machine as GUPPI assumed. Preferred path is to extend Whisperheim to expose an interface GUPPI can subscribe to; fallback is in-process listening inside GUPPI. To be resolved in the architecture foundation pass.
- **Detecting bare-terminal `claude` sessions GUPPI did not spawn.** Best-effort via filesystem (tasks in `doing/`, hook outputs). Acceptable for v1 since v1 is read-only anyway.
- **Canvas state persistence scope.** Tile positions, clusters, zoom level — stored in GUPPI's own state directory, not in each project's `.agentheim/`. Exact location/format decided in foundation pass.
