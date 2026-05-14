---
id: design-system-001-styleguide
type: feature
status: done
completed: 2026-05-14
commit: b7db68e
scope: global
depends_on: [infrastructure-012-walking-skeleton]
---

# Feature: styleguide

The design system for GUPPI's frontend. Built on top of the walking skeleton (so it's validated against a running app, not in a vacuum).

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

## Outcome

The GUPPI styleguide — the gate for every future frontend feature task — is
**code-complete**. Marco authorised proceeding with sensible defaults for the
three open questions; he refines properly afterward via a dedicated design
skill. The in-person sign-off remains a separate human gate, **deferred by
Marco's explicit instruction** — see "Pending" below.

### Delivered and code-complete

- **Token set** — colour, typography, spacing, shape, and motion tokens, expressed in the ADR-002 stack two ways:
  - `src/lib/design/tokens.ts` — canonical, PixiJS-ready numeric colours + scales + the `statusColor` / `statusGlyph` / `statusLabel` maps and a `cssHex` bridge.
  - `src/lib/design/tokens.css` — the same values as `--guppi-*` CSS custom properties for the HTML overlay layer (ADR-003).
- **Status palette** — four states (`idle` / `running` / `blocked` / `missing`), **colourblind-friendly**: hue *and* lightness differ, and every state pairs with a distinct glyph so colour is never the only signal.
- **Component states documented** — `STYLEGUIDE.md` documents states for the project tile, BC node, edge, status badge, and voice-state indicator, plus the focus/hover and camera-affordance patterns.
- **Walking skeleton upgraded greybox → styleguide baseline** — `src/lib/Canvas.svelte` rewritten to consume the tokens exclusively (no magic numbers): tile visual hierarchy (project tile vs BC node), token-coloured edges, per-BC status badges derived from task counts, the three-size type scale, an eased zoom-to-fit camera affordance (`F` key), a focus-ring hover affordance, and an ambient voice-state indicator (renders the `idle` state — visual contract for the voice BC). `src/lib/camera.svelte.ts` gained `fitTo` / `lerpTo` for eased camera transitions. `src/routes/+page.svelte` chrome reads the tokens.
- **Validated** — `pnpm check` (0 errors / 0 warnings) and `pnpm build` both pass. Greybox no longer exists in the codebase; downstream tasks migrate from `STYLEGUIDE.md`.
- **Design-system BC README updated** — references `STYLEGUIDE.md`, the token files, the dark-mode default, and an expanded ubiquitous-language section.

### Open-question defaults chosen (Marco can override — see `STYLEGUIDE.md` §5)

1. **Light mode required vs optional → optional, deferred; dark-default.** One coherent theme now beats two half-tuned ones; `tokens.css` is structured so light mode is additive later with no consumer changes.
2. **Tile shape → rounded rectangle.** Rectangles carry title + subtitle + badge legibly at small zoom; project-vs-BC distinction is carried by size + border colour, not shape. Single call site (`makeNode`) if Marco wants to change it.
3. **Motion budget → restrained.** Short eased transitions for navigation only; one slow ambient pulse token reserved for the single "running" signal; nothing else moves.

### Pending (not satisfied by this task — by design)

- **Marco's in-person sign-off** — the acceptance-criterion gate. Deferred by Marco's explicit instruction; this task is moved to `done/` with the deliverables complete, but the human sign-off is still required before it is truly "signed off" for the frontend-gate rule.
- **Marco's design-skill refinement pass** — Marco will refine the visual design properly via a dedicated skill; the defaults above are the starting point.
- **Visual confirmation** — `pnpm check` / `pnpm build` pass, but an agent cannot do live GUI interaction. The actual *look* of the baseline canvas (tile hierarchy reading correctly, status colours/glyphs legible at zoom, zoom-to-fit easing feel, voice glyph placement) needs Marco's eyes via `pnpm tauri dev`.
- **Frontend-bearing BC READMEs must reference this styleguide** — the task asks for this, but a worker may not edit other BCs' READMEs. Follow-up for the orchestrator/Marco: update the `canvas` BC README (and any other frontend-bearing BC) to reference `contexts/design-system/STYLEGUIDE.md` and restate the frontend gate.

### Key files

- Styleguide doc: `.agentheim/contexts/design-system/STYLEGUIDE.md`
- Tokens: `src/lib/design/tokens.ts`, `src/lib/design/tokens.css`
- Upgraded frontend: `src/lib/Canvas.svelte`, `src/lib/camera.svelte.ts`, `src/routes/+page.svelte`
- BC README: `.agentheim/contexts/design-system/README.md`
