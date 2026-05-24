// GUPPI design tokens — the single source of truth for the visual language
// (design-system-001-styleguide; brand+status refresh in design-system-003;
// optional light theme added in design-system-004).
//
// Every frontend feature task references these; nothing in the canvas should
// hard-code a colour, size, or duration.
//
// Why a TypeScript object and not only CSS variables: the canvas renders
// through PixiJS v8 (ADR-003), whose APIs take *numeric* colours (0xrrggbb)
// and numeric sizes. CSS custom properties are strings. So this module is the
// canonical source; `tokens.css` mirrors the same values as CSS custom
// properties for the HTML overlay layer (markdown viewer, command palette,
// terminal-panel chrome — ADR-003's overlay approach).
//
// Convention: `color*` values are PixiJS-ready numbers. `cssHex()` exposes the
// matching CSS hex string for DOM consumers that need to bridge.
//
// ─── Light/dark theme (design-system-004) ───────────────────────────────
//
// `colorDark` / `colorLight` hold the two palette objects. `color` is the
// *active* palette, mutated in place by `applyPalette(theme)` on theme flip
// (Marco's 2026-05-16 design-system-004 sign-off — the light values come
// verbatim from `references/claude-design-2026-05-16/project/guppi-tokens.css`
// `[data-theme="light"]` block, with CSS `#rrggbb` translated to numeric
// `0xrrggbb`).
//
// **Why mutate `color` in place rather than re-bind it?** Consumers across
// the codebase have already imported `color` as a value binding and read
// `color.frameBorder` literally; an import re-bind would never be observed.
// Mutating the object's keys (via `Object.assign`) means every consumer sees
// the new values on the next read — which the PixiJS canvas guarantees by
// re-calling `renderScene()` on theme change (the trigger is the `theme`
// `$state` rune in `src/lib/theme.svelte.ts`).
//
// Same trick for `statusColor` (whose values derive from `color.statusIdle`
// etc.) and `glow` (RGBA strings used by the CSS pulse keyframe).

/* ------------------------------------------------------------------ */
/* Palette type — the shape both `colorDark` and `colorLight` must     */
/* satisfy. Derived from `colorDark` so adding a new key forces both   */
/* palettes to stay in sync (a type error otherwise).                  */
/* ------------------------------------------------------------------ */

/* ------------------------------------------------------------------ */
/* Dark palette — Marco's brand-true 2026-05-16 colours.               */
/* The default-active palette on first run (per the migration's        */
/* `('theme','dark')` seed).                                           */
/* ------------------------------------------------------------------ */

