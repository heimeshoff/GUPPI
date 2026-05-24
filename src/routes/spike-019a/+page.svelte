<script lang="ts">
	// =====================================================================
	// THROWAWAY PERF HARNESS — canvas-019a
	// =====================================================================
	// This route is a SPIKE harness, NOT production. It is intentionally
	// kept OUT of the production `Canvas.svelte` render path. Do not import
	// it, link to it from the app shell, or merge its render logic into
	// Canvas.svelte. It exists only so Marco can open `/spike-019a` in the
	// Tauri dev shell and capture frame-time numbers per the
	// `canvas-perf-2026-05-17` reproducer protocol.
	//
	// It is EXEMPT from the design-system styleguide gate: all content is
	// dummy text. No tokens, no brand palette — just enough structure to
	// reproduce the worst-case DOM cost of decision #1 (every project ALWAYS
	// renders as a full kanban-accordion of BCs × columns × cards).
	//
	// OPTION A (HYBRID) ONLY:
	//   - PixiJS keeps the camera/world transform (ADR-016) and draws each
	//     frame's SHELL (border + header bar) in world coordinates.
	//   - Each on-screen frame's INTERIOR is a DOM overlay positioned via
	//     `camera.worldToScreen` (ADR-003 / ADR-016 contract). The interior
	//     is a populated kanban-accordion (BCs → BACKLOG/DOING/DONE columns →
	//     dozens of dummy task cards).
	//   - VIEWPORT CULLING: only frames intersecting the viewport mount a DOM
	//     interior; off-screen frames are a cheap Pixi shell only.
	//   - ZOOM-THRESHOLD LOD: below LOD_ZOOM_FLOOR the DOM interior is
	//     suppressed (Pixi shell only) and RE-MOUNTS on zoom-in. This is a
	//     rendering-layer LOD, not a model-level summary (decision #1 holds).
	//
	// Option B (full-DOM) is NOT built here — it is the fallback the findings
	// note flags only if hybrid FAILS the target.
	// =====================================================================

	import { onMount } from 'svelte';
	import { Application, Container, Graphics, Text } from 'pixi.js';
	import { Camera } from '$lib/camera.svelte';

	// ---- Harness knobs --------------------------------------------------
	const N_FRAMES = 10; // N≈10 project frames (AC #1)
	const BCS_PER_FRAME = 5; // vertical accordion of BCs
	const COLUMNS = ['BACKLOG', 'DOING', 'DONE'];
	// Live-adjustable for the recovery-knob sweep (no rebuild needed): the
	// DOM interiors derive from this; the Pixi shells do not depend on it.
	let cardsPerColumn = $state(8); // dozens of cards per BC (5 BCs × 3 × 8 = 120/frame)

	// World-space frame geometry (CSS-px at zoom 1).
	const FRAME_W = 880;
	const FRAME_H = 620;
	const HEADER_H = 44;
	const GRID_COLS = 4;
	const GRID_GAP_X = 220;
	const GRID_GAP_Y = 200;

	// LOD: below this zoom the DOM interior is suppressed (Pixi shell only).
	// Chosen so that the interior text is still legible-ish above it; below
	// it the cards would be sub-pixel anyway, so culling is free visually.
	// Live-adjustable for the knob sweep.
	let lodFloor = $state(0.45);

	// Viewport-cull margin: mount interiors for frames within this many
	// screen px of the viewport edge so a fast pan doesn't show empty shells
	// at the moment a frame scrolls in.
	const CULL_MARGIN_PX = 120;

	// ---- Dummy model ----------------------------------------------------
	interface DummyCard {
		id: string;
		title: string;
	}
	interface DummyColumn {
		name: string;
		cards: DummyCard[];
	}
	interface DummyBc {
		name: string;
		columns: DummyColumn[];
	}
	interface DummyFrame {
		id: number;
		title: string;
		wx: number; // world x
		wy: number; // world y
		bcs: DummyBc[];
	}

	function buildFrames(cardsPerCol: number): DummyFrame[] {
		const frames: DummyFrame[] = [];
		for (let i = 0; i < N_FRAMES; i++) {
			const col = i % GRID_COLS;
			const row = Math.floor(i / GRID_COLS);
			const bcs: DummyBc[] = [];
			for (let b = 0; b < BCS_PER_FRAME; b++) {
				const columns: DummyColumn[] = COLUMNS.map((cn) => ({
					name: cn,
					cards: Array.from({ length: cardsPerCol }, (_, c) => ({
						id: `${i}-${b}-${cn}-${c}`,
						title: `task-${b}${String.fromCharCode(97 + (c % 26))}: dummy work item ${c}`
					}))
				}));
				bcs.push({ name: `bounded-context-${b}`, columns });
			}
			frames.push({
				id: i,
				title: `project-${i}`,
				wx: col * (FRAME_W + GRID_GAP_X),
				wy: row * (FRAME_H + GRID_GAP_Y),
				bcs
			});
		}
		return frames;
	}

	// Rebuilt only when the card-count knob changes (memoised; not per frame).
	// Pixi shells key off frame.id (stable) so they survive a rebuild.
	const frames = $derived(buildFrames(cardsPerColumn));

	// Build mode is decided at compile time by Vite; surfaced on-screen so the
	// snapshot records dev-vs-release unambiguously (devtools are off in
	// release builds, so this can't be checked from a console there).
	const isDevBuild = import.meta.env.DEV;

	// ---- Reactive overlay state -----------------------------------------
	// One overlay descriptor per frame that is currently both (a) within the
	// viewport-cull window AND (b) above the LOD zoom floor. The DOM block
	// renders these; everything else is Pixi-shell-only.
	interface OverlayPlacement {
		frame: DummyFrame;
		sx: number; // screen x of frame's world origin
		sy: number; // screen y
		z: number; // current zoom (for CSS transform scale)
	}
	let overlays = $state<OverlayPlacement[]>([]);
	let lodActive = $state(false); // true when below the LOD floor (interiors suppressed)
	let mountedInteriorCount = $state(0);

	const camera = new Camera();

	let host: HTMLDivElement;
	let app: Application | null = null;

	// ---- Frame-time sampler (instrumentation for the reproducer) --------
	// Exposes a rolling window of rAF deltas on `window.__guppiSpike` so the
	// operator can read sustained FPS / ms-per-frame from the devtools
	// console WITHOUT source-patching (canvas-018 dev-seam idea).
	interface SpikeSeam {
		app: Application | null;
		camera: Camera;
		frames: DummyFrame[];
		mountedInteriorCount: () => number;
		lodActive: () => boolean;
		// rolling frame-time stats over the last `window` samples
		stats: () => { count: number; avgMs: number; p95Ms: number; maxMs: number; fps: number };
		reset: () => void;
		// drive a scripted pan-circle for ~`seconds` s (operator convenience)
		autopan: (seconds?: number) => void;
		config: Record<string, number>;
	}

	const SAMPLE_CAP = 600; // ~10s at 60fps
	let samples: number[] = [];
	let lastT = 0;

	function pushSample(t: number) {
		if (lastT > 0) {
			const dt = t - lastT;
			samples.push(dt);
			if (samples.length > SAMPLE_CAP) samples.shift();
		}
		lastT = t;
	}

	function computeStats() {
		const n = samples.length;
		if (n === 0) return { count: 0, avgMs: 0, p95Ms: 0, maxMs: 0, fps: 0 };
		const sorted = [...samples].sort((a, b) => a - b);
		const sum = sorted.reduce((s, v) => s + v, 0);
		const avg = sum / n;
		const p95 = sorted[Math.min(n - 1, Math.floor(n * 0.95))];
		const max = sorted[n - 1];
		return {
			count: n,
			avgMs: +avg.toFixed(2),
			p95Ms: +p95.toFixed(2),
			maxMs: +max.toFixed(2),
			fps: +(1000 / avg).toFixed(1)
		};
	}

	// Autopan drive-state (component scope so the on-screen button can trigger
	// it; the rAF tick reads these imperatively each frame).
	let autopanUntil = 0;
	let autopanT0 = 0;

	// On-screen readouts so the FULL protocol runs without devtools (release
	// builds disable F12). Stats are refreshed on a 250ms timer, NOT every
	// rAF, to keep the measured render loop clean.
	let rendererType = $state(0); // 2 = WebGL, 1 = canvas fallback
	let displayStats = $state({ count: 0, avgMs: 0, p95Ms: 0, maxMs: 0, fps: 0 });
	let autopanning = $state(false);

	function resetSamples() {
		samples = [];
		lastT = 0;
		displayStats = { count: 0, avgMs: 0, p95Ms: 0, maxMs: 0, fps: 0 };
	}
	function triggerAutopan(seconds = 5) {
		autopanT0 = performance.now();
		autopanUntil = autopanT0 + seconds * 1000;
		resetSamples();
	}

	onMount(() => {
		let disposed = false;
		let raf = 0;
		let statsTimer = 0;

		const world = new Container();

		// Per-frame persistent Pixi shells (ADR-016: instantiate once, never
		// rebuild per render). Each shell = border Graphics + header Graphics
		// + title Text, drawn in WORLD coordinates.
		interface Shell {
			container: Container;
			border: Graphics;
			header: Graphics;
			title: Text;
		}
		const shells = new Map<number, Shell>();

		function makeShell(f: DummyFrame): Shell {
			const container = new Container();
			container.position.set(f.wx, f.wy);
			const border = new Graphics();
			const header = new Graphics();
			const title = new Text({
				text: f.title,
				style: { fill: 0xffffff, fontSize: 22, fontFamily: 'sans-serif' }
			});
			title.position.set(12, 10);
			container.addChild(border, header, title);
			return { container, border, header, title };
		}

		function drawShell(s: Shell, z: number) {
			// Stroke width pre-divided by z so on-screen width is constant
			// CSS-px under world.scale (ADR-016 §3).
			const w = Math.max(1 / z, 2 / z);
			s.border
				.clear()
				.roundRect(0, 0, FRAME_W, FRAME_H, 10)
				.fill(0x1e1e26)
				.stroke({ width: w, color: 0xff8b00 });
			s.header.clear().roundRect(0, 0, FRAME_W, HEADER_H, 10).fill(0x2a2a36);
		}

		(async () => {
			app = new Application();
			await app.init({
				resizeTo: host,
				background: 0x16161c,
				antialias: true,
				resolution: window.devicePixelRatio || 1,
				autoDensity: true
			});
			if (disposed) {
				app.destroy(true);
				return;
			}
			host.appendChild(app.canvas);
			app.stage.addChild(world);
			rendererType = app.renderer.type; // 2 = WebGL (expected), 1 = canvas fallback

			// Start camera so the grid is roughly centred at a default zoom.
			camera.zoom = 0.6;
			camera.pan_x = 80;
			camera.pan_y = 80;

			// Instantiate every shell once (persistent scene graph, ADR-016).
			for (const f of frames) {
				const s = makeShell(f);
				shells.set(f.id, s);
				world.addChild(s.container);
			}

			// ---- pan: pointer drag on the canvas background --------------
			let panning = false;
			let lastX = 0;
			let lastY = 0;
			app.canvas.addEventListener('pointerdown', (e: PointerEvent) => {
				panning = true;
				lastX = e.clientX;
				lastY = e.clientY;
			});
			window.addEventListener('pointermove', (e: PointerEvent) => {
				if (!panning) return;
				camera.panBy(e.clientX - lastX, e.clientY - lastY);
				lastX = e.clientX;
				lastY = e.clientY;
			});
			window.addEventListener('pointerup', () => {
				panning = false;
			});

			// ---- zoom: wheel about cursor --------------------------------
			app.canvas.addEventListener(
				'wheel',
				(e: WheelEvent) => {
					e.preventDefault();
					const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
					const rect = (app!.canvas as HTMLCanvasElement).getBoundingClientRect();
					camera.zoomAt(factor, e.clientX - rect.left, e.clientY - rect.top);
				},
				{ passive: false }
			);

			// ---- render loop ---------------------------------------------
			// Camera-as-stage-transform (ADR-016): pan/zoom = world.position +
			// world.scale only. Shells are NOT rebuilt per frame; stroke
			// widths are re-divided only when zoom changes. The DOM overlay
			// placement is recomputed each frame (cheap: ≤N entries) and
			// Svelte reconciles only the mounted interiors.
			let lastZ = -1;
			const tick = (t: number) => {
				if (disposed || !app) return;
				pushSample(t);

				if (autopanUntil > t) {
					// scripted pan-circle for the operator (AC #4 stance)
					if (!autopanning) autopanning = true;
					const elapsed = (t - autopanT0) / 1000;
					const r = 220;
					camera.pan_x = 80 + Math.cos(elapsed * 2) * r;
					camera.pan_y = 80 + Math.sin(elapsed * 2) * r;
				} else if (autopanning) {
					autopanning = false;
				}

				const z = camera.zoom;
				world.position.set(camera.pan_x, camera.pan_y);
				world.scale.set(z);

				// repaint stroke widths only on zoom change (ADR-016 §3 — never
				// on pan). This is the "no per-pan allocation" invariant.
				if (z !== lastZ) {
					for (const s of shells.values()) drawShell(s, z);
					lastZ = z;
				}

				// ---- viewport culling + LOD ------------------------------
				const vw = app.renderer.width / app.renderer.resolution;
				const vh = app.renderer.height / app.renderer.resolution;
				const below = z < lodFloor;
				const next: OverlayPlacement[] = [];
				if (!below) {
					for (const f of frames) {
						const sx = f.wx * z + camera.pan_x;
						const sy = f.wy * z + camera.pan_y;
						const fw = FRAME_W * z;
						const fh = FRAME_H * z;
						// AABB intersection test against viewport + margin.
						const onScreen =
							sx + fw >= -CULL_MARGIN_PX &&
							sx <= vw + CULL_MARGIN_PX &&
							sy + fh >= -CULL_MARGIN_PX &&
							sy <= vh + CULL_MARGIN_PX;
						if (onScreen) next.push({ frame: f, sx, sy, z });
					}
				}
				overlays = next;
				lodActive = below;
				mountedInteriorCount = next.length;

				raf = requestAnimationFrame(tick);
			};
			raf = requestAnimationFrame(tick);

			// Refresh the on-screen stats readout off the hot path (250ms timer,
			// not per rAF) so reading the numbers never perturbs the measurement.
			statsTimer = window.setInterval(() => {
				displayStats = computeStats();
			}, 250);

			// ---- dev diagnostic seam (canvas-018 idea) -------------------
			const seam: SpikeSeam = {
				app,
				camera,
				frames,
				mountedInteriorCount: () => mountedInteriorCount,
				lodActive: () => lodActive,
				stats: computeStats,
				reset: resetSamples,
				autopan: triggerAutopan,
				config: {
					N_FRAMES,
					BCS_PER_FRAME,
					cardsPerColumn,
					cardsPerFrame: BCS_PER_FRAME * COLUMNS.length * cardsPerColumn,
					lodFloor,
					CULL_MARGIN_PX
				}
			};
			(window as unknown as { __guppiSpike: SpikeSeam }).__guppiSpike = seam;
			// eslint-disable-next-line no-console
			console.log(
				'[spike-019a] harness ready.',
				'\n  On-screen HUD drives the full protocol (no devtools needed):',
				'\n    Autopan 5s / Reset buttons · cards-per-col + LOD-floor sliders',
				'\n    live p95/avg/max/fps · DEV-vs-RELEASE + WebGL indicators.',
				'\n  Console seam also available: window.__guppiSpike.autopan(5) / .stats()'
			);
		})();

		return () => {
			disposed = true;
			cancelAnimationFrame(raf);
			if (statsTimer) clearInterval(statsTimer);
			if (app) {
				app.destroy(true);
				app = null;
			}
			delete (window as unknown as { __guppiSpike?: SpikeSeam }).__guppiSpike;
		};
	});
