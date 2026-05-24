# GUPPI Styleguide

The visual vocabulary for GUPPI's canvas. Every frontend feature task in every
bounded context implements against this document — it is the contract that
keeps the UI coherent.

**Status:** signed off by Marco in person on 2026-05-14 — the frontend gate is
open. A design-skill refinement pass is still expected; the open questions
below have *defensible defaults* (all approved as-is at sign-off) and Marco can
still override any of them.

- **Tokens (source of truth):** `src/lib/design/tokens.ts` (PixiJS-ready
  numeric values + scales) and `src/lib/design/tokens.css` (CSS custom
  properties for the HTML overlay layer — ADR-003).
- **First consumer:** `src/lib/Canvas.svelte` — the walking skeleton, upgraded
  from greybox to this baseline as part of `design-system-001-styleguide`.
- **Stack:** Svelte 5 + SvelteKit (ADR-002), PixiJS v8 canvas with HTML
  overlays at world coordinates (ADR-003).

---

## 1. Why a TS object *and* a CSS file

The canvas renders through PixiJS, whose APIs take **numeric** colours
(`0xrrggbb`) and numeric sizes. CSS custom properties are strings. So
`tokens.ts` is the **canonical** source; `tokens.css` mirrors the exact same
values as `--guppi-*` custom properties for DOM consumers (markdown viewer,
command palette, terminal-panel chrome — all ADR-003 overlay-layer surfaces).

**Rule for downstream tasks:** never hard-code a colour, size, font, or
duration. Import from `tokens.ts` (canvas / PixiJS code) or use the
`--guppi-*` variables (DOM / overlay code). If a value you need is missing,
add it to *both* files in the same change.

---

## 2. Tokens

### 2.1 Colour — dark mode

Dark mode is the **default and only theme shipped now** (open question 1,
resolved below). All values are in `tokens.ts` as numbers and `tokens.css` as
hex strings.

| Group | Token | Value | Use |
|---|---|---|---|
| Surface | `canvasBg` | `#16161c` | Canvas backdrop (PixiJS `Application` background) |
| Surface | `canvasBgRaised` | `#1e1e26` | Optional faint world grid / vignette |
| Project tile | `tileFill` | `#262636` | Project tile body — warm, high-weight |
| Project tile | `tileBorder` | `#ff8b00` | Project tile border — **brand orange** (design-system-003) |
| Project tile | `tileText` / `tileTextMuted` | `#f2f2f7` / `#b9b9c8` | Title / subtitle |
| BC node | `bcFill` | `#20242b` | BC node body — calmer, cooler |
| BC node | `bcBorder` | `#25abfe` | BC node border — **brand blue** (design-system-003) |
| BC node | `bcText` / `bcTextMuted` | `#e6e6ec` / `#9a9aa6` | Title / subtitle |
| Edge | `edge` | `#4a4a58` | Project → BC connectors (legacy / orbit baseline) |
| Edge | `edgeHighlight` | `#ff8b00` | Connector when its endpoint is focused — brand orange |
| Edge | `edgeUpstream` / `edgeMutual` / `edgeACL` / `edgeConformist` | `#3a3b46` | Intra-project edges — single neutral palette, geometry distinguishes types (see §3.8). Resolves to `hairlineStrong` (design-system-003; was `fgMuted`). |
| Hairline | `hairlineStrong` | `#3a3b46` | Edge / divider hue (design-system-003). Distinct from `fgMuted` (muted *text*); used for structural lines. |
| Edge | `fgMuted` | `#6e6e80` | Muted foreground for placeholder copy and disabled labels (no longer the edge hue) |
| Frame | `frameFill` / `frameBorder` / `frameHeaderFill` / `frameHeaderDivider` | `#1a1a22` / `#ff8b00` / `#262636` / `#2a2b35` | Project frame body, border (brand orange), header bar, header underline hairline (see §3.6) |
| Frame | `frameTitleText` / `frameTitleTextMuted` / `frameEmptyText` | `#f2f2f7` / `#b9b9c8` / `#6e6e80` | Header text + empty-frame placeholder |
| BC bubble (inside frame) | `bcInsideFill` / `bcInsideBorder` / `bcInsideText` / `bcInsideTextMuted` / `bcInsidePillFill` | `#20242b` / `#25abfe` / `#e6e6ec` / `#9a9aa6` / `#2a2f38` | Inside-frame BC bubble (brand-blue border) + task-counts pill (see §3.7) |
| Affordance | `focusRing` | `#ffb05a` | Ring drawn around a hovered/focused node — warm tint matches `frameBorder` |

**Kanban-accordion interior (design-system-006).** The DOM/HTML-overlay
interior introduced by the canvas pivot. `canvas-019` (ADR-017) ratified the
hybrid substrate: the project frame **shell** stays in PixiJS; the
accordion-row → kanban-board → task-card → detail-panel **interior** is a DOM
overlay. These colours dress that overlay (consumed via `--guppi-*` CSS vars).
See §3.9–§3.12. Dark values below; light mirror in `tokens.css`
`[data-theme="light"]`.

