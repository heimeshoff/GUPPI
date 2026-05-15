<script lang="ts">
	// The infinite canvas — PixiJS v8 / WebGL (ADR-003), rendered at the
	// design-system styleguide baseline (design-system-001-styleguide).
	//
	// canvas-002 restructures this component from one tile to **one tile per
	// registered project**, keyed by `project_id`. Per-project state is held in
	// a single `$state` array of records (`projects`) rather than a `Map` —
	// `renderScene` already iterates, Svelte 5 array reactivity is well-trodden,
	// and the mutation path (`bcs.push`, count edits via `applyDomainEvent`) is
	// what we already exercised in canvas-001 on a single snapshot. A `Map<…>`
	// in `$state` would also work but reacts only on reassignment/method calls,
	// making it easy to mutate unreactively by mistake; we picked the array.
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
	import { spiralPosition } from './tile-layout';
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

	// One record per rendered project. Keyed by `snapshot.id`; the canvas keys
	// every per-project concern (position, drag target, event routing) off it,
	// so there is no separate `projectId` scalar anywhere in this component.
	// `snapshot` is deeply reactive Svelte 5 `$state`, so `applyDomainEvent`'s
	// in-place mutation of `bcs` / `task_counts` is picked up on the next
	// ticker frame, exactly as in canvas-001.
	interface ProjectEntry {
		id: number;
		snapshot: ProjectSnapshot;
		pos: Point;
	}
	let projects = $state<ProjectEntry[]>([]);

	let status = $state('starting…');

	// Voice-state affordance — a single ambient indicator. The voice BC will
	// drive this later; for the styleguide baseline it sits at "idle" so the
	// visual contract (corner glyph, token colour) is established and testable.
	type VoiceState = 'idle' | 'listening' | 'muted';
	let voiceState = $state<VoiceState>('idle');

	// Which node the pointer is over — drives the focus-ring affordance. Keys
	// are now project-scoped (`project:${id}` / `bc:${id}:${name}`) so hover
	// rings do not collide across tiles.
	let hoveredKey = $state<string | null>(null);

	/** Find a project entry by id; null if not currently rendered. */
	function findProject(id: number): ProjectEntry | null {
		return projects.find((p) => p.id === id) ?? null;
	}

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

		// --- shared drag controller (canvas-002) ----------------------
		// One set of `window` pointer listeners for *all* tiles. The active
		// drag is identified by `dragProjectId`; tiles register themselves by
		// setting it on `pointerdown` and clearing it on `pointerup`. This
		// replaces the per-tile `window.addEventListener` pattern that would
		// have leaked N listener sets and fired all of them on every move.
		let dragProjectId: number | null = null;
		let dragOriginX = 0;
		let dragOriginY = 0;

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
			try {
				const savedCamera = await loadCamera();
				if (savedCamera) camera.restore(savedCamera);
			} catch (e) {
				logToCore('warn', `could not restore persisted camera: ${e}`);
			}

			// --- the render pass: project world -> screen via the camera --
			// Loops over every entry in `projects`; each one draws its tile +
			// BC nodes + edges with its own world-space origin (`entry.pos`).
			renderScene = () => {
				if (!app) return;
				world.removeChildren();

				const z = camera.zoom;

				// Edges first across all projects, so nodes draw on top.
				const edges = new Graphics();
				for (const entry of projects) {
					const projScreen = camera.worldToScreen(entry.pos.x, entry.pos.y);
					const bcCount = entry.snapshot.bcs.length;
					for (let i = 0; i < bcCount; i++) {
						const bcWorld = bcWorldPosition(entry.pos, i, bcCount);
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
					}
				}
				edges.stroke({
					width: Math.max(1, shape.borderWidth * z),
					color: color.edge
				});
				world.addChild(edges);

				// BC nodes + project tiles, per entry. Node keys are
				// project-scoped so hover/focus rings do not collide across
				// tiles even when two projects share a BC name.
				for (const entry of projects) {
					const projScreen = camera.worldToScreen(entry.pos.x, entry.pos.y);
					const bcCount = entry.snapshot.bcs.length;

					entry.snapshot.bcs.forEach((bc, i) => {
						const bcWorld = bcWorldPosition(entry.pos, i, bcCount);
						const bcScreen = camera.worldToScreen(bcWorld.x, bcWorld.y);
						world.addChild(
							makeNode({
								key: `bc:${entry.id}:${bc.name}`,
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

					const tile = makeNode({
						key: `project:${entry.id}`,
						screenX: projScreen.x,
						screenY: projScreen.y,
						w: shape.tileWidth * z,
						h: shape.tileHeight * z,
						radius: shape.radiusTile * z,
						fill: color.tileFill,
						border: color.tileBorder,
						titleColor: color.tileText,
						subtitleColor: color.tileTextMuted,
						title: entry.snapshot.name,
						subtitle: entry.snapshot.path,
						statusState: null,
						z
					});
					attachTileDrag(tile, entry.id, projScreen, z);
					world.addChild(tile);
				}

				// Voice-state affordance — a single ambient glyph pinned to
				// the bottom-right of the viewport (screen space, not world
				// space, so it stays put while the canvas pans).
				world.addChild(makeVoiceIndicator());
			};

			// --- camera interaction: pan (drag empty space) + zoom (wheel) -
			let panning = false;
			let lastX = 0;
			let lastY = 0;

			app.canvas.addEventListener('pointerdown', (e) => {
				// Tile dragging is handled by each tile's own hit area; a
				// pointerdown that reaches the canvas is empty-space = pan.
				panning = true;
				lastX = e.clientX;
				lastY = e.clientY;
				cameraTarget = null; // a manual gesture cancels any eased transition
			});

			// One shared window-level pointermove for both camera-pan and
			// active tile drag — canvas-002's shared drag controller.
			window.addEventListener('pointermove', (e) => {
				if (dragProjectId !== null) {
					const entry = findProject(dragProjectId);
					if (entry) {
						// Drag delta in screen space -> world space.
						entry.pos = {
							x: entry.pos.x + (e.clientX - dragOriginX) / camera.zoom,
							y: entry.pos.y + (e.clientY - dragOriginY) / camera.zoom
						};
						dragOriginX = e.clientX;
						dragOriginY = e.clientY;
						renderScene();
					}
					return;
				}
				if (!panning) return;
				camera.panBy(e.clientX - lastX, e.clientY - lastY);
				lastX = e.clientX;
				lastY = e.clientY;
				renderScene();
			});
			window.addEventListener('pointerup', () => {
				if (dragProjectId !== null) {
					// Persist exactly the dragged project's new position.
					const entry = findProject(dragProjectId);
					if (entry) {
						void saveTilePosition(entry.id, entry.pos);
					}
					dragProjectId = null;
					return;
				}
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
			window.addEventListener('keydown', (e) => {
				if (e.key === 'f' || e.key === 'F') {
					beginZoomToFit();
				}
			});

			// --- initial fetch + live updates (ADR-009) -------------------
			await refresh();
			unlistenEvent = await onDomainEvent((event) => {
				switch (event.kind) {
					case 'project_added':
						// Live-add path: the registry just registered (or the
						// startup seed announced) a project. If we already
						// have an entry with this id, this is the seed
						// double-add — no-op. Otherwise fetch its snapshot,
						// pick the next spiral slot, persist that slot, and
						// add it to the collection.
						if (findProject(event.project_id)) return;
						void addLiveProject(event.project_id);
						return;
					case 'project_missing':
						// Reserved for canvas-005 (missing-tile state). The
						// canvas does not break when this fires; it renders
						// nothing different.
						return;
					case 'resync_required': {
						// The bridge lagged and lost events it cannot
						// reconstruct — the one full re-fetch path (ADR-009).
						// Per-project: re-fetch exactly the affected project,
						// preserve its on-canvas position, leave every other
						// tile alone.
						const entry = findProject(event.project_id);
						if (!entry) return;
						void refreshOne(event.project_id);
						return;
					}
					default: {
						// A fine-grained filesystem-observation event:
						// `task_moved` / `task_added` / `task_removed` /
						// `bc_appeared` / `bc_disappeared`. Route by id; if
						// the project is not rendered, ignore (it is a
						// project the canvas does not have — or the live-add
						// race, in which case the matching `project_added`
						// will arrive and trigger a fresh fetch that already
						// reflects the change).
						const entry = findProject(event.project_id);
						if (!entry) return;
						applyDomainEvent(entry.snapshot, event, (msg) =>
							void logToCore('warn', msg)
						);
						// `entry.snapshot` is part of Svelte 5 `$state`
						// (deeply reactive). The mutation is picked up by
						// the ticker's `renderScene()` on the next frame.
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

		// Start an eased zoom-to-fit transition framing every rendered tile
		// and its BCs within the viewport.
		function beginZoomToFit() {
			if (!app || projects.length === 0) return;
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

		// World-space bounding box of the whole scene — the union of every
		// project tile and every BC orbit. Used by zoom-to-fit ('f').
		function sceneWorldBounds(): { x: number; y: number; w: number; h: number } {
			if (projects.length === 0) {
				return { x: 0, y: 0, w: shape.tileWidth, h: shape.tileHeight };
			}
			let minX = Number.POSITIVE_INFINITY;
			let minY = Number.POSITIVE_INFINITY;
			let maxX = Number.NEGATIVE_INFINITY;
			let maxY = Number.NEGATIVE_INFINITY;
			for (const entry of projects) {
				minX = Math.min(minX, entry.pos.x);
				minY = Math.min(minY, entry.pos.y);
				maxX = Math.max(maxX, entry.pos.x + shape.tileWidth);
				maxY = Math.max(maxY, entry.pos.y + shape.tileHeight);
				const bcCount = entry.snapshot.bcs.length;
				for (let i = 0; i < bcCount; i++) {
					const p = bcWorldPosition(entry.pos, i, bcCount);
					minX = Math.min(minX, p.x);
					minY = Math.min(minY, p.y);
					maxX = Math.max(maxX, p.x + shape.bcWidth);
					maxY = Math.max(maxY, p.y + shape.bcHeight);
				}
			}
			return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
		}

		/** Build a `ProjectEntry` for `snapshot`: restore its saved position
		 * if any, otherwise pick the next spiral slot and persist it
		 * immediately so it is stable across restarts even if never dragged.
		 * `spiralIndex` is the registration-order index used when no saved
		 * position exists. */
		async function buildEntry(
			snapshot: ProjectSnapshot,
			spiralIndex: number
		): Promise<ProjectEntry> {
			let pos: Point;
			try {
				const saved = await loadTilePosition(snapshot.id);
				if (saved) {
					pos = saved;
				} else {
					pos = spiralPosition(spiralIndex);
					// Persist auto-placement immediately so a never-dragged
					// tile still lands in the same spot after a restart.
					try {
						await saveTilePosition(snapshot.id, pos);
					} catch (e) {
						logToCore(
							'warn',
							`could not persist auto-placed position for project ${snapshot.id}: ${e}`
						);
					}
				}
			} catch (e) {
				logToCore(
					'warn',
					`could not restore tile position for project ${snapshot.id}: ${e}`
				);
				pos = spiralPosition(spiralIndex);
			}
			return { id: snapshot.id, snapshot, pos };
		}

		/** On-mount initial population: list every registered project, build
		 * an entry per project (restoring or auto-placing its position), and
		 * commit them all to the `projects` array in one assignment so
		 * reactivity fires once. */
		async function refresh() {
			try {
				const snapshots = await listProjects();
				if (snapshots.length === 0) {
					status = 'no projects registered yet';
					projects = [];
					return;
				}
				const entries: ProjectEntry[] = [];
				for (let i = 0; i < snapshots.length; i++) {
					entries.push(await buildEntry(snapshots[i], i));
				}
				projects = entries;
				status = `${projects.length} project${projects.length === 1 ? '' : 's'} · press F to fit`;
				renderScene();
			} catch (e) {
				status = `error: ${e}`;
				logToCore('error', `list_projects failed: ${e}`);
			}
		}

		/** Re-fetch exactly one project's snapshot (`resync_required` —
		 * ADR-009). Preserves the entry's existing world-space position so the
		 * tile does not jump on a resync. */
		async function refreshOne(id: number) {
			try {
				const fresh = await getProject(id);
				const idx = projects.findIndex((p) => p.id === id);
				if (idx === -1) return;
				const existing = projects[idx];
				projects[idx] = { id, snapshot: fresh, pos: existing.pos };
				renderScene();
			} catch (e) {
				logToCore('error', `get_project failed for ${id}: ${e}`);
			}
		}

		/** Live-add path: a `project_added` arrived for a project not already
		 * in the collection. Fetch its snapshot, auto-place it at the next
		 * spiral slot, persist that slot, and append. The next ticker frame
		 * draws it — no manual refresh button. */
		async function addLiveProject(id: number) {
			try {
				// Re-check now that we are async: a parallel arrival or the
				// `refresh()` race could have inserted it. Keeps idempotency.
				if (findProject(id)) return;
				const fresh = await getProject(id);
				if (findProject(id)) return;
				const entry = await buildEntry(fresh, projects.length);
				projects = [...projects, entry];
				status = `${projects.length} project${projects.length === 1 ? '' : 's'} · press F to fit`;
				renderScene();
			} catch (e) {
				logToCore('error', `live-add get_project failed for ${id}: ${e}`);
			}
		}

		// World-space position of BC node `i` of `count`, radiating around
		// the supplied tile origin so edges fan out cleanly. canvas-002 added
		// the `origin` parameter so each tile has its own ring of BCs rather
		// than every BC orbiting the world's single former-module-level
		// `tilePos`.
		function bcWorldPosition(origin: Point, i: number, count: number): Point {
			if (count === 0) return { x: origin.x, y: origin.y };
			const angle = (i / count) * Math.PI * 2 - Math.PI / 2;
			return {
				x: origin.x + Math.cos(angle) * shape.bcOrbitRadius,
				y: origin.y + Math.sin(angle) * shape.bcOrbitRadius
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

		/** Wire a project tile into the shared drag controller. The tile
		 * itself only handles its own `pointerdown` (to claim the drag and
		 * suppress the camera-pan); the actual `pointermove` / `pointerup`
		 * handlers live once at window-level above. Persistence is on
		 * `pointerup` via that shared handler (ADR-004 — tile position
		 * persisted on drag). */
		function attachTileDrag(tile: Container, id: number, screenPos: Point, z: number) {
			tile.eventMode = 'static';
			tile.hitArea = {
				contains: (x: number, y: number) =>
					x >= screenPos.x &&
					x <= screenPos.x + shape.tileWidth * z &&
					y >= screenPos.y &&
					y <= screenPos.y + shape.tileHeight * z
			};

			tile.on('pointerdown', (e) => {
				e.stopPropagation(); // do not let this start a camera pan
				cameraTarget = null;
				dragProjectId = id;
				dragOriginX = e.global.x;
				dragOriginY = e.global.y;
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
