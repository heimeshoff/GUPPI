# canvas

## Purpose

The Miro-like infinite surface that is guppi's primary view. Every AgentHeim project appears as a tile, its bounded contexts as connected child nodes, with status badges per BC and task counts (backlog / doing / done). Supports pan, zoom, drag-to-reposition, click-or-keyboard focus-zoom, and a project-detail view that renders markdown (vision.md, research/*.md, ADRs, BC READMEs). The v1 MVP is canvas-only and read-only — when it lands, the "no overview" pain from the vision is gone.

This BC also owns the rendered-markdown detail pane (originally considered a separate `document-viewer` context; folded in because it has no distinct language or actor of its own).

## Classification

**Core.** Guppi exists to provide this ambient overview surface. The canvas is one of guppi's two headline differentiators (the other being live agent-awareness).

## Frontend gate

This BC has a frontend. Every frontend task in this BC must `depends_on` the styleguide task in `contexts/design-system/`. No UI work here is promoted to `doing/` before the styleguide is signed off.

## Ubiquitous language (seed)

- **Canvas** — the infinite surface itself.
- **Tile** — visual representation of a project (large node).
- **Node** — visual representation of a bounded context (small node, child of a project tile).
- **Connection** — the line between a tile and its BC nodes.
- **Viewport** — the currently visible window onto the canvas (pan position + zoom level).
- **Focus** — a "zoom to" operation that frames a specific tile or node.
- **Layout** — positions of tiles/nodes on the canvas (persisted in guppi's own state directory, not in the target project's `.agentheim/`).
- **Status badge** — the per-BC visual indicator (running / idle / blocked-on-question dot), driven by `agent-awareness`.
- **Detail view** — the project-detail pane that renders markdown documents from a project.
- **Markdown pane** — the renderer for `vision.md`, `research/*.md`, ADRs, BC READMEs in the detail view.

## Upstream dependencies

- `project-registry` — supplies the list of projects, their BCs, and task counts (customer-supplier; canvas is downstream).
- `agent-awareness` — supplies tile state and question-at-BC-location overlays (customer-supplier; canvas is downstream).
- `claude-runner` — supplies the orchestrator/sub-agent streams that the terminal panel inside the detail view renders (canvas owns the rendering component, runner owns the stream).
- `infrastructure` — canvas state persistence (tile positions, zoom, clusters) lives in guppi's own state directory via the infrastructure-provided persistence API.

## Open questions

- Terminal panel ownership boundary with `claude-runner` (rendering here, stream from there — confirm during walking-skeleton).
- Layout persistence format and location (foundation pass).
