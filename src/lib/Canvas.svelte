<script lang="ts">
	// The infinite canvas — PixiJS v8 / WebGL (ADR-003), rendered at the
	// design-system styleguide baseline (design-system-001-styleguide).
	//
	// This is the first consumer of the design tokens. Every colour, size,
	// font, and duration comes from `./design/tokens` — nothing here is a
	// magic number. Downstream frontend feature tasks follow the same rule.
	//
	// What the styleguide baseline establishes here:
	//   - Tile visual hierarchy: project tile (larger, warm border) vs BC node
	//     (smaller, cool border).
	//   - Status palette: each BC node carries a colourblind-friendly status
	//     badge (idle / running / blocked / missing) — colour + glyph.
	//   - Typography scale: one family, three sizes.
	//   - Edge style: project → BC connectors at the `edge` token colour.
	//   - Camera affordance: zoom-to-fit (press "f"), eased per the motion budget.
	//   - Voice-state affordance: a single ambient corner glyph (idle for now —
	//     real mic wiring is voice BC future work; this is the visual contract).
	//   - Focus/hover affordance: a focus ring on the hovered/dragged tile.
	//
	// Camera state (ADR-003) lives in the `Camera` rune store; this component
	// drives pan (drag) and zoom (wheel) into it and re-projects the scene on
	// every change.

	import { onMount } from 'svelte';
	import { Application, Container, Graphics, Text } from 'pixi.js';
	import { Camera } from './camera.svelte';
	import {
		getProject,
		listProjects,
		loadCamera,
		loadTilePosition,
		onDomainEvent,
		saveCamera,
		saveTilePosition,
		logToCore
	} from './ipc';
	import type { ProjectSnapshot, Point, BcSnapshot, CameraState } from './types';
	import { applyDomainEvent } from './snapshot-patch';
	import {
		color,
		typography,
		shape,
		motion,
		statusColor,
		statusGlyph,
		type TaskState
	} from './design/tokens';

	let host: HTMLDivElement;
	const camera = new Camera();

	// The persisted world-space position of the project tile. Starts at origin
	// (0,0); any drag is persisted (ADR-004).
	let tilePos = $state<Point>({ x: 0, y: 0 });
	let snapshot = $state<ProjectSnapshot | null>(null);
	let status = $state('starting…');

	// The id of the loaded project. The skeleton renders one project; the id
	// flows in two ways:
	//   1. Directly on the `ProjectSnapshot` returned by `listProjects()` /
	//      `getProject(projectId)` (`project-registry-001` adds this field).
	//   2. On the `project_added` domain event, for live registration after
	//      mount (the `add project…` flow `project-registry-002` will add).
	// Either way the canvas keeps this single id so it can filter fine-grained
	// domain events (`canvas-001` `project_id` filtering) and pass it to the
	// per-project IPC commands (`saveTilePosition`, etc.). `canvas-002` will
	// generalise this to a per-id map of tiles.
	let projectId = $state<number | null>(null);

	// Voice-state affordance — a single ambient indicator. The voice BC will
	// drive this later; for the styleguide baseline it sits at "idle" so the
	// visual contract (corner glyph, token colour) is established and testable.
	type VoiceState = 'idle' | 'listening' | 'muted';
	let voiceState = $state<VoiceState>('idle');

	// Which node the pointer is over — drives the focus-ring affordance.
	let hoveredKey = $state<string | null>(null);

	/**
	 * Derive a BC node's status from its task counts. This is the styleguide's
	 * status vocabulary applied to real data:
	 *   - missing : the BC has no task files at all (expected but absent)
	 *   - blocked : at least one task is parked in `backlog` and nothing is in
	 *               `doing` — the canvas reads this as "waiting on a decision"
	 *   - running : something is in `doing`
	 *   - idle    : has tasks, none in flight, nothing stuck
	 * The mapping is intentionally simple for the baseline; the canvas BC can
	 * refine it once real per-task status exists.
	 */
	export function deriveBcStatus(bc: BcSnapshot): TaskState {
		const c = bc.task_counts;
		const total = c.backlog + c.todo + c.doing + c.done;
		if (total === 0) return 'missing';
		if (c.doing > 0) return 'running';
		if (c.backlog > 0 && c.todo === 0) return 'blocked';
		return 'idle';
	}

	onMount(() => {
		let app: Application | null = null;
		let unlistenEvent: (() => void) | null = null;
		let disposed = false;

		// PixiJS scene graph: world container holds everything; camera maps it.
		const world = new Container();
		let renderScene: () => void = () => {};

		// Eased camera transition state (motion budget): when set, the ticker
		// lerps the camera toward `cameraTarget` and clears it when close.
		let cameraTarget: CameraState | null = null;
		let cameraAnimStart = 0;

		(async () => {
			app = new Application();
			await app.init({
				resizeTo: host,
				background: color.canvasBg,
				antialias: true
			});
			if (disposed) {
				app.destroy(true);
				return;
			}
			host.appendChild(app.canvas);
			app.stage.addChild(world);

			// --- restore persisted camera (ADR-004) ----------------------
			// Tile position restore moves into `refresh()` — it needs the
			// project id from `listProjects()`, which `project-registry-001`
			// made the load-bearing fetch.
			try {
				const savedCamera = await loadCamera();
				if (savedCamera) camera.restore(savedCamera);
			} catch (e) {
				logToCore('warn', `could not restore persisted camera: ${e}`);
			}

			// --- the render pass: project world -> screen via the camera --
			renderScene = () => {
				if (!app || !snapshot) return;
				world.removeChildren();

				const projScreen = camera.worldToScreen(tilePos.x, tilePos.y);
				const z = camera.zoom;

				// Edges first, so nodes draw on top.
				const edges = new Graphics();
				const bcCount = snapshot.bcs.length;
				snapshot.bcs.forEach((_, i) => {
					const bcWorld = bcWorldPosition(i, bcCount);
					const bcScreen = camera.worldToScreen(bcWorld.x, bcWorld.y);
					edges
						.moveTo(
							projScreen.x + (shape.tileWidth * z) / 2,
							projScreen.y + (shape.tileHeight * z) / 2
						)
						.lineTo(
							bcScreen.x + (shape.bcWidth * z) / 2,
							bcScreen.y + (shape.bcHeight * z) / 2
						);
				});
				edges.stroke({
					width: Math.max(1, shape.borderWidth * z),
					color: color.edge
				});
				world.addChild(edges);

				// BC nodes — secondary hierarchy: smaller, cool border, with a
				// status badge derived from task counts.
				snapshot.bcs.forEach((bc, i) => {
					const bcWorld = bcWorldPosition(i, bcCount);
					const bcScreen = camera.worldToScreen(bcWorld.x, bcWorld.y);
					const key = `bc:${bc.name}`;
					world.addChild(
						makeNode({
							key,
							screenX: bcScreen.x,
							screenY: bcScreen.y,
							w: shape.bcWidth * z,
							h: shape.bcHeight * z,
							radius: shape.radiusBc * z,
							fill: color.bcFill,
							border: color.bcBorder,
							titleColor: color.bcText,
							subtitleColor: color.bcTextMuted,
							title: bc.name,
							subtitle:
								`b ${bc.task_counts.backlog}   t ${bc.task_counts.todo}   ` +
								`d ${bc.task_counts.doing}   done ${bc.task_counts.done}`,
							statusState: deriveBcStatus(bc),
							z
						})
					);
				});

				// The project tile — primary hierarchy: larger, warm border,
				// drawn last (on top).
				const tile = makeNode({
					key: 'project',
					screenX: projScreen.x,
					screenY: projScreen.y,
					w: shape.tileWidth * z,
					h: shape.tileHeight * z,
					radius: shape.radiusTile * z,
					fill: color.tileFill,
					border: color.tileBorder,
					titleColor: color.tileText,
					subtitleColor: color.tileTextMuted,
					title: snapshot.name,
					subtitle: snapshot.path,
					statusState: null,
					z
				});
				makeDraggable(tile, projScreen, z);
				world.addChild(tile);

				// Voice-state affordance — a single ambient glyph pinned to the
				// bottom-right corner of the viewport (not world space, so it
				// stays put while the canvas pans). Established here as the
				// visual contract; the voice BC drives `voiceState` later.
				world.addChild(makeVoiceIndicator());
			};

			// --- camera interaction: pan (drag empty space) + zoom (wheel) -
			let panning = false;
			let lastX = 0;
			let lastY = 0;

			app.canvas.addEventListener('pointerdown', (e) => {
				// Tile dragging is handled by the tile's own hit area; a
				// pointerdown that reaches the canvas is empty-space = pan.
				panning = true;
				lastX = e.clientX;
				lastY = e.clientY;
				cameraTarget = null; // a manual gesture cancels any eased transition
			});
			window.addEventListener('pointermove', (e) => {
				if (!panning) return;
				camera.panBy(e.clientX - lastX, e.clientY - lastY);
				lastX = e.clientX;
				lastY = e.clientY;
				renderScene();
			});
			window.addEventListener('pointerup', () => {
				if (panning) {
					panning = false;
					void saveCamera(camera.snapshot());
				}
			});
			app.canvas.addEventListener(
				'wheel',
				(e) => {
					e.preventDefault();
					const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
					const rect = host.getBoundingClientRect();
					camera.zoomAt(factor, e.clientX - rect.left, e.clientY - rect.top);
					cameraTarget = null;
					renderScene();
					void saveCamera(camera.snapshot());
				},
				{ passive: false }
			);

			// --- camera affordance: zoom-to-fit on "f" --------------------
			// Eased per the motion budget (durationCamera). The keyboard hint
			// is shown in the corner overlay below.
			window.addEventListener('keydown', (e) => {
				if (e.key === 'f' || e.key === 'F') {
					beginZoomToFit();
				}
			});

			// --- initial fetch + live updates (ADR-009) -------------------
			// The fine-grained filesystem events patch the snapshot model in
			// place — no `getProject` round-trip per change (`canvas-001`).
			// Only `resync_required` (the lag-only escape hatch) re-fetches.
			await refresh();
			unlistenEvent = await onDomainEvent((event) => {
				switch (event.kind) {
					case 'project_added':
						// The skeleton sees this for the seed project at
						// startup (supervisor.add publishes it) and would see
						// it again for any live-added project. We already
						// learn the seed id from `listProjects()` in
						// `refresh()`, so this is harmless re-confirmation;
						// `canvas-002` extends it to maintain a per-id tile
						// map for live-add tiles.
						if (projectId === null) {
							projectId = event.project_id;
						}
						return;
					case 'project_missing':
						return;
					case 'resync_required':
						// The bridge lagged and lost events it cannot
						// reconstruct — the one full re-fetch path (ADR-009).
						// Per-project resync: re-fetch exactly the affected
						// project's snapshot, not the whole list.
						if (projectId !== null && event.project_id === projectId) {
							void refreshOne(event.project_id);
						}
						return;
					default: {
						// A fine-grained filesystem-observation event:
						// `task_moved` / `task_added` / `task_removed` /
						// `bc_appeared` / `bc_disappeared`. Patch in place.
						if (!snapshot) return;
						// Ignore events for a different project (the skeleton
						// has one, but the filter is real).
						if (projectId !== null && event.project_id !== projectId) {
							return;
						}
						applyDomainEvent(snapshot, event, (msg) =>
							void logToCore('warn', msg)
						);
						// `snapshot` is Svelte 5 `$state` (deeply reactive);
						// the mutation is picked up by the ticker's
						// `renderScene()` on the next frame — no explicit
						// re-render call, no animation system (silent count
						// update per the styleguide decision).
						return;
					}
				}
			});

			renderScene();

			// Re-render on PixiJS ticker so a window resize re-projects, and
			// step any in-progress eased camera transition.
			app.ticker.add(() => {
				if (cameraTarget) stepCameraTransition();
				renderScene();
			});
		})();

		// Start an eased zoom-to-fit transition framing the project tile and
		// all BC nodes within the viewport.
		function beginZoomToFit() {
			if (!app || !snapshot) return;
			const box = sceneWorldBounds();
			const viewport = { w: app.renderer.width, h: app.renderer.height };
			cameraTarget = camera.fitTo(box, viewport);
			cameraAnimStart = performance.now();
		}

		// Advance the eased camera transition; clear the target when settled.
		function stepCameraTransition() {
			if (!cameraTarget) return;
			const elapsed = performance.now() - cameraAnimStart;
			const t = Math.min(1, elapsed / motion.durationCamera);
			// ease-out cubic — matches motion.easeStandard's character.
			const eased = 1 - Math.pow(1 - t, 3);
			// Lerp by the *incremental* eased step so the motion decelerates.
			camera.lerpTo(cameraTarget, eased);
			if (t >= 1) {
				camera.restore(cameraTarget);
				cameraTarget = null;
				void saveCamera(camera.snapshot());
			}
		}

		// World-space bounding box of the whole scene (project tile + BCs),
		// used by zoom-to-fit.
		function sceneWorldBounds(): { x: number; y: number; w: number; h: number } {
			let minX = tilePos.x;
			let minY = tilePos.y;
			let maxX = tilePos.x + shape.tileWidth;
			let maxY = tilePos.y + shape.tileHeight;
			const bcCount = snapshot?.bcs.length ?? 0;
			for (let i = 0; i < bcCount; i++) {
				const p = bcWorldPosition(i, bcCount);
				minX = Math.min(minX, p.x);
				minY = Math.min(minY, p.y);
				maxX = Math.max(maxX, p.x + shape.bcWidth);
				maxY = Math.max(maxY, p.y + shape.bcHeight);
			}
			return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
		}

		async function refresh() {
			// `project-registry-001`: the canvas now learns the project set
			// from `listProjects()` rather than a path-implicit `getProject()`.
			// The skeleton renders the first project; `canvas-002` will render
			// a tile per snapshot in the returned list.
			try {
				const projects = await listProjects();
				if (projects.length === 0) {
					status = 'no projects registered yet';
					snapshot = null;
					return;
				}
				const first = projects[0];
				projectId = first.id;
				snapshot = first;
				// Now that we know the id, restore this project's persisted
				// tile position (ADR-004).
				try {
					const savedTile = await loadTilePosition(first.id);
					if (savedTile) tilePos = savedTile;
				} catch (e) {
					logToCore('warn', `could not restore tile position: ${e}`);
				}
				status = `${snapshot.bcs.length} bounded contexts · press F to fit`;
				renderScene();
			} catch (e) {
				status = `error: ${e}`;
				logToCore('error', `list_projects failed: ${e}`);
			}
		}

		/** Re-fetch exactly one project's snapshot (the `resync_required`
		 * lag-recovery path — ADR-009). Keyed by id so a multi-project canvas
		 * does not redraw the world for one tile's resync. */
		async function refreshOne(id: number) {
			try {
				const fresh = await getProject(id);
				// Single-project skeleton: replace the snapshot wholesale.
				// `canvas-002` patches the per-id entry in its tile map.
				snapshot = fresh;
				status = `${snapshot.bcs.length} bounded contexts · press F to fit`;
				renderScene();
			} catch (e) {
				logToCore('error', `get_project failed for ${id}: ${e}`);
			}
		}

		// World-space position of BC node `i` of `count`, radiating around the
		// project tile so edges fan out cleanly.
		function bcWorldPosition(i: number, count: number): Point {
			if (count === 0) return { x: 0, y: 0 };
			const angle = (i / count) * Math.PI * 2 - Math.PI / 2;
			return {
				x: tilePos.x + Math.cos(angle) * shape.bcOrbitRadius,
				y: tilePos.y + Math.sin(angle) * shape.bcOrbitRadius
			};
		}

		// A styleguide node: filled rounded rect, border, title + subtitle in
		// the type scale, an optional status badge, and a focus ring when the
		// pointer is over it.
		function makeNode(opts: {
			key: string;
			screenX: number;
			screenY: number;
			w: number;
			h: number;
			radius: number;
			fill: number;
			border: number;
			titleColor: number;
			subtitleColor: number;
			title: string;
			subtitle: string;
			statusState: TaskState | null;
			z: number;
		}): Container {
			const node = new Container();
			const { screenX, screenY, w, h, z } = opts;

			const focused = hoveredKey === opts.key;

			const g = new Graphics();
			g.roundRect(screenX, screenY, w, h, opts.radius)
				.fill(opts.fill)
				.stroke({
					width: Math.max(1, opts.border && opts.z ? shape.borderWidth * z : 1),
					color: opts.border
				});
			// Focus/hover affordance — a brighter ring just outside the border.
			if (focused) {
				g.roundRect(
					screenX - 2 * z,
					screenY - 2 * z,
					w + 4 * z,
					h + 4 * z,
					opts.radius + 2 * z
				).stroke({
					width: Math.max(1, shape.borderWidthFocus * z),
					color: color.focusRing
				});
			}
			node.addChild(g);

			const titleText = new Text({
				text: opts.title,
				style: {
					fill: opts.titleColor,
					fontFamily: typography.fontFamily,
					fontSize: Math.max(8, typography.sizeTitle * z),
					fontWeight: String(typography.weightBold) as '700'
				}
			});
			titleText.position.set(screenX + shape.radiusBadge * z + 6 * z, screenY + 10 * z);
			node.addChild(titleText);

			const subText = new Text({
				text: opts.subtitle,
				style: {
					fill: opts.subtitleColor,
					fontFamily: typography.fontFamilyMono,
					fontSize: Math.max(6, typography.sizeCaption * z)
				}
			});
			subText.position.set(
				screenX + shape.radiusBadge * z + 6 * z,
				screenY + h - 20 * z
			);
			node.addChild(subText);

			// Status badge — a small pill in the top-right corner. Colour +
			// glyph (colourblind-friendly: colour is never the only signal).
			if (opts.statusState) {
				node.addChild(makeStatusBadge(screenX, screenY, w, opts.statusState, z));
			}

			// Hover tracking for the focus-ring affordance.
			node.eventMode = 'static';
			node.hitArea = {
				contains: (x: number, y: number) =>
					x >= screenX && x <= screenX + w && y >= screenY && y <= screenY + h
			};
			node.on('pointerover', () => {
				hoveredKey = opts.key;
				renderScene();
			});
			node.on('pointerout', () => {
				if (hoveredKey === opts.key) {
					hoveredKey = null;
					renderScene();
				}
			});

			return node;
		}

		// A status badge — a coloured pill with the status glyph, pinned to a
		// node's top-right corner.
		function makeStatusBadge(
			nodeX: number,
			nodeY: number,
			nodeW: number,
			state: TaskState,
			z: number
		): Container {
			const badge = new Container();
			const size = shape.badgeHeight * z;
			const bx = nodeX + nodeW - size - 6 * z;
			const by = nodeY + 6 * z;

			const g = new Graphics();
			g.roundRect(bx, by, size, size, shape.radiusBadge * z).fill(statusColor[state]);
			badge.addChild(g);

			const glyph = new Text({
				text: statusGlyph[state],
				style: {
					fill: color.statusText,
					fontFamily: typography.fontFamily,
					fontSize: Math.max(6, typography.sizeCaption * z),
					fontWeight: String(typography.weightBold) as '700'
				}
			});
			glyph.anchor.set(0.5);
			glyph.position.set(bx + size / 2, by + size / 2);
			badge.addChild(glyph);

			return badge;
		}

		// The ambient voice-state indicator — a small glyph + dot pinned to the
		// bottom-right of the viewport. Screen-space (not world-space) so it
		// does not move when the canvas pans. This is the styleguide's voice
		// affordance contract; the voice BC supplies real state later.
		function makeVoiceIndicator(): Container {
			const indicator = new Container();
			if (!app) return indicator;
			const w = app.renderer.width;
			const h = app.renderer.height;

			const voiceColor =
				voiceState === 'listening'
					? color.voiceListening
					: voiceState === 'muted'
						? color.voiceMuted
						: color.voiceIdle;

			const r = 5;
			const cx = w - 22;
			const cy = h - 22;

			const dot = new Graphics();
			dot.circle(cx, cy, r).fill(voiceColor);
			indicator.addChild(dot);

			const label = new Text({
				text: voiceState === 'listening' ? 'mic' : voiceState === 'muted' ? 'muted' : 'mic',
				style: {
					fill: voiceColor,
					fontFamily: typography.fontFamily,
					fontSize: typography.sizeCaption,
					fontWeight: String(typography.weightMedium) as '500'
				}
			});
			label.anchor.set(1, 0.5);
			label.position.set(cx - r - 6, cy);
			indicator.addChild(label);

			return indicator;
		}

		// Make the project tile draggable; persist the new world position on
		// release (ADR-004 — "Tile position persisted on drag").
		function makeDraggable(tile: Container, screenPos: Point, z: number) {
			tile.eventMode = 'static';
			tile.hitArea = {
				contains: (x: number, y: number) =>
					x >= screenPos.x &&
					x <= screenPos.x + shape.tileWidth * z &&
					y >= screenPos.y &&
					y <= screenPos.y + shape.tileHeight * z
			};

			let dragging = false;
			let originScreenX = 0;
			let originScreenY = 0;

			tile.on('pointerdown', (e) => {
				dragging = true;
				e.stopPropagation(); // do not let this start a camera pan
				cameraTarget = null;
				originScreenX = e.global.x;
				originScreenY = e.global.y;
			});
			window.addEventListener('pointermove', (e) => {
				if (!dragging) return;
				// Drag delta in screen space -> world space (divide by zoom).
				tilePos = {
					x: tilePos.x + (e.clientX - originScreenX) / camera.zoom,
					y: tilePos.y + (e.clientY - originScreenY) / camera.zoom
				};
				originScreenX = e.clientX;
				originScreenY = e.clientY;
				renderScene();
			});
			window.addEventListener('pointerup', () => {
				if (dragging) {
					dragging = false;
					// `project-registry-001`: per-project tile position requires
					// passing the project id explicitly. Skipped if the id is
					// not yet known (only possible mid-bootstrap, before
					// `refresh()` resolves).
					if (projectId !== null) {
						void saveTilePosition(projectId, tilePos);
					}
				}
			});
		}

		return () => {
			disposed = true;
			unlistenEvent?.();
			app?.destroy(true);
		};
	});
</script>

<div class="canvas-host" bind:this={host}></div>
<div class="status">{status}</div>

<style>
	/* The HTML chrome layer reads the design tokens (ADR-003 overlay layer). */
	@import './design/tokens.css';

	.canvas-host {
		position: absolute;
		inset: 0;
		overflow: hidden;
		background: var(--guppi-canvas-bg);
	}
	.status {
		position: absolute;
		left: var(--guppi-space-sm);
		bottom: var(--guppi-space-sm);
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-caption);
		color: var(--guppi-bc-text-muted);
		pointer-events: none;
	}
</style>