| Group | Token | Value | Use |
|---|---|---|---|
| Accordion row | `accordionRowFill` / `accordionRowHeaderFill` / `accordionRowDivider` | `#1e1e26` / `#20242b` / `#2a2b35` | Expanded-row body / header band / divider hairline (see §3.9) |
| Accordion row | `accordionRowText` / `accordionRowTextMuted` | `#e6e6ec` / `#9a9aa6` | BC name / roll-up + count text |
| Accordion row | `accordionChevron` | `#9a9aa6` | Disclosure chevron glyph |
| Accordion row | `accordionRollupPillFill` | `#262636` | Neutral capsule behind the active/blocked/idling roll-up; in-pill text uses `statusColor[...]` |
| Kanban column | `kanbanColumnFill` / `kanbanColumnHeaderText` / `kanbanColumnDivider` / `kanbanColumnEmptyText` | `#1a1a22` / `#9a9aa6` / `#2a2b35` / `#6e6e80` | Column body / header label / divider / empty-column placeholder (see §3.10) |
| Task card | `cardFill` / `cardFillHover` | `#20242b` / `#262636` | Card body — default / hover (see §3.11) |
| Task card | `cardBorder` / `cardBorderSelected` / `cardBorderBlocked` | `#2a2b35` / `#25abfe` / `#e85454` | Border — default / selected (brand blue) / blocked (status red) |
| Task card | `cardIdText` / `cardTitleText` | `#6e6e80` / `#e6e6ec` | Id label / title |
| Task card | `cardTagFill` / `cardTagText` | `#262636` / `#9a9aa6` | Tag chip fill / text |
| Task card | `cardAgentLineRunning` / `cardAgentLineBlocked` | `#25abfe` / `#e85454` | Live-agent indicator line — running (brand blue) / blocked (status red); aliases of the §2.2 palette, named for intent |
| Detail panel | `panelFill` / `panelBorder` | `#1a1a22` / `#2a2b35` | Panel body / left-edge border (see §3.12) |
| Detail panel | `panelHeaderText` / `panelHeaderTextMuted` / `panelBodyText` | `#f2f2f7` / `#9a9aa6` / `#e6e6ec` | Header title / header muted / body prose |
| Panel callout | `panelCalloutFill` / `panelCalloutBorder` / `panelCalloutLabel` / `panelCalloutText` | `#2a211a` / `#ff8b00` / `#ff8b00` / `#f2f2f7` | "AGENT NEEDS AN ANSWER" boxed region — warm tint / brand-orange border + label / body text |
| Panel buttons | `panelButtonPrimaryFill` / `panelButtonPrimaryText` | `#ff8b00` / `#10131a` | Primary action (answer) — brand-orange fill, dark glyph |
| Panel buttons | `panelButtonSecondaryFill` / `panelButtonSecondaryText` | `#262636` / `#e6e6ec` | Secondary actions (defer / edit) — quiet fill |

### 2.2 Status palette — colourblind-friendly

Four discrete states. The palette is built so it survives deuteranopia and
protanopia: the states differ in **hue *and* lightness**, and **each pairs
with a distinct glyph** — colour is never the only signal. Revised in
`design-system-003` to lock in Marco's brand hues (blue + orange) — these
two are reused as accents on `bcBorder` and `frameBorder`, so the status
palette and the brand palette read as one system.

| State | Token | Colour | Glyph | Meaning |
|---|---|---|---|---|
| `idle` | `statusIdle` | `#7b7c8a` grey | `○` hollow circle | At rest — nothing in flight |
| `running` | `statusRunning` | `#25abfe` brand blue | `▶` play | Work in progress |
| `blocked` | `statusBlocked` | `#e85454` red | `◆` diamond | Blocked on a question |
| `missing` | `statusMissing` | `#ff8b00` brand orange | `✕` cross | Expected but absent |

`statusText` (`#10131a`) is the dark text/glyph colour that sits *on top* of a
status fill (badges). `statusColor`, `statusGlyph`, `statusLabel` maps in
`tokens.ts` keep the mapping in one place — consume those, don't re-derive.

**Status glow tokens (design-system-003).** Each status has an
accompanying RGBA halo used by the `running`-pulse keyframe and any
ambient badge glow. These are RGBA strings — see §5 "Glow capture" for
why this token group is strings rather than Pixi numerics.

| State | Glow token (CSS) | Glow token (TS) | Value |
|---|---|---|---|
| `idle` | `--guppi-status-idle-glow` | `glow.statusIdle` | `rgba(123,124,138,.18)` |
| `running` | `--guppi-status-running-glow` | `glow.statusRunning` | `rgba(37,171,254,.22)` |
| `blocked` | `--guppi-status-blocked-glow` | `glow.statusBlocked` | `rgba(232,84,84,.22)` |
| `missing` | `--guppi-status-missing-glow` | `glow.statusMissing` | `rgba(255,139,0,.22)` |

### 2.3 Typography — one family, three sizes

| Token | Value | Use |
|---|---|---|
| `fontFamily` | Inter → Segoe UI → system-ui stack | All UI text. Zero web-font payload. |
| `fontFamilyMono` | Cascadia Code → Consolas → system mono | Paths, task counts, terminal panels |
| `sizeTitle` | `16px` | Tile / BC node titles |
| `sizeBody` | `12px` | Subtitles, body copy |
| `sizeCaption` | `10px` | Task counts, badge labels, hints |
| `sizePanelBody` | `14px` | Detail-panel reader prose (design-system-006 — the canvas-009 reader pins); DOM-overlay px, not world-space |
| `lineHeightPanelBody` | `1.65` | Line-height for the detail-panel reader prose |
| `weightRegular / Medium / Bold` | `400 / 500 / 700` | — |

Sizes are world-space px **at zoom 1**; canvas code multiplies by the camera
zoom and floors at a legible minimum.

### 2.4 Spacing — 4px base scale

`xs 4` · `sm 8` · `md 12` · `lg 16` · `xl 24` · `xxl 32` (px, world-space at
zoom 1). Use the scale; don't invent intermediate values.

### 2.5 Shape — geometry

| Token | Value | Use |
|---|---|---|
| `tileWidth` × `tileHeight` | `240 × 132` | Project tile — larger, the anchor (orbit baseline) |
| `bcWidth` × `bcHeight` | `184 × 96` | BC node — smaller, secondary (orbit baseline) |
| `bcOrbitRadius` | `380` | World distance of BC nodes from the project tile (orbit baseline) |
| `radiusTile` / `radiusBc` / `radiusBadge` | `12 / 10 / 6` | Corner radii |
| `borderWidth` / `borderWidthFocus` | `2 / 3` | Border weights |
| `badgeHeight` / `badgeMinWidth` | `18 / 18` | Status badge pill |
| `radiusFrame` / `borderWidthFrame` | `12 / 1` | Project frame corner + border (see §3.6) |
| `frameHeaderHeight` / `framePadding` | `36 / 16` | Header bar height + inner body padding (height bumped from 24 → 36 in design-system-003) |
| `frameMinInnerWidth` / `frameMinInnerHeight` | `320 / 200` | Floor for auto-fit frame sizing |
| `bcInsideWidth` × `bcInsideHeight` | `188 × 60` | BC bubble inside a frame — denser than orbit BC node; revised from 160×56 in design-system-003 |
| `radiusBcInside` | `8` | BC bubble corner radius (rounded rectangle, denser than orbit) |
| `bcInsidePillHeight` / `bcInsidePillMinWidth` / `bcInsidePillRadius` | `16 / 32 / 8` | Task-counts pill inline with the BC name |
| `edgeWeight` / `edgeWeightConformist` | `2 / 1` | Intra-project edge line weights |
| `arrowheadLength` / `arrowheadWidth` | `10 / 8` | Arrowhead at the downstream end of directional edges |
| `aclNotchSize` | `10` | Triangle notch at the midpoint of an ACL edge |