export const colorDark = {
	/** Canvas backdrop — PixiJS `Application` background. */
	canvasBg: 0x16161c,
	/** A faint world grid / vignette tone, if a backdrop texture is added. */
	canvasBgRaised: 0x1e1e26,

	/** Project tile — the visual anchor of the canvas. Warm, high-weight.
	 *  Brand orange anchors the project hierarchy (design-system-003). */
	tileFill: 0x262636,
	tileBorder: 0xff8b00,
	tileText: 0xf2f2f7,
	tileTextMuted: 0xb9b9c8,

	/** Bounded-context node — secondary to the project tile. Cool, calmer.
	 *  Brand blue distinguishes BCs from the warm project hierarchy
	 *  (design-system-003). */
	bcFill: 0x20242b,
	bcBorder: 0x25abfe,
	bcText: 0xe6e6ec,
	bcTextMuted: 0x9a9aa6,

	/** Edges — project → BC connectors (legacy, used by the orbit baseline). */
	edge: 0x4a4a58,
	edgeHighlight: 0xff8b00,

	/**
	 * Neutral muted foreground — used for muted text (placeholder copy,
	 * disabled labels). The four intra-project edge variants no longer
	 * resolve here; they resolve to `hairlineStrong` (design-system-003).
	 */
	fgMuted: 0x6e6e80,

	/**
	 * Hairline-strong — the dedicated edge/divider hue introduced by
	 * design-system-003. Distinct from `fgMuted` (which is muted *text*),
	 * `hairlineStrong` is structural: intra-project edges, table dividers,
	 * subtle boundaries. Slightly darker than `fgMuted` so it recedes
	 * behind type rather than competing with it.
	 */
	hairlineStrong: 0x3a3b46,

	/* --- Project-as-frame (design-system-002) ------------------------ */
	/** Frame body — transparent canvas-bg tone so it reads as containment. */
	frameFill: 0x1a1a22,
	/** Frame border — brand orange (warm anchor). Reads continuous with the
	 *  legacy `tileBorder`; weight differs (see `shape.borderWidthFrame`).
	 *  Updated to brand orange in design-system-003. */
	frameBorder: 0xff8b00,
	/** Frame header bar — slightly raised tone, divides title from body. */
	frameHeaderFill: 0x262636,
	/** The divider line between header bar and frame body. A quiet hairline
	 *  (not the brand accent) so the underline reads as boundary rather than
	 *  decoration. Updated in design-system-003. */
	frameHeaderDivider: 0x2a2b35,
	/** Header bar text — project name + counts. */
	frameTitleText: 0xf2f2f7,
	frameTitleTextMuted: 0xb9b9c8,
	/** Empty-frame placeholder text — "no bounded contexts yet". */
	frameEmptyText: 0x6e6e80,

	/* --- BC bubble (inside-frame variant — design-system-002) -------- */
	bcInsideFill: 0x20242b,
	bcInsideBorder: 0x25abfe,
	bcInsideText: 0xe6e6ec,
	bcInsideTextMuted: 0x9a9aa6,
	bcInsidePillFill: 0x2a2f38,

	/* --- Intra-project edges (design-system-002) --------------------- */
	/** Customer-supplier (directional) — line + arrowhead at downstream end. */
	edgeUpstream: 0x3a3b46,
	/** Mutual / shared-kernel / partnership — line, no arrowhead. */
	edgeMutual: 0x3a3b46,
	/** Anti-corruption-layer (directional) — line + arrowhead + notch glyph. */
	edgeACL: 0x3a3b46,
	/** Conformist (directional) — line + arrowhead, lighter weight. */
	edgeConformist: 0x3a3b46,

	/** Focus / hover affordance — the ring drawn around an interactive node.
	 *  Warm focus ring matches the brand orange `frameBorder`
	 *  (design-system-003). */
	focusRing: 0xffb05a,

	/* --- Status palette (colourblind-friendly) ----------------------- */
	statusIdle: 0x7b7c8a, // plain grey — "nothing happening"
	statusRunning: 0x25abfe, // brand blue — "work in progress"
	statusBlocked: 0xe85454, // red — "blocked on a question"
	statusMissing: 0xff8b00, // brand orange — "expected but absent"

	/** Text colour that sits *on top* of a status fill (badges). */
	statusText: 0x10131a,

	/** Voice-state indicator — the ambient corner glyph. */
	voiceIdle: 0x5a5a68, // mic available, not listening
	voiceListening: 0x25abfe, // actively listening — matches statusRunning blue
	voiceMuted: 0x6e6e80, // mic unavailable / muted — neutral fgMuted

	/* --- Kanban-accordion interior (design-system-006) --------------- */
	/* The DOM/HTML overlay interior introduced by the canvas pivot
	 * (canvas-019 ratified the hybrid Pixi-shell + DOM-interior substrate,
	 * ADR-017). The frame SHELL stays Pixi; this colour group dresses the
	 * accordion rows, kanban columns, task cards, and docked detail panel
	 * that live in the DOM overlay. Mirrors §3.9–3.12 of STYLEGUIDE.md. */

	/** BC accordion row — header band + body when expanded. The row header
	 *  is a quiet raised band; the body (holding the kanban board) drops back
	 *  to the canvas tone so cards read as raised against it. */
	accordionRowFill: 0x1e1e26,
	accordionRowHeaderFill: 0x20242b,
	accordionRowDivider: 0x2a2b35,
	accordionRowText: 0xe6e6ec,
	accordionRowTextMuted: 0x9a9aa6,
	/** Disclosure chevron — the collapse/expand affordance glyph. */
	accordionChevron: 0x9a9aa6,
	/** Roll-up pill background — the neutral capsule behind "1 active · 2
	 *  blocked · 1 idling"; the per-state glyph+count text inside uses the
	 *  matching `statusColor[...]`. */
	accordionRollupPillFill: 0x262636,

	/** Kanban board — the scrollable strip of four columns inside an
	 *  expanded accordion row. */
	kanbanColumnFill: 0x1a1a22,
	kanbanColumnHeaderText: 0x9a9aa6,
	kanbanColumnDivider: 0x2a2b35,
	/** Empty-column placeholder copy. */
	kanbanColumnEmptyText: 0x6e6e80,

	/** Task card — the per-task tile inside a column. Default body + border;
	 *  hover/selected raise the border; blocked swaps to the red accent. */
	cardFill: 0x20242b,
	cardFillHover: 0x262636,
	cardBorder: 0x2a2b35,
	cardBorderSelected: 0x25abfe,
	cardBorderBlocked: 0xe85454,
	cardIdText: 0x6e6e80,
	cardTitleText: 0xe6e6ec,
	cardTagFill: 0x262636,
	cardTagText: 0x9a9aa6,
	/** Live-agent indicator line ("orchestrator · waiting 2m 14s"). Running
	 *  reads brand-blue; blocked reads the status-red. The two are aliases of
	 *  the status palette, named so card code reads intent. */
	cardAgentLineRunning: 0x25abfe,
	cardAgentLineBlocked: 0xe85454,

	/** Docked detail panel — the right-edge panel + its blocked-question
	 *  callout. */
	panelFill: 0x1a1a22,
	panelBorder: 0x2a2b35,
	panelHeaderText: 0xf2f2f7,
	panelHeaderTextMuted: 0x9a9aa6,
	panelBodyText: 0xe6e6ec,
	/** "AGENT NEEDS AN ANSWER" callout — the warm-accent boxed region.
	 *  Tinted fill + brand-orange border + label; the primary answer button
	 *  uses the brand-orange fill, secondary buttons stay quiet. */
	panelCalloutFill: 0x2a211a,
	panelCalloutBorder: 0xff8b00,
	panelCalloutLabel: 0xff8b00,
	panelCalloutText: 0xf2f2f7,
	panelButtonPrimaryFill: 0xff8b00,
	panelButtonPrimaryText: 0x10131a,
	panelButtonSecondaryFill: 0x262636,
	panelButtonSecondaryText: 0xe6e6ec
} as const;

