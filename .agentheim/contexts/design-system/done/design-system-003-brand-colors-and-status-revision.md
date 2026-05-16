---
id: design-system-003
title: Brand colors + status palette + v1 dimensional refinement
status: done
type: feature
context: design-system
created: 2026-05-16
completed: 2026-05-16
commit:
depends_on: []
blocks: []
tags: [tokens, styleguide, branding, status-palette]
related_adrs: [ADR-003]
related_research: []
prior_art: [design-system-001, design-system-002]
---

## Why

Marco generated a full visual design at `claude.ai/design` (handoff bundle
saved to `references/claude-design-2026-05-16/`). Two business decisions
emerged from the chat transcript that override the orbit-baseline defaults
shipped by `design-system-001`:

1. **Brand accents** — replace the periwinkle (`#8a8ad0`) / teal (`#3c8b8e`)
   placeholder pair with Marco's business colors: **orange `#ff8b00`** (warm
   anchor — project frame border + focus ring + `missing` status) and
   **blue `#25abfe`** (cool secondary — BC border + `running` status).
2. **Status palette** — the four-state colour semantics shift:
   - `idle` → grey `#7b7c8a` ○ (was grey-blue)
   - `running` → blue `#25abfe` ▶ (was a different blue)
   - `blocked` → red `#e85454` ◆ (was amber)
   - `missing` → orange `#ff8b00` ✕ (was magenta-pink)

The dimensional defaults shipped by `design-system-002` are also refined
in this pass — the design fixes BC-bubble dimensions to **188×60** (current
160×56), header bar to **36px** (current 24), and pins an explicit edge
color **`#3a3b46`** (current `fgMuted` `#6e6e80` is meant for muted text;
hairline-strong is a distinct concept).

This is foundational — every visual in GUPPI references these tokens, so
this task lands first, then `design-system-004` (light theme) layers on top,
then `canvas-008` re-validates the canvas against the revised tokens.

## What

**Token updates (`src/lib/design/tokens.ts` — canonical):**

| Token | Current | New | Notes |
|---|---|---|---|
| `color.tileBorder` | `0x8a8ad0` | `0xff8b00` | Project tile (orbit baseline — still referenced by legacy renderer) |
| `color.frameBorder` | `0x8a8ad0` | `0xff8b00` | Project frame (canvas-007) |
| `color.frameHeaderDivider` | `0x8a8ad0` | (new) `0x2a2b35` hairline | Header underline reads as boundary, not accent |
| `color.bcBorder` | `0x3c8b8e` | `0x25abfe` | Orbit BC |
| `color.bcInsideBorder` | `0x3c8b8e` | `0x25abfe` | Inside-frame BC bubble |
| `color.focusRing` | `0xc8c8ff` | `0xffb05a` | Warm focus ring matches frame accent |
| `color.statusIdle` | `0x6f7585` | `0x7b7c8a` | Plain grey, slightly warmer |
| `color.statusRunning` | `0x2f9fe0` | `0x25abfe` | Brand blue |
| `color.statusBlocked` | `0xe6a020` | `0xe85454` | Red (was amber) |
| `color.statusMissing` | `0xd05a8a` | `0xff8b00` | Brand orange (was magenta) |
| `color.voiceListening` | `0x2f9fe0` | `0x25abfe` | Matches new `statusRunning` |
| `color.voiceMuted` | `0xd05a8a` | `0x6e6e80` (`fgMuted`) | Muted slash on neutral; voice indicator design (`references/claude-design-2026-05-16/project/guppi-voice-views.jsx` §4) renders the slash on `fg-2` |
| **NEW** `color.hairlineStrong` | — | `0x3a3b46` | Edge color + table dividers; distinct from existing `fgMuted` muted text |
| `color.edgeUpstream` / `edgeMutual` / `edgeACL` / `edgeConformist` | `0x6e6e80` (`fgMuted`) | `color.hairlineStrong` (`0x3a3b46`) | All four edge variants point at the new `hairlineStrong` |
| `shape.bcInsideWidth` | `160` | `188` | |
| `shape.bcInsideHeight` | `56` | `60` | |
| `shape.frameHeaderHeight` | `24` | `36` | Design specifies a 36px-tall header bar accommodating drag handle dots + project name + per-state mini badges + task-count row |
| **NEW** `color.statusIdleGlow` etc. | — | `rgba(...,.18)` per design | Optional — used by `running` pulse and badge glow ring; capture as RGBA strings or extend Pixi-numeric tokens |

