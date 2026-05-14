# canvas

## Purpose

The Miro-like infinite surface that is GUPPI's primary view. Every Agentheim project appears as a tile, its bounded contexts as connected child nodes, with status badges per BC and task counts (backlog / doing / done). Supports pan, zoom, drag-to-reposition, click-or-keyboard focus-zoom, and a project-detail view that renders markdown (vision.md, research/*.md, ADRs, BC READMEs). The v1 MVP is canvas-only and read-only — when it lands, the "no overview" pain from the vision is gone.

This BC also owns the rendered-markdown detail pane (originally considered a separate `document-viewer` context; folded in because it has no distinct language or actor of its own).

## Classification

**Core.** GUPPI exists to provide this ambient overview surface. The canvas is one of GUPPI's two headline differentiators (the other being live agent-awareness).

## Frontend gate

This BC has a frontend. Every frontend task in this BC must `depends_on` the styleguide task `design-system-001-styleguide` in `contexts/design-system/`, and must be implemented against the styleguide itself: [`contexts/design-system/STYLEGUIDE.md`](../design-system/STYLEGUIDE.md) — the visual vocabulary (tokens, component states, motion budget) that keeps the canvas coherent. No UI work here is promoted to `doing/` before the styleguide is signed off.

The styleguide was signed off in person by Marco on 2026-05-14, so the gate is open — but the `depends_on` link and the "build against `STYLEGUIDE.md`" rule still apply to every frontend task.

## Ubiquitous language (seed)

- **Canvas** — the infinite surface itself.
- **Tile** — visual representation of a project (large node).
- **Node** — visual representation of a bounded context (small node, child of a project tile).
- **Connection** — the line between a tile and its BC nodes.
- **Viewport** — the currently visible window onto the canvas (pan position + zoom level).
- **Focus** — a "zoom to" operation that frames a specific tile or node.
- **Layout** — positions of tiles/nodes on the canvas (persisted in GUPPI's own state directory, not in the target project's `.agentheim/`).
- **Status badge** — the per-BC visual indicator (running / idle / blocked-on-question dot), driven by `agent-awareness`.
- **Detail view** — the project-detail pane that renders markdown documents from a project.
- **Markdown pane** — the renderer for `vision.md`, `research/*.md`, ADRs, BC READMEs in the detail view.
- **Targeted update** — patching the client-side `ProjectSnapshot` in place from a fine-grained filesystem event (`task_moved` / `task_added` / `task_removed` / `bc_appeared` / `bc_disappeared`) instead of re-fetching the whole snapshot. A tile's task counts tick, a BC node appears/disappears, without a `get_project` round-trip (`canvas-001`).
- **Resync** — the one remaining full `get_project` re-fetch, triggered only by the `resync_required` domain event. The Rust core's event bridge emits it when its broadcast receiver lags and loses events it cannot reconstruct (ADR-009 lag-resync strategy).

## How the canvas stays live

The canvas does not poll. The Rust core watches each project's `.agentheim/`
and emits fine-grained domain events; the frontend applies them to its
in-memory model as **targeted updates** (see `src/lib/snapshot-patch.ts`).
Robustness rules baked into the patching: a `task_*` event for a BC not yet in
the model lazily creates a zero-count node (filesystem events can arrive before
the `bc_appeared` for the same batch); a delta that would push a count below
zero is clamped at 0 and logged (the client model has drifted from disk); and
every event is filtered by `project_id`. A full re-fetch happens only on
**resync** (`resync_required`).

## Upstream dependencies

- `project-registry` — supplies the list of projects, their BCs, and task counts (customer-supplier; canvas is downstream).
- `agent-awareness` — supplies tile state and question-at-BC-location overlays (customer-supplier; canvas is downstream).
- `claude-runner` — supplies the orchestrator/sub-agent streams that the terminal panel inside the detail view renders (canvas owns the rendering component, runner owns the stream).
- `infrastructure` — canvas state persistence (tile positions, zoom, clusters) lives in GUPPI's own state directory via the infrastructure-provided persistence API.

## Open questions

- Terminal panel ownership boundary with `claude-runner` (rendering here, stream from there — confirm during walking-skeleton).
- Layout persistence format and location (foundation pass).