export type Palette = { -readonly [K in keyof typeof colorDark]: number };

/* ------------------------------------------------------------------ */
/* Light palette — design-system-004. Values lifted verbatim from      */
/* `references/claude-design-2026-05-16/project/guppi-tokens.css`'s    */
/* `[data-theme="light"]` block, with `#rrggbb` translated to numeric  */
/* `0xrrggbb` for PixiJS consumers. Slightly darker accents than the   */
/* brand-orange/-blue dark anchors so they read at AA contrast on a    */
/* near-white surface (verified against the design's prototype).       */
/* ------------------------------------------------------------------ */

export const colorLight: Palette = {
	// Surfaces — cool, slightly off-white.
	canvasBg: 0xf4f5f8,
	canvasBgRaised: 0xebedf2,

	// Project tile — same brand-orange border for hierarchy continuity;
	// fill is the design's `surface-1` (white) so the tile reads raised
	// against the near-white canvas backdrop.
	tileFill: 0xffffff,
	tileBorder: 0xff8b00,
	tileText: 0x15161c,
	tileTextMuted: 0x4a4b58,

	// BC node — cool secondary. The light palette deepens brand blue to
	// `#1d8cd4` for AA contrast against the design's `surface-2` BC fill.
	bcFill: 0xf6f7fa,
	bcBorder: 0x1d8cd4,
	bcText: 0x15161c,
	bcTextMuted: 0x4a4b58,

	// Edges (orbit baseline) — neutral mid-grey on white.
	edge: 0xc8cad3,
	edgeHighlight: 0xff8b00,

	// Muted text / structural hairlines.
	fgMuted: 0x6b6c7a,
	hairlineStrong: 0xc8cad3,

	// Project-as-frame: white frame on near-white canvas, with the same
	// brand-orange border as dark. Header tone uses `surface-2` for the
	// raised feel; divider is `hairline` from the design's light block.
	frameFill: 0xffffff,
	frameBorder: 0xff8b00,
	frameHeaderFill: 0xf6f7fa,
	frameHeaderDivider: 0xe2e4eb,
	frameTitleText: 0x15161c,
	frameTitleTextMuted: 0x4a4b58,
	frameEmptyText: 0x6b6c7a,

	// BC bubble (inside-frame variant). Bubble fill = `surface-2`; border =
	// the same deepened brand blue. Pill = `surface-4` for the on-white
	// hovered/raised feel.
	bcInsideFill: 0xf6f7fa,
	bcInsideBorder: 0x1d8cd4,
	bcInsideText: 0x15161c,
	bcInsideTextMuted: 0x4a4b58,
	bcInsidePillFill: 0xebedf2,

	// Intra-project edges — single neutral, geometry distinguishes type.
	// Resolves to the light palette's `hairline-strong` (`#c8cad3`).
	edgeUpstream: 0xc8cad3,
	edgeMutual: 0xc8cad3,
	edgeACL: 0xc8cad3,
	edgeConformist: 0xc8cad3,

	// Focus ring — brand orange on light (the design uses
	// `--g-focus-ring: #ff8b00` in the light block; the warm `#ffb05a` of
	// dark would wash out on white).
	focusRing: 0xff8b00,

	// Status — same hues, slightly darker for AA on white. The four
	// values are sized for AA contrast against the light frame body:
	//   • #8a8d99 (idle grey)   ~5.0:1  on #ffffff
	//   • #1d8cd4 (running blue) ~4.6:1 on #ffffff
	//   • #d04545 (blocked red)  ~4.9:1 on #ffffff
	//   • #d97500 (missing orng) ~4.5:1 on #ffffff
	// Verified against the design's [data-theme="light"] block (acceptance
	// criterion #6 — AA contrast on light surface).
	statusIdle: 0x8a8d99,
	statusRunning: 0x1d8cd4,
	statusBlocked: 0xd04545,
	statusMissing: 0xd97500,

	// Status text sits on top of a status fill (badges). On dark, the
	// fills are mid-saturation against a dark surface, so dark text on
	// the fill reads. On light the fills are the same hues, just deeper,
	// so dark text still reads (we keep the dark glyph colour). Verified
	// in the prototype.
	statusText: 0x10131a,

	// Voice — listening matches `statusRunning` blue; muted is `fgMuted`.
	voiceIdle: 0x8a8d99,
	voiceListening: 0x1d8cd4,
	voiceMuted: 0x6b6c7a,

	// Kanban-accordion interior (design-system-006). On light, the
	// accordion body sits on the near-white canvas; cards are `surface-1`
	// (white) raised against the `surface-2` board, so the same raised/
	// recessed reading holds, inverted. Borders use the light hairlines;
	// blocked/selected accents use the AA-tuned light status hues.
	accordionRowFill: 0xebedf2,
	accordionRowHeaderFill: 0xf6f7fa,
	accordionRowDivider: 0xe2e4eb,
	accordionRowText: 0x15161c,
	accordionRowTextMuted: 0x4a4b58,
	accordionChevron: 0x4a4b58,
	accordionRollupPillFill: 0xffffff,

	kanbanColumnFill: 0xebedf2,
	kanbanColumnHeaderText: 0x4a4b58,
	kanbanColumnDivider: 0xe2e4eb,
	kanbanColumnEmptyText: 0x6b6c7a,

	cardFill: 0xffffff,
	cardFillHover: 0xf6f7fa,
	cardBorder: 0xe2e4eb,
	cardBorderSelected: 0x1d8cd4,
	cardBorderBlocked: 0xd04545,
	cardIdText: 0x6b6c7a,
	cardTitleText: 0x15161c,
	cardTagFill: 0xebedf2,
	cardTagText: 0x4a4b58,
	cardAgentLineRunning: 0x1d8cd4,
	cardAgentLineBlocked: 0xd04545,

	panelFill: 0xffffff,
	panelBorder: 0xe2e4eb,
	panelHeaderText: 0x15161c,
	panelHeaderTextMuted: 0x4a4b58,
	panelBodyText: 0x15161c,
	// Callout — a warm tint on white; brand-orange border + label survive on
	// the near-white surface (the light status-missing #d97500 would muddy
	// the brand label, so the label keeps the brand orange #ff8b00).
	panelCalloutFill: 0xfff3e6,
	panelCalloutBorder: 0xff8b00,
	panelCalloutLabel: 0xd97500,
	panelCalloutText: 0x15161c,
	panelButtonPrimaryFill: 0xff8b00,
	panelButtonPrimaryText: 0x10131a,
	panelButtonSecondaryFill: 0xf6f7fa,
	panelButtonSecondaryText: 0x15161c
};

