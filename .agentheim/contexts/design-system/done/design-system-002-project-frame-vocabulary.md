---
id: design-system-002-project-frame-vocabulary
type: feature
status: done
completed: 2026-05-16
commit: 38f48ab
scope: bc
depends_on:
  - design-system-001-styleguide
related_adrs:
  - ADR-003
related_research: []
prior_art:
  - design-system-001-styleguide
---

# Project-as-frame visual vocabulary

## Why

`canvas-007-project-as-frame` swaps the orbit rendering (project = bubble,
BCs orbiting outside) for a containment rendering (project = frame, BCs as
bubbles inside, edges between BCs by relationship type). The styleguide
currently has no vocabulary for: frame, BC-as-interior-bubble, or any of the
four intra-project edge types. canvas-007 cannot ship without those tokens
and component descriptions in `STYLEGUIDE.md`.

This task extends the styleguide with that vocabulary so canvas-007 has a
single source of truth to build against.

## What

### Frame

- New token group `frame.*` covering: border colour, border weight,
  border-radius, header-bar height, header-bar background, header-bar
  typography (project name, status badges, task counts row).
- States: idle, hover, dragging (matches today's tile hover/drag affordance
  vocabulary).
- The frame's title bar is the new drag handle and right-click target;
  styling must read as "click here for project-level actions".
- Empty-frame state (project with zero BCs) — minimal placeholder treatment
  inside the body so the frame doesn't render as an empty box.

### BC bubble (inside-frame variant)

- Distinct from today's orbit-node — likely smaller, denser, with the BC
  name + task counts pill visible at default zoom.
- States: idle, hover, dragging (per-BC drag inside the frame), focus-ring.
- Status badge slot retained (driven by `agent-awareness` later).

### Intra-project edge styles

Four variants, one per relationship type from `project-registry-004`:

| Type | Visual | Direction |
|---|---|---|
| `customer-supplier` (upstream → downstream) | line + arrowhead at downstream end | directional |
| `shared-kernel` / `partnership` (mutual) | line, no arrowhead | non-directional |
| `anti-corruption-layer` | upstream/downstream variant + notch glyph on the line (or hover-label) | directional |
| `conformist` | upstream/downstream variant, lighter weight | directional |
| *(no relationship)* | no edge drawn | n/a |

Tokens: `edgeUpstream`, `edgeMutual`, `edgeACL`, `edgeConformist` —
colour + weight + arrowhead/notch geometry. Hover-on-edge highlight (open
question: highlight related BCs?).

### Motion

- Edge appearance/disappearance — fade in/out per `durationAffordance`
  (120ms) when relationships change via `bc_relationships_changed`.
- Force-directed re-layout animation — out of scope here; canvas-007 owns
  whether the relayout is instant or eased. (Lean: instant, matching the
  "restrained" budget.)

### Open aesthetic questions (Marco's call, in-person sign-off)

- Frame border: solid vs. dashed vs. inset shadow?
- Header bar: integrated (one-piece-with-frame) or attached (sits above)?
- BC bubble shape: same circle as today, or rounded rectangle?
- ACL notch glyph: small triangle on the line, small zig-zag, or hover-only
  label?
- Edge colour: single neutral palette for all types (geometry distinguishes
  them), or each type its own hue?

Default proposals to bring to sign-off:
- Frame: 1px solid `tileBorder` border, no shadow, `border-radius: 12px`,
  header bar integrated (same border, dividing line inside).
- Header bar height: `spacing.xl`; project name `typography.body`,
  task-counts pill row right-aligned.
- BC bubble: rounded rectangle (`border-radius: spacing.sm`), denser than
  today's circle — title + count pill in one row at default zoom.
- ACL: line + small triangle notch at the midpoint, in `edgeUpstream` hue.
- Edge palette: single neutral (`fgMuted`) hue for all types; arrowhead /
  notch / weight do the differentiating. Cleaner read at canvas zoom-out.

## Acceptance criteria

- [ ] `STYLEGUIDE.md` has a new section §3.x "Project frame" defining
      tokens, states, header-bar treatment, and (in pseudocode/markup) the
      empty-frame state.
- [ ] `STYLEGUIDE.md` has a section "BC bubble (inside frame)" with the
      same shape (tokens + states + status-badge slot).
- [ ] `STYLEGUIDE.md` has a section "Intra-project edges" with the four
      relationship-type styles named, plus the no-relationship rule.
- [ ] All new tokens appear in `tokens.ts` and `tokens.css` matching the
      existing styleguide's import-from-`tokens.ts` (canvas / PixiJS) and
      CSS vars (HTML overlay) pattern from styleguide-001.
- [ ] Marco's in-person sign-off on the aesthetic choices recorded in this
      task's done/ note (matches the styleguide-001 sign-off pattern).
- [ ] No code in `Canvas.svelte` changes in this task — purely styleguide
      + tokens. canvas-007 consumes both.

## Scope (in)