**`tokens.css` mirror updates:**

- Mirror every new/changed `color.*` and `shape.*` token as a CSS custom
  property under the existing `:root` block.
- Existing var names are preserved; only their *values* change. (Light-theme
  variants land in `design-system-004` — do NOT add the `[data-theme="light"]`
  block here.)

**STYLEGUIDE.md updates:**

- §3.1–§3.5 (orbit baseline): swap example hex codes in the body text where
  they reference the old periwinkle/teal/amber/magenta values.
- §3.6 (Project frame): update `frameBorder` example to brand orange; update
  `frameHeaderHeight` to 36.
- §3.7 (BC bubble inside frame): update `bcInsideBorder` to brand blue;
  update `bcInsideWidth/Height` to 188/60.
- §3.8 (Intra-project edges): note that all four variants use the new
  `hairlineStrong` token (was `fgMuted`); reaffirm geometry-not-hue as the
  type-distinguishing principle.
- §3.x (Status palette): rewrite the four entries with the new colors
  and the same four glyphs (○ ▶ ◆ ✕). Reaffirm the colorblind-safety
  contract (hue + lightness + glyph; never colour alone).
- §5 (Open-question defaults): add an entry recording the brand-color
  decision with a pointer to `references/claude-design-2026-05-16/`.

**Bundle reference:**

- The handoff bundle at `references/claude-design-2026-05-16/` is the
  source of truth for every value above. The chat transcript at
  `references/claude-design-2026-05-16/chats/chat1.md` records Marco's
  intent. The rendered HTML + JSX modules + `guppi-tokens.css` show the
  design as it was meant to read.

## Acceptance criteria

- [ ] `tokens.ts` updated per the table above; every new value lands on the
      exact token name (or new `hairlineStrong` token created); no token
      *removed* without a comment explaining why.
- [ ] `tokens.css` mirrors every change name-for-name; no orphan CSS var.
- [ ] `STYLEGUIDE.md` sections §3.1–§3.8 + status palette + §5 reflect
      the new values; every example hex matches `tokens.ts`.
- [ ] No inline hex left in `STYLEGUIDE.md` that contradicts `tokens.ts`.
- [ ] `pnpm check` passes (0 errors / 0 warnings / 0 hints).
- [ ] Existing `Canvas.svelte` builds and renders without code changes
      against the revised tokens (the visual is now wrong/right per the
      new palette — verification of the visual lands in `canvas-008`).
- [ ] A short ADR is written **only if** Marco's brand-color choice
      conflicts with any prior styleguide ADR; if not, no ADR — the
      §5 entry suffices (matches the `design-system-002` pattern).

## Notes

**Why this lands before `design-system-004` (light theme):** the light theme
defines a *parallel* palette indexed by theme name. If we land light-theme
support first, then the brand-color update has to be applied in two places
(dark palette + light palette) by the same task — twice the surface, half
the focus. Doing the brand colors first means `design-system-004` can be
narrowly scoped to "make tokens theme-switchable", consuming the revised
dark values as its starting point.

**`canvas-007`'s §3.7 contract still holds** — the pill tokens
(`bcInsidePillFill`, `bcInsidePillHeight`, etc.) are not renamed or removed
by this task. The pill keeps its right-aligned position inside the bubble;
only the bubble's outer dimensions change.

**Pulse token note:** the `g-pulse-running` keyframe in the design uses
`var(--g-status-running-glow)` (an `rgba()` value). The current `motion`
+ `color` tokens don't include glow strings — extend `color` with the
four `statusXGlow` strings OR add a separate `glow` token group. Either
is fine; document the choice in `STYLEGUIDE.md`.

