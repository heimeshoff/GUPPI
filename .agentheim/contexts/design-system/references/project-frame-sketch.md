# Project-frame sketch — for design-system-002 sign-off

Non-rendering reference. Shows the frame + interior BCs + the four edge
types side by side, so Marco can react to the *defaults* before
`canvas-007` builds against them.

## Single project, four edge types in one frame

```
+- guppi (▶3  ◆1  ✕0)  [tasks: 47] -----------------------------------+
|                                                                     |
|     +-----------------+                +-----------------+          |
|     | brainstorm   ▶2 |--upstream-CS-->| model        ◆1 |          |
|     +-----------------+                +-----------------+          |
|              |                                 |                    |
|              | mutual                          | upstream-CS        |
|              v                                 v                    |
|     +-----------------+                +-----------------+          |
|     | canvas       ▶1 |<====ACL====<>=| design-system ▶0 |          |
|     +-----------------+                +-----------------+          |
|              :                                                      |
|              :  conformist (lighter)                                |
|              :                                                      |
|              v                                                      |
|     +-----------------+                                             |
|     | infrastructure ○ |                                            |
|     +-----------------+                                             |
|                                                                     |
+---------------------------------------------------------------------+
```

### Reading the sketch

- **Frame** — `frameBorder` 1px around the whole region; the top edge
  `+- guppi (...) -+` is the header bar (`frameHeaderHeight` 24,
  `frameHeaderFill`). Project name on the left, status badges next, task
  count right-aligned in mono.
- **BC bubble** — `bcInsideWidth × bcInsideHeight` (160 × 56),
  `radiusBcInside` 8. BC name + counts pill in one row at default zoom;
  the status glyph in the corner is the per-state badge.
- **Edge: customer-supplier (`upstream-CS-->`)** — `edgeUpstream`,
  `edgeWeight` 2, arrowhead at downstream end.
- **Edge: mutual / shared-kernel** — bare line, no arrowhead.
- **Edge: anti-corruption-layer (`<==ACL==<>=`)** — `edgeACL`, arrowhead
  + the `<>` triangle notch glyph at the midpoint pointing toward the
  upstream end.
- **Edge: conformist (dotted in the sketch — lighter in render)** —
  `edgeConformist`, `edgeWeightConformist` 1, still has an arrowhead.

## Empty-frame state

```
+- new-project (no tasks yet) ----------------------------------------+
|                                                                     |
|                                                                     |
|              No bounded contexts yet — add a contexts/<bc>/         |
|              directory to populate.                                 |
|                                                                     |
|                                                                     |
+---------------------------------------------------------------------+
```

The placeholder line in `frameEmptyText` reads as a hint; no controls,
no animation.

## What the sketch is *not*

This is a layout reference, not a pixel-true mock. Force-directed
positioning is `canvas-007`'s call; the sketch fixes the **visual
vocabulary** (border weights, edge geometry, header chrome, badge
slots) so the rendering task has unambiguous defaults to build against.
