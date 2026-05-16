---
id: design-system-005
title: Document the modal-backdrop token in STYLEGUIDE.md
status: backlog
type: feature
context: design-system
created: 2026-05-16
completed:
commit:
depends_on: []
blocks: []
tags: [styleguide, tokens, modal, documentation]
related_adrs: [ADR-003]
related_research: []
prior_art: [design-system-001, design-system-004]
---

## Why

`canvas-008-apply-revised-tokens`'s light-mode audit surfaced
`src/lib/Modal.svelte`'s previously-inline `rgb(22 22 28 / 70%)` backdrop
as a stale dark-theme literal that did not flip in light mode. The fix
landed a new theme-invariant token:

- `tokens.css` → `--guppi-modal-backdrop: rgba(10, 10, 14, 0.62);`
- `tokens.ts` → `export const modalBackdrop = 'rgba(10, 10, 14, 0.62)';`

The value matches the 2026-05-16 design's "modal scrim" reference
(`references/claude-design-2026-05-16/project/GUPPI.html` §5 footnote).
Single-valued across themes because the design intends the scrim to read
as "scrimmed" regardless of which canvas backdrop is underneath.

The token exists in both contract files, but **`STYLEGUIDE.md` does not
yet document it**. A future worker reading the styleguide as the canon
won't know this token exists — which violates the styleguide rule
"every visual value has a tokens home" by omission.

## What

- Add a `Modal backdrop` row to `STYLEGUIDE.md` §2.1 colour table, under
  a new "Affordance" or "Overlay" sub-grouping (whichever fits the
  surrounding table best — the existing `focusRing` row is the closest
  neighbour). Note that the token is **theme-invariant** (no light-mode
  override in the §2.1 light block) so the styleguide is explicit about
  why the light theme doesn't re-bind it.
- Optionally add a short §3 sub-section ("Modal — *implemented*") naming
  `src/lib/Modal.svelte` as the consumer, with the backdrop dim called
  out as the visual. This is where the styleguide documents component
  states; the four existing modal consumers (discovery checklist,
  scan-roots management, cascade-remove confirmation) all share this
  primitive.

## Acceptance criteria

- [ ] `STYLEGUIDE.md` §2.1 lists `modalBackdrop` / `--guppi-modal-backdrop`
      with the value `rgba(10, 10, 14, 0.62)` and a one-line use.
- [ ] The "theme-invariant" qualifier is explicit (so a future light-mode
      regression doesn't add a `[data-theme="light"]` override by reflex).
- [ ] (Optional) §3 has a `Modal` row pointing at `src/lib/Modal.svelte`.
- [ ] No code change required — this is a documentation pass against
      tokens already in `tokens.ts` + `tokens.css`.

## Notes

This is the **kind** of task that the canvas-008 audit was designed to
surface: a "tokens defined but contract doc out of date" gap that the
type-checker is blind to. The fix is small and contained; pulling it
into a separate task keeps canvas-008's scope on the canvas BC and
respects the design-system BC's authorship over its styleguide doc.

**Bundle reference:** the design's `GUPPI.html` §5 footnote
"modal scrim · rgba(10,10,14,0.62) over canvas" is the source.