/* ------------------------------------------------------------------ */
/* Active palette — `color`. Mutated in place by `applyPalette()`.     */
/* Initialised to a (mutable) copy of `colorDark` so consumers can     */
/* keep reading `color.frameBorder` literally; the reactivity trigger  */
/* is the `theme` `$state` rune in `theme.svelte.ts`, which re-calls   */
/* `renderScene()` after the mutation lands.                           */
/* ------------------------------------------------------------------ */

export const color: Palette = { ...colorDark };

/* ------------------------------------------------------------------ */
/* Glow — RGBA strings used by the running-badge pulse + status halos. */
/* PixiJS doesn't take RGBA strings directly (it takes a number + an   */
/* alpha arg), but the CSS overlay layer's pulse keyframe consumes the */
/* `--guppi-status-*-glow` variables verbatim. Captured here as RGBA   */
/* strings (per design-system-003's resolved capture choice — see      */
/* STYLEGUIDE.md §5 "Glow capture") rather than as numeric Pixi colours */
/* so the CSS mirror is a 1:1 string copy.                             */
/* ------------------------------------------------------------------ */

export const glowDark = {
	statusIdle: 'rgba(123,124,138,.18)',
	statusRunning: 'rgba(37,171,254,.22)',
	statusBlocked: 'rgba(232,84,84,.22)',
	statusMissing: 'rgba(255,139,0,.22)'
} as const;

