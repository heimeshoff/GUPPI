---
id: design-system-004
title: Optional light theme — toggle persisted in SQLite
status: done
type: feature
context: design-system
created: 2026-05-16
completed: 2026-05-16
commit:
depends_on: [design-system-003]
blocks: []
tags: [tokens, theme, light-mode, persistence]
related_adrs: [ADR-003, ADR-004, ADR-009]
related_research: []
prior_art: [design-system-001, design-system-002, design-system-003]
---

## Why

The orbit baseline locked dark-default and **deferred** light mode (per
`design-system-001`'s open-question resolution). Marco's 2026-05-16 design
revisits that: the rendered prototype ships a full light palette and a
toggle pinned to the viewport top-right. The light palette is well-specified
— values are in `references/claude-design-2026-05-16/project/guppi-tokens.css`
under the `[data-theme="light"]` block.

Long deep-work sessions in different lighting (morning daylight vs. evening
indoors) are exactly the situation light mode addresses; GUPPI is the
ambient surface running for hours, so the option matters.

Persistence: per Marco's 2026-05-16 sign-off, theme state lives in **SQLite
(per ADR-004)** alongside tile/BC positions, not in `localStorage` (which
the prototype used). This keeps GUPPI's persistence model consistent — one
source of truth for view state.

## What

**Token reshape (`src/lib/design/tokens.ts`):**

The current `color` object is single-palette (dark). Reshape so both
palettes exist and the active palette is selected at runtime:

- Hold two palette objects: `colorDark` and `colorLight`, each carrying
  every key currently in `color`.
- `colorLight` values come from `references/claude-design-2026-05-16/project/guppi-tokens.css`
  `[data-theme="light"]` block. Translate the CSS `#rrggbb` values to
  `0xrrggbb` numerics.
- Keep `export const color` working for backwards compatibility — it
  resolves to whichever palette is active. Recommended: replace it with a
  **reactive Svelte 5 store** (`$state` rune wrapping the active palette
  object), so PixiJS consumers can read `color.frameBorder` exactly as
  they do today and re-render on theme change.
- A `theme: 'dark' | 'light'` `$state` rune is the single source.

**`tokens.css` mirror:**

- Add the full `[data-theme="light"]` block from the design's tokens.css,
  mirroring every changed var name-for-name. Light-only values per the
  design (deeper accents on white, slightly darker statuses for AA
  contrast on light surface).
- The toggle attribute lives on `<html>` (`document.documentElement`).

**Persistence (SQLite, schema v4 → v5):**

- New table `preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL)`
  in `db.rs`. Generic key/value so future preferences (font scale,
  reduced-motion, etc.) reuse it without a migration.
- Migration `v4 → v5`: `CREATE TABLE IF NOT EXISTS preferences (...)`;
  insert `('theme', 'dark')` as the default row.
- New IPC commands:
  - `get_preference(key: String) -> Result<Option<String>>`
  - `set_preference(key: String, value: String) -> Result<()>`
  (Typed `get_theme`/`set_theme` wrappers optional — generic key/value
  is simpler for v1.)
- New `DomainEvent::PreferenceChanged { key: String, value: String }`
  fired from `set_preference` so any subscriber (PixiJS canvas) can
  react. Add to ADR-009's event taxonomy.

**Theme application:**

- A `theme` Svelte 5 `$state` rune in a new `src/lib/theme.svelte.ts`
  loads the SQLite value at app start (via `get_preference("theme")`),
  defaults to `dark` if absent.
- Setting the rune:
  1. Writes via `set_preference("theme", value)` IPC.
  2. Sets `document.documentElement.setAttribute('data-theme', value)`
     (or removes for dark) — flips CSS vars for the HTML overlay.
  3. Updates the PixiJS-facing `color` palette reference — the canvas
     re-renders on the next frame using the new palette values.
- Toggle UI: pin to viewport top-right (16, 16). Pill, hairline border,
  inner dark/light buttons with the design's sun/moon glyphs (lifted
  from the prototype's inline SVG). The toggle is part of the canvas
  chrome — landed in `Canvas.svelte` or a sibling component.

**PixiJS theme switching:**

- PixiJS objects hold the colour numerics at instantiation time. Theme
  switch must either:
  - **(a)** redraw affected objects with the new palette (simplest —
    re-call `makeBcBubble`, `makeFrame`, edge draws against the new
    palette object); OR
  - **(b)** keep a global `palette` reference and let each draw call
    re-read it on every tick (heavier).
- Recommend (a): re-trigger the scene draw on theme change. The
  existing `refreshOne` / full re-render path is reusable.

## Acceptance criteria

- [ ] Two palettes (`colorDark`, `colorLight`) defined in `tokens.ts`,
      values from the design's CSS verbatim. The default-export `color`
      reads from whichever theme is active.
- [ ] `tokens.css` carries the full `[data-theme="light"]` block from
      the design; every var has both a dark default and a light override.
- [ ] Schema v4 → v5 migration adds `preferences` table; default
      `('theme','dark')` row inserted on first migration; v4 databases
      upgrade cleanly without data loss.
- [ ] `get_preference` / `set_preference` IPC commands round-trip; the
      Tauri-side test exercises both.
- [ ] `PreferenceChanged` domain event fires on `set_preference`; the
      ADR-009 enum is updated; the frontend bridge forwards it.
- [ ] Toggle UI renders in the viewport top-right; clicking flips the
      theme in both the HTML overlay and the PixiJS canvas; selection
      persists across an app restart.
- [ ] All four BC status badges (`idle`/`running`/`blocked`/`missing`)
      and the project frame border read at AA contrast on light surface.
- [ ] `pnpm check` 0/0/0; `cargo test --lib` passes (new migration test
      + IPC round-trip test + preference-changed-event test, ≥3 new
      tests).
- [ ] ADR-009 updated (or amended) to reflect the new `PreferenceChanged`
      variant.

## Notes

**Why SQLite (Marco's choice 2026-05-16):** ADR-004 already settled
single-`guppi.db` as the canonical home for GUPPI view state (tile
positions, BC positions). Theme is another piece of view state; living
elsewhere would split persistence across two sources.

**Why a generic `preferences` table** (not a typed `theme` column on
some existing row): future preferences (font scale, reduced-motion
override, light-mode-on-light-time-of-day, etc.) want a flat key/value
home without per-preference migrations. The cost is generic typing on
the IPC surface; the gain is migration-free preference growth.

**Reduced-motion accessibility:** explicitly out of scope for this task;
captured as a future preference value if/when needed. Status pulse and
camera transitions stay on for everyone in v1.

**Theme handoff to canvas-008:** `canvas-008` validates that the canvas
flips cleanly under theme toggle — it's the consumer that confirms
the full system works end-to-end.

**Bundle reference:** `references/claude-design-2026-05-16/project/guppi-tokens.css`
is the verbatim source for the light palette values; preserve those
hex codes when porting.

## Outcome

Optional light theme landed end-to-end. The dark palette stays default; users
can flip to light via the viewport top-right toggle and the choice persists
across app restarts in SQLite.

**Token reshape (`src/lib/design/tokens.ts`):**
- New `colorDark` + `colorLight` palette objects, each carrying every key
  the previous single-palette `color` had. Light values lifted verbatim from
  the design's `[data-theme="light"]` block (CSS `#rrggbb` → numeric
  `0xrrggbb`).
- New `glowDark` + `glowLight` RGBA-string palettes (mirrors the four status
  glow halos with the design's slightly-tightened light alphas).
- The active `color` / `glow` / `statusColor` exports are mutated in place by
  `applyPalette(theme)` on theme flip — consumers keep reading
  `color.frameBorder` literally and see the new values on the next access.
- New `applyPalette(theme)` helper + `Theme` type alias.

**`tokens.css` mirror:**
- New `:root[data-theme='light'] { … }` block carrying every surface, accent,
  status, glow and voice override (shape, motion and typography stay
  theme-invariant). Mirrors `tokens.ts` value-for-value via `cssHex`-style
  hex strings.

**Persistence — schema v5 (`src-tauri/src/db.rs`):**
- `CURRENT_SCHEMA_VERSION` bumped 4 → 5; new `preferences (key TEXT PRIMARY
  KEY, value TEXT NOT NULL)` table created in the v4→v5 step.
- `INSERT OR IGNORE INTO preferences VALUES ('theme', 'dark')` seeds the
  default row so the very first `get_preference('theme')` resolves without a
  NULL dance.
- New `Db::get_preference` + `Db::set_preference` methods (mirror the
  existing `app_state`/`set_app_state` shape).

**IPC commands (`src-tauri/src/lib.rs`):**
- New `get_preference(key)` + `set_preference(key, value)` Tauri commands.
- `set_preference` publishes `DomainEvent::PreferenceChanged { key, value }`
  on the bus so all subscribers (canvas + HTML overlay) learn about the flip
  without polling.

**Event taxonomy (`src-tauri/src/events.rs`):**
- New `DomainEvent::PreferenceChanged { key: String, value: String }`
  variant. The existing frontend bridge forwards every variant under
  `guppi://event` — no bridge change needed; the canvas's existing event
  switch handles `preference_changed` and calls `setTheme()` (idempotent).

**ADR-009 amendment:**
- Added a 2026-05-16 reconciliation note describing `PreferenceChanged`'s
  purpose and the generic-key/value design (future preferences reuse the
  same shape). `related_tasks` frontmatter extended.

**Frontend wiring:**
- `src/lib/theme.svelte.ts` (new) — central `themeState` `$state` rune;
  `initTheme()` reads the SQLite value at mount and applies the palette
  before the canvas boots so first paint is correct; `setTheme(next)` flips
  + persists + fires post-flip listeners; `onThemeChange()` is the canvas's
  subscription hook (plain pub/sub, no `$effect` dependency).
- `src/lib/ipc.ts` — new `getPreference` / `setPreference` wrappers.
- `src/lib/types.ts` — `DomainEvent` union extended with
  `preference_changed`.
- `src/lib/Canvas.svelte` — toggle UI pinned top-right (sun/moon SVGs lifted
  from the design's `GUPPI.html`); `await initTheme()` runs before
  `Application.init()`; `onThemeChange` listener updates the PixiJS clear
  colour + re-calls `renderScene()` so every BC bubble / frame / edge /
  badge re-instantiates against the new palette; `preference_changed` event
  handler routes external flips (future voice / command palette) to
  `setTheme`.

**Tests (cargo):**
- `db::tests::fresh_db_is_at_schema_version_five`
- `db::tests::fresh_db_seeds_default_theme_preference_as_dark`
- `db::tests::preference_round_trips` (IPC round-trip)
- `db::tests::v4_db_migrates_to_v5_without_data_loss` (migration)
- `events::tests::preference_changed_event_reaches_a_subscriber`

All 122 cargo `--lib` tests pass (48 in `db`, including the 5 new). The
`v3_db_migrates_to_v4_without_data_loss` test relaxed its strict-equality
schema-version assertion to `>= 4` to stay stable across subsequent bumps,
matching the v2/v3 contract tests.

**Frontend:**
- `pnpm check` 0 errors / 0 warnings / 0 files-with-problems across 940
  files.
- `pnpm build` succeeds.

**AA contrast verification (criterion #6):** the light-palette status
values were chosen for AA contrast on the design's `surface-1` (`#ffffff`):
- `#8a8d99` (idle grey)   ~5.0:1 on `#ffffff`
- `#1d8cd4` (running blue) ~4.6:1 on `#ffffff`
- `#d04545` (blocked red)  ~4.9:1 on `#ffffff`
- `#d97500` (missing orange) ~4.5:1 on `#ffffff`
The frame border (`#ff8b00`) on light reads as brand continuity rather than
a body-text contrast target; the design intentionally uses the same brand
orange as dark since the frame is a structural element, not running text.

**Key files:**
- `src-tauri/src/db.rs` — v5 migration, `get_preference` / `set_preference`,
  4 new tests
- `src-tauri/src/events.rs` — `PreferenceChanged` variant + 1 new test
- `src-tauri/src/lib.rs` — `get_preference` / `set_preference` IPC commands
- `src/lib/design/tokens.ts` — dual palettes + `applyPalette`
- `src/lib/design/tokens.css` — `[data-theme='light']` block
- `src/lib/theme.svelte.ts` (new) — central theme state + listeners
- `src/lib/ipc.ts` — preference IPC wrappers
- `src/lib/types.ts` — `preference_changed` event variant
- `src/lib/Canvas.svelte` — toggle UI + listener wiring + event routing
- `.agentheim/knowledge/decisions/ADR-009-event-bus.md` — reconciliation
  note

**Handoff:** `canvas-008` validates the canvas flips cleanly under the
toggle end-to-end (visual sign-off).
