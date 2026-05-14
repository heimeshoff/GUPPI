<script lang="ts">
	// The infinite canvas — PixiJS v8 / WebGL (ADR-003). Greybox visuals only;
	// the styleguide is not signed off (skeleton scope: "plain rectangles and
	// lines are fine"). Renders one project tile at world origin with its BC
	// nodes radiating from it, edges connecting them, and live task counts.
	//
	// Camera state (ADR-003) lives in the `Camera` rune store; this component
	// drives pan (drag) and zoom (wheel) into it and re-projects the scene on
	// every change.

	import { onMount } from 'svelte';
	import { Application, Container, Graphics, Text } from 'pixi.js';
	import { Camera } from './camera.svelte';
	import {
		getProject,
		loadCamera,
		loadTilePosition,
		onDomainEvent,
		saveCamera,
		saveTilePosition,
		logToCore
	} from './ipc';
	import type { ProjectSnapshot, Point } from './types';

	const TILE_W = 220;
	const TILE_H = 120;
	const BC_W = 180;
	const BC_H = 90;
	const BC_RADIUS = 360; // world-space distance of BC nodes from the project tile

	let host: HTMLDivElement;
	const camera = new Camera();

	// The persisted world-space position of the project tile. The skeleton
	// requirement is that it starts at origin (0,0) and any drag is persisted.
	let tilePos = $state<Point>({ x: 0, y: 0 });
	let snapshot = $state<ProjectSnapshot | null>(null);
	let status = $state('starting…');

	onMount(() => {
		let app: Application | null = null;
		let unlistenEvent: (() => void) | null = null;
		let disposed = false;

		// PixiJS scene graph: world container holds everything; camera maps it.
		const world = new Container();
		let renderScene: () => void = () => {};

		(async () => {
			app = new Application();
			await app.init({
				resizeTo: host,
				background: 0x1e1e1e,
				antialias: true
			});
			if (disposed) {
				app.destroy(true);
				return;
			}
			host.appendChild(app.canvas);
			app.stage.addChild(world);

			// --- restore persisted camera + tile position (ADR-004) ------
			try {
				const savedCamera = await loadCamera();
				if (savedCamera) camera.restore(savedCamera);
				const savedTile = await loadTilePosition();
				if (savedTile) tilePos = savedTile;
			} catch (e) {
				logToCore('warn', `could not restore persisted view: ${e}`);
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
						.moveTo(projScreen.x + (TILE_W * z) / 2, projScreen.y + (TILE_H * z) / 2)
						.lineTo(bcScreen.x + (BC_W * z) / 2, bcScreen.y + (BC_H * z) / 2);
				});
				edges.stroke({ width: Math.max(1, 2 * z), color: 0x555555 });
				world.addChild(edges);

				// BC nodes.
				snapshot.bcs.forEach((bc, i) => {
					const bcWorld = bcWorldPosition(i, bcCount);
					const bcScreen = camera.worldToScreen(bcWorld.x, bcWorld.y);
					world.addChild(
						makeNode(
							bcScreen.x,
							bcScreen.y,
							BC_W * z,
							BC_H * z,
							0x2d2d2d,
							0x3c8b8e,
							bc.name,
							`b:${bc.task_counts.backlog}  t:${bc.task_counts.todo}  ` +
								`d:${bc.task_counts.doing}  ✓:${bc.task_counts.done}`,
							z
						)
					);
				});

				// The project tile, drawn last (on top).
				const tile = makeNode(
					projScreen.x,
					projScreen.y,
					TILE_W * z,
					TILE_H * z,
					0x252540,
					0x6c6cae,
					snapshot.name,
					snapshot.path,
					z
				);
				makeDraggable(tile, projScreen, z);
				world.addChild(tile);
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
					renderScene();
					void saveCamera(camera.snapshot());
				},
				{ passive: false }
			);

			// --- initial fetch + live updates (ADR-009) -------------------
			await refresh();
			unlistenEvent = await onDomainEvent((event) => {
				// Crude per skeleton scope: any .agentheim change -> re-fetch.
				if (event.kind === 'agentheim_changed') void refresh();
			});

			renderScene();

			// Re-render on PixiJS ticker so a window resize re-projects.
			app.ticker.add(renderScene);
		})();

		async function refresh() {
			try {
				snapshot = await getProject();
				status = `${snapshot.bcs.length} bounded contexts`;
				renderScene();
			} catch (e) {
				status = `error: ${e}`;
				logToCore('error', `get_project failed: ${e}`);
			}
		}

		// World-space position of BC node `i` of `count`, radiating around the
		// project tile so edges fan out cleanly.
		function bcWorldPosition(i: number, count: number): Point {
			if (count === 0) return { x: 0, y: 0 };
			const angle = (i / count) * Math.PI * 2 - Math.PI / 2;
			return {
				x: tilePos.x + Math.cos(angle) * BC_RADIUS,
				y: tilePos.y + Math.sin(angle) * BC_RADIUS
			};
		}

		// A greybox node: filled rounded rect, border, title, subtitle.
		function makeNode(
			screenX: number,
			screenY: number,
			w: number,
			h: number,
			fill: number,
			border: number,
			title: string,
			subtitle: string,
			z: number
		): Container {
			const node = new Container();
			const g = new Graphics();
			g.roundRect(screenX, screenY, w, h, 8 * z)
				.fill(fill)
				.stroke({ width: Math.max(1, 2 * z), color: border });
			node.addChild(g);

			const titleText = new Text({
				text: title,
				style: { fill: 0xffffff, fontSize: Math.max(8, 16 * z), fontWeight: 'bold' }
			});
			titleText.position.set(screenX + 12 * z, screenY + 10 * z);
			node.addChild(titleText);

			const subText = new Text({
				text: subtitle,
				style: { fill: 0xaaaaaa, fontSize: Math.max(6, 11 * z) }
			});
			subText.position.set(screenX + 12 * z, screenY + h - 22 * z);
			node.addChild(subText);

			return node;
		}

		// Make the project tile draggable; persist the new world position on
		// release (ADR-004 — "Tile position persisted on drag").
		function makeDraggable(tile: Container, screenPos: Point, z: number) {
			tile.eventMode = 'static';
			tile.hitArea = {
				contains: (x: number, y: number) =>
					x >= screenPos.x &&
					x <= screenPos.x + TILE_W * z &&
					y >= screenPos.y &&
					y <= screenPos.y + TILE_H * z
			};

			let dragging = false;
			let originScreenX = 0;
			let originScreenY = 0;

			tile.on('pointerdown', (e) => {
				dragging = true;
				e.stopPropagation(); // do not let this start a camera pan
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
					void saveTilePosition(tilePos);
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
	.canvas-host {
		position: absolute;
		inset: 0;
		overflow: hidden;
	}
	.status {
		position: absolute;
		left: 8px;
		bottom: 8px;
		font-family: monospace;
		font-size: 12px;
		color: #888;
		pointer-events: none;
	}
</style>
