// GUPPI design tokens — the single source of truth for the visual language
// (design-system-001-styleguide). Every frontend feature task references
// these; nothing in the canvas should hard-code a colour, size, or duration.
//
// Why a TypeScript object and not only CSS variables: the canvas renders
// through PixiJS v8 (ADR-003), whose APIs take *numeric* colours (0xrrggbb)
// and numeric sizes. CSS custom properties are strings. So this module is the
// canonical source; `tokens.css` mirrors the same values as CSS custom
// properties for the HTML overlay layer (markdown viewer, command palette,
// terminal-panel chrome — ADR-003's overlay approach).
//
// Convention: `color*` values are PixiJS-ready numbers. `cssVar()` exposes the
// matching CSS custom property name for DOM consumers.

/* ------------------------------------------------------------------ */
/* Colour — dark mode is the default and only theme shipped now.       */
/* (Open question resolved: dark-default, light mode deferred. See the */
/*  styleguide doc for the reasoning.)                                 */
/* ------------------------------------------------------------------ */

export const color = {
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
	// The "frame" is the surrounding region drawn around a project; its
	// interior contains the project's BCs as bubbles, with intra-project
	// edges between them by relationship type. Title bar runs across the
	// top edge and carries project name + status badges + task counts;
	// it is the project's drag handle + right-click target.
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
	// Distinct from the legacy orbit `bcFill` / `bcBorder` only by the
	// intent (sits *inside* the frame). Border updated to brand blue in
	// design-system-003 so the BC bubble reads cool against the warm
	// project frame. `bcInsidePillFill` is the small task-counts pill that
	// sits inline with the BC title at default zoom.
	bcInsideFill: 0x20242b,
	bcInsideBorder: 0x25abfe,
	bcInsideText: 0xe6e6ec,
	bcInsideTextMuted: 0x9a9aa6,
	bcInsidePillFill: 0x2a2f38,

	/* --- Intra-project edges (design-system-002) --------------------- */
	// Single neutral palette (resolved default: geometry, not hue, carries
	// the type). All four resolve to `hairlineStrong` since design-system-003
	// (was `fgMuted` — but `fgMuted` is muted *text*; edges deserve their own
	// hairline hue). The tokens exist as named aliases so consumers spell
	// intent at the call site rather than reaching for `hairlineStrong`
	// directly. Hover/focus highlight uses `edgeHighlight` (above).
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
	// Chosen to stay distinguishable under deuteranopia/protanopia: the four
	// states differ in HUE *and* in lightness, and each pairs with a distinct
	// glyph (see styleguide) so colour is never the only signal. The palette
	// was revised in design-system-003 to lock in Marco's brand hues: blue
	// for "running" and orange for "missing" both pull double duty as brand
	// accents (matching `frameBorder` / `bcBorder`).
	statusIdle: 0x7b7c8a, // plain grey — "nothing happening"
	statusRunning: 0x25abfe, // brand blue — "work in progress"
	statusBlocked: 0xe85454, // red — "blocked on a question"
	statusMissing: 0xff8b00, // brand orange — "expected but absent"

	/** Text colour that sits *on top* of a status fill (badges). */
	statusText: 0x10131a,

	/** Voice-state indicator — the ambient corner glyph.
	 *  `voiceListening` matches `statusRunning` (brand blue) so listening
	 *  reads as "an agent is working" — voice is just another running
	 *  process. `voiceMuted` reads on the neutral `fgMuted` hue with a
	 *  slash glyph (see the design's `guppi-voice-views.jsx`, §4); the
	 *  earlier magenta-pink read as "alarm" which is wrong for muted. */
	voiceIdle: 0x5a5a68, // mic available, not listening
	voiceListening: 0x25abfe, // actively listening — matches statusRunning blue
	voiceMuted: 0x6e6e80 // mic unavailable / muted — neutral fgMuted
} as const;

/* ------------------------------------------------------------------ */
/* Glow — RGBA strings used by the running-badge pulse + status halos. */
/* PixiJS doesn't take RGBA strings directly (it takes a number + an   */
/* alpha arg), but the CSS overlay layer's pulse keyframe consumes the */
/* `--guppi-status-*-glow` variables verbatim. Captured here as RGBA   */
/* strings (per design-system-003's resolved capture choice — see      */
/* STYLEGUIDE.md §5 "Glow capture") rather than as numeric Pixi colours */
/* so the CSS mirror is a 1:1 string copy.                             */
/* ------------------------------------------------------------------ */