**Kanban-accordion interior (design-system-006).** DOM-overlay px
(**screen-space**, not world-space px at zoom 1 like the orbit tokens above —
the interior is the HTML overlay layer, canvas-019 / ADR-017).

| Token | Value | Use |
|---|---|---|
| `accordionRowHeaderHeight` | `44` | Collapsed/expanded row header band height (see §3.9) |
| `accordionRowGap` / `accordionRowRadius` / `accordionRowPadding` | `8 / 10 / 12` | Inter-row gap / corner radius / inner padding |
| `accordionChevronSize` | `12` | Disclosure chevron glyph box |
| `accordionRollupPillHeight` / `accordionRollupPillRadius` | `18 / 9` | Roll-up pill (capsule — radius = half height) |
| `kanbanColumnMinWidth` / `kanbanColumnMaxWidth` | `200 / 280` | Column width floor / ceiling (see §3.10) |
| `kanbanColumnGap` / `kanbanColumnHeaderHeight` / `kanbanColumnRadius` / `kanbanColumnPadding` | `12 / 28 / 8 / 8` | Inter-column gap / header height / corner / inner padding |
| `cardMinHeight` / `cardRadius` / `cardPadding` / `cardGap` | `56 / 8 / 10 / 8` | Task-card min height / corner / inner padding / inter-card gap (see §3.11) |
| `cardBorderWidth` / `cardBorderWidthAccent` | `1 / 2` | Border weight — default / selected + blocked accent |
| `cardTagHeight` / `cardTagRadius` | `16 / 4` | Tag chip height / corner |
| `panelWidth` / `panelRadius` / `panelPadding` / `panelBorderWidth` | `340 / 12 / 20 / 1` | Detail-panel width / corner / inner padding / border (see §3.12) |
| `panelHeaderHeight` | `52` | Panel header band (task id + status pill + tags) |
| `panelCalloutRadius` / `panelCalloutPadding` / `panelCalloutBorderWidth` | `10 / 14 / 1` | Callout box corner / padding / border |
| `panelButtonHeight` / `panelButtonRadius` | `30 / 6` | Answer/defer/edit button height / corner |

### 2.6 Motion — the animation budget

Budget character: **restrained**. Short eased transitions for *navigation*; a
single slow ambient pulse reserved for the *one* "running" signal; nothing
else moves. (Open question 3, resolved below.)

| Token | Value | Use |
|---|---|---|
| `durationCamera` | `320ms` | Camera transitions — zoom-to-fit, focus-on-tile |
| `durationAffordance` | `120ms` | Hover / focus ring fade |
| `durationPulse` | `1600ms` | "Running" badge pulse — one full cycle, slow so it reads ambient |
| `easeStandard` | `cubic-bezier(0.22, 0.61, 0.36, 1)` | ease-out — navigation feels responsive |
| `easePulse` | `cubic-bezier(0.45, 0, 0.55, 1)` | symmetric in/out — even breathing |
| `durationPanel` | `320ms` | Detail-panel slide-in / slide-out (design-system-006; matches `durationCamera` budget) |
| `easePanel` | `cubic-bezier(0.16, 0.84, 0.36, 1)` | Soft settle for the docked panel — decelerate-into-place without overshoot (design-system-006; see §5 Q11) |
| `durationAccordion` | `240ms` | BC accordion collapse / expand; shares `easePanel` |

`durationPanel` reuses the 320ms primary-motion budget but pairs with its
**own** ease (`easePanel`) rather than the camera's `easeStandard` — see §5
Q11 for why the panel warrants a distinct curve.

---

## 3. Components — states

Each visual element and its discrete states. "Implemented" = present in the
`Canvas.svelte` baseline; "contract" = tokens + shape defined, full behaviour
is downstream BC work.

### 3.1 Project tile — *implemented*

The canvas anchor. Primary hierarchy: larger geometry, warm `tileBorder`
(`#ff8b00` brand orange), `fontFamily` bold title + mono path subtitle.

| State | Visual |
|---|---|
| Default | `tileFill` body, `tileBorder` 2px border |
| Hover / focus | adds a `focusRing` 3px ring just outside the border |
| Dragging | same as focus; world position persists on pointer-up (ADR-004) |

The project tile carries **no status badge** — status is a per-BC concept.

### 3.2 BC node — *implemented*

Secondary hierarchy: smaller geometry, cool `bcBorder` (`#25abfe` brand
blue), same type scale at lower weight. Carries a **status badge** (top-right
corner) derived from task counts.

| State | Visual |
|---|---|
| Default | `bcFill` body, `bcBorder` 2px border, status badge |
| Hover / focus | adds the `focusRing` ring |

Baseline status derivation (`deriveBcStatus` in `Canvas.svelte`) — intentionally
simple, the `canvas` BC refines it once real per-task status exists:
- `missing` — no task files at all
- `running` — at least one task in `doing/`
- `blocked` — tasks parked in `backlog/` with nothing in `todo/`
- `idle` — has tasks, none in flight, nothing stuck

### 3.3 Edge — *implemented*

Project → BC connector. `edge` colour, `borderWidth`-scaled stroke, drawn
*under* nodes. `edgeHighlight` is reserved for "endpoint focused" and for
future cross-BC relationship styles if the context-map introduces them.

### 3.4 Status badge — *implemented*

A `radiusBadge` rounded pill, `badgeHeight` square, pinned to a node's
top-right corner. `statusColor[state]` fill + `statusGlyph[state]` glyph in
`statusText`. Colour + glyph together — never colour alone.

### 3.5 Voice-state indicator — *contract (idle baseline)*

