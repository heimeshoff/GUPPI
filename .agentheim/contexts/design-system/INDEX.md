# design-system — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
- [design-system-006-kanban-accordion-card-and-panel-vocabulary](backlog/design-system-006-kanban-accordion-card-and-panel-vocabulary.md) — `type: feature`. **Kanban-accordion pivot, styleguide GATE.** New `STYLEGUIDE.md` sections + dual-theme tokens (`tokens.ts` + `tokens.css`) for the BC accordion row, kanban board/column, task card (all states + live-agent indicator), and the docked detail panel (callout + slide motion). Annotates §3.7 (BC bubble) + §3.8 (intra-project edges) as superseded for the canvas interior. Blocks canvas-020/021/022 + agent-awareness-002. Created 2026-05-24.
- [design-system-005-document-modal-backdrop-token](backlog/design-system-005-document-modal-backdrop-token.md) — `type: feature`. Documentation-only follow-up from canvas-008. Add `modalBackdrop` / `--guppi-modal-backdrop` to `STYLEGUIDE.md` §2.1 colour table (theme-invariant qualifier explicit), optionally a §3 `Modal` row pointing at `src/lib/Modal.svelte`. Source value: `rgba(10, 10, 14, 0.62)` per the 2026-05-16 design's §5 modal-scrim footnote.
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
*(None.)*
<!-- todo-list:end -->

**Todo count:** 0

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [design-system-004-light-theme](done/design-system-004-light-theme.md) — `feature` — Optional light theme landed end-to-end. Dual palettes (`colorDark` + `colorLight`) in `tokens.ts`; `[data-theme="light"]` block in `tokens.css`. Theme persisted via new v5 SQLite `preferences (key, value)` table (5 new cargo tests including v4→v5 data-loss check). `get_preference` / `set_preference` IPC commands + new `DomainEvent::PreferenceChanged` variant (ADR-009 amended with reconciliation note). New `src/lib/theme.svelte.ts` Svelte 5 rune loads at mount, persists via SQLite. Top-right pill toggle in `Canvas.svelte` flips both HTML overlay (`data-theme` on `<html>`) and PixiJS scene (`renderScene()` on theme change; `app.renderer.background.color` re-set). `pnpm check` 0/0/0 (940 files); `cargo test --lib` 122/122. Verifier: PASS iter 1. (2026-05-16)
- [design-system-003-brand-colors-and-status-revision](done/design-system-003-brand-colors-and-status-revision.md) — `feature` — Brand palette (orange `#ff8b00` warm anchor + blue `#25abfe` cool secondary), revised status palette (grey/blue/red/orange + ○▶◆✕ glyphs), new `hairlineStrong` `#3a3b46` token + four status-glow RGBA strings, and v1 dimensional refinements (`frameHeaderHeight` 24→36, `bcInsideWidth×Height` 160×56→188×60) landed across `tokens.ts`, `tokens.css`, and `STYLEGUIDE.md` (§2.1, §2.2, §2.5, §3.1, §3.2, §3.5, §3.6, §3.7, §3.8, §5 Q9 + Q10). BC README updated. `Canvas.svelte` untouched — visual re-validation is canvas-008's job. `pnpm check` 0/0/0 (939 files); `pnpm build` passes. Verifier: PASS iter 1. (2026-05-16)
- [design-system-002-project-frame-vocabulary](done/design-system-002-project-frame-vocabulary.md) — `feature` — Extended `STYLEGUIDE.md` with three new contract sections (§3.6 Project frame, §3.7 BC bubble inside frame, §3.8 Intra-project edges) and mirrored 17 new colour tokens + 17 new shape tokens across `tokens.ts` and `tokens.css`. Defaults shipped per the styleguide-001 pattern (1px solid frameBorder, integrated header bar at frameHeaderHeight 24, rounded-rectangle BC bubble at radiusBcInside 8, filled-triangle ACL notch at midpoint, single-neutral `fgMuted` edge palette letting geometry distinguish the four relationship types). New `references/project-frame-sketch.md` ASCII layout reference for the sign-off conversation. No `Canvas.svelte` change — `canvas-007` is the consumer. Marco's in-person sign-off Q4–Q8 deferred. `pnpm check` 0/0/0 (938 files). Commit `38f48ab`. (2026-05-16)
- [design-system-001-styleguide](done/design-system-001-styleguide.md) — `type: feature`. GUPPI styleguide — colour/typography/spacing/shape/motion tokens (TS + CSS vars), colourblind-friendly status palette, documented component states; walking-skeleton canvas upgraded greybox → baseline. Open-question defaults chosen; Marco's in-person sign-off + design-skill refinement deferred.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