export type GlowPalette = { -readonly [K in keyof typeof glowDark]: string };

/** Glow halos for the light theme — slightly tightened alphas to match the
 *  design's `[data-theme="light"]` block. The RGB matches each light status
 *  colour so a pulse on a `running` BC reads "this hue, slightly bigger". */
export const glowLight: GlowPalette = {
	statusIdle: 'rgba(138,141,153,.16)',
	statusRunning: 'rgba(29,140,212,.20)',
	statusBlocked: 'rgba(208,69,69,.20)',
	statusMissing: 'rgba(217,117,0,.22)'
};

/** Active glow palette (mutated in place by `applyPalette()`). */
export const glow: GlowPalette = { ...glowDark };

/* ------------------------------------------------------------------ */
/* Modal backdrop — the theme-invariant scrim behind a modal dialog.   */
/* RGBA string, mirrors `--guppi-modal-backdrop` in `tokens.css`. The  */
/* only consumer today is the CSS overlay layer (`src/lib/Modal.svelte`*/
/* — ADR-003 overlay surface); kept here for the styleguide rule      */
/* "every visual value has a tokens home" even though PixiJS does not  */
/* render modals. Theme-invariant per the 2026-05-16 design reference  */
/* (canvas-008 — see `tokens.css` comment for the audit story).        */
/* ------------------------------------------------------------------ */

export const modalBackdrop = 'rgba(10, 10, 14, 0.62)';

/* ------------------------------------------------------------------ */
/* Typography — one family, three sizes (per the task scope).          */
/* ------------------------------------------------------------------ */