A single ambient indicator: a small dot + label pinned to the
**bottom-right of the viewport** (screen-space, so it does not move when the
canvas pans). Non-intrusive by design — one glyph, no chrome.

| State | Token | Visual |
|---|---|---|
| `idle` | `voiceIdle` `#5a5a68` | mic available, not listening — "mic" |
| `listening` | `voiceListening` `#25abfe` | actively listening (matches `running` brand blue) — "mic" |
| `muted` | `voiceMuted` `#6e6e80` | mic unavailable / muted — slash glyph on the neutral `fgMuted` hue (design-system-003 revision; the earlier magenta-pink read as "alarm") |

The baseline renders the `idle` state. The **voice BC** wires real mic state
into `voiceState` later; this establishes the visual contract and the token
set so that work has something explicit to land against.

### 3.6 Project frame — *contract*

The frame is the project-level container introduced by
`design-system-002-project-frame-vocabulary` and consumed by
`canvas-007-project-as-frame`. It supersedes the orbit baseline's
**project tile + project→BC line** rendering: the project becomes a
*region* on the canvas; its BCs sit *inside* the region as bubbles
(§3.7); BC↔BC edges (§3.8) carry the relationship structure.

The frame has three parts: a **header bar** along the top edge (drag
handle, right-click target, status read-out), a **body** containing the
project's BC bubbles, and a **border + corners** that mark the boundary.
Auto-fit: the frame sizes itself to its content with `framePadding`
inside and the `frameMin*` floors below — no user-resize at v1.

| State | Visual |
|---|---|
| Default | `frameFill` body, `frameBorder` 1px brand-orange border (`#ff8b00`), `radiusFrame` 12px corners, header bar in `frameHeaderFill` divided from the body by `frameHeaderDivider` 1px hairline (`#2a2b35` — quiet, not the brand accent) |
| Hover (over body) | no change — frame is a region, not an interactive node; hover affordance applies to the header bar only |
| Hover (header bar) | `focusRing` 3px halo around the header bar edges — "click here for project-level actions" |
| Dragging (header bar) | same as hover; the whole frame (header + body + interior BCs) follows the pointer. World position persists on pointer-up (ADR-004). |