- `contexts/design-system/STYLEGUIDE.md` — new sections.
- `tokens.ts` + `tokens.css` (or wherever they currently live — verify via
  styleguide-001's done file) — new token additions.
- A small `references/` markup snippet showing the frame + 2-3 BCs + edges,
  if helpful for sign-off.

## Scope (out)

- Rendering code in `Canvas.svelte` — canvas-007.
- Hover-edge-highlight behaviour decision — surface for Marco's sign-off;
  if "yes", add to styleguide; if "no", document the deferral.
- Animation polish on relayout — canvas-007 decides whether eased or
  instant.

## Notes

- **Frontend gate context:** this task IS the gate-extension for canvas-007.
  No `depends_on canvas-007` — it's upstream.
- **Override path** (per styleguide-001 pattern): if any of Marco's
  sign-off choices land differently, the `STYLEGUIDE.md` is the override
  point and the task records the chosen variant in the done/ note.
- **Why a separate task** (not folded into canvas-007): visual vocabulary
  decisions need standalone sign-off before integration code goes in;
  matches the precedent set by styleguide-001 gating all frontend work.

## Outcome

The project-as-frame visual vocabulary is code-complete and added to the
styleguide. `canvas-007-project-as-frame` now has a single source of
truth to build against, with named tokens for every visual it must
render. Matching the styleguide-001 pattern: defaults are shipped and
the in-person sign-off is a **deferred human gate** — Marco can
override any default via the override paths documented in
`STYLEGUIDE.md` §5.Q4–Q8 without touching consumer code.

### Delivered

- **Tokens** — `frame.*`, `bcInside.*`, `edge{Upstream,Mutual,ACL,Conformist}`,
  `fgMuted`, plus shape tokens (`radiusFrame`, `borderWidthFrame`,
  `frameHeaderHeight`, `framePadding`, `frameMin*`, `bcInside{Width,Height}`,
  `radiusBcInside`, `bcInsidePill*`, `edgeWeight`, `edgeWeightConformist`,
  `arrowheadLength`, `arrowheadWidth`, `aclNotchSize`) added to both
  `src/lib/design/tokens.ts` (canonical, PixiJS-ready numerics) and
  `src/lib/design/tokens.css` (mirrored CSS custom properties).
- **`STYLEGUIDE.md` §3.6 Project frame** — frame body, border, header bar,
  states (default / hover / dragging on header), empty-frame placeholder
  treatment, plus PixiJS pseudocode for the frame draw call.
- **`STYLEGUIDE.md` §3.7 BC bubble (inside frame)** — denser variant of
  the orbit BC node, title + counts pill in one row at default zoom,
  states (default / hover / focus / dragging), per-BC drag persistence
  hooks named (`project-registry-004`'s `save_bc_position`).
- **`STYLEGUIDE.md` §3.8 Intra-project edges** — four relationship-type
  variants, the no-relationship rule (no edge drawn), arrowhead and
  notch geometry, hover-on-edge deferred, motion (fade per
  `durationAffordance`), plus PixiJS pseudocode for the edge draw call.
- **`STYLEGUIDE.md` §5.Q4–Q8** — five aesthetic open questions captured
  with the defaults shipped + override paths, matching the styleguide-001
  pattern.
- **`references/project-frame-sketch.md`** — non-rendering ASCII
  reference showing the frame + four edge types + empty-frame state.
  Useful for the deferred sign-off conversation; not a pixel mock.
- **BC README updated** — `Styleguide` section now distinguishes the
  orbit baseline (§3.1–3.5, design-system-001) from the project-frame
  additions (§3.6–3.8, design-system-002), and the ubiquitous-language
  section gained entries for **Frame**, **BC bubble (inside frame)**,
  and **Intra-project edge** with the four variants.

### Resolved aesthetic defaults (deferred sign-off)

The five open questions from the task brief were resolved to defaults
per `STYLEGUIDE.md` §5:

- **Q4 (frame border):** 1px solid `frameBorder`, no shadow.
- **Q5 (header bar):** integrated; divided from body by
  `frameHeaderDivider` 1px line.
- **Q6 (BC bubble shape):** rounded rectangle, `radiusBcInside` 8 —
  denser than orbit (`radiusBc` 10).
- **Q7 (ACL notch):** filled triangle at midpoint, `aclNotchSize` 10,
  pointing toward upstream.
- **Q8 (edge colour):** single neutral `fgMuted` palette; geometry
  (arrowhead / notch / weight) distinguishes the four types.

Hover-on-edge highlight is the additional deferred question; until
Marco decides, edges have no hover state. The `edgeHighlight` token
already exists if the decision goes "yes".

### Validation

- `pnpm check` — 0 errors / 0 warnings (938 files).
- `pnpm build` — passes.
- No `Canvas.svelte` changes (scope-out respected; canvas-007 owns
  consumer integration).

### Open follow-ups (not satisfied by this task — by design)

- **Marco's in-person sign-off** on §5.Q4–Q8 defaults — deferred human
  gate, matches styleguide-001 pattern. Override path documented per
  default.
- **Hover-on-edge highlight decision** — open question for the same
  sign-off conversation.
- **Visual confirmation via `pnpm tauri dev`** — requires Marco's eyes;
  also requires `canvas-007` to ship before there is anything visual
  to confirm (this task ships vocabulary, not rendering).

### Key files

- Styleguide: `.agentheim/contexts/design-system/STYLEGUIDE.md` (new §3.6, §3.7, §3.8, §5.Q4–Q8; §2.1 + §2.5 tables extended)
- Tokens: `src/lib/design/tokens.ts`, `src/lib/design/tokens.css`
- Reference sketch: `.agentheim/contexts/design-system/references/project-frame-sketch.md`
- BC README: `.agentheim/contexts/design-system/README.md`