export const typography = {
	/** UI family. System stack — zero web-font payload, native feel in the WebView. */
	fontFamily:
		"'Inter', 'Segoe UI', system-ui, -apple-system, 'Helvetica Neue', Arial, sans-serif",
	/** Monospace — terminal panels, paths, task counts. */
	fontFamilyMono: "'Cascadia Code', 'Consolas', 'SF Mono', 'Menlo', monospace",

	/** Three-step scale (world-space px at zoom 1). */
	sizeTitle: 16, // tile / BC node titles
	sizeBody: 12, // subtitles, paths
	sizeCaption: 10, // task counts, badge labels, hints

	/** Detail-panel reader body (design-system-006). The docked panel's prose
	 *  reads denser than canvas chrome: 14px Inter at line-height 1.65 (the
	 *  canvas-009 reader pins). Distinct from `sizeBody` (12px canvas labels).
	 *  DOM-overlay px, not world-space. */
	sizePanelBody: 14,
	lineHeightPanelBody: 1.65,

	weightRegular: 400,
	weightMedium: 500,
	weightBold: 700
} as const;

/* ------------------------------------------------------------------ */
/* Spacing — a 4px base scale. World-space px at zoom 1.               */
/* ------------------------------------------------------------------ */

export const spacing = {
	xs: 4,
	sm: 8,
	md: 12,
	lg: 16,
	xl: 24,
	xxl: 32
} as const;

/* ------------------------------------------------------------------ */
/* Shape / sizing — tile + node geometry, corner radii, borders.       */
/* ------------------------------------------------------------------ */

export const shape = {
	/** Project tile — larger, the canvas anchor. */
	tileWidth: 240,
	tileHeight: 132,
	/** BC node — smaller, secondary. */
	bcWidth: 184,
	bcHeight: 96,
	/** World-space distance of BC nodes radiating from the project tile. */
	bcOrbitRadius: 380,

	/** Corner radius. Open question resolved: rounded rectangle (see styleguide). */
	radiusTile: 12,
	radiusBc: 10,
	radiusBadge: 6,

	/** Border weights (world-space px at zoom 1). */
	borderWidth: 2,
	borderWidthFocus: 3,

	/** Status badge — a small pill in a node's corner. */
	badgeHeight: 18,
	badgeMinWidth: 18,

	/* --- Project frame (design-system-002) --------------------------- */
	radiusFrame: 12,
	borderWidthFrame: 1,
	frameHeaderHeight: 36,
	framePadding: 16,
	frameMinInnerWidth: 320,
	frameMinInnerHeight: 200,

	/* --- BC bubble (inside-frame variant — design-system-002) -------- */
	bcInsideWidth: 188,
	bcInsideHeight: 60,
	radiusBcInside: 8,
	bcInsidePillHeight: 16,
	bcInsidePillMinWidth: 32,
	bcInsidePillRadius: 8,

	/* --- Intra-project edge geometry (design-system-002) ------------- */
	edgeWeight: 2,
	edgeWeightConformist: 1,
	arrowheadLength: 10,
	arrowheadWidth: 8,
	aclNotchSize: 10,

	/* --- Kanban-accordion interior (design-system-006) --------------- */
	/* DOM-overlay px (screen-space, NOT world-space) — the interior is the
	 * HTML overlay layer (canvas-019 / ADR-017), so these are device px the
	 * DOM renders at, not zoom-1 world units like the orbit shape tokens. */

	/** BC accordion row. Collapsed = just the header band; expanded adds the
	 *  kanban board body. */
	accordionRowHeaderHeight: 44,
	accordionRowGap: 8,
	accordionRowRadius: 10,
	accordionRowPadding: 12,
	accordionChevronSize: 12,
	accordionRollupPillHeight: 18,
	accordionRollupPillRadius: 9,

	/** Kanban board + column. */
	kanbanColumnMinWidth: 240,
	kanbanColumnMaxWidth: 320,
	kanbanColumnGap: 12,
	kanbanColumnHeaderHeight: 28,
	kanbanColumnRadius: 8,
	kanbanColumnPadding: 8,

	/** Task card. */
	cardMinHeight: 56,
	cardRadius: 8,
	cardPadding: 10,
	cardGap: 8,
	cardBorderWidth: 1,
	cardBorderWidthAccent: 2,
	cardTagHeight: 16,
	cardTagRadius: 4,

	/** Docked detail panel. */
	panelWidth: 340,
	panelRadius: 12,
	panelPadding: 20,
	panelBorderWidth: 1,
	panelHeaderHeight: 52,
	panelCalloutRadius: 10,
	panelCalloutPadding: 14,
	panelCalloutBorderWidth: 1,
	panelButtonHeight: 30,
	panelButtonRadius: 6
} as const;