**Voice indicator color** in `tokens.ts` is currently a peer of the status
palette. The design's voice views (`references/claude-design-2026-05-16/project/guppi-voice-views.jsx`)
treat `voiceListening` as identical to `statusRunning` and `voiceMuted` as a
slash-on-neutral. Aligning the values is small (this task), but conceptually
voice's full UI lives in `voice-001-voice-state-indicator-spec` (backlog).

## Outcome

Brand palette + status palette + v1 dimensional refinements landed across
`tokens.ts`, `tokens.css`, and `STYLEGUIDE.md`. Marco's brand orange
`#ff8b00` (warm anchor) and brand blue `#25abfe` (cool secondary) now drive
the project tile/frame borders, BC borders, the focus ring, and double as
the `running` (blue) and `missing` (orange) status hues — status and brand
read as one system. The status palette also picks up the new `blocked`
red `#e85454` and `idle` plain grey `#7b7c8a` from the design.

New token: `color.hairlineStrong` `#3a3b46` — a dedicated edge/divider
hue distinct from `fgMuted` (which now reads only as muted text). All
four intra-project edge variants (`edgeUpstream` / `edgeMutual` /
`edgeACL` / `edgeConformist`) repoint at it; `fgMuted` is no longer
the edge hue.

New token group: `glow` (TypeScript) + matching `--guppi-status-*-glow`
CSS custom properties. Four RGBA strings (one per status), captured as
strings rather than Pixi-numeric so the dual-file contract stays trivial
(see STYLEGUIDE.md §5 Q10 for the rationale and override path).

Dimensional refinements:
- `shape.frameHeaderHeight` 24 → 36 (fits drag-handle dots + project
  name + per-state mini badges + task-count row in one bar).
- `shape.bcInsideWidth` × `shape.bcInsideHeight` 160×56 → 188×60 (more
  breathing room for the BC title + counts-pill row).

The frame-header divider also lost its brand accent — `frameHeaderDivider`
goes from `#8a8ad0` (legacy periwinkle) to `#2a2b35` (quiet hairline) so
the underline reads as a boundary, not as decoration.

STYLEGUIDE.md §2.1 / §2.2 / §2.5 tables and the §3.1 / §3.2 / §3.5 /
§3.6 / §3.7 / §3.8 prose all reflect the new values. §5 picked up two
new entries: **Q9 — Brand-accent palette** (records Marco's decision
and the override path) and **Q10 — Status-glow capture** (records the
RGBA-strings-not-numerics choice). Per acceptance criterion #7, **no
new ADR was written** — Marco's brand-hue selection doesn't conflict
with any prior styleguide ADR; the §5 entry suffices (matching the
`design-system-002` Q4–Q8 pattern).

**Validated:** `pnpm check` 0 errors / 0 warnings / 0 hints across 939
files; `pnpm build` passes (7.34s). Existing `Canvas.svelte` builds and
ships unchanged against the revised tokens — the visual is now wrong/right
per the new palette, and visual re-validation lands in `canvas-008` (the
next downstream task).

**Scope held:** no `Canvas.svelte` change (consumer integration is
canvas-007's done work; visual re-validation is canvas-008's job); no
`[data-theme="light"]` block in `tokens.css` (that's design-system-004);
no rename or removal of the `bcInsidePill*` tokens (canvas-007's §3.7
contract is preserved).

Key files (current):
- Tokens (canonical): `src/lib/design/tokens.ts`
- Tokens (CSS mirror): `src/lib/design/tokens.css`
- Styleguide: `.agentheim/contexts/design-system/STYLEGUIDE.md`
- BC README: `.agentheim/contexts/design-system/README.md` (open-questions section refreshed; light theme moved from "deferred" to "design-system-004")
- Source of truth for hex values: `.agentheim/contexts/design-system/references/claude-design-2026-05-16/project/guppi-tokens.css`
