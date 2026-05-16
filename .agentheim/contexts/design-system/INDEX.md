# design-system — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
*(None yet.)*
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
- [design-system-004-light-theme](todo/design-system-004-light-theme.md) — `type: feature`, depends on `design-system-003`. Optional light theme via `[data-theme="light"]` attribute; full light palette from design's `tokens.css`; theme persisted in SQLite per ADR-004 (schema v4→v5: new `preferences(key,value)` table, `get_preference`/`set_preference` IPC, `PreferenceChanged` event added to ADR-009). PixiJS canvas re-renders on theme change. Toggle UI pinned viewport top-right.
<!-- todo-list:end -->

**Todo count:** 1

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [design-system-003-brand-colors-and-status-revision](done/design-system-003-brand-colors-and-status-revision.md) — `feature` — Brand palette (orange `#ff8b00` warm anchor + blue `#25abfe` cool secondary), revised status palette (grey/blue/red/orange + ○▶◆✕ glyphs), new `hairlineStrong` `#3a3b46` token + four status-glow RGBA strings, and v1 dimensional refinements (`frameHeaderHeight` 24→36, `bcInsideWidth×Height` 160×56→188×60) landed across `tokens.ts`, `tokens.css`, and `STYLEGUIDE.md` (§2.1, §2.2, §2.5, §3.1, §3.2, §3.5, §3.6, §3.7, §3.8, §5 Q9 + Q10). BC README updated. `Canvas.svelte` untouched — visual re-validation is canvas-008's job. `pnpm check` 0/0/0 (939 files); `pnpm build` passes. Verifier: PASS iter 1. (2026-05-16)
- [design-system-002-project-frame-vocabulary](done/design-system-002-project-frame-vocabulary.md) — `feature` — Extended `STYLEGUIDE.md` with three new contract sections (§3.6 Project frame, §3.7 BC bubble inside frame, §3.8 Intra-project edges) and mirrored 17 new colour tokens + 17 new shape tokens across `tokens.ts` and `tokens.css`. Defaults shipped per the styleguide-001 pattern (1px solid frameBorder, integrated header bar at frameHeaderHeight 24, rounded-rectangle BC bubble at radiusBcInside 8, filled-triangle ACL notch at midpoint, single-neutral `fgMuted` edge palette letting geometry distinguish the four relationship types). New `references/project-frame-sketch.md` ASCII layout reference for the sign-off conversation. No `Canvas.svelte` change — `canvas-007` is the consumer. Marco's in-person sign-off Q4–Q8 deferred. `pnpm check` 0/0/0 (938 files). Commit `38f48ab`. (2026-05-16)
- [design-system-001-styleguide](done/design-system-001-styleguide.md) — `type: feature`. GUPPI styleguide — colour/typography/spacing/shape/motion tokens (TS + CSS vars), colourblind-friendly status palette, documented component states; walking-skeleton canvas upgraded greybox → baseline. Open-question defaults chosen; Marco's in-person sign-off + design-skill refinement deferred.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
