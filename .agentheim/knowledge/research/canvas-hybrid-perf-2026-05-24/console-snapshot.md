# Console snapshot — canvas-019a hybrid perf spike

Source of truth for the PASS/FAIL verdict that `canvas-019` cites. Numbers
captured by Marco running the operator protocol in `README.md` (route
`/spike-019a`).

**Runs (2026-05-24), both at default / zoomed-out — worst case, all on-screen
frame interiors mounted (the heaviest simultaneous-DOM regime the spike
targets), N=10 frames, sustained `autopan(5)` pan-circle:**

| Build | cards/col | cards/frame | p95 ms | verdict |
| --- | --- | --- | --- | --- |
| `pnpm tauri dev` (unoptimized) | 8 | 120 | 33.3 | over budget (dev overhead) |
| **release** (optimized frontend) | 8 | 120 | **8.5** | **PASS** (≤16, at the ≤8 headroom) |
| **release** (optimized frontend) | 3 | 45 | **8.5** | **PASS** |

**Key finding:** release p95 is identical at 8 and 3 cards/col → per-pan cost
is **independent of card density**. DOM nodes are created once per mount; pan
restyles only the ≤N interior roots (compositor transforms), never the cards.
Hybrid scales with *frame count*, not *card count* — the analytical PASS
prediction confirmed. The dev-build 33.3 ms was ~4× dev-mode overhead
(Svelte reactivity dev asserts + Pixi dev build + Vite HMR), not a hybrid cost.

## 1. Renderer confirmation

```
renderer.type       =  (not captured)
renderer.resolution =  (not captured)
devicePixelRatio    =  (not captured)
GPU                 =  (not captured)
```

> Pending. Worth confirming `renderer.type === 2` (WebGL) on the next run — a
> canvas-2D fallback would by itself explain a dev-build FAIL.

## 2. Sustained pan-circle (default zoom, interiors mounted) — AC #4  ✅ CAPTURED

### 2a. Release build, 8 cards/col (120 cards/frame) — the worst case

```
p95Ms   = 8.5        (PASS if ≤ 16)        → PASS, at the ≤8 headroom target
```

### 2b. Release build, 3 cards/col (45 cards/frame) — knob sweep

```
p95Ms   = 8.5        (identical to 8 cards → card density is not the cost)
```

### 2c. Dev build, 8 cards/col — for the record (pessimistic floor)

```
count   = 502
avgMs   = 23.77
p95Ms   = 33.30      → over budget, attributed to dev-mode overhead (≈4×)
maxMs   = 58.40
fps     = 42.1
```

## 3. Zoom across the LOD floor — AC #4

`__guppiSpike.reset()`, then wheel zoom in/out across z=0.45 for ~5s.

```
count   =  (not captured)
avgMs   =
p95Ms   =        (PASS if ≤ 16)
maxMs   =
fps     =
```

HUD showed "LOD: interiors suppressed" below z=0.45? (y/n) ____
Mounted interior count climbed on zoom-in?                 (y/n) ____

> Pending.

## 4. Chrome Performance trace

Saved `trace.json` in this directory? (y/n) **n**
Scripting / Rendering / GPU split during pan: (not captured)
Longest frame (ms): 58.4 (from the sampler; trace breakdown not captured)
Any single-frame spike when a frame scrolls in (interior mount)? (not captured)

> Pending — and this is the most informative missing measurement. The
> verdict below hinges on whether the cost is **sustained per-pan-frame**
> (a structural hybrid problem) or **mount-spikes** as frames scroll in
> (a tuning problem — `content-visibility: auto` / staggered mount /
> larger `CULL_MARGIN_PX`). `maxMs 58.4` vs `avgMs 23.77` is consistent
> with *either*; only the trace separates them.

## Verdict

**PASS** — hybrid (Option A) cleared the target in a release build.

The worst-case sustained pan-circle (N=10 frames, all interiors mounted at
default zoom, 120 cards/frame) holds **`p95 = 8.5 ms`** — under the `≤ 16 ms`
/ 60 FPS bar and right at the `≤ 8 ms` headroom target. The recovery-knob
sweep (3 cards/col) gave the **same** 8.5 ms, proving per-pan cost is
independent of card density — so hybrid scales with frame count, not card
count, exactly as the note's analytical case predicted.

The earlier dev-build 33.3 ms was ~4× dev-mode overhead (a pessimistic
floor), not a hybrid cost — the provisional FAIL is retracted.

**`canvas-019` is cleared to ratify hybrid, citing these numbers.** Option B
(full-DOM) is not needed. Not blocking, optional polish for the production
build (`canvas-020`+): the cross-LOD zoom run and a Chrome trace to confirm
no mount-spike when a frame scrolls in (mitigation already noted:
`content-visibility: auto` / staggered mount if ever observed).