**Header bar.** Height = `frameHeaderHeight` (36 — design-system-003;
was 24). The taller bar fits, left to right: drag-handle dots, project
name in `frameTitleText` at `sizeBody` `weightMedium`, the status badges
(one badge per status state that has count > 0; same `statusColor` /
`statusGlyph` palette as a BC node), and the task-counts row right-aligned
in `frameTitleTextMuted` at `sizeCaption` `fontFamilyMono`. Right-clicking
the header bar opens the existing tile context menu (`Remove project`,
etc. — same surface as today's tile right-click).

**Empty-frame state.** A project with zero BCs is still a frame — but
rendering an empty box reads as "broken". The body shows a single
centred placeholder line in `frameEmptyText` at `sizeBody`:

> No bounded contexts yet — add a `contexts/<bc>/` directory to populate.

The placeholder reads as a hint, not a control; it does not pulse, link,
or animate.

**Pseudocode (PixiJS world coords):**

```
// Frame = header + body + border, drawn as one Container at project.pos.
const w = max(frameMinInnerWidth + 2*framePadding, autofit.width);
const h = max(frameMinInnerHeight + 2*framePadding + frameHeaderHeight,
              autofit.height + frameHeaderHeight);

frame.lineStyle(borderWidthFrame, frameBorder)
     .beginFill(frameFill).drawRoundedRect(0, 0, w, h, radiusFrame).endFill();

// Header bar
frame.beginFill(frameHeaderFill)
     .drawRoundedRect(0, 0, w, frameHeaderHeight, radiusFrame).endFill();
frame.lineStyle(1, frameHeaderDivider)
     .moveTo(0, frameHeaderHeight).lineTo(w, frameHeaderHeight);
// → project name (left), status badges (centre/right), task counts (right)

// Body: lay out BC bubbles inside the {framePadding, frameHeaderHeight+framePadding}
// inset region using the force-directed layout canvas-007 owns.
```

### 3.7 BC bubble (inside frame) — *superseded for the canvas interior (design-system-006)*

> **Superseded for the canvas interior.** The canvas pivot (`canvas-019`,
> ADR-017) replaces the in-frame **BC bubble** with the **BC accordion row**
> (§3.9) + its kanban interior (§3.10–§3.12): a project's BCs no longer sit as
> floating bubbles inside the frame body — they stack as collapsible accordion
> rows, each expanding to a kanban board of its tasks. The frame **shell**
> (§3.6 header + border) stays in PixiJS; the interior is now a DOM overlay.
> The vocabulary below is **preserved, not deleted** — the inside-frame bubble
> + its task-counts pill may resurface in a future zoomed-out / cross-project
> view where per-BC chrome (not per-task cards) is the right density. For
> current canvas-interior work, build against §3.9–§3.12.

The BC bubble inside a frame is distinct from the orbit-baseline BC node
(§3.2). It is **denser** — title + counts pill in one row at default
zoom, lower height — because many BCs share a frame and the eye needs to
read the relationship graph between them, not each bubble's chrome.

Shape: a rounded rectangle (`radiusBcInside` 8), `bcInsideWidth × bcInsideHeight`
(188 × 60 — design-system-003 revision; was 160 × 56), `bcInsideFill` body,
`bcInsideBorder` 2px brand-blue stroke (`#25abfe`). Inside, in one row at
default zoom:

- BC name — `bcInsideText`, `sizeBody`, `weightMedium`, left-aligned.
- Task-counts pill — right-aligned, `bcInsidePillFill` rounded rect
  (`bcInsidePillRadius` 8, `bcInsidePillHeight` 16, min width
  `bcInsidePillMinWidth` 32), `sizeCaption` `fontFamilyMono` glyph + count
  e.g. `▶3`. The pill colour itself is neutral; the in-pill text uses the
  matching `statusColor[...]` per state. (Future: one pill per non-zero
  state.)
- Status badge — top-right corner of the bubble (same surface as §3.4):
  `radiusBadge` pill carrying `statusColor[state]` + `statusGlyph[state]`.
  The badge is the at-a-glance signal; the pill is the count read-out.

| State | Visual |
|---|---|
| Default | `bcInsideFill` body, `bcInsideBorder` 2px border, status badge top-right, counts pill inline |
| Hover | adds the `focusRing` 3px ring just outside the border |
| Focus | same as hover; persists until pointer leaves or focus moves |
| Dragging | same as focus; world position inside the frame persists via `project-registry-004`'s `save_bc_position` IPC on pointer-up |

Status derivation matches §3.2's `deriveBcStatus` rules — unchanged.

### 3.8 Intra-project edges — *superseded for the canvas interior (design-system-006)*

> **Superseded for the canvas interior.** With the BC bubble replaced by the
> stacked accordion rows (§3.9), there is no longer a free-positioned bubble
> graph for these edges to connect on the canvas surface — the kanban-accordion
> interior carries no BC↔BC relationship lines. The four edge variants below
> are **preserved, not deleted**: the context-map relationship vocabulary
> (`upstream` / `mutual` / `ACL` / `conformist`) still exists as data and the
> geometry contract may resurface in a dedicated **cross-project relationship
> view** (a future, separate surface from the kanban interior). For
> current canvas-interior work, no intra-project edges are drawn; see
> `canvas-019` / ADR-017.

Four edge variants, one per context-map relationship type from
`project-registry-004`. The project→BC parent edge of the orbit baseline
**retires**; containment (BC inside frame) replaces it.

**Resolved default: single neutral palette.** All four variants use the
same `hairlineStrong` hue `#3a3b46` (revised in design-system-003 — the
named tokens `edgeUpstream`, `edgeMutual`, `edgeACL`, `edgeConformist`
all resolve to this value; previously they resolved to `fgMuted` `#6e6e80`,
but `fgMuted` is muted *text* and edges deserve their own structural
hairline). **Geometry** — arrowhead presence, notch glyph, line weight —
carries the type distinction. The reasoning matches the "restrained"
motion budget: a dense canvas reads cleanest when colour is consistent
and shape varies.

| Relationship | Token | Weight | Arrowhead | Notch | Direction |
|---|---|---|---|---|---|
| customer-supplier (upstream → downstream) | `edgeUpstream` | `edgeWeight` (2) | yes, at downstream end | — | directional |
| shared-kernel / partnership / mutual | `edgeMutual` | `edgeWeight` (2) | — | — | non-directional |
| anti-corruption-layer (upstream → downstream) | `edgeACL` | `edgeWeight` (2) | yes, at downstream end | yes, triangle at midpoint, `aclNotchSize` 10, pointing toward upstream | directional |
| conformist (upstream → downstream) | `edgeConformist` | `edgeWeightConformist` (1) | yes, at downstream end | — | directional |
| *(no relationship declared)* | — | — | — | — | no edge drawn |

**Arrowhead geometry.** A filled isosceles triangle at the downstream
end, `arrowheadLength` × `arrowheadWidth` (10 × 8), tip touching the
edge of the downstream BC bubble. The shaft of the line terminates at
the base of the arrowhead.

**ACL notch.** A small filled triangle at the **midpoint** of the edge,
`aclNotchSize` 10, oriented perpendicular to the line and pointing
**toward the upstream end** (the side being protected from). Resolved
default: a triangle (vs zig-zag, vs hover-only label) keeps the
"anti-corruption" semantics visible at zoom-out without adding chrome.

**Conformist.** Visually lighter (`edgeWeightConformist` 1) and reads
as "downstream defers to upstream" — the lower contrast carries the
asymmetry. Same hue + arrowhead direction as `edgeUpstream` otherwise.

**Hover-on-edge highlight.** *Deferred to sign-off* — open question on
the task: should hovering an edge highlight its two endpoint BCs (and
optionally tween edges connected to either endpoint)? The token
`edgeHighlight` already exists from the orbit baseline and is the
natural surface for this behaviour if Marco accepts it. Until decided,
edges have no hover state.

**Motion.** Edge appearance/disappearance fades in/out per
`durationAffordance` (120ms) when `bc_relationships_changed` arrives.
Force-directed re-layout of BC positions is `canvas-007`'s call (lean:
instant, matching the restrained budget).

**Pseudocode (PixiJS):**

```
function drawEdge(g, a, b, kind) {
  const colour = edgeColour[kind];          // all four → hairlineStrong today
  const weight = kind === 'conformist' ? edgeWeightConformist : edgeWeight;
  g.lineStyle(weight, colour);

  if (kind === 'mutual') {
    // bare line, no arrowhead, no notch
    g.moveTo(a.x, a.y).lineTo(b.x, b.y);
    return;
  }
  // directional: line + arrowhead at b
  const tip   = edgeOfBubble(b, fromDirection(a, b));
  const base  = pointAlong(tip, a, arrowheadLength);
  g.moveTo(a.x, a.y).lineTo(base.x, base.y);
  drawArrowhead(g, tip, base, arrowheadWidth, colour);

  if (kind === 'acl') {
    const mid = midpoint(a, tip);
    drawNotchTriangle(g, mid, towardA(a, mid), aclNotchSize, colour);
  }
}
```

### 3.9 BC accordion row — *contract (design-system-006)*

The canvas-pivot interior. Each bounded context inside a project frame is a
**collapsible accordion row** (replacing the §3.7 bubble). Rows stack
vertically inside the frame body with `accordionRowGap` (8) between them. A
row has two states.

**Collapsed.** Just the header band, `accordionRowHeaderHeight` (44),
`accordionRowHeaderFill` body, `accordionRowRadius` (10) corners. Left to
right:

- **Disclosure chevron** — `accordionChevron` glyph, `accordionChevronSize`
  (12), pointing right when collapsed / down when expanded.
