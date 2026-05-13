---
id: design-system-001-styleguide
type: feature
status: todo
scope: global
depends_on: [infrastructure-012-walking-skeleton]
---

# Feature: styleguide

The design system for guppi's frontend. Built on top of the walking skeleton (so it's validated against a running app, not in a vacuum).

## Goal

A complete-enough visual vocabulary that any frontend feature task in any BC can be implemented coherently. Lands **before** v1 polish, after the walking skeleton.

## Scope

At minimum, the styleguide must define:

- **Tile visual hierarchy** — project tile vs BC node (size, weight, colour).
- **Edge styles** — project→BC connections; future styles for cross-BC relationships if the context-map introduces them later.
- **Status palette** — idle / running / blocked-on-question / missing. **Colourblind-friendly.**
- **Typography scale** — one family, three sizes is probably enough.
- **Camera affordances** — zoom-to-fit, focus-on-tile transitions, keyboard nav hints.
- **Voice state affordance** — a single ambient indicator (e.g. corner glyph) that shows mic state without being intrusive.
- **Dark mode default**; light mode optional (confirm with Marco).
- **Greybox baseline** — what the canvas should look like before any of the above is applied, so downstream tasks have something explicit to migrate from.

## Acceptance criteria

- [ ] Token set defined (colour, typography, spacing, motion) and expressed in the chosen frontend stack (CSS variables / theme object / whatever ADR-002 lands on).
- [ ] Component states documented for each visual element (tile, BC node, edge, status badge, voice indicator).
- [ ] **Marco has reviewed and signed off on the design system in person.** This is a gate, not just a deliverable.
- [ ] Walking skeleton is upgraded from greybox to the styleguide's baseline as part of this task.
- [ ] BC READMEs for any frontend-bearing BC reference this completed styleguide.

## Critical gate (for `model` and `work`)

**No frontend feature task in any BC is promoted to `doing/` before this task is closed and signed off.** Each frontend-bearing BC's README notes this rule. `model` should fail fast on any frontend task that lacks `depends_on: [design-system-001-styleguide]`.

## Open questions

- Light mode required or optional? (Architect leaned dark-default; confirm.)
- Tile shape (rounded rectangle? circle? something more visually distinctive?) — TBD with Marco.
- Motion budget — how much animation feels right on the canvas (zoom-to-fit, badge pulses for "running" state) vs distracting?
