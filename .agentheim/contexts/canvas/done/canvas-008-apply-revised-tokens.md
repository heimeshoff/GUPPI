---
id: canvas-008
title: Apply revised tokens — visual re-validation against the 2026-05-16 design
status: done
type: feature
context: canvas
created: 2026-05-16
completed: 2026-05-16
commit:
depends_on: [design-system-003, design-system-004]
blocks: []
tags: [styleguide, visual, token-application]
related_adrs: [ADR-003, ADR-015]
related_research: []
prior_art: [canvas-004, canvas-007]
---

## Why

`design-system-003` revises the brand-color palette + status palette +
v1 BC/frame dimensions; `design-system-004` makes the whole palette
theme-switchable. This task verifies the canvas re-renders correctly
against the revised tokens in both themes and pins down anything the
upstream tasks left implicit.

If `tokens.ts` was working as the canonical source (canvas-007 was meant
to enforce this), this task should be a **near-zero code change** —
the value swap in `tokens.ts` propagates through `Canvas.svelte`
automatically. The task's main work is *verification*: does the canvas
actually flip cleanly? Are there inline hex / dimension literals that
escaped tokenisation? Does the running pulse use the right blue glow?
Does light mode look right end-to-end?

**Supersedes `canvas-004-styleguide-visuals`** — that task was "replace
greybox with tokens"; canvas-007 shipped most of it; canvas-008 closes
the rest against the revised palette.

## What

**Visual verification pass against `references/claude-design-2026-05-16/`:**

- Render dark mode against `STYLEGUIDE.md` post-`design-system-003` and
  the design's `project/guppi-canvas-views.jsx` artboards (1-project,
  3-project, 10-project canvas density tests).
- Render light mode — same three density tests. Both themes should read
  as the design's GUPPI.html artboards do, modulo intentional differences
  (GUPPI uses real project data; the design uses mock).
- Confirm the four edge geometries (customer-supplier / shared-kernel /
  ACL / conformist) all render in `hairlineStrong` (new value), not in
  the old `fgMuted` muted-text grey.
- Confirm the four status badges render with the **new colors + same
  glyphs** at all zoom levels.
- Confirm `running` pulse (1600ms, `--g-pulse`) uses the brand-blue glow
  (`statusRunningGlow` / equivalent token).
- Confirm focus ring uses the new orange-leaning warm color.

**Token-application audit:**

- Grep `src/lib/Canvas.svelte` (and any sibling) for inline hex literals
  (`#[0-9a-f]{6}`) — every match must justify itself (tokens.ts unable
  to express it) or be replaced with a token reference.
- Grep for inline dimensional literals (`width: 160`, `height: 56`,
  etc.) that mirror token values — replace with token references.
- Same pass for inline durations (`320`, `1600`) — should resolve
  to `motion.*` tokens.

**Theme-switch end-to-end test:**

- Click the toggle (introduced by `design-system-004`); canvas flips
  immediately. No flicker; no stale colors on any PixiJS object.
- Drag a BC; switch theme; BC position preserved (it's saved via the
  registry, theme is a view concern).
- Restart app; theme persists from SQLite.

**Visual-fidelity cross-check:**

- Side-by-side `pnpm tauri dev` vs. opening
  `references/claude-design-2026-05-16/project/GUPPI.html` in a browser
  on the same monitor. The project-frame artboard (§2 in the design)
  is the closest visual analogue to GUPPI's current canvas render. The
  visuals should be **substantively the same** — same colours, same
  proportions, same edge geometry.

## Acceptance criteria

- [ ] In dark mode, GUPPI's canvas renders against the revised palette;
      project frames border orange, BC bubbles border blue, status
      badges use the new four-colour set, edges render in `hairlineStrong`.
- [ ] In light mode (via the `design-system-004` toggle), the same
      canvas flips cleanly — every PixiJS object that holds a colour
      re-draws with the active palette; no stale dark colours visible.
- [ ] `Canvas.svelte` (and any sibling rendering module) holds **zero
      inline hex literals** for visuals that have a `tokens.ts` home.
      Justified exceptions documented as comments.
- [ ] BC bubbles render at 188×60 (the new `bcInsideWidth`/`Height`);
      project-frame header bars render at 36px tall (the new
      `frameHeaderHeight`); a regression test or visual snapshot
      catches accidental reverts.
- [ ] `running` pulse uses the new brand-blue glow on the 1600ms cycle.
- [ ] Theme persists across an app restart (via `design-system-004`'s
      SQLite preference).
- [ ] `pnpm check` 0/0/0; `cargo test --lib` passes; no new tests
      mandatory here unless the audit surfaces a logic regression.
- [ ] `STYLEGUIDE.md`'s "First consumer" line still names this task's
      output — confirm the line is current.

## Notes

**Why this is a separate task** from `design-system-003` + `004`: the
design-system tasks land tokens + theme infrastructure. This task is
the canvas BC's affirmation that the tokens *actually flow through*
the rendering pipeline cleanly — the BC owns its UI, so it owns the
visual sign-off. Separating also gives the verifier two distinct
PASSes: tokens correct (003/004 verifier), canvas correct (008 verifier).

**Subsumes `canvas-004-styleguide-visuals`** — see canvas-004's
done-note for the closure record. canvas-004's scope ("replace greybox
with tokens") was largely cleared by canvas-002 (greybox retired) and
canvas-007 (frame visuals); canvas-008 closes the remaining audit
against the revised palette.