- **BC name** — `accordionRowText`, `sizeBody`, `weightMedium`.
- **Roll-up pills** — one capsule per non-zero state ("1 active · 2 blocked ·
  1 idling" in the reference), `accordionRollupPillFill` background,
  `accordionRollupPillHeight` (18), `accordionRollupPillRadius` (9 — capsule).
  In-pill glyph + count use the matching `statusColor[...]` + `statusGlyph[...]`
  from §2.2 (running = `▶`, blocked = `◆`, idle = `○`). Colour is never the
  sole signal — the glyph rides along.
- **Total task count** — right-aligned, `accordionRowTextMuted`, `sizeCaption`
  `fontFamilyMono` (e.g. `6 tasks`).

**Expanded.** The header band stays; below it, divided by an
`accordionRowDivider` (1px) hairline, the row body (`accordionRowFill`) holds
the **kanban board** (§3.10). Body inner padding `accordionRowPadding` (12).

| State | Visual |
|---|---|
| Collapsed | header band only; chevron points right; roll-up pills + total count |
| Expanded | header band + divider + kanban-board body; chevron points down |
| Hover (header) | header band lifts to `accordionRowHeaderFill` hover tone via `focusRing`-tinted outline; the whole header is the click target that toggles collapse |

**Motion.** Collapse / expand animates the body height over
`durationAccordion` (240ms) with `easePanel`. Just under the panel budget so
the reflow reads quick (restrained budget — §5 Q3). The chevron rotates on the
same timing.

### 3.10 Kanban board + column — *contract (design-system-006)*

Inside an expanded accordion row: a horizontal strip of **four columns** in
fixed order — **BACKLOG · TODO · DOING · DONE** — mapping the task-file
lifecycle (`project-registry-005` owns the data spine). Columns sit in a
horizontally-scrollable track with `kanbanColumnGap` (12) between them.

- **Column** — `kanbanColumnFill` body, `kanbanColumnRadius` (8) corners,
  `kanbanColumnPadding` (8) inner. Width is fluid between
  `kanbanColumnMinWidth` (200) and `kanbanColumnMaxWidth` (280); when the four
  columns exceed the row width the track scrolls horizontally (the row, not
  the page, owns the scroll affordance — a thin scrollbar or edge fade).
- **Column header** — `kanbanColumnHeaderHeight` (28), the column label
  (`BACKLOG` / `TODO` / `DOING` / `DONE`) in `kanbanColumnHeaderText`,
  `sizeCaption`, `weightMedium`, letter-spaced uppercase. A
  `kanbanColumnDivider` hairline separates header from the card stack.
- **Card stack** — task cards (§3.11) stacked top-to-bottom with `cardGap`
  (8); the column scrolls vertically if the stack overflows.

**Empty-column state.** A column with no cards shows a single centred
placeholder line in `kanbanColumnEmptyText`, `sizeCaption` — e.g. `—` or
"nothing here". Reads as a hint; no control, no animation.

### 3.11 Task card — *contract (design-system-006)*

The per-task tile inside a column. `cardMinHeight` (56) — grows to fit
content — `cardRadius` (8) corners, `cardPadding` (10) inner, `cardFill`
body, `cardBorderWidth` (1) `cardBorder` stroke. Top to bottom:

- **Id label** — `cardIdText`, `sizeCaption` `fontFamilyMono` (e.g. `#41`).
- **Title** — `cardTitleText`, `sizeBody`, `weightMedium`. Wrap to a max of
  **2 lines**, then clamp with an ellipsis (the full title lives in the
  detail panel).
- **Tag chips** — optional row, each a `cardTagFill` capsule
  (`cardTagHeight` 16, `cardTagRadius` 4) with `cardTagText` `sizeCaption`
  label (e.g. `thumb-gen`).
- **Live-agent indicator line** — *only when a task is in `DOING` with an
  agent attached.* A single line ("orchestrator · waiting 2m 14s" in the
  reference), `sizeCaption` `fontFamilyMono`. Treatment by state:
  - **running** — `cardAgentLineRunning` (brand blue) text, prefixed by the
    `▶` glyph; the line may carry the §2.6 `durationPulse` ambient pulse (the
    one sanctioned ambient loop — §5 Q3).
  - **blocked** — `cardAgentLineBlocked` (status red) text, prefixed by the
    `◆` glyph; static (a blocked agent is *not* working, so no pulse).

| State | Visual |
|---|---|
| Default | `cardFill` body, `cardBorderWidth` (1) `cardBorder` |
| Hover | `cardFillHover` body; cursor pointer |
| Selected | `cardBorderWidthAccent` (2) `cardBorderSelected` (brand blue) border — the card whose detail panel (§3.12) is open |
| Blocked | `cardBorderWidthAccent` (2) `cardBorderBlocked` (status red) border + the blocked agent line; the red-accent card in the reference |

`selected` and `blocked` can co-occur (a blocked card can be the open one) —
the selected blue border wins on the border; the red `◆` glyph + agent line
keep blocked legible (glyph, not colour alone).

### 3.12 Docked detail panel — *contract (design-system-006)*

The right-edge panel that opens when a task card is selected (§3.11
`selected`). It **docks** against the viewport's right edge (screen-space, so
it does not pan with the canvas), `panelWidth` (340), full viewport height,
`panelFill` body, a `panelBorderWidth` (1) `panelBorder` along its left edge,
`panelRadius` (12) on the inner corners, `panelPadding` (20) inner.

- **Header** — `panelHeaderHeight` (52): the task **id** (`fontFamilyMono`,
  `panelHeaderTextMuted`) + a **status pill** (the §3.4 badge —
  `statusColor` + `statusGlyph`) + the **tags**. The task title sits just
  below in `panelHeaderText`, `sizeTitle`, `weightBold`.
- **Body** — the task's prose (the `## Why` / `## What` / acceptance
  criteria a refined task carries), rendered at the **reader pins**:
  `sizePanelBody` (14px Inter) at `lineHeightPanelBody` (1.65) — denser-
  reading than the 12px canvas chrome, matching the canvas-009 reader
  treatment. `panelBodyText` colour.
- **"AGENT NEEDS AN ANSWER" callout** — *only when the task is `blocked` on a
  question.* A boxed region, `panelCalloutFill` (warm tint),
  `panelCalloutBorderWidth` (1) `panelCalloutBorder` (brand orange),
  `panelCalloutRadius` (10), `panelCalloutPadding` (14). A small uppercase
  label "AGENT NEEDS AN ANSWER" in `panelCalloutLabel` (brand orange),
  `sizeCaption` `weightBold`, then the agent's question in `panelCalloutText`.
  Below, an action row of buttons (`panelButtonHeight` 30,
  `panelButtonRadius` 6):
  - **answer** — primary, `panelButtonPrimaryFill` (brand orange) +
    `panelButtonPrimaryText`.
  - **defer**, **edit** — secondary, `panelButtonSecondaryFill` +
    `panelButtonSecondaryText`.

  The callout reuses the warm brand-orange family (the `missing`/`focusRing`
  warmth) deliberately: a blocked agent waiting on the human is the canvas's
  single most attention-worthy state, and warm-on-cool draws the eye without a
  klaxon.

