# agent-awareness

## Purpose

Aggregates a single, unified "what's running, what's idle, what's blocked-on-question" view across every observed session, regardless of source. The canvas's status badges and the question-at-BC-location overlay are driven entirely from this BC.

Two signal sources, two fidelities:

1. **Rich signals from `claude-runner`** for guppi-owned sessions: orchestrator stream events, prompt-waiting detection, lifecycle events. Subscribes to runner events.
2. **Best-effort filesystem signals** for bare-terminal sessions guppi didn't spawn: presence of tasks in `doing/`, hook output files, mtime patterns. Conforms to the AgentHeim-on-disk shape.

The value-add is the *unified state model* that survives this source split: downstream consumers (the canvas) see one shape, not two.

## Classification

**Core.** "Live indicators of what's running and what needs attention" is one of guppi's two headline differentiators (the other being the canvas surface itself). The unified-state model — and especially the question-at-BC-location feature — is guppi-specific intelligence, not off-the-shelf plumbing.

## Ubiquitous language (seed)

- **Tile state** / **BC state** — the per-BC status: running / idle / blocked-on-question.
- **Running** — at least one session is actively producing output for this BC.
- **Idle** — no active session for this BC, or active session is waiting on nothing.
- **Blocked-on-question** — an active session is waiting for human input. The question text is captured and surfaced at the BC's location on the canvas.
- **Owned session** — observed via `claude-runner` events (rich signal).
- **Observed session** — observed via filesystem only (best-effort signal).
- **Indicator** — the visual badge consumed by the canvas (blinking dot, color, etc.).
- **Question-at-location** — the rendered question text overlayed at the relevant BC node on the canvas.
- **Signal source** — runner-event-stream | filesystem-watch.

## Upstream / downstream

- **Downstream of:**
  - `claude-runner` (customer-supplier; subscribes to session events).
  - The filesystem / `project-registry` (conformist to the AgentHeim-on-disk shape; reads `doing/` contents, hook outputs, mtimes).
- **Upstream of:** `canvas` (supplies status badges and question-at-location overlays).

## Open questions

- Shared watcher with `project-registry`, or independent? Foundation pass — likely one watcher, two consumers via the infrastructure event bus.
- "Blocked-on-question" detection — owned here as a state transition, but the underlying signal comes from `claude-runner`. Where does the heuristic live? Lean: runner detects the raw "stream went quiet after a `?`" signal; this BC interprets it as state.
- v1 scope: do we ship without filesystem-observed sessions (runner-only), or do we ship both from the start? Canvas-only MVP says no live observation at all in v1 — so this BC's v1 surface is minimal.
