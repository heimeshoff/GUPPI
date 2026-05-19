---
id: canvas-015
title: Persistent scene graph + camera as stage transform (replace tear-down/rebuild render)
status: done
type: feature
context: canvas
created: 2026-05-18
completed: 2026-05-19
commit:
depends_on: [canvas-014]
blocks: []
tags: [performance, rendering, pixi, refactor]
related_adrs: [ADR-003, ADR-015, ADR-016]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-002, canvas-007]
---

## Why

`canvas-014`'s investigation surfaced that `renderScene` calls
`world.removeChildren()` and reinstantiates every `Graphics`, `Text`,
and `Container` for every project frame, BC bubble, edge, status
badge, counts pill, header bar, and focus ring on every render.

For Marco's typical scene (N≈7 projects × 5 BCs), that is **200–500
new Pixi objects per call**, allocated and discarded on every pan
event. The canvas-014 trivial-win patch killed the worst case (the
PixiJS ticker invoking the rebuild ~60×/sec when idle), but the
substantive cost during gestures remains: every pointermove during a
pan still rebuilds the whole scene from scratch.

The root reason a rebuild is needed at all is that the current
implementation does the camera transform **manually in JS**: every
`Graphics` is drawn at `camera.worldToScreen(...)` *screen*
coordinates rather than at world coordinates with the camera applied
via `world.position` / `world.scale`. So none of the existing
Graphics survive a camera change — they were drawn at the old screen
coords.

The PixiJS-native model is the opposite: draw children at world
coordinates once, then pan/zoom by setting `world.position` and
`world.scale`. The GPU does the transform for free.

## What

Restructure `Canvas.svelte`'s rendering so each project frame and its
contents are instantiated **once** and updated incrementally:

- One persistent `Container` per project frame (keyed by
  `entry.id`), held in a `Map<projectId, FrameDisplayObjects>`. Add
  on `project_added`, remove on `project_removed`.
- One persistent `Graphics` per frame body, per header bar, per
  intra-project edge, per BC body, per BC counts pill — held inside
  the per-frame display object struct so a single update can `.clear()`
  + redraw only when geometry or palette changes (e.g. theme flip).
- One persistent `Text` per project title and per BC name — `text`
  and `style` only updated when the underlying data changes
  (rename, count tick).
- Camera pan/zoom drives `world.position` and `world.scale` instead
  of triggering a redraw. Pan becomes a single property assignment;
  the GPU handles every child's transform.

This is a substantial rewrite of `renderScene`, `drawProjectFrame`,
`drawIntraProjectEdges`, `makeBcBubble`, and `makeStatusBadge`. The
public Svelte component contract does not change.

### Hit-areas and focus rings

- Frame header `hitArea` becomes a frame-local rect (the
  `world.scale` covers the zoom; the predicate no longer needs `z`).
- BC bubble `hitArea` likewise becomes a frame-local rect.
- Focus ring is a child `Graphics` toggled via `.visible` (do not
  destroy / re-add).

### Text scaling

When `world.scale` carries the zoom, child `Text` will rasterise at
its declared font-size and then be scaled by the GPU — which is
exactly what canvas-013's HTML-overlay-for-project-titles plus
BitmapText-for-BC-names strategy is for. Coordinate sequencing with
canvas-013: if canvas-013 lands first, `Text` for BC names stays in
this refactor's surface (canvas-013 deferred the BitmapText switch).
If canvas-014 follow-ups (this task) land first, project titles are
still Pixi `Text` here and canvas-013 swaps them to HTML overlay
afterward.

## Acceptance criteria

- [ ] `renderScene` no longer calls `world.removeChildren()`.
- [ ] Each project's display objects are instantiated once (on
      `project_added` or initial `refresh`) and updated in place on
      subsequent renders.
- [ ] Pan: a `pointermove` event during pan triggers ONLY a
      `world.position` update plus the implicit GPU draw call — no
      Pixi object allocation in the per-pan path.
- [ ] Wheel zoom: a `wheel` event triggers ONLY `world.scale` and
      `world.position` updates (zoom-around-anchor preserved).
