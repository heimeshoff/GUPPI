# design-system

## Purpose

Frontend infrastructure: the visual language GUPPI uses on its canvas and detail panes. Tokens (color, typography, spacing, motion), components (tile, BC node, edge, status badge, terminal-panel chrome, command palette, voice-state indicator), patterns (focus/hover, zoom-to-fit transitions, ambient mic state), and the review process for keeping all of it coherent.

Structurally analogous to `infrastructure` — both own globally-true foundation — but kept separate because the actors and the review process differ. The design system's questions are *visual* and *experiential*, the infrastructure's are *technical*.

## Classification

**Supporting.** Not GUPPI's reason to exist (the canvas's *behavior* is core; the canvas's *look* is supporting), but the entire product is frontend, so the design system is a hard prerequisite for every frontend task.

## Frontend gate (critical rule for `model` and `work`)

**Every frontend task in any BC must `depends_on` this BC's styleguide task.** No BC's UI work is promoted to `doing/` before the styleguide is signed off by Marco. Each frontend-bearing BC's README notes this rule.

## Ubiquitous language (seed)

- **Token** — a primitive design value (a colour, a spacing unit, a font weight).
- **Component** — a reusable visual element with defined states.
- **Pattern** — a recurring interaction or layout that combines components and tokens.
- **State** — discrete visual mode of a component (idle / running / blocked-on-question / missing for a tile).
- **Affordance** — the visual cue that something is interactive (cursor, glow, focus ring).
- **Greybox** — placeholder UI used before the styleguide is signed off; the walking-skeleton may ship in greybox.

## Upstream dependencies

- `infrastructure` — the chosen frontend framework determines how tokens/components are expressed (CSS variables, Svelte components, etc.).

## Downstream consumers

- `canvas` — every visual element on the canvas references design-system tokens and components.
- Every other BC that grows a frontend later (likely none beyond canvas in v1.5+; voice indicators may live within canvas).

## Open questions

- Light mode is optional per the architect's draft styleguide note — confirm with Marco during the styleguide task.
- Tile visual hierarchy (project vs BC) — TBD in styleguide.
- Status palette — must be colorblind-friendly per architect; confirm Marco's preference.