</script>

<svelte:head>
	<title>SPIKE 019a — hybrid perf harness (throwaway)</title>
</svelte:head>

<div class="spike-root">
	<div class="pixi-host" bind:this={host}></div>

	<!-- DOM overlay layer: one populated kanban-accordion interior per
	     on-screen frame, positioned via camera.worldToScreen. Suppressed
	     entirely below the LOD zoom floor. -->
	{#if !lodActive}
		<div class="overlay-layer">
			{#each overlays as o (o.frame.id)}
				<div
					class="frame-interior"
					style="
						left: {o.sx}px;
						top: {o.sy + HEADER_H * o.z}px;
						width: {FRAME_W}px;
						height: {(FRAME_H - HEADER_H)}px;
						transform: scale({o.z});
						transform-origin: top left;
					"
				>
					<div class="accordion">
						{#each o.frame.bcs as bc (bc.name)}
							<section class="bc">
								<header class="bc-head">{bc.name}</header>
								<div class="kanban">
									{#each bc.columns as col (col.name)}
										<div class="column">
											<div class="col-head">{col.name} · {col.cards.length}</div>
											<div class="cards">
												{#each col.cards as card (card.id)}
													<div class="card">{card.title}</div>
												{/each}
											</div>
										</div>
									{/each}
								</div>
							</section>
						{/each}
					</div>
				</div>
			{/each}
		</div>
	{/if}

	<!-- HUD + controls: the FULL reproducer protocol runs on-screen so no
	     devtools are needed (release builds disable F12). Not part of the
	     measured render-cost analysis. -->
	<div class="hud">
		<div class="row title">
			<strong>SPIKE 019a · hybrid (throwaway)</strong>
			<span class="build" class:release={!isDevBuild}>
				{isDevBuild ? 'DEV build' : 'RELEASE build'}
			</span>
		</div>
		<div class="row">
			renderer <span class:ok={rendererType === 2} class:bad={rendererType !== 2}>
				{rendererType === 2 ? 'WebGL ✓' : rendererType === 1 ? 'canvas fallback ✗' : '…'}
			</span>
			· zoom {camera.zoom.toFixed(2)}
			· interiors {mountedInteriorCount}/{N_FRAMES}
			{#if lodActive}· <span class="lod">LOD suppressed</span>{/if}
		</div>

		<!-- Live frame-time stats (refreshed 250ms, off the hot path). The
		     PASS bar is p95 ≤ 16ms; avg ≤ 8ms is the headroom target. -->
		<div class="row stats">
			<span class:pass={displayStats.p95Ms > 0 && displayStats.p95Ms <= 16} class:fail={displayStats.p95Ms > 16}>
				p95 {displayStats.p95Ms.toFixed(1)}ms
			</span>
			· avg {displayStats.avgMs.toFixed(1)}ms
			· max {displayStats.maxMs.toFixed(1)}ms
			· {displayStats.fps.toFixed(0)} fps
			· n={displayStats.count}
			{#if autopanning}<span class="lod">· PANNING…</span>{/if}
		</div>

		<div class="row controls">
			<button onclick={() => triggerAutopan(5)}>Autopan 5s</button>
			<button onclick={resetSamples}>Reset</button>
		</div>

		<div class="row knob">
			<label>
				cards/col {cardsPerColumn}
				<span class="dim">({BCS_PER_FRAME * 3 * cardsPerColumn}/frame)</span>
				<input type="range" min="1" max="8" step="1" bind:value={cardsPerColumn} />
			</label>
		</div>
		<div class="row knob">
			<label>
				LOD floor {lodFloor.toFixed(2)}
				<input type="range" min="0.1" max="1.0" step="0.05" bind:value={lodFloor} />
			</label>
		</div>

		<div class="row hint">drag = pan · wheel = zoom</div>
	</div>
</div>

<style>
	.spike-root {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: #16161c;
	}
	.pixi-host {
		position: absolute;
		inset: 0;
	}
	.overlay-layer {
		position: absolute;
		inset: 0;
		pointer-events: none; /* let pan/zoom reach the canvas; spike doesn't test card interaction */
	}
	.frame-interior {
		position: absolute;
		overflow: hidden;
		box-sizing: border-box;
		padding: 6px;
		color: #e6e6ec;
		font-family: sans-serif;
		font-size: 11px;
	}
	.accordion {
		display: flex;
		flex-direction: column;
		gap: 6px;
		height: 100%;
		overflow-y: auto; /* real scroll container — part of decision #1's cost */
	}
	.bc {
		border: 1px solid #2a2a36;
		border-radius: 6px;
		background: #1a1a22;
	}
	.bc-head {
		padding: 4px 6px;
		font-weight: 600;
		color: #9a9aa6;
		border-bottom: 1px solid #2a2a36;
	}
	.kanban {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 4px;
		padding: 4px;
	}
	.col-head {
		font-size: 10px;
		color: #9a9aa6;
		margin-bottom: 3px;
	}
	.cards {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}
	.card {
		background: #242430;
		border: 1px solid #30303c;
		border-radius: 4px;
		padding: 3px 5px;
		line-height: 1.2;
	}
	.hud {
		position: absolute;
		left: 12px;
		bottom: 12px;
		padding: 10px 12px;
		background: rgba(0, 0, 0, 0.82);
		border: 1px solid #ff8b00;
		border-radius: 6px;
		color: #e6e6ec;
		font-family: monospace;
		font-size: 12px;
		line-height: 1.5;
		min-width: 280px;
		/* interactive: the on-screen controls replace the console seam so the
		   protocol runs in a release build with no devtools. */
		pointer-events: auto;
		user-select: none;
	}
	.hud .row {
		margin-top: 4px;
	}
	.hud .row.title {
		display: flex;
		justify-content: space-between;
		align-items: center;
		gap: 10px;
		margin-top: 0;
	}
	.hud .build {
		font-size: 10px;
		padding: 1px 6px;
		border-radius: 4px;
		background: #6b3a00;
		color: #ffce99;
	}
	.hud .build.release {
		background: #0d4d2b;
		color: #8ff0bd;
	}
	.hud .stats {
		font-size: 13px;
	}
	.hud .pass {
		color: #8ff0bd;
		font-weight: 700;
	}
	.hud .fail {
		color: #ff6b6b;
		font-weight: 700;
	}
	.hud .ok {
		color: #8ff0bd;
	}
	.hud .bad {
		color: #ff6b6b;
	}
	.hud .lod {
		color: #ff8b00;
	}
	.hud .dim {
		color: #9a9aa6;
	}
	.hud .controls {
		display: flex;
		gap: 8px;
	}
	.hud button {
		font-family: monospace;
		font-size: 12px;
		padding: 4px 10px;
		background: #2a2a36;
		color: #e6e6ec;
		border: 1px solid #ff8b00;
		border-radius: 4px;
		cursor: pointer;
	}
	.hud button:hover {
		background: #3a3a48;
	}
	.hud .knob label {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.hud .knob input[type='range'] {
		flex: 1;
		accent-color: #ff8b00;
	}
	.hud .hint {
		color: #9a9aa6;
	}
</style>
