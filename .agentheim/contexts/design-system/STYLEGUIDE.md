# GUPPI Styleguide

The visual vocabulary for GUPPI's canvas. Every frontend feature task in every
bounded context implements against this document — it is the contract that
keeps the UI coherent.

**Status:** code-complete baseline, pending Marco's in-person sign-off and his
design-skill refinement pass. The open questions below have *defensible
defaults* chosen so frontend work is unblocked; Marco can override any of them.

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
| Project tile | `tileBorder` | `#8a8ad0` | Project tile border — periwinkle |
| Project tile | `tileText` / `tileTextMuted` | `#f2f2f7` / `#b9b9c8` | Title / subtitle |
| BC node | `bcFill` | `#20242b` | BC node body — calmer, cooler |
| BC node | `bcBorder` | `#3c8b8e` | BC node border — teal |
| BC node | `bcText` / `bcTextMuted` | `#e6e6ec` / `#9a9aa6` | Title / subtitle |
| Edge | `edge` | `#4a4a58` | Project → BC connectors |
| Edge | `edgeHighlight` | `#8a8ad0` | Connector when its endpoint is focused |
| Affordance | `focusRing` | `#c8c8ff` | Ring drawn around a hovered/focused node |

### 2.2 Status palette — colourblind-friendly

Four discrete states. The palette is built so it survives deuteranopia and
protanopia: the states differ in **hue *and* lightness**, and **each pairs
with a distinct glyph** — colour is never the only signal.

| State | Token | Colour | Glyph | Meaning |
|---|---|---|---|---|
| `idle` | `statusIdle` | `#6f7585` grey-blue | `○` hollow circle | At rest — nothing in flight |
| `running` | `statusRunning` | `#2f9fe0` bright blue | `▶` play | Work in progress |
| `blocked` | `statusBlocked` | `#e6a020` amber | `◆` diamond | Blocked on a question |
| `missing` | `statusMissing` | `#d05a8a` magenta-pink | `✕` cross | Expected but absent |

`statusText` (`#10131a`) is the dark text/glyph colour that sits *on top* of a
status fill (badges). `statusColor`, `statusGlyph`, `statusLabel` maps in
`tokens.ts` keep the mapping in one place — consume those, don't re-derive.

### 2.3 Typography — one family, three sizes

| Token | Value | Use |
|---|---|---|
| `fontFamily` | Inter → Segoe UI → system-ui stack | All UI text. Zero web-font payload. |
| `fontFamilyMono` | Cascadia Code → Consolas → system mono | Paths, task counts, terminal panels |
| `sizeTitle` | `16px` | Tile / BC node titles |
| `sizeBody` | `12px` | Subtitles, body copy |
| `sizeCaption` | `10px` | Task counts, badge labels, hints |
| `weightRegular / Medium / Bold` | `400 / 500 / 700` | — |

Sizes are world-space px **at zoom 1**; canvas code multiplies by the camera
zoom and floors at a legible minimum.

### 2.4 Spacing — 4px base scale

`xs 4` · `sm 8` · `md 12` · `lg 16` · `xl 24` · `xxl 32` (px, world-space at
zoom 1). Use the scale; don't invent intermediate values.

### 2.5 Shape — geometry

| Token | Value | Use |
|---|---|---|
| `tileWidth` × `tileHeight` | `240 × 132` | Project tile — larger, the anchor |
| `bcWidth` × `bcHeight` | `184 × 96` | BC node — smaller, secondary |
| `bcOrbitRadius` | `380` | World distance of BC nodes from the project tile |
| `radiusTile` / `radiusBc` / `radiusBadge` | `12 / 10 / 6` | Corner radii |
| `borderWidth` / `borderWidthFocus` | `2 / 3` | Border weights |
| `badgeHeight` / `badgeMinWidth` | `18 / 18` | Status badge pill |

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

---

## 3. Components — states

Each visual element and its discrete states. "Implemented" = present in the
`Canvas.svelte` baseline; "contract" = tokens + shape defined, full behaviour
is downstream BC work.

### 3.1 Project tile — *implemented*

The canvas anchor. Primary hierarchy: larger geometry, warm `tileBorder`,
`fontFamily` bold title + mono path subtitle.

| State | Visual |
|---|---|
| Default | `tileFill` body, `tileBorder` 2px border |
| Hover / focus | adds a `focusRing` 3px ring just outside the border |
| Dragging | same as focus; world position persists on pointer-up (ADR-004) |

The project tile carries **no status badge** — status is a per-BC concept.

### 3.2 BC node — *implemented*

Secondary hierarchy: smaller geometry, cool `bcBorder`, same type scale at
lower weight. Carries a **status badge** (top-right corner) derived from task
counts.

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
| `listening` | `voiceListening` `#2f9fe0` | actively listening (matches `running` blue) — "mic" |
| `muted` | `voiceMuted` `#d05a8a` | mic unavailable / muted — "muted" |

The baseline renders the `idle` state. The **voice BC** wires real mic state
into `voiceState` later; this establishes the visual contract and the token
set so that work has something explicit to land against.

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

---

## 6. Frontend gate

Per this BC's README and the styleguide task's critical gate: **no frontend
feature task in any BC is promoted to `doing/` before this styleguide is
closed and signed off.** `model` should fail fast on any frontend task that
lacks `depends_on: [design-system-001-styleguide]`. Each frontend-bearing BC's
README must reference this completed styleguide (follow-up — see the task
Outcome).
