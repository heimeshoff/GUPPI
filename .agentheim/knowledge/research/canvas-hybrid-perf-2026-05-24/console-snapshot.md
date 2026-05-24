# Console snapshot — canvas-019a hybrid perf spike (OPERATOR-PENDING)

Marco: paste the outputs here after running the operator protocol in
`README.md` (route `/spike-019a` in the Tauri dev shell). This file is
the source of truth for the PASS/FAIL verdict that `canvas-019` cites.

## 1. Renderer confirmation

```
renderer.type       =        (expect 2 = WebGL; 1 = canvas fallback → STOP, flag canvas-018-class issue)
renderer.resolution =
devicePixelRatio    =
GPU                 =
```

## 2. Sustained pan-circle (default zoom, interiors mounted) — AC #4

Ran `__guppiSpike.autopan(5)`; `mountedInteriorCount()` during run = ____

```
count   =
avgMs   =        (target ≤ 8 for headroom)
p95Ms   =        (PASS if ≤ 16)
maxMs   =
fps     =
```

## 3. Zoom across the LOD floor — AC #4

`__guppiSpike.reset()`, then wheel zoom in/out across z=0.45 for ~5s.

```
count   =
avgMs   =
p95Ms   =        (PASS if ≤ 16)
maxMs   =
fps     =
```

HUD showed "LOD: interiors suppressed" below z=0.45? (y/n) ____
Mounted interior count climbed on zoom-in?                 (y/n) ____

## 4. Chrome Performance trace

Saved `trace.json` in this directory? (y/n) ____
Scripting / Rendering / GPU split during pan: ____
Longest frame (ms): ____
Any single-frame spike when a frame scrolls in (interior mount)? ____

## Verdict

PASS / FAIL (circle one): ____

If FAIL: which gesture failed, and did tightening LOD_ZOOM_FLOOR /
lowering CARDS_PER_COLUMN recover it? If not → canvas-019 costs Option B
(full-DOM) as the fallback.
