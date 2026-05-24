# Kanban-accordion interior sketch — for design-system-006 sign-off

Non-rendering reference. Fixes the layout vocabulary for the canvas-pivot
interior (BC accordion row → kanban board → task card → docked detail panel)
so Marco can react to the **defaults** before `canvas-020/021/022` and
`agent-awareness-002` build against them. The pixel-true visual anchor is
`references/kanban.png`; this sketch fixes the *vocabulary*, not the layout.

Substrate: the project frame **shell** stays in PixiJS; everything below the
frame header is a **DOM/HTML overlay** (`canvas-019` / ADR-017). So the
dimensions in §2.5 for this interior are **screen-space px**, not
world-space-at-zoom-1.

## A project frame with its BC accordion rows

```
+- image-gallery   ▶1  ◆2  ○1     drag a BC header to reorder -----------------+   +========================+
|                                                                              |   | thumbnails / #13       |
|  v  media        ▶1 ◆1                                            6 tasks     |   |                        |
|  +----------------------------------------------------------------------+    |   | ◆ BLOCKED   thumb-gen  |  ← header (§3.12)
|  | BACKLOG          | TODO              | DOING            | DONE        |    |   |   id · status pill · tags
|  | #11 Validate img | #14 Introduce     | #17 Implement    | #19 Migrate |    |   |                        |
|  |     MIME magic   |     StorageBackend|     multipart    |     from raw|    |   | Choose thumbnail       |  ← title (sizeTitle)
|  | #12 EXIF orient  |     port          |  [thumb-gen ▶3m] | #20 Add img |    |   | generation library     |
|  |                  |                   |     ▶ orchestr.  |     endpoint|    |   |                        |
|  +----------------------------------------------------------------------+    |   | +--------------------+ |  ← callout (§3.12)
|                                                                              |   | | AGENT NEEDS AN     | |     panelCalloutBorder
|  >  thumbnails   ◆1                                               3 tasks    |   | | ANSWER             | |     (brand orange)
|  >  albums       ▶1 ◆1                                            4 tasks    |   | | Use sharp (native, | |
|  >  sharing      ○                                               4 tasks    |   | | ~4x faster) or jimp| |
|  >  uploads      ○                                               2 tasks    |   | | (pure-JS portable)?| |
|  >  users        ✕                                               1 task     |   | |                    | |
|                                                                              |   | | [answer] [defer]   | |  ← buttons (§3.12)
+------------------------------------------------------------------------------+   | +--------------------+ |     primary / secondary
                                                                                   |                        |
   ^ frame SHELL = PixiJS (§3.6 header + border)                                   | The project doesn't   |  ← reader body
   v accordion rows + kanban + cards = DOM overlay (§3.9-3.11)                      | ship native deps ...   |     14px / lh 1.65
                                                                                   |                  [edit]|
                                                                                   +========================+
                                                                                     ^ docked panel (§3.12), slides
                                                                                       in from the right edge
```

### Reading the sketch

- **Frame header** (`+- image-gallery ▶1 ◆2 ○1 -+`) — unchanged §3.6 PixiJS
  shell: project name + per-state status badges. The interior below is DOM.
- **BC accordion row** (§3.9) — `v media` is **expanded** (chevron down),
  the rest are **collapsed** (chevron `>`). Each header carries the BC name,
  the roll-up pills (`▶1 ◆1` — glyph + count per non-zero state, in
  `statusColor`), and a right-aligned `N tasks` total in mono.
- **Roll-up "1 active · 2 blocked · 1 idling"** — in the reference image this
  is the project-level roll-up at the frame header; the same pill vocabulary
  recurs per accordion row.
- **Kanban board** (§3.10) — the four columns `BACKLOG · TODO · DOING · DONE`
  in fixed order, inside the expanded row body. Scrolls horizontally past four.
- **Task card** (§3.11) — `#11 Validate img MIME magic`: id (mono) + title
  (clamp 2 lines) + optional tag chips. The `DOING` card carries the
  **live-agent indicator line** `▶ orchestr.` / `[thumb-gen ▶3m]`
  (`cardAgentLineRunning`, brand blue, may pulse).
- **Blocked card** — a card whose task is blocked gets the
  `cardBorderBlocked` red border + a `◆`-prefixed `cardAgentLineBlocked`
  agent line (static, no pulse — a blocked agent isn't working).
- **Selected card** — the one whose detail panel is open gets the
  `cardBorderSelected` brand-blue 2px border.
- **Docked detail panel** (§3.12) — `panelWidth` 340, docked to the right
  viewport edge (screen-space). Header = id + status pill + tags + title;
  body = the task prose at the reader pins (14px / 1.65); the
  **AGENT NEEDS AN ANSWER** callout appears only for a `blocked` task, with
  `answer` (primary, brand orange) / `defer` / `edit` (secondary) buttons.

## Collapsed vs expanded accordion row

```
collapsed:   >  thumbnails   ◆1                                   3 tasks
                ^chevron      ^roll-up pills (statusColor+glyph)   ^total (mono)

expanded:    v  media        ▶1 ◆1                                6 tasks
             +--------------------------------------------------------------+
             | BACKLOG  | TODO     | DOING            | DONE               |   ← kanban board
             | ...cards | ...cards | ...cards         | ...cards           |
             +--------------------------------------------------------------+
```

Collapse/expand animates the body height over `durationAccordion` (240ms),
`easePanel`; the chevron rotates on the same timing.

## Empty column

```
| DONE               |
| ─                  |   ← kanbanColumnEmptyText placeholder, sizeCaption,
|                    |     centred; no control, no animation
```

## What the sketch is *not*

A layout reference, not a pixel-true mock. Column widths flex (200–280px),
row order is user-reorderable, force-fit and scroll behaviour are the
consuming canvas tasks' call. The sketch fixes the **visual vocabulary** —
which chrome lives where, which token dresses it, which states a card and a
panel have — so `canvas-020/021/022` and `agent-awareness-002` have
unambiguous defaults to build against. `kanban.png` is the visual anchor for
the pixel feel.
