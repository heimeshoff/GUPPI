# design-system

## Purpose

Frontend infrastructure: the visual language GUPPI uses on its canvas and detail panes. Tokens (color, typography, spacing, motion), components (tile, BC node, edge, status badge, terminal-panel chrome, command palette, voice-state indicator), patterns (focus/hover, zoom-to-fit transitions, ambient mic state), and the review process for keeping all of it coherent.

## Styleguide

The styleguide is **code-complete** (`design-system-001-styleguide`) — pending Marco's in-person sign-off and his design-skill refinement pass.

- **The styleguide document:** `STYLEGUIDE.md` (this directory) — tokens, component states, patterns, and the resolved open-question defaults.
- **Tokens (source of truth):** `src/lib/design/tokens.ts` (PixiJS-ready numeric values) and `src/lib/design/tokens.css` (CSS custom properties for the HTML overlay layer). The TS object is canonical; the CSS file mirrors it.
- **First consumer:** `src/lib/Canvas.svelte` — the walking skeleton, upgraded from greybox to the styleguide baseline.
- **Theme:** dark mode is the default and only theme shipped now; light mode is deferred and structured to be additive (open question resolved in `STYLEGUIDE.md`).

Structurally analogous to `infrastructure` — both own globally-true foundation — but kept separate because the actors and the review process differ. The design system's questions are *visual* and *experiential*, the infrastructure's are *technical*.

## Classification

**Supporting.** Not GUPPI's reason to exist (the canvas's *behavior* is core; the canvas's *look* is supporting), but the entire product is frontend, so the design system is a hard prerequisite for every frontend task.

## Frontend gate (critical rule for `model` and `work`)

**Every frontend task in any BC must `depends_on` this BC's styleguide task.** No BC's UI work is promoted to `doing/` before the styleguide is signed off by Marco. Each frontend-bearing BC's README notes this rule.

## Ubiquitous language

- **Token** — a primitive design value (a colour, a spacing unit, a font weight). Lives in `src/lib/design/tokens.ts` (canonical) and `tokens.css` (mirror).
- **Component** — a reusable visual element with defined states (project tile, BC node, edge, status badge, voice-state indicator).
- **Pattern** — a recurring interaction or layout that combines components and tokens (focus/hover affordance, camera affordances, greybox baseline).
- **State** — discrete visual mode of a component. The **status palette** has four: `idle` / `running` / `blocked` / `missing`, each a colour **and** a glyph.
- **Affordance** — the visual cue that something is interactive (focus ring, hover ring, keyboard hint).
- **Status palette** — the four-state, colourblind-friendly set of colours+glyphs a BC node displays. Hue *and* lightness differ; a glyph always accompanies the colour.
- **Voice-state indicator** — the single ambient screen-space glyph (`idle` / `listening` / `muted`) showing mic state without intruding.
- **Greybox** — placeholder UI used before the styleguide is signed off. The walking skeleton shipped greybox; it has been **superseded** by the styleguide baseline and no longer exists in `Canvas.svelte`. Downstream tasks migrate from `STYLEGUIDE.md`, not from greybox.

## Upstream dependencies

- `infrastructure` — the chosen frontend framework determines how tokens/components are expressed (CSS variables, Svelte components, etc.).

## Downstream consumers

- `canvas` — every visual element on the canvas references design-system tokens and components.
- Every other BC that grows a frontend later (likely none beyond canvas in v1.5+; voice indicators may live within canvas).

## Open questions

- Light mode is optional per the architect's draft styleguide note — confirm with Marco during the styleguide task.
- Tile visual hierarchy (project vs BC) — TBD in styleguide.
- Status palette — must be colorblind-friendly per architect; confirm Marco's preference.