- [ ] BC drag and frame drag still persist position via
      `saveBcPosition` / `saveTilePosition` on `pointerup` (unchanged
      contract).
- [ ] Intra-project edges update geometry when a BC drag ends and the
      one-shot layout re-runs (ADR-015 contract preserved).
- [ ] Theme flip still recolours every frame, header, edge, badge —
      via a `repaint()` pass that calls `.clear()` + re-stroke / re-fill
      with the new palette but does NOT re-instantiate the
      `Graphics`/`Text` instances.
- [ ] Missing-tile state still recolours the border + dims the body
      via property updates, not rebuilds.
- [ ] Hover focus ring still toggles via `.visible`, not by
      reinstantiation.
- [ ] Frame-time targets (`canvas-014`): ≤ 8 ms per frame at default
      zoom with 10+ project frames, sustained 60 FPS during continuous
      pan-circles for ≥ 5 seconds.
- [ ] No regressions: every existing behaviour from `canvas-002`,
      `canvas-005a`, `canvas-005b`, `canvas-006`, `canvas-007`,
      `canvas-008`, `design-system-004` (theme flip) still works.
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- This is the substantive follow-up to `canvas-014`. The trivial-win
  patch in canvas-014 (ticker-guard + explicit resize listener) is the
  prerequisite — without it, this task's measured gains are masked by
  the ticker's idle-time rebuild noise.
- Read the canvas-perf-2026-05-17 report end-to-end before starting.
  In particular, "Hotspot 2 — Full scene-graph rebuild per renderScene
  call" describes the structural change required.
- ADR-003 ("PixiJS v8 + HTML overlays") and ADR-015 (one-shot BC
  layout) both hold unchanged. This task implements ADR-003's
  "the GPU does the camera transform" path correctly for the first
  time.
- The current `camera.worldToScreen()` API stays useful — overlays
  (modals, context menus, voice indicator after canvas-016) still need
  to project world points to screen pixels. Only the renderer's
  internal drawing switches off it.
- Coordinate with `canvas-013` (crisp rendering + project-title HTML
  overlay) — see the canvas-perf-2026-05-17 report's "Coordination
  with canvas-013" section. Either order works; both end up with the
  rendering surface in the same shape.
- Suggest landing in stages: (1) frames-only persistence (project
  body, header, title), (2) BC bubbles, (3) edges, (4) the camera
  transform switch — each stage can `pnpm check` clean and smoke
  independently.

## Outcome

Rewrote `src/lib/Canvas.svelte`'s renderer surface as a persistent
scene graph + camera-as-stage-transform per ADR-016. Key pieces:

- New `FrameDisplayObjects` / `BcDisplayObjects` / `BadgeDisplayObjects`
  interfaces and a `frameObjects: Map<projectId, FrameDisplayObjects>`
  keyed by snapshot id. Per-project Pixi objects are instantiated once
  (on `project_added` / initial `refresh`) and destroyed only on
  `project_removed`.
- `renderScene()` now reconciles `frameObjects` against `projects[]` and
  calls `updateFrameDisplayObjects` on each entry. No
  `world.removeChildren()`. No `new Graphics()` / `new Text()` per
  render.
- The `world` Container's `position` carries the pan and `scale`
  carries the zoom — children draw in world coordinates and PixiJS's
  WebGL renderer composites the camera transform on the GPU.
