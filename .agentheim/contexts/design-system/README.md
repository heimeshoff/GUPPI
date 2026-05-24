---
name: design-system
classification: supporting
relationships:
  - to: infrastructure
    type: shared-kernel
---

# design-system

## Purpose

Frontend infrastructure: the visual language GUPPI uses on its canvas and detail panes. Tokens (color, typography, spacing, motion), components (tile, BC node, edge, status badge, terminal-panel chrome, command palette, voice-state indicator), patterns (focus/hover, zoom-to-fit transitions, ambient mic state), and the review process for keeping all of it coherent.

## Styleguide

The styleguide is **code-complete** (`design-system-001-styleguide` — orbit baseline; `design-system-002-project-frame-vocabulary` — project-as-frame additions; `design-system-003-brand-colors-and-status-revision` — Marco's `claude.ai/design` refinement landing brand orange `#ff8b00` + brand blue `#25abfe`, revised status palette, and v1 dimensional refinements; `design-system-004-light-theme` — optional light palette + theme toggle, theme persisted in SQLite per ADR-004; `design-system-006-kanban-accordion-card-and-panel-vocabulary` — the canvas-pivot interior vocabulary: BC accordion row, kanban board/column, task card, docked detail panel + callout (STYLEGUIDE §3.9–3.12), mirrored into `tokens.ts` + `tokens.css` in both themes). Canvas re-validation against the revised tokens follows in `canvas-008`; the kanban-accordion interior is consumed by `canvas-020/021/022` + `agent-awareness-002`.

- **The styleguide document:** `STYLEGUIDE.md` (this directory) — tokens, component states, patterns, and the resolved open-question defaults. §3.1–3.5 = orbit baseline; §3.6–3.8 = project frame, BC bubble (inside frame), intra-project edges (§3.7/§3.8 superseded for the canvas interior by design-system-006, preserved for a future cross-project view); §3.9–3.12 = the canvas-pivot interior (BC accordion row, kanban board/column, task card, docked detail panel).
- **Tokens (source of truth):** `src/lib/design/tokens.ts` (PixiJS-ready numeric values) and `src/lib/design/tokens.css` (CSS custom properties for the HTML overlay layer). The TS object is canonical; the CSS file mirrors it.
- **First consumer:** `src/lib/Canvas.svelte` — the walking skeleton, upgraded from greybox to the styleguide baseline (orbit). `canvas-007-project-as-frame` is the consumer of the §3.6–3.8 additions.
- **Theme:** dark mode is the default; an optional light theme is available via the viewport top-right toggle (design-system-004). The active theme persists across sessions in the v5 SQLite `preferences (key, value)` table (ADR-004), keyed `'theme'`; flipping fires a `PreferenceChanged` domain event so the PixiJS canvas re-renders against the newly-active `tokens.ts` palette and the HTML overlay layer re-resolves CSS vars under `[data-theme="light"]`.
- **References (sketches for sign-off):** `references/` — ASCII / pseudocode sketches that fix the visual vocabulary without prejudging pixel-true layout.

Structurally analogous to `infrastructure` — both own globally-true foundation — but kept separate because the actors and the review process differ. The design system's questions are *visual* and *experiential*, the infrastructure's are *technical*.

## Classification

**Supporting.** Not GUPPI's reason to exist (the canvas's *behavior* is core; the canvas's *look* is supporting), but the entire product is frontend, so the design system is a hard prerequisite for every frontend task.

## Frontend gate (critical rule for `model` and `work`)

**Every frontend task in any BC must `depends_on` this BC's styleguide task.** No BC's UI work is promoted to `doing/` before the styleguide is signed off by Marco. Each frontend-bearing BC's README notes this rule.

## Ubiquitous language

- **Token** — a primitive design value (a colour, a spacing unit, a font weight). Lives in `src/lib/design/tokens.ts` (canonical) and `tokens.css` (mirror).
- **Component** — a reusable visual element with defined states (project tile, BC node, edge, status badge, voice-state indicator; plus project frame, BC bubble inside frame, intra-project edge variants — design-system-002).
- **Pattern** — a recurring interaction or layout that combines components and tokens (focus/hover affordance, camera affordances, greybox baseline).
- **State** — discrete visual mode of a component. The **status palette** has four: `idle` / `running` / `blocked` / `missing`, each a colour **and** a glyph.
- **Affordance** — the visual cue that something is interactive (focus ring, hover ring, keyboard hint).
- **Status palette** — the four-state, colourblind-friendly set of colours+glyphs a BC node displays. Hue *and* lightness differ; a glyph always accompanies the colour.
- **Voice-state indicator** — the single ambient screen-space glyph (`idle` / `listening` / `muted`) showing mic state without intruding.
- **Greybox** — placeholder UI used before the styleguide is signed off. The walking skeleton shipped greybox; it has been **superseded** by the styleguide baseline and no longer exists in `Canvas.svelte`. Downstream tasks migrate from `STYLEGUIDE.md`, not from greybox.
- **Frame** *(design-system-002)* — the surrounding region drawn around a project; replaces the orbit baseline's project tile + project→BC line. Has a header bar (drag handle, right-click target, project name + status + counts), a body containing BC bubbles, and a 1px border. Auto-fits its content.
- **BC bubble (inside frame)** *(design-system-002)* — denser variant of the BC node, rendered inside a frame. Title + counts pill in one row at default zoom; status badge in the corner.
- **Intra-project edge** *(design-system-002; superseded for the canvas interior in design-system-006)* — a line drawn between two BC bubbles inside the same frame, styled by their context-map relationship: `upstream` (customer-supplier, arrowhead), `mutual` (shared-kernel / partnership, no arrowhead), `ACL` (anti-corruption-layer, arrowhead + midpoint triangle notch), `conformist` (lighter weight, arrowhead). Single neutral `hairlineStrong` palette; geometry distinguishes types. The canvas pivot (`canvas-019` / ADR-017) replaced the in-frame BC bubble + edge graph with the BC accordion row + kanban interior; the edge vocabulary is preserved (STYLEGUIDE §3.7/§3.8 annotated, not deleted) for a possible future cross-project relationship view.
- **BC accordion row** *(design-system-006)* — the canvas-pivot replacement for the in-frame BC bubble: a collapsible row per bounded context inside a project frame. Collapsed = a header band (disclosure chevron, BC name, per-state roll-up pills, total task count); expanded reveals the BC's kanban board. STYLEGUIDE §3.9.
- **Kanban board / column** *(design-system-006)* — inside an expanded accordion row, a horizontally-scrollable strip of four lifecycle columns `BACKLOG · TODO · DOING · DONE`, each holding a vertical stack of task cards. STYLEGUIDE §3.10.
- **Task card** *(design-system-006)* — the per-task tile inside a kanban column: id + title (2-line clamp) + tag chips + (when in `DOING`) a live-agent indicator line ("orchestrator · waiting 2m 14s"). States: default / hover / selected (its detail panel is open) / blocked (red accent + blocked agent line). Reuses the §2.2 status palette. STYLEGUIDE §3.11.
- **Live-agent indicator** *(design-system-006)* — the line on a `DOING` task card showing the attached agent + elapsed; brand-blue + `▶` (running, may pulse) or status-red + `◆` (blocked, static). Consumed by `agent-awareness-002`.
- **Docked detail panel** *(design-system-006)* — the right-edge, screen-space panel that opens when a task card is selected: header (task id + status pill + tags + title), reader body (14px Inter / line-height 1.65 — the canvas-009 reader pins), and — only for a blocked task — the **AGENT NEEDS AN ANSWER** callout (warm brand-orange box with answer/defer/edit buttons). Slides in/out over `durationPanel` (320ms) with the dedicated `easePanel` curve. STYLEGUIDE §3.12.
- **Theme** *(design-system-004)* — the active palette: `dark` (default) or `light`. Two `tokens.ts` palette objects (`colorDark`, `colorLight`) feed a single mutable `color` export; `applyPalette(theme)` mutates that export in place. The `<html data-theme="light">` attribute flips the HTML overlay layer's CSS custom properties simultaneously. Persisted in SQLite via the v5 `preferences` table (key `'theme'`), changes broadcast via the `PreferenceChanged` domain event (ADR-009).

## Upstream dependencies

- `infrastructure` — the chosen frontend framework determines how tokens/components are expressed (CSS variables, Svelte components, etc.).

## Downstream consumers

- `canvas` — every visual element on the canvas references design-system tokens and components.
- Every other BC that grows a frontend later (likely none beyond canvas in v1.5+; voice indicators may live within canvas).

## Open questions

- Light mode — **resolved** in `design-system-004-light-theme`: optional light palette landed in `tokens.ts` (`colorLight`) and `tokens.css` (`[data-theme="light"]`); active theme persisted in SQLite (v5 `preferences` table) and flipped via the viewport top-right toggle; canvas re-renders on flip, HTML overlay re-resolves CSS vars automatically.
- Tile visual hierarchy (project vs BC) — **resolved** in STYLEGUIDE.md §3.1/§3.2: warm brand-orange tile/frame, cool brand-blue BC.
- Status palette — **resolved** in STYLEGUIDE.md §2.2: four states with brand-coherent hues (`idle` grey, `running` brand blue, `blocked` red, `missing` brand orange), each paired with a distinct glyph; colourblind-safe by geometry.