| State | Visual |
|---|---|
| Closed | not rendered (off-screen right) |
| Open (running / idle task) | header + reader body, no callout |
| Open (blocked task) | header + reader body + the "AGENT NEEDS AN ANSWER" callout |

**Motion.** Slide-in from the right edge over `durationPanel` (320ms) with
`easePanel` (`cubic-bezier(0.16, 0.84, 0.36, 1)` — a soft settle that reads as
"snapping into the dock" without overshoot); slide-out reverses. The 320ms
duration matches the camera-navigation budget so primary motion has one
"feel"; the dedicated `easePanel` curve (distinct from the camera's
`easeStandard`) is the deliberate exception — see §5 Q11.

---

## 4. Patterns

### 4.1 Focus / hover affordance — *implemented*

Any interactive node (project tile, BC node) shows a `focusRing` ring on
pointer-over and while dragged. `durationAffordance` is the intended fade
duration for the eventual tween.

### 4.2 Camera affordances — *zoom-to-fit implemented*

- **Zoom-to-fit** — press **`F`**. Frames the project tile + all BC nodes
  within the viewport with a margin, via an eased `durationCamera` transition
  (`Camera.fitTo` + `Camera.lerpTo`, ease-out cubic). A manual pan/zoom/drag
  gesture cancels an in-progress transition.
- **Focus-on-tile** — *contract*: `Camera.fitTo` already accepts an arbitrary
  world box, so focusing a single tile is the same machinery with a one-node
  box. The `canvas` BC wires the trigger (click / keyboard nav).
- **Keyboard nav hints** — the corner status line shows "press F to fit". A
  fuller hint surface (command-palette-style) is `canvas` BC work.

### 4.3 Greybox baseline — superseded

The walking skeleton's greybox (plain `0x252540` / `0x2d2d2d` rectangles,
hard-coded sizes, no status, no fonts) is **replaced** by this styleguide
baseline. Downstream tasks migrate *from this document*, not from greybox —
greybox no longer exists in `Canvas.svelte`.

---

## 5. Open questions — resolved with defaults

Marco authorised proceeding with sensible defaults; he refines properly via a
dedicated design skill afterward. Each default is **overridable**.

### Q1 — Light mode required or optional? → **Optional, deferred. Dark-default.**

Reasoning: GUPPI is a single-user desktop canvas tool used in focused sessions;
a dark canvas keeps tile/status colour vivid and reduces eye strain. The
architect leaned dark-default. Shipping one coherent theme now beats shipping
two half-tuned ones. `tokens.css` is structured so light mode is *additive*
later — a `:root[data-theme='light']` block plus a light branch in `tokens.ts`
— with **no consumer changes**. **Override path:** add the light theme; no
canvas/feature code changes.

### Q2 — Tile shape? → **Rounded rectangle.**

Reasoning: rectangles carry a title + subtitle + status badge legibly at
small zoom; circles waste space and crowd text. Rounded corners
(`radiusTile 12`, `radiusBc 10`) soften them and read as "card". Project vs BC
distinction is carried by **size + border colour + warmth**, not by shape —
keeping shape uniform makes dense canvases scan cleanly. **Override path:**
change `radius*` / introduce a shape token; `makeNode` is the single call site.

### Q3 — Motion budget? → **Restrained.**

Reasoning: a canvas with hundreds of tiles becomes noisy fast. Budget: short
eased transitions for *navigation only* (`durationCamera`, `durationAffordance`),
**one** ambient loop reserved for the single "running" signal
(`durationPulse`, slow at 1600ms so it breathes rather than flashes), and
nothing else animates. The "running" pulse is a defined token; the baseline
renders the static badge — wiring the pulse tween is a small follow-up the
`canvas` BC can pick up. **Override path:** adjust `motion.*` durations, or
add tokens for more motion if Marco wants a livelier canvas.

### Q4–Q8 — Project-frame aesthetics (design-system-002) → **defaults shipped, sign-off deferred**

