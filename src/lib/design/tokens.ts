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

	/** Project tile — the visual anchor of the canvas. Warm, high-weight. */
	tileFill: 0x262636,
	tileBorder: 0x8a8ad0,
	tileText: 0xf2f2f7,
	tileTextMuted: 0xb9b9c8,

	/** Bounded-context node — secondary to the project tile. Cool, calmer. */
	bcFill: 0x20242b,
	bcBorder: 0x3c8b8e,
	bcText: 0xe6e6ec,
	bcTextMuted: 0x9a9aa6,

	/** Edges — project → BC connectors. */
	edge: 0x4a4a58,
	edgeHighlight: 0x8a8ad0,

	/** Focus / hover affordance — the ring drawn around an interactive node. */
	focusRing: 0xc8c8ff,

	/* --- Status palette (colourblind-friendly) ----------------------- */
	// Chosen to stay distinguishable under deuteranopia/protanopia: the four
	// states differ in HUE *and* in lightness, and each pairs with a distinct
	// glyph (see styleguide) so colour is never the only signal.
	statusIdle: 0x6f7585, // neutral grey-blue — "nothing happening"
	statusRunning: 0x2f9fe0, // bright blue — "work in progress"
	statusBlocked: 0xe6a020, // amber — "blocked on a question"
	statusMissing: 0xd05a8a, // magenta-pink — "expected but absent"

	/** Text colour that sits *on top* of a status fill (badges). */
	statusText: 0x10131a,

	/** Voice-state indicator — the ambient corner glyph. */
	voiceIdle: 0x5a5a68, // mic available, not listening
	voiceListening: 0x2f9fe0, // actively listening — matches statusRunning blue
	voiceMuted: 0xd05a8a // mic unavailable / muted
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
	badgeMinWidth: 18
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
