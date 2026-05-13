# claude-runner

## Purpose

Spawns and owns `claude` processes for GUPPI. The load-bearing rule from the vision: **`claude` always runs with cwd set to the target project's folder, never in GUPPI's own folder**. This is what makes the target project's `CLAUDE.md`, skills, hooks, and settings actually apply. GUPPI is a *controller*, not a host.

Responsibilities:
- Spawn `claude` inside the right folder, with PTY/stdio attached.
- Maintain the session lifecycle (start, alive, exited).
- Route user input (keyboard, voice-derived commands at v2+) to the correct session.
- Expose the orchestrator stream and sub-agent stream for rendering and for `agent-awareness` to subscribe to.
- Power the emulated terminal panel inside the canvas's detail view (a shallow view of what a real terminal would show — orchestrator + sub-agent comms).

GUPPI is the *primary* launcher of `claude`, not the *exclusive* one. Sessions started in a bare terminal exist, are accepted, and are observed by `agent-awareness` via filesystem signals only — `claude-runner` doesn't try to attach to processes it didn't spawn.

## Classification

**Core.** The in-folder-spawning model is the architectural reason GUPPI can be Agentheim-aware without reimplementing Agentheim. The rule "GUPPI never hosts `claude` in its own folder" is enumerated as a non-goal in the vision; this BC is where that rule lives.

## Ubiquitous language (seed)

- **Session** — a running `claude` process.
- **GUPPI-owned session** — a session spawned by `claude-runner`; full visibility.
- **Bare-terminal session** — a session Marco started directly in a terminal; observed by filesystem only, not managed here.
- **Spawn** — start a new `claude` process inside a target project folder.
- **Target project folder** — the cwd of the spawned process. Non-negotiable; never GUPPI's own folder.
- **PTY / stdio** — the pseudo-terminal and standard streams attached to a session.
- **Terminal panel** — the emulated terminal UI rendered by `canvas`, fed from this BC's session streams.
- **Orchestrator stream** — the main `claude` process's output.
- **Sub-agent stream** — output from sub-agents the orchestrator spawns; surfaced shallowly in the panel.
- **Session routing** — directing an inbound input event to the correct session.

## Frontend gate

The terminal panel rendering lives in `canvas`, not here — so the frontend gate applies to `canvas`'s implementation of the panel, not to this BC. This BC stays UI-free (streams in, streams out).

## Upstream / downstream

- **Downstream of:** the `claude` CLI itself (external integration; this BC is essentially an adapter).
- **Upstream of:** `agent-awareness` (publishes session events / prompt-waiting signals); `canvas` (supplies streams to the terminal panel).

## Open questions

- PTY library choice (Windows-specific concerns — ConPTY) — foundation pass.
- How is "blocked-on-question" detected from the stream? Heuristic on output patterns, or a contract with `claude` itself? Foundation / spike.
- Session persistence across GUPPI restarts — out of scope for v1, but flag for later.