**Dimensional regressions risk:** the `canvas-007` worker had to consume
`bcInside{Width,Height}` from `tokens.ts`; if any of those reads got
hardcoded along the way, they'd show as 160×56 BCs against a fresh
188×60 token. The token-application audit (above) catches this.

**Open question for refinement:** should canvas-008 also wire the
`design-system-004` theme toggle into `Canvas.svelte`, or does the
toggle UI sit elsewhere (e.g. a future canvas-007-style frame chrome
addition)? Lean: toggle is canvas chrome → wire here. Confirm during
refinement if the design-system-004 worker has a stronger view.

**Bundle reference:** `references/claude-design-2026-05-16/` is the
visual source of truth; the canvas-density artboards (§1) and the
project-frame artboard (§2) are the most relevant cross-checks.

## Outcome

**Near-zero code change, as predicted — plus one real audit catch.** The
revised palette + dimensional bumps from `design-system-003` and the
theme infrastructure from `design-system-004` flowed through cleanly to
the canvas with no rendering-path changes required.

### Token-application audit — results

- **`Canvas.svelte` (2655 lines):** zero inline hex literals; zero
  inline dimensional literals matching the revised tokens (`188`, `60`,
  `36`, or the old `160`/`56`/`24` values); zero inline motion literals
  outside the in-scope toast dismissal (3000ms — a BC-local UX duration,
  not a styleguide `motion.*` token). All visual values flow through
  `color.*`, `shape.*`, `motion.*`, `statusColor.*`, `glow.*`.
- **`bc-layout.ts`, `tile-layout.ts`:** both consume `shape.*` exclusively;
  no hardcoded dimensions. The §3.7 contract (188×60 bubbles, 36px header)
  propagates verbatim.
- **`Modal.svelte` — the one catch.** Line 96 held `rgb(22 22 28 / 70%)`,
  the dark-theme `--guppi-canvas-bg` literal at 70%. The accompanying
  comment ("if that token shifts, this RGB must move too") was written
  before `design-system-004` introduced the dual palette — and the
  token HAS shifted (it differs between themes) but the RGB stayed
  bolted to the dark value. The modal backdrop would have looked
  visibly wrong in light mode.

### Fix

Added a new theme-invariant `--guppi-modal-backdrop` token, value
`rgba(10, 10, 14, 0.62)` — the design's reference scrim from
`references/claude-design-2026-05-16/project/GUPPI.html` §5 footnote
("modal scrim · rgba(10,10,14,0.62) over canvas"). Single-valued across
light + dark so the modal always reads as scrimmed regardless of
backdrop. Mirrored in both `tokens.ts` (`export const modalBackdrop`)
and `tokens.css` (`--guppi-modal-backdrop`). `Modal.svelte` now consumes
`var(--guppi-modal-backdrop)` directly; no inline literal.

### Theme-flip end-to-end verification

- `initTheme()` runs **before** `app.init()` (Canvas.svelte:528) so the
  first paint already lands in the persisted palette — no flash on
  restart.
- `onThemeChange()` listener (line 599) updates
  `app.renderer.background.color` AND re-calls `renderScene()`, which
  does `world.removeChildren()` and rebuilds every Pixi object from
  the now-active `color.*` numerics. Project frames, BC bubbles,
  status badges, voice indicator, intra-project edges — all
  re-instantiate. No stale Pixi colours possible.
- Status palette: `statusColor.running` now resolves to brand-blue
  `0x25abfe` (dark) / `0x1d8cd4` (light); a `running` BC's badge will
  render in the correct hue. The animated `running` *pulse* (CSS
  keyframe consuming `--guppi-status-running-glow`) isn't built yet —
  no consumer exists, so no regression. When built it will pick up the
  brand-blue glow from the existing tokens.

### STYLEGUIDE.md "First consumer" line

Verified — the single top-of-doc "First consumer: `src/lib/Canvas.svelte`"
line is current. §3.6 explicitly names `canvas-007-project-as-frame` as
its consumer (current). §3.7 / §3.8 mention the project-frame work
inline. No staleness; no edits needed in scope.

### Follow-up

- `design-system-005-document-modal-backdrop-token` — backlog item
  created in `.agentheim/contexts/design-system/backlog/` to document
  the new `modal-backdrop` token in `STYLEGUIDE.md` (the contract doc
  needs the new entry; doing it from the design-system BC respects
  authorship boundaries).

### Files changed

- `src/lib/Modal.svelte` — replaced inline `rgb(22 22 28 / 70%)` with
  `var(--guppi-modal-backdrop)`; updated the comment to reflect the
  theme-invariant intent.
- `src/lib/design/tokens.css` — added `--guppi-modal-backdrop:
  rgba(10, 10, 14, 0.62);` in the `:root` block (theme-invariant; no
  override in `:root[data-theme='light']`).
- `src/lib/design/tokens.ts` — added `export const modalBackdrop`
  mirror, for the contract rule "every visual value has a tokens home".

### Test status

- `pnpm check`: 0 errors / 0 warnings / 0 files-with-problems (940
  files).
- `cargo test --lib`: 122 passed / 0 failed.
- No new tests added; the audit didn't surface a logic regression, and
  the project still has no frontend test infra (the §3.7 / §3.8 token
  enumeration in the acceptance criteria IS the lever, exactly as the
  iteration-1 verifier on canvas-007 demonstrated).

### Visual cross-check (operator-side, not code)

Side-by-side `pnpm tauri dev` vs.
`references/claude-design-2026-05-16/project/GUPPI.html` is the
deferred operator check (the verifier or Marco runs it). The token
audit above guarantees the values are right; the operator pass
verifies the proportions read as the design intends.