The project-as-frame vocabulary (§3.6 / §3.7 / §3.8) carried five open
aesthetic questions. Per the styleguide-001 pattern, defaults are
shipped and `canvas-007` builds against them; Marco's in-person sign-off
is a deferred human gate (recorded in the task's `done/` note). Each
default is **overridable**.

| # | Question | Default shipped | Override path |
|---|---|---|---|
| Q4 | Frame border — solid vs dashed vs inset shadow? | **1px solid `frameBorder`**, no shadow | change `borderWidthFrame` or swap the stroke style in the frame draw call |
| Q5 | Header bar — integrated (one-piece) or attached (above)? | **integrated**, divided from body by `frameHeaderDivider` 1px | drop the divider line and detach the header geometry |
| Q6 | BC bubble shape — circle (like orbit) or rounded rectangle? | **rounded rectangle**, `radiusBcInside` 8, denser than orbit | change `radiusBcInside` to half of `bcInsideHeight` for a pill / capsule |
| Q7 | ACL notch — triangle on the line, zig-zag, or hover-only label? | **filled triangle at midpoint**, `aclNotchSize` 10, pointing toward upstream | swap `drawNotchTriangle` for a zig-zag polyline or remove and gate on hover |
| Q8 | Edge colour — single neutral, or per-type hues? | **single neutral `hairlineStrong`** (`#3a3b46` — was `fgMuted` until design-system-003), geometry distinguishes types | change `edgeUpstream` / `edgeMutual` / `edgeACL` / `edgeConformist` to distinct hues; consumers already reference the named tokens |

**Hover-on-edge highlight** (extra deferred question): until Marco
decides, edges have no hover state. The `edgeHighlight` token is in
place if the decision goes "yes".

### Q9 — Brand-accent palette → **brand orange `#ff8b00` (warm) + brand blue `#25abfe` (cool)**

Decided by Marco on 2026-05-16 in the `claude.ai/design` session; the
full handoff bundle sits at `references/claude-design-2026-05-16/`
(chat transcript at `chats/chat1.md`, design tokens at
`project/guppi-tokens.css`).

**Reasoning.** The orbit-baseline placeholder pair (periwinkle `#8a8ad0`
for the project tile, teal `#3c8b8e` for the BC node) shipped without
brand identity attached — they were chosen for legibility, not signal.
Marco's design session locked in:
- **Brand orange `#ff8b00`** — warm anchor; project frame border,
  project tile border, edge highlight, the `missing` status hue, and the
  `focusRing` (`#ffb05a`, a warm tint of the same family).
- **Brand blue `#25abfe`** — cool secondary; BC borders (both orbit and
  inside-frame), the `running` status hue, `voiceListening`.

The two hues pair: warm/cool, orange/blue at near-complementary angles
on the wheel. The status palette folds into the brand palette (running
= brand blue, missing = brand orange) so the canvas reads as one
coherent system rather than two parallel ones.

**Override path.** Swap the four hue tokens (`tileBorder`, `bcBorder`,
`frameBorder`, `bcInsideBorder`) and the two status tokens
(`statusRunning`, `statusMissing`) plus the matching glow strings.
Everything else cascades. There is **no styleguide ADR** for this — the
prior styleguide ADRs concern structural choices (TS-canonical + CSS
mirror; status-palette colourblind contract; restrained-motion budget),
and Marco's hue selection doesn't conflict with any of them; this §5
entry is the durable record (matching the `design-system-002` pattern
where Q4–Q8 also live as §5 entries, not as ADRs).

### Q10 — Status-glow capture → **RGBA strings, top-level `glow` group**

Each status has a matching halo used by the `running`-pulse keyframe and
any ambient badge glow. Two capture options were available: extend each
`statusX` Pixi-numeric colour with a paired numeric + alpha, or capture
the four halos as RGBA strings under a new top-level token group.
**Resolved default: RGBA strings under `export const glow`.** The CSS
overlay layer is the primary consumer (the `g-pulse-running` keyframe
reads `var(--guppi-status-running-glow)` directly as a `box-shadow`
colour), and a 1:1 string copy from `tokens.ts` to `tokens.css` keeps
the dual-file contract trivial. If a PixiJS draw call ever needs the
glow, it can parse the RGBA string at the call site (small cost, one
helper) or — preferred — receive a numeric + alpha pair derived from
the same RGBA source. **Override path:** add a numeric+alpha variant
to the `glow` group when the first PixiJS consumer lands.

### Q11 — Kanban-accordion interior defaults (design-system-006) → **defaults shipped, sign-off deferred**

The canvas pivot (`canvas-019`, ADR-017) introduced the DOM-overlay interior:
the BC **accordion row** (§3.9), the **kanban board** (§3.10), the **task
card** (§3.11), and the docked **detail panel** (§3.12). Per the
`design-system-002` pattern, defaults are shipped and the consumers
(`canvas-020/021/022`, `agent-awareness-002`) build against them; Marco's
in-person sign-off happens when `canvas-020` first runs in `pnpm tauri dev`.
Each default is **overridable**.

| # | Question | Default shipped | Override path |
|---|---|---|---|
| Q11a | Detail-panel motion — reuse the camera ease, or its own? | **Own ease.** `durationPanel` reuses the 320ms primary-motion budget (one "feel" for primary motion) but pairs with `easePanel` `cubic-bezier(0.16, 0.84, 0.36, 1)` — a soft decelerate-into-place rather than the camera's `easeStandard`. A panel docking against an edge wants a settle, not a navigation ease. | point `durationPanel`/`easePanel` at `durationCamera`/`easeStandard` to collapse them back into one curve |
| Q11b | Accordion collapse duration | **240ms** (`durationAccordion`), just under the panel budget so the row reflow reads quick; shares `easePanel` | adjust `durationAccordion` |
| Q11c | Column width behaviour | **fluid 200–280px** (`kanbanColumnMin/MaxWidth`), row scrolls horizontally past four columns | fix a single column width, or change the min/max band |
| Q11d | Card title overflow | **wrap to 2 lines then ellipsis-clamp** (full title in the panel) | change the clamp line-count |
| Q11e | Callout accent | **warm brand-orange family** (`panelCalloutBorder`/`Label` = `#ff8b00`), matching the `missing`/`focusRing` warmth — the blocked-on-human state is the highest-attention state and warm-on-cool draws the eye | swap the callout accent to `statusBlocked` red if Marco prefers the question read as an alarm rather than an invitation |
| Q11f | Interior substrate | **DOM/HTML overlay** for the interior, PixiJS for the frame shell (canvas-019 / ADR-017). The interior tokens are therefore **screen-space px**, not world-space-at-zoom-1 like the orbit tokens | n/a — substrate decided in ADR-017, not here |

The interior tokens land in **both** `tokens.ts` (so the dual-file contract
holds and a future Pixi consumer can read them) **and** `tokens.css` (the DOM
overlay's actual consumer), exactly as the orbit tokens do. The status palette
(§2.2) and the §2.6 motion tokens are **reused** by the interior rather than
duplicated — the card status glyph, the roll-up pills, and the agent-line
running pulse all consume the existing `statusColor`/`statusGlyph`/`glow` +
`durationPulse`.

**No styleguide ADR.** Like Q4–Q10, this is an aesthetic/default-selection
entry, not a structural decision that conflicts with a prior styleguide ADR
(TS-canonical + CSS mirror; status-palette colourblind contract;
restrained-motion budget all hold — the interior obeys all three). The
substrate decision itself is ADR-017 (`canvas-019`), not a design-system ADR.

---

## 6. Frontend gate

Per this BC's README and the styleguide task's critical gate: **no frontend
feature task in any BC is promoted to `doing/` before this styleguide is
closed and signed off.** `model` should fail fast on any frontend task that
lacks `depends_on: [design-system-001-styleguide]`. Each frontend-bearing BC's
README must reference this completed styleguide (follow-up — see the task
Outcome).