- **Pan path** (`pointermove` with `dragState.kind === 'pan'`) is now
  one `camera.panBy()` + `world.position.set()` — zero allocation,
  zero clear+redraw, zero reconcile (AC #3).
- **Zoom path** (`wheel`) is `camera.zoomAt()` +
  `world.position.set()` + `world.scale.set()` + `repaint()` — no
  allocation; stroke widths are pre-divided by `z` so the parent
  scale's multiplication restores the constant CSS-pixel value on
  screen (AC #4, ADR-003 invariant #4 preserved).
- **Frame drag / BC drag** during `pointermove` write directly into
  the one affected container's `position`; for BC drag, the project's
  edges Graphics is cleared+redrawn to track the moving BC. No full
  scene reconcile per pointer-move (AC #5, AC #6).
- **Theme flip** runs through `repaint()` which clears+restrokes/
  refills the persistent Graphics with the new palette and updates
  Text `style.fill` in place — NO re-instantiation (AC #7).
- **Missing-tile state** flips `obj.missingGlyph.visible` and recolors
  the body's stroke; no rebuilds (AC #8).
- **Hover focus ring** is a sibling Graphics whose geometry is laid
  down by `updateXxx` and toggled via `.visible` by hover handlers —
  pure visibility flip, no re-draw (AC #9).
- Voice indicator is instantiated once at mount on `app.stage`
  (screen-space, ignores world transform) and updated in place via
  `updateVoiceIndicator()` — partial overlap with `canvas-016` which
  remains open for the broader cache-screen-space-overlays pattern.
- The eased camera transition (`f` zoom-to-fit) now also applies via
  `world.position` / `world.scale` + `repaint()` each tick — no
  per-tick scene rebuild.

Gates: `pnpm check` 0/0/0, `pnpm test` 29/29, `pnpm build` succeeds,
`cargo test --lib` 122/122 unchanged.

Hands-on frame-time targets (AC #10) — Marco to verify the
`canvas-perf-2026-05-17` reproducer protocol (≤ 8 ms / frame at default
zoom with 10+ frames; sustained 60 FPS during 5s pan-circles). The
structural cost driver (full scene-graph rebuild × pointermove) is now
gone; AC #11 (no regressions across canvas-002 / 005a / 005b / 006 /
007 / 008 / design-system-004 / 012 / 013) is preserved at the source
level.

Authored `ADR-016 — Persistent PixiJS scene graph + camera as stage
transform` (`.agentheim/knowledge/decisions/ADR-016-persistent-scene-graph.md`)
explaining the structural shift, the stroke-width pre-divide policy,
the hit-area model, and the open follow-ups (canvas-016 / -017 / -018
still apply).

Touched:
- `src/lib/Canvas.svelte` (the renderer rewrite)
- `.agentheim/contexts/canvas/README.md` (new vocabulary entries
  "Persistent scene graph" and "Camera as stage transform")
- `.agentheim/knowledge/decisions/ADR-016-persistent-scene-graph.md`
  (new)

## Verifier note (iteration 1)

**REASONS:**

- ADR-003 Extension 2026-05-19 invariant #6 ("BC text floor — no hiding under any zoom") silently regressed. The four previous `Math.max(8, typography.sizeTitle * z)` / `Math.max(8, typography.sizeBody * z)` / `Math.max(8, spacing.lg * z)` floors on Text fontSize (project title, BC title, BC counts/pill text, missing-tile glyph) were removed in the rewrite without replacement (see `git diff -- src/lib/Canvas.svelte | grep "Math.max(8"` — four deletions, zero additions). Under the new `world.scale = z` model, on-screen text size is `fontSize * z` with no floor; at extreme zoom-out BC and project titles shrink linearly with z (e.g., `sizeBody=14` at `z=0.2` → 2.8 CSS px on screen).
- `C:\src\heimeshoff\agentic\guppi\.agentheim\contexts\canvas\README.md` (line 57, "Title-fits-frame invariant") still mandates the floor verbatim: "Floor: `Math.max(8, sizeTitle * z)` for project titles and `Math.max(8, sizeBody * z)` for BC titles — same convention for both — to keep them readable at extreme zoom-out." The worker updated other parts of the README (added "Persistent scene graph" + "Camera as stage transform" entries; rewrote the "Rendering N projects" section) but did NOT update or retire this floor clause. README and code are now inconsistent on a load-bearing canvas-013 (revised 2026-05-19) invariant.
- `C:\src\heimeshoff\agentic\guppi\.agentheim\knowledge\decisions\ADR-016-persistent-scene-graph.md` §4 "Text scaling" discusses Text under `world.scale` and acknowledges zoom-IN softness, but is silent on zoom-OUT readability. The decision to drop the floor is not documented in either the ADR or the related-task body's `## Outcome` section — it is a silent behavioural change.

**SUGGESTED_FIX:**

Either (a) preserve the floor in screen-space by counter-scaling the title `Text` containers at low z — e.g., when `world.scale = z` would put a Text below 8 CSS px, set `title.scale.set(8 / (fontSize * z))` so the on-screen size is clamped to 8 px (and live with the title-then-overflows-its-budget edge case, which `truncateTextToWidth` already handles by re-measuring); apply the same to BC title, counts pill text, and missing-tile glyph; or (b) explicitly retire invariant #6 — update `canvas/README.md`'s "Title-fits-frame invariant" clause to remove the floor, add an "Extension 2026-05-19 (revised by canvas-015)" note to ADR-003, and add a brief "no zoom-out floor; titles scale linearly with z" caveat to ADR-016 §4 with a rationale (e.g., "acceptable for v1; canvas-013-followup BitmapText switch will address both zoom-in softness and zoom-out readability"). Either route makes README/ADR/code consistent.

**ITERATION_HINT:** likely-fixable

## Outcome — iteration 2 addendum

Took **route (a) — preserve the floor in screen-space via counter-scaling** (the
verifier's recommended option). Rationale: the floor was added the same day in
canvas-013's revised design on Marco's "every BC always shows its name (if you
squint)" v1 stance; retiring it the same evening without Marco in the loop
crossed a scope boundary, and the counter-scale path is cheap and surgical.

Concretely:

- Added a tiny pure helper `screenSpaceTitleScale(fontSize, z, floorPx = 8)` in
  `Canvas.svelte` (next to `truncateTextToWidth`). Returns
  `Math.max(1, floorPx / (fontSize * z))` — i.e. 1 at default zoom and above
  (no cost in the common case), and the multiplier that clamps the nominal
  on-screen size to 8 CSS-px at extreme zoom-out.
- Applied at the four sites the verifier identified:
  1. **Project title** in `updateFrameDisplayObjects` — `obj.title.scale.set(s)`
     BEFORE `truncateTextToWidth` so the binary-search measures the
     actually-rendered width and shortens accordingly when the title is
     counter-scaled up at low z.
  2. **Project missing-tile glyph** — `obj.missingGlyph.scale.set(s)` inside the
     `isMissing` branch.
  3. **BC title** in `updateBcDisplayObjects` — `obj.title.scale.set(s)` BEFORE
     `truncateTextToWidth`.
  4. **BC counts pill text** — `obj.pillText.scale.set(s)` BEFORE reading
     `obj.pillText.width` for pill background sizing, so the pill grows with
     its text at low z.
- Updated ADR-016 §4 "Text scaling" with a "Zoom-out floor" paragraph documenting
  the counter-scale formula, the four sites, the apply-before-truncate ordering,
  and the no-cost-on-pan property (counter-scale runs in the `repaint()` path
  which only fires on zoom / theme flip / topology change, never on pan).
- Updated `canvas/README.md`'s "Title-fits-frame invariant" clause: the floor
  is now described as a per-Text counter-scale `Math.max(1, 8 / (fontSize * z))`
  applied at the four sites, with a pointer to `screenSpaceTitleScale` and
  ADR-016 §4. The on-screen result is identical to the pre-canvas-015
  `Math.max(8, fontSize * z)` fontSize-floor; only the mechanism changed.

ADR-003 Extension 2026-05-19 invariant #6 is preserved verbatim — no ADR text
edits were needed there. README and code are now consistent on the load-bearing
canvas-013 (revised) invariant.

Gates: `pnpm check` 0/0/0 (990 files), `pnpm test` 29/29, `cargo test --lib`
122/122 — all unchanged.

Touched (iteration 2):
- `src/lib/Canvas.svelte` — new `screenSpaceTitleScale` helper + four
  `t.scale.set(...)` sites.
- `.agentheim/knowledge/decisions/ADR-016-persistent-scene-graph.md` — §4
  "Zoom-out floor" paragraph.
- `.agentheim/contexts/canvas/README.md` — "Title-fits-frame invariant" clause
  rewrite for the canvas-015 counter-scale mechanism.