/* ------------------------------------------------------------------ */
/* Motion — the animation budget.                                      */
/* ------------------------------------------------------------------ */

export const motion = {
	durationCamera: 320, // ms
	durationAffordance: 120, // ms
	durationPulse: 1600, // ms
	easeStandard: 'cubic-bezier(0.22, 0.61, 0.36, 1)',
	easePulse: 'cubic-bezier(0.45, 0, 0.55, 1)',

	/* --- Kanban-accordion interior motion (design-system-006) -------- */
	/** Docked detail panel slide-in / slide-out. 320ms matches the camera
	 *  navigation budget (one "feel" for primary motion), but the panel gets
	 *  its OWN ease — a soft settle (`easePanel`) rather than the camera's
	 *  ease-out — because it docks against the viewport edge and a gentle
	 *  decelerate reads as "snapping into place" without overshoot. Decided
	 *  in design-system-006; see STYLEGUIDE.md §5 Q11. */
	durationPanel: 320, // ms
	easePanel: 'cubic-bezier(0.16, 0.84, 0.36, 1)',
	/** BC accordion collapse / expand. Kept just under the panel budget so
	 *  the row reflow reads quick (restrained budget — §5 Q3); shares the
	 *  panel ease. */
	durationAccordion: 240 // ms
} as const;

/* ------------------------------------------------------------------ */
/* Status helpers — keep the palette/glyph mapping in one place.       */
/* ------------------------------------------------------------------ */

export type TaskState = 'idle' | 'running' | 'blocked' | 'missing';

/** PixiJS-ready fill colour for a task/tile status. Mutated in place by
 *  `applyPalette()` — see the theme-flip explanation at the top of this
 *  module. */
export const statusColor: Record<TaskState, number> = {
	idle: colorDark.statusIdle,
	running: colorDark.statusRunning,
	blocked: colorDark.statusBlocked,
	missing: colorDark.statusMissing
};

/**
 * The glyph that pairs with each status. Colour is never the sole signal
 * (colourblind-friendly requirement) — the glyph carries the meaning too.
 * Theme-invariant.
 */
export const statusGlyph: Record<TaskState, string> = {
	idle: '○', // ○  hollow circle — at rest
	running: '▶', // ▶  play — work in progress
	blocked: '◆', // ◆  diamond — needs attention
	missing: '✕' // ✕  cross — absent
};

/** Human-readable label for a status (badge tooltip / hint). Theme-invariant. */
export const statusLabel: Record<TaskState, string> = {
	idle: 'Idle',
	running: 'Running',
	blocked: 'Blocked on a question',
	missing: 'Missing'
};

/* ------------------------------------------------------------------ */
/* Theme flip — design-system-004.                                     */
/*                                                                    */
/* `applyPalette(theme)` mutates the active `color`, `statusColor`,    */
/* and `glow` exports in place so consumers reading `color.frameBorder`*/
/* see the new palette on the next read. The PixiJS canvas pairs this  */
/* with an explicit `renderScene()` call on the next frame             */
/* (`theme.svelte.ts` triggers via its `$state` rune subscription).    */
/* ------------------------------------------------------------------ */

export type Theme = 'dark' | 'light';

export function applyPalette(theme: Theme): void {
	const palette = theme === 'light' ? colorLight : colorDark;
	const glows = theme === 'light' ? glowLight : glowDark;
	Object.assign(color, palette);
	Object.assign(glow, glows);
	statusColor.idle = palette.statusIdle;
	statusColor.running = palette.statusRunning;
	statusColor.blocked = palette.statusBlocked;
	statusColor.missing = palette.statusMissing;
}

/* ------------------------------------------------------------------ */
/* CSS-variable bridge — for the HTML overlay layer (ADR-003).         */
/* ------------------------------------------------------------------ */

/** Convert a PixiJS numeric colour to a CSS hex string. */
export function cssHex(value: number): string {
	return '#' + value.toString(16).padStart(6, '0');
}
