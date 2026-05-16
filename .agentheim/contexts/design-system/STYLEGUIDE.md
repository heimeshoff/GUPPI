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
| Project tile | `tileBorder` | `#8a8ad0` | Project tile border — periwinkle |
| Project tile | `tileText` / `tileTextMuted` | `#f2f2f7` / `#b9b9c8` | Title / subtitle |
| BC node | `bcFill` | `#20242b` | BC node body — calmer, cooler |
| BC node | `bcBorder` | `#3c8b8e` | BC node border — teal |
| BC node | `bcText` / `bcTextMuted` | `#e6e6ec` / `#9a9aa6` | Title / subtitle |
| Edge | `edge` | `#4a4a58` | Project → BC connectors (legacy / orbit baseline) |
| Edge | `edgeHighlight` | `#8a8ad0` | Connector when its endpoint is focused |
| Edge | `edgeUpstream` / `edgeMutual` / `edgeACL` / `edgeConformist` | `#6e6e80` | Intra-project edges — single neutral palette, geometry distinguishes types (see §3.8) |
| Edge | `fgMuted` | `#6e6e80` | The underlying neutral hue all four intra-project edges resolve to |
| Frame | `frameFill` / `frameBorder` / `frameHeaderFill` / `frameHeaderDivider` | `#1a1a22` / `#8a8ad0` / `#262636` / `#8a8ad0` | Project frame body, border, header bar (see §3.6) |
| Frame | `frameTitleText` / `frameTitleTextMuted` / `frameEmptyText` | `#f2f2f7` / `#b9b9c8` / `#6e6e80` | Header text + empty-frame placeholder |
| BC bubble (inside frame) | `bcInsideFill` / `bcInsideBorder` / `bcInsideText` / `bcInsideTextMuted` / `bcInsidePillFill` | `#20242b` / `#3c8b8e` / `#e6e6ec` / `#9a9aa6` / `#2a2f38` | Inside-frame BC bubble + task-counts pill (see §3.7) |
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
| `tileWidth` × `tileHeight` | `240 × 132` | Project tile — larger, the anchor (orbit baseline) |
| `bcWidth` × `bcHeight` | `184 × 96` | BC node — smaller, secondary (orbit baseline) |
| `bcOrbitRadius` | `380` | World distance of BC nodes from the project tile (orbit baseline) |
| `radiusTile` / `radiusBc` / `radiusBadge` | `12 / 10 / 6` | Corner radii |
| `borderWidth` / `borderWidthFocus` | `2 / 3` | Border weights |
| `badgeHeight` / `badgeMinWidth` | `18 / 18` | Status badge pill |
| `radiusFrame` / `borderWidthFrame` | `12 / 1` | Project frame corner + border (see §3.6) |
| `frameHeaderHeight` / `framePadding` | `24 / 16` | Header bar height + inner body padding |
| `frameMinInnerWidth` / `frameMinInnerHeight` | `320 / 200` | Floor for auto-fit frame sizing |
| `bcInsideWidth` × `bcInsideHeight` | `160 × 56` | BC bubble inside a frame (denser than orbit BC node) |
| `radiusBcInside` | `8` | BC bubble corner radius (rounded rectangle, denser than orbit) |
| `bcInsidePillHeight` / `bcInsidePillMinWidth` / `bcInsidePillRadius` | `16 / 32 / 8` | Task-counts pill inline with the BC name |
| `edgeWeight` / `edgeWeightConformist` | `2 / 1` | Intra-project edge line weights |
| `arrowheadLength` / `arrowheadWidth` | `10 / 8` | Arrowhead at the downstream end of directional edges |
| `aclNotchSize` | `10` | Triangle notch at the midpoint of an ACL edge |

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
| Default | `frameFill` body, `frameBorder` 1px border, `radiusFrame` 12px corners, header bar in `frameHeaderFill` divided from the body by `frameHeaderDivider` 1px |
| Hover (over body) | no change — frame is a region, not an interactive node; hover affordance applies to the header bar only |
| Hover (header bar) | `focusRing` 3px halo around the header bar edges — "click here for project-level actions" |
| Dragging (header bar) | same as hover; the whole frame (header + body + interior BCs) follows the pointer. World position persists on pointer-up (ADR-004). |

**Header bar.** Height = `frameHeaderHeight` (24). Carries, left to right:
project name in `frameTitleText` at `sizeBody` `weightMedium`, then the
status badges (one badge per status state that has count > 0; same
`statusColor` / `statusGlyph` palette as a BC node), then the task-counts
row right-aligned in `frameTitleTextMuted` at `sizeCaption` `fontFamilyMono`.
Right-clicking the header bar opens the existing tile context menu
(`Remove project`, etc. — same surface as today's tile right-click).

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

### 3.7 BC bubble (inside frame) — *contract*

The BC bubble inside a frame is distinct from the orbit-baseline BC node
(§3.2). It is **denser** — title + counts pill in one row at default
zoom, lower height — because many BCs share a frame and the eye needs to
read the relationship graph between them, not each bubble's chrome.

Shape: a rounded rectangle (`radiusBcInside` 8), `bcInsideWidth × bcInsideHeight`
(160 × 56), `bcInsideFill` body, `bcInsideBorder` 2px stroke. Inside, in
one row at default zoom:

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

### 3.8 Intra-project edges — *contract*

Four edge variants, one per context-map relationship type from
`project-registry-004`. The project→BC parent edge of the orbit baseline
**retires**; containment (BC inside frame) replaces it.

**Resolved default: single neutral palette.** All four variants use the
same `fgMuted` hue (the named tokens — `edgeUpstream`, `edgeMutual`,
`edgeACL`, `edgeConformist` — resolve to the same value). **Geometry**
— arrowhead presence, notch glyph, line weight — carries the type
distinction. The reasoning matches the "restrained" motion budget: a
dense canvas reads cleanest when colour is consistent and shape varies.

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
  const colour = edgeColour[kind];          // all four → fgMuted today
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
| Q8 | Edge colour — single neutral, or per-type hues? | **single neutral `fgMuted`**, geometry distinguishes types | change `edgeUpstream` / `edgeMutual` / `edgeACL` / `edgeConformist` to distinct hues; consumers already reference the named tokens |

**Hover-on-edge highlight** (extra deferred question): until Marco
decides, edges have no hover state. The `edgeHighlight` token is in
place if the decision goes "yes".

---

## 6. Frontend gate

Per this BC's README and the styleguide task's critical gate: **no frontend
feature task in any BC is promoted to `doing/` before this styleguide is
closed and signed off.** `model` should fail fast on any frontend task that
lacks `depends_on: [design-system-001-styleguide]`. Each frontend-bearing BC's
README must reference this completed styleguide (follow-up — see the task
Outcome).