export const glow = {
	statusIdle: 'rgba(123,124,138,.18)',
	statusRunning: 'rgba(37,171,254,.22)',
	statusBlocked: 'rgba(232,84,84,.22)',
	statusMissing: 'rgba(255,139,0,.22)'
} as const;

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
	// The frame contains a project's BCs. Border is intentionally thinner
	// than a tile (1px vs 2px) so the frame reads as a boundary, not as a
	// peer of the BC bubbles inside it. Corner radius matches `radiusTile`
	// so frames feel "project-shaped" continuous with the orbit baseline.
	/** Frame corner radius (resolved default: 12 — same as `radiusTile`). */
	radiusFrame: 12,
	/** Frame border weight — 1 keeps it a quiet container. */
	borderWidthFrame: 1,
	/** Frame title-bar height — 36 (design-system-003). Accommodates the
	 *  drag-handle dots + project name + per-state mini badges + the
	 *  task-count row in one bar without crowding. */
	frameHeaderHeight: 36,
	/** Frame body inner padding — distance from frame edge to BC layout area. */
	framePadding: 16,
	/** Minimum frame inner width / height when auto-fitting empty or sparse
	 *  content. Below these, the frame looks too small to read as a region. */
	frameMinInnerWidth: 320,
	frameMinInnerHeight: 200,

	/* --- BC bubble (inside-frame variant — design-system-002) -------- */
	// Denser than the orbit BC node (resolved default). Title + counts pill
	// fit in one row at default zoom; height drops accordingly. Corner
	// radius drops to `spacing.sm` (8) — smaller than the orbit BC's 10 —
	// so the inside-frame variant reads as a sibling-cluster element, not
	// as a peer of the surrounding frame. Dimensions revised from
	// 160×56 → 188×60 in design-system-003 to give the title + pill row
	// more breathing room without breaking the dense-canvas read.
	bcInsideWidth: 188,
	bcInsideHeight: 60,
	radiusBcInside: 8,
	/** Inline task-counts pill that sits beside the BC name at default zoom. */
	bcInsidePillHeight: 16,
	bcInsidePillMinWidth: 32,
	bcInsidePillRadius: 8,

	/* --- Intra-project edge geometry (design-system-002) ------------- */
	/** Default edge weight for upstream/downstream + mutual + ACL. */
	edgeWeight: 2,
	/** Conformist — visually lighter so it reads as "downstream defers". */
	edgeWeightConformist: 1,
	/** Arrowhead size at the downstream end of a directional edge. */
	arrowheadLength: 10,
	arrowheadWidth: 8,
	/** ACL notch — a small triangle drawn at the midpoint of an ACL edge.
	 *  Resolved default: a triangle (vs zig-zag or hover-only label) keeps
	 *  the meaning visible at zoom-out without adding chrome. */
	aclNotchSize: 10
} as const;

/* ------------------------------------------------------------------ */
/* Motion — the animation budget. Open question resolved: "restrained" */
/* — short, eased transitions for navigation; one slow ambient pulse   */
/* for the single "running" signal; nothing else moves. See styleguide.*/
/* ------------------------------------------------------------------ */

export const motion = {
	/** Camera transitions — zoom-to-fit, focus-on-tile. */
	durationCamera: 320, // ms
	/** Hover / focus affordance fade. */
	durationAffordance: 120, // ms
	/** The "running" badge pulse — one full cycle. Slow, so it reads as ambient. */
	durationPulse: 1600, // ms
	/** Standard easing — ease-out for navigation, feels responsive. */
	easeStandard: 'cubic-bezier(0.22, 0.61, 0.36, 1)',
	/** Pulse easing — symmetric in/out so the breathing looks even. */
	easePulse: 'cubic-bezier(0.45, 0, 0.55, 1)'
} as const;

/* ------------------------------------------------------------------ */
/* Status helpers — keep the palette/glyph mapping in one place.       */
/* ------------------------------------------------------------------ */

export type TaskState = 'idle' | 'running' | 'blocked' | 'missing';

/** PixiJS-ready fill colour for a task/tile status. */
export const statusColor: Record<TaskState, number> = {
	idle: color.statusIdle,
	running: color.statusRunning,
	blocked: color.statusBlocked,
	missing: color.statusMissing
};

/**
 * The glyph that pairs with each status. Colour is never the sole signal
 * (colourblind-friendly requirement) — the glyph carries the meaning too.
 */
export const statusGlyph: Record<TaskState, string> = {
	idle: '○', // ○  hollow circle — at rest
	running: '▶', // ▶  play — work in progress
	blocked: '◆', // ◆  diamond — needs attention
	missing: '✕' // ✕  cross — absent
};

/** Human-readable label for a status (badge tooltip / hint). */
export const statusLabel: Record<TaskState, string> = {
	idle: 'Idle',
	running: 'Running',
	blocked: 'Blocked on a question',
	missing: 'Missing'
};

/* ------------------------------------------------------------------ */
/* CSS-variable bridge — for the HTML overlay layer (ADR-003).         */
/* ------------------------------------------------------------------ */

/** Convert a PixiJS numeric colour to a CSS hex string. */
export function cssHex(value: number): string {
	return '#' + value.toString(16).padStart(6, '0');
}
