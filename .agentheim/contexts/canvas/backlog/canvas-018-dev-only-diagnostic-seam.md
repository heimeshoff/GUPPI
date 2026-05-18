---
id: canvas-018
title: Dev-only diagnostic seam — expose Pixi Application + camera to the console
status: backlog
type: feature
context: canvas
created: 2026-05-18
completed:
commit:
depends_on: []
blocks: []
tags: [diagnostics, dev-only, debug-seam, profiling]
related_adrs: [ADR-003]
related_research: [canvas-perf-2026-05-17]
prior_art: [canvas-007]
---

## Why

`canvas-014`'s investigation needed renderer-internal state (`app.renderer.type`,
`renderer.resolution`, `app.ticker.FPS`, the WebGL GPU string) from the
Tauri devtools console to confirm which renderer was active and to
ground the analysis in real measured values. Today there is no
ergonomic way to reach into the running `Application` from the console
— the variable is closed over inside `Canvas.svelte`'s onMount IIFE.

For canvas-014, Marco patched a temporary global into the source for
the duration of the spike. The next time a perf or input spike comes
up (and there will be next times — canvas-015 / canvas-016 / canvas-017
will all want to verify their gains), we don't want to repeat the
"temporarily patch a global" dance.

## What

Add a tiny dev-only diagnostic seam in `Canvas.svelte` that, behind a
guard, attaches the live `Application` instance + the `camera` rune +
helper inspectors (current `projects.length`, `cameraTarget` state, a
`measureRender()` function that times one `renderScene()` call) to
`window` under a single namespace, e.g. `window.__guppi`.

The guard: only attach in development builds (`import.meta.env.DEV`).
In a production Tauri bundle the global must not exist. Test by
running `pnpm build` and verifying the bundled JS does not reference
`__guppi`.

API surface (initial):

```ts
window.__guppi = {
  app: Application,                      // the PixiJS Application
  camera: Camera,                        // the rune store
  projects: ProjectEntry[],              // current rendered set
  cameraTarget: CameraState | null,      // active eased transition target
  measureRender(n = 100): { mean: number, p95: number, max: number },
  rendererInfo(): { type, resolution, dpr, fps, gpu },
};
```

`measureRender(n)` calls `renderScene()` n times back-to-back with
`performance.now()` bracketing, returns the distribution. Useful for
"did canvas-015 actually drop per-call cost?" without leaving Marco
clicking around for a Performance trace.

`rendererInfo()` returns the four values canvas-014's reproducer
protocol asks for, in one call.

## Acceptance criteria

- [ ] `window.__guppi` is attached in dev builds and absent in
      production builds (verified via grep on the bundled output).
- [ ] `__guppi.app`, `__guppi.camera`, `__guppi.projects`,
      `__guppi.cameraTarget` reflect live state (mutations to the
      camera or projects are visible from the console).
- [ ] `__guppi.measureRender(n)` returns mean/p95/max in
      milliseconds.
- [ ] `__guppi.rendererInfo()` returns the four-field record the
      canvas-perf-2026-05-17 report describes.
- [ ] The diagnostic seam is documented in the BC README (a short
      "Diagnostic seam" subsection under "How the canvas stays live").
- [ ] `pnpm check` clean; `cargo test --lib` unchanged.

## Notes

- Production safety: an `import.meta.env.DEV` guard around the
  attachment is the recommended idiom; Vite tree-shakes the dev
  branch in production builds. Verify with a grep over `dist/` or
  the Tauri build output.
- Future perf spikes (post canvas-015 / canvas-016 / canvas-017) will
  use this seam to capture a console snapshot next to
  `canvas-perf-<date>/console-snapshot.md` without source patches.
- Estimate: ½ day.
- See `canvas-perf-2026-05-17` report, "Reproducer — operator protocol"
  section for the motivating use case.
