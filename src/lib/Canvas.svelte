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
	import { open as openDialog } from '@tauri-apps/plugin-dialog';
	import { Camera } from './camera.svelte';
	import Modal from './Modal.svelte';
	import {
		addScanRoot,
		getProject,
		importScannedProjects,
		listProjects,
		listProjectsByScanRoot,
		listScanRoots,
		loadCamera,
		loadTilePosition,
		onDomainEvent,
		registerProject,
		removeProject,
		removeScanRoot,
		rescanScanRoot,
		saveCamera,
		saveTilePosition,
		logToCore
	} from './ipc';
	import type {
		BcSnapshot,
		CameraState,
		Point,
		ProjectSnapshot,
		ScanCandidate,
		ScanRootRow
	} from './types';
	import { applyDomainEvent } from './snapshot-patch';
	import { spiralPosition } from './tile-layout';
	import {
		color,
		typography,
		shape,
		spacing,
		motion,
		statusColor,
		statusGlyph,
		type TaskState
	} from './design/tokens';

	// --- Right-click context menu (canvas-005a) -----------------------
	// An items-array shape so canvas-005b can append "Scan folder for
	// projects…" and "Manage scan roots…" to the empty-canvas menu without
	// touching this component's menu rendering. `hidden` is reserved for
	// 005b's "Manage scan roots…" (hidden when the scan-roots list is empty).
	interface MenuItem {
		label: string;
		onClick: () => void;
		hidden?: boolean;
	}
	interface MenuState {
		x: number;
		y: number;
		items: MenuItem[];
	}
	let menu = $state<MenuState | null>(null);

	// --- Scan-root modals state (canvas-005b) ------------------------
	// Three modals share the codebase's new `Modal.svelte` primitive:
	//
	//   1. `checklistModal` — the discovery checklist after `add_scan_root`
	//      OR `rescan_scan_root`. The `isRescan` flag differentiates the
	//      header label per the task spec.
	//   2. `manageModal` — the scan-roots management surface
	//      (`list_scan_roots` + per-row child counts via
	//      `list_projects_by_scan_root`).
	//   3. `confirmRemoveModal` — the cascade-remove confirmation. The
	//      ONE exception to "one modal at a time": when set, it renders ON
	//      TOP of `manageModal`, which stays mounted behind it.
	//
	// `scanRootsCount` caches the live count of registered roots so the
	// "Manage scan roots…" menu item can be hidden when zero. Refreshed on
	// mount, after `addScanRoot` resolves, and after `removeScanRoot`
	// resolves.
	interface ChecklistRow {
		candidate: ScanCandidate;
		ticked: boolean;
	}
	interface ChecklistModalState {
		scanRootId: number;
		rootPath: string;
		isRescan: boolean;
		rows: ChecklistRow[];
	}
	interface ManageRoot {
		root: ScanRootRow;
		childCount: number;
	}
	interface ManageModalState {
		roots: ManageRoot[];
	}
	interface ConfirmRemoveModalState {
		root: ScanRootRow;
		childCount: number;
	}
	let scanRootsCount = $state(0);
	let checklistModal = $state<ChecklistModalState | null>(null);
	let manageModal = $state<ManageModalState | null>(null);
	let confirmRemoveModal = $state<ConfirmRemoveModalState | null>(null);

	// --- Error toast (canvas-005a) -----------------------------------
	// One toast at a time; a new toast replaces the current one rather than
	// stacking. Used for the `register_project` rejection path
	// ("not an Agentheim project"). Auto-dismisses after 3000ms.
	let toastMessage = $state<string | null>(null);
	let toastTimer: ReturnType<typeof setTimeout> | null = null;
	function showToast(message: string) {
		if (toastTimer) clearTimeout(toastTimer);
		toastMessage = message;
		toastTimer = setTimeout(() => {
			toastMessage = null;
			toastTimer = null;
		}, 3000);
	}

	// --- Scan-root flows (canvas-005b) -------------------------------
	// Defined at script top-level so the modal templates can call them
	// directly. They only depend on the reactive `$state` declared above
	// and the `./ipc` wrappers — no PixiJS internals — so this is a clean
	// extraction from `onMount`.

	/** Refresh the cached count of registered scan roots — drives the
	 * "Manage scan roots…" menu item's `hidden` flag. Best-effort; on IPC
	 * failure the cache stays at its last value and the count is re-checked
	 * the next time the menu opens. */
	async function refreshScanRootsCount() {
		try {
			const roots = await listScanRoots();
			scanRootsCount = roots.length;
		} catch (e) {
			logToCore('warn', `list_scan_roots failed: ${e}`);
		}
	}

	/** "Scan folder for projects…" flow. Opens a Tauri-native folder
	 * picker, invokes `addScanRoot` on a chosen path, opens the discovery
	 * checklist modal with the returned candidates. The scan root is
	 * persisted by the backend BEFORE the walk runs, so even an empty
	 * candidate set leaves a rescannable root behind (ADR-013) — the
	 * empty-state modal still opens, with an "OK" footer button. */
	async function runScanFolderFlow() {
		menu = null;
		let picked: string | string[] | null = null;
		try {
			picked = await openDialog({ directory: true, multiple: false });
		} catch (e) {
			logToCore('error', `open dialog failed: ${e}`);
			showToast(`could not open folder picker: ${e}`);
			return;
		}
		if (picked === null) return;
		const path = Array.isArray(picked) ? picked[0] : picked;
		if (!path) return;
		try {
			const result = await addScanRoot(path);
			checklistModal = {
				scanRootId: result.scan_root_id,
				rootPath: path,
				isRescan: false,
				rows: result.candidates.map((c) => ({
					candidate: c,
					// Already-imported rows are pre-checked AND disabled — the
					// imported pre-checks do NOT count toward the "Import
					// selected" disabled-when-zero rule.
					ticked: c.already_imported
				}))
			};
			// The scan root is persisted regardless of candidate count —
			// refresh the menu's "Manage scan roots…" visibility.
			await refreshScanRootsCount();
		} catch (e) {
			const msg = String(e);
			showToast(`could not scan folder: ${msg}`);
			logToCore('warn', `add_scan_root rejected: ${msg}`);
		}
	}

	/** Pull the live scan-root list with per-row child counts. Shared by
	 * the open flow and the post-cascade refresh. Soft-deleted children are
	 * filtered out at the DB layer (`project-registry-003`). */
	async function fetchManageRoots(): Promise<ManageRoot[]> {
		const roots = await listScanRoots();
		scanRootsCount = roots.length;
		return Promise.all(
			roots.map(async (root) => {
				try {
					const children = await listProjectsByScanRoot(root.id);
					return { root, childCount: children.length };
				} catch (e) {
					logToCore(
						'warn',
						`list_projects_by_scan_root failed for ${root.id}: ${e}`
					);
					return { root, childCount: 0 };
				}
			})
		);
	}

	/** "Manage scan roots…" flow. Fetches the live scan-root list and the
	 * per-row child-project counts and opens the management modal. */
	async function runManageScanRootsFlow() {
		menu = null;
		try {
			const entries = await fetchManageRoots();
			manageModal = { roots: entries };
		} catch (e) {
			const msg = String(e);
			showToast(`could not list scan roots: ${msg}`);
			logToCore('error', `list_scan_roots failed: ${msg}`);
		}
	}

	/** Refresh the open management modal's rows (after a cascade-remove
	 * resolves). If zero roots remain, close the management modal AND the
	 * menu visibility cache flips so "Manage scan roots…" hides on the
	 * next right-click. */
	async function refreshManageModal() {
		try {
			const entries = await fetchManageRoots();
			if (entries.length === 0) {
				manageModal = null;
				return;
			}
			manageModal = { roots: entries };
		} catch (e) {
			logToCore('error', `refresh manage modal failed: ${e}`);
		}
	}

	/** "Rescan" button on a manage-modal row. Invokes `rescanScanRoot` and
	 * re-opens the checklist modal with `isRescan: true` in the header.
	 * The management modal closes — the checklist modal takes its place
	 * (one modal at a time, with the cascade-confirm stack-on-top being
	 * the lone exception). */
	async function runRescanFlow(root: ScanRootRow) {
		try {
			const candidates = await rescanScanRoot(root.id);
			manageModal = null;
			checklistModal = {
				scanRootId: root.id,
				rootPath: root.path,
				isRescan: true,
				rows: candidates.map((c) => ({
					candidate: c,
					ticked: c.already_imported
				}))
			};
		} catch (e) {
			const msg = String(e);
			showToast(`could not rescan: ${msg}`);
			logToCore('error', `rescan_scan_root failed: ${msg}`);
		}
	}

	/** Open the cascade-remove confirmation dialog for a scan root. Stacks
	 * ON TOP of the open management modal — the explicit exception to
	 * "one modal at a time". */
	function openConfirmRemove(entry: ManageRoot) {
		confirmRemoveModal = {
			root: entry.root,
			childCount: entry.childCount
		};
	}

	/** Confirm cascade-remove of a scan root. The backend cascade fires N
	 * `project_removed` events BEFORE the DB rows are gone — the
	 * canvas-005a `project_removed` handler is the single canonical
	 * listener that drops each tile (this code does NOT duplicate the
	 * subscription). After the cascade resolves, the management modal's
	 * row list is refreshed; if zero roots remain, it closes. */
	async function runRemoveScanRoot(scanRootId: number) {
		try {
			await removeScanRoot(scanRootId);
			confirmRemoveModal = null;
			await refreshManageModal();
		} catch (e) {
			const msg = String(e);
			showToast(`could not remove scan root: ${msg}`);
			logToCore(
				'error',
				`remove_scan_root failed for ${scanRootId}: ${msg}`
			);
			confirmRemoveModal = null;
		}
	}

	/** Submit the discovery checklist modal's picks. Already-imported rows
	 * are pre-ticked AND disabled — they cannot be unticked, and they are
	 * already in the registry, so they are filtered out of the
	 * `import_scanned_projects` request entirely. Tiles arrive via N
	 * `project_added` events; the canvas-006 serialised live-add chain
	 * gives them distinct spiral slots. */
	async function runImportSelected(state: ChecklistModalState) {
		const picks = state.rows
			.filter((r) => r.ticked && !r.candidate.already_imported)
			.map((r) => r.candidate.path);
		if (picks.length === 0) return;
		try {
			await importScannedProjects(state.scanRootId, picks);
			checklistModal = null;
		} catch (e) {
			const msg = String(e);
			showToast(`could not import projects: ${msg}`);
			logToCore('error', `import_scanned_projects failed: ${msg}`);
		}
	}

	/** Whether the checklist modal's "Import selected" button is enabled.
	 * Already-imported pre-ticks do NOT count toward this. */
	function hasNewSelection(state: ChecklistModalState): boolean {
		return state.rows.some(
			(r) => r.ticked && !r.candidate.already_imported
		);
	}

	/** Header controls — "Select all" / "Select none" — operate ONLY on
	 * togglable (not-already-imported) rows. The header hides these when
	 * there are zero togglable rows. */
	function selectAllTogglable(state: ChecklistModalState) {
		for (const r of state.rows) {
			if (!r.candidate.already_imported) r.ticked = true;
		}
	}
	function selectNoneTogglable(state: ChecklistModalState) {
		for (const r of state.rows) {
			if (!r.candidate.already_imported) r.ticked = false;
		}
	}
	function countTogglableRows(state: ChecklistModalState): number {
		return state.rows.filter((r) => !r.candidate.already_imported).length;
	}

	// Menu DOM ref + clamped position (canvas-005a). The menu opens at the
	// raw click coords, then an `$effect` measures and clamps so the menu
	// stays fully within the viewport (the simplest viable: `top + height <=
	// viewport.height`, `left + width <= viewport.width`). Initial paint at
	// the raw click is briefly possible; in practice the effect lands on the
	// next microtask before the user sees mis-clipping.
	let menuEl = $state<HTMLDivElement | null>(null);
	let menuLeft = $state(0);
	let menuTop = $state(0);
	$effect(() => {
		if (!menu) return;
		const x = menu.x;
		const y = menu.y;
		// First paint at the raw coords; measure-and-clamp follows in the
		// post-effect microtask once the DOM node is attached.
		menuLeft = x;
		menuTop = y;
		queueMicrotask(() => {
			if (!menuEl) return;
			const rect = menuEl.getBoundingClientRect();
			const vw = window.innerWidth;
			const vh = window.innerHeight;
			menuLeft = Math.max(0, Math.min(x, vw - rect.width));
			menuTop = Math.max(0, Math.min(y, vh - rect.height));
		});
	});

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

					// Missing-tile rendering (canvas-005a): a registry row whose
					// `.agentheim/` is gone on disk renders dim, with the
					// `statusMissing` border and a `✕` corner glyph. The
					// snapshot's `bcs: []` is already empty so no BC nodes
					// loop for it above. The tile is NOT filtered out — the
					// missing visual is the affordance.
					const isMissing = entry.snapshot.missing;
					const tile = makeNode({
						key: `project:${entry.id}`,
						screenX: projScreen.x,
						screenY: projScreen.y,
						w: shape.tileWidth * z,
						h: shape.tileHeight * z,
						radius: shape.radiusTile * z,
						fill: color.tileFill,
						border: isMissing ? color.statusMissing : color.tileBorder,
						titleColor: color.tileText,
						subtitleColor: color.tileTextMuted,
						title: entry.snapshot.name,
						subtitle: entry.snapshot.path,
						statusState: null,
						z
					});
					if (isMissing) {
						tile.alpha = 0.5;
						tile.addChild(
							makeMissingGlyph(projScreen, shape.tileWidth * z, z)
						);
					}
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
				// Right-button on empty canvas = open the empty-canvas context
				// menu at the click coordinates (canvas-005a). It is the only
				// way to start the "Add project…" flow. The window-level
				// capture-phase dismisser will have already nulled any
				// currently-open menu before this listener runs, so opening a
				// new one here works cleanly.
				if (e.button === 2) {
					e.preventDefault();
					openEmptyCanvasMenu(e.clientX, e.clientY);
					return;
				}
				// Tile dragging is handled by each tile's own hit area; a
				// pointerdown that reaches the canvas is empty-space = pan.
				panning = true;
				lastX = e.clientX;
				lastY = e.clientY;
				cameraTarget = null; // a manual gesture cancels any eased transition
				// `menu` was already cleared by the capture-phase listener
				// above; no need to repeat it here.
			});
			// Suppress the browser's native context menu so our overlay can
			// own the right-click affordance.
			app.canvas.addEventListener('contextmenu', (e) => e.preventDefault());

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
					// Any zoom gesture dismisses an open context menu — the menu
					// is screen-space-anchored to a click point and would lose
					// its semantic anchor under the moving canvas.
					menu = null;
					renderScene();
					void saveCamera(camera.snapshot());
				},
				{ passive: false }
			);

			// --- camera affordance: zoom-to-fit on "f" --------------------
			window.addEventListener('keydown', (e) => {
				if (e.key === 'Escape' && menu) {
					// Escape dismisses any open context menu (canvas-005a).
					menu = null;
					return;
				}
				if (e.key === 'f' || e.key === 'F') {
					beginZoomToFit();
				}
			});

			// --- outside-click dismissal for the context menu (canvas-005a) -
			// Capture-phase so it runs BEFORE the canvas/tile pointerdown
			// that may itself open a new menu (right-click). Sequence:
			// dismiss → handler optionally opens a new one. "Last click wins"
			// (right-clicking on a tile while the empty-canvas menu is open
			// swaps to the tile menu) falls out for free.
			window.addEventListener(
				'pointerdown',
				(e) => {
					if (!menu) return;
					if (
						menuEl &&
						e.target instanceof Node &&
						menuEl.contains(e.target)
					)
						return;
					menu = null;
				},
				true
			);

			// --- initial fetch + live updates (ADR-009) -------------------
			await refresh();
			// canvas-005b: prime the "Manage scan roots…" menu item's
			// visibility cache. The menu only renders this entry when at
			// least one scan root exists; without the prime, the first
			// right-click after launch would always omit it.
			await refreshScanRootsCount();
			unlistenEvent = await onDomainEvent((event) => {
				switch (event.kind) {
					case 'project_added':
						// Live-add path: the registry just registered (or the
						// startup seed announced) a project. If we already
						// have an entry with this id, this is the seed
						// double-add — no-op. Otherwise enqueue the live-add
						// on the serialisation chain (canvas-006): bursts of
						// `project_added` (e.g. from `import_scanned_projects`
						// announcing N picks back-to-back on the event bus)
						// must process strictly sequentially, otherwise the
						// concurrent closures all read the same `projects.length`
						// at the spiral-index step and the `projects = [...]`
						// reassignment loses entries to last-write-wins.
						if (findProject(event.project_id)) return;
						enqueueLiveAdd(event.project_id);
						return;
					case 'project_missing':
						// Reserved for canvas-005 (missing-tile state). The
						// canvas does not break when this fires; it renders
						// nothing different.
						return;
					case 'project_removed': {
						// THE canonical `project_removed` handler (canvas-005a).
						// Both code paths in `project-registry-003` fire the
						// same event variant: the user-initiated single
						// `remove_project` (this task's "Remove project"
						// affordance) and `remove_scan_root`'s cascade fan-out
						// (canvas-005b territory). canvas-005b MUST NOT
						// duplicate this listener — one event variant, one
						// listener.
						//
						// Drop the entry from `projects`. The backend has
						// already torn down the watcher; `tile_positions` is
						// either preserved (soft-delete via `remove_project`,
						// the 30-day undo window) or hard-deleted (cascade via
						// `remove_scan_root`'s `ON DELETE CASCADE`). The
						// canvas does not need to distinguish — the position
						// either revives on re-add (single-remove) or stays
						// gone (cascade), both correct.
						const idx = projects.findIndex(
							(p) => p.id === event.project_id
						);
						if (idx === -1) return;
						projects = projects.filter((p) => p.id !== event.project_id);
						if (hoveredKey?.startsWith(`project:${event.project_id}`)) {
							hoveredKey = null;
						}
						status = `${projects.length} project${projects.length === 1 ? '' : 's'} · press F to fit`;
						renderScene();
						return;
					}
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

		// --- live-add serialisation chain (canvas-006) ----------------
		// `addLiveProject` reads `projects.length` and then performs a
		// read-modify-write of `projects` across an `await`. If two
		// `project_added` events fire back-to-back (the normal shape of
		// `import_scanned_projects`'s announce phase), unserialised
		// invocations collide: all closures read the same `length` for the
		// spiral index, and the array reassignment is last-write-wins so
		// every loser silently drops its entry — while its
		// `saveTilePosition` row is already persisted at the colliding
		// position. The fix is structural: enqueue every live-add onto a
		// single promise chain so each `addLiveProject` runs only after
		// the previous one has settled. A failing step `catch`-es so a
		// single broken arrival cannot wedge the chain for the rest of
		// the burst.
		let liveAddChain: Promise<void> = Promise.resolve();
		function enqueueLiveAdd(id: number) {
			liveAddChain = liveAddChain
				.then(() => addLiveProject(id))
				.catch((e) => {
					logToCore('error', `live-add chain step failed for ${id}: ${e}`);
				});
		}

		/** Live-add path: a `project_added` arrived for a project not already
		 * in the collection. Fetch its snapshot, auto-place it at the next
		 * spiral slot, persist that slot, and append. The next ticker frame
		 * draws it — no manual refresh button.
		 *
		 * Invoked only via `enqueueLiveAdd` so calls are strictly serialised
		 * (canvas-006). The post-await `findProject` re-check below is now
		 * redundant under strict serialisation but is kept as defence in
		 * depth — the cost is a single Array#find and it preserves
		 * idempotency if the serialisation contract is ever broken
		 * upstream. */
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

		// --- Context-menu openers (canvas-005a) ----------------------
		// `menu` is a `$state` MenuState | null. Opening a new menu simply
		// reassigns; "last click wins" falls out for free, so right-clicking
		// on the empty canvas while a tile menu is open swaps to the
		// empty-canvas menu (acceptance criterion).

		/** Open the empty-canvas context menu at viewport coords `(x, y)`.
		 * Items, in order:
		 *   - "Add project…"               (canvas-005a, always shown)
		 *   - "Scan folder for projects…"  (canvas-005b, always shown)
		 *   - "Manage scan roots…"         (canvas-005b, hidden when
		 *                                   `scanRootsCount === 0`)
		 *
		 * The `scanRootsCount` cache is refreshed on mount, after every
		 * `addScanRoot` resolves, and after every `removeScanRoot` resolves
		 * — see `refreshScanRootsCount`. */
		function openEmptyCanvasMenu(x: number, y: number) {
			menu = {
				x,
				y,
				items: [
					{
						label: 'Add project…',
						onClick: () => void runAddProjectFlow()
					},
					{
						label: 'Scan folder for projects…',
						onClick: () => void runScanFolderFlow()
					},
					{
						label: 'Manage scan roots…',
						onClick: () => void runManageScanRootsFlow(),
						hidden: scanRootsCount === 0
					}
				]
			};
		}

		/** Open the tile context menu at viewport coords `(x, y)` for the
		 * project with `projectId`. canvas-005a contributes a single item:
		 * "Remove project". A missing tile uses the same menu, by design —
		 * removing a missing project is the supported recovery affordance. */
		function openTileMenu(x: number, y: number, projectId: number) {
			menu = {
				x,
				y,
				items: [
					{
						label: 'Remove project',
						onClick: () => void runRemoveProjectFlow(projectId)
					}
				]
			};
		}

		/** "Add project…" flow (canvas-005a). Opens a Tauri-native folder
		 * picker, invokes `registerProject` on a chosen path, and routes the
		 * three terminal states:
		 *
		 *   - cancelled picker  → silent close, no toast
		 *   - register success  → backend fires `ProjectAdded`; the live-add
		 *                         chain renders the tile (no UI side-effect
		 *                         here)
		 *   - "not an Agentheim project" → error toast for 3000ms
		 *
		 * Any other unexpected error is also routed through the toast so the
		 * user sees something — silent failure on a user-initiated affordance
		 * is the worst UX outcome. */
		async function runAddProjectFlow() {
			menu = null;
			let picked: string | string[] | null = null;
			try {
				picked = await openDialog({ directory: true, multiple: false });
			} catch (e) {
				logToCore('error', `open dialog failed: ${e}`);
				showToast(`could not open folder picker: ${e}`);
				return;
			}
			if (picked === null) return; // user cancelled — silent
			const path = Array.isArray(picked) ? picked[0] : picked;
			if (!path) return;
			try {
				await registerProject(path);
				// Success path is silent here — the backend's `ProjectAdded`
				// flows through `onDomainEvent` → `enqueueLiveAdd` → tile.
			} catch (e) {
				const msg = String(e);
				// The IPC contract: `register_project` rejects with the
				// exact string "not an Agentheim project". Surface it
				// verbatim (it is the message the user needs to see).
				showToast(msg);
				logToCore('warn', `register_project rejected: ${msg}`);
			}
		}

		/** "Remove project" flow (canvas-005a). No confirmation step — ADR-005's
		 * 30-day undo window (re-add via `register_project` restores the tile
		 * in place from the preserved `tile_positions` row) is the safety net.
		 * The actual tile-drop happens through the `project_removed` event
		 * handler in `onDomainEvent`, not here. */
		async function runRemoveProjectFlow(projectId: number) {
			menu = null;
			try {
				await removeProject(projectId);
			} catch (e) {
				const msg = String(e);
				showToast(`could not remove project: ${msg}`);
				logToCore('error', `remove_project failed for ${projectId}: ${msg}`);
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

		// Missing-tile corner glyph (canvas-005a). A `✕` in the tile's
		// top-right corner, `statusMissing` colour, glyph size driven by
		// `spacing.lg` (16px world-space) so it scales with the camera. Paired
		// with the dimmed tile body + magenta border, this is the styleguide's
		// "missing" state applied to project tiles (status palette is colour +
		// glyph, not colour alone).
		function makeMissingGlyph(
			nodeScreen: Point,
			nodeW: number,
			z: number
		): Container {
			const c = new Container();
			const glyph = new Text({
				text: statusGlyph.missing,
				style: {
					fill: color.statusMissing,
					fontFamily: typography.fontFamily,
					fontSize: Math.max(8, spacing.lg * z),
					fontWeight: String(typography.weightBold) as '700'
				}
			});
			glyph.anchor.set(1, 0);
			glyph.position.set(
				nodeScreen.x + nodeW - spacing.sm * z,
				nodeScreen.y + spacing.sm * z
			);
			c.addChild(glyph);
			return c;
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
				// Right-button on a tile = open the tile context menu at the
				// click coordinates (canvas-005a). The window-level
				// capture-phase dismisser has already nulled any previous
				// menu, so this reassignment effectively swaps menus when the
				// user right-clicks elsewhere while a menu is open.
				if (e.button === 2) {
					// Pixi's federated stopPropagation doesn't reach the DOM,
					// so the canvas-level pointerdown listener would otherwise
					// fire next and overwrite our tile menu with the empty-
					// canvas menu. Halt the underlying DOM event.
					if (e.nativeEvent && 'stopImmediatePropagation' in e.nativeEvent) {
						e.nativeEvent.stopImmediatePropagation();
					}
					openTileMenu(e.global.x, e.global.y, id);
					return;
				}
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

<!--
	Right-click context menu (canvas-005a). A screen-space HTML overlay (ADR-003
	overlay layer) positioned absolutely at the click coordinates and styled
	from the design tokens. Items array is the seam canvas-005b extends; the
	component otherwise stays put.
-->
{#if menu}
	{@const visibleItems = menu.items.filter((it) => !it.hidden)}
	<div
		class="context-menu"
		role="menu"
		tabindex="-1"
		bind:this={menuEl}
		style="left: {menuLeft}px; top: {menuTop}px;"
		onpointerdown={(e) => e.stopPropagation()}
		oncontextmenu={(e) => e.preventDefault()}
	>
		{#each visibleItems as item (item.label)}
			<button
				type="button"
				class="context-menu-item"
				role="menuitem"
				onclick={() => {
					item.onClick();
				}}
			>
				{item.label}
			</button>
		{/each}
	</div>
{/if}

<!--
	Error toast (canvas-005a). One toast at a time; new toast replaces the
	current one. Pinned to top-center of the viewport. Used for the
	`register_project` rejection path.
-->
{#if toastMessage}
	<div class="error-toast" role="status" aria-live="polite">
		{toastMessage}
	</div>
{/if}

<!--
	Discovery checklist modal (canvas-005b). One reactive snapshot of
	`checklistModal`; the same modal is reused for the post-`add_scan_root`
	flow and the post-`rescan_scan_root` flow (the `isRescan` flag
	differentiates the header).
-->
{#if checklistModal}
	{@const state = checklistModal}
	{@const togglableCount = countTogglableRows(state)}
	{@const isEmpty = state.rows.length === 0}
	<Modal onclose={() => (checklistModal = null)}>
		{#snippet header()}
			<div class="checklist-header">
				<span class="checklist-header-path" title={state.rootPath}>
					{state.rootPath}
				</span>
				<span class="checklist-header-suffix">
					{#if isEmpty}
						— no Agentheim projects found{state.isRescan ? ' (rescan)' : ''}
					{:else}
						— {state.rows.length} project{state.rows.length === 1 ? '' : 's'} found{state.isRescan
							? ' (rescan)'
							: ''}
					{/if}
				</span>
				{#if !isEmpty && togglableCount > 0}
					<div class="checklist-header-controls">
						<button
							type="button"
							class="modal-link-button"
							onclick={() => selectAllTogglable(state)}
						>
							Select all
						</button>
						<button
							type="button"
							class="modal-link-button"
							onclick={() => selectNoneTogglable(state)}
						>
							Select none
						</button>
					</div>
				{/if}
			</div>
		{/snippet}
		{#snippet body()}
			{#if isEmpty}
				<p class="checklist-empty">
					Nothing to import. Re-running the scan later will pick up new
					clones.
				</p>
			{:else}
				<ul class="checklist-list">
					{#each state.rows as row (row.candidate.path)}
						<li class="checklist-row" class:already-imported={row.candidate.already_imported}>
							<label class="checklist-row-label">
								<input
									type="checkbox"
									bind:checked={row.ticked}
									disabled={row.candidate.already_imported}
								/>
								<span class="checklist-row-content">
									<span class="checklist-row-path">
										<span class="checklist-row-pathtext">{row.candidate.path}</span>
										{#if row.candidate.already_imported}
											<span class="checklist-row-badge">imported</span>
										{/if}
									</span>
									<span class="checklist-row-nickname">
										{row.candidate.nickname_suggestion}
									</span>
								</span>
							</label>
						</li>
					{/each}
				</ul>
			{/if}
		{/snippet}
		{#snippet footer()}
			{#if isEmpty}
				<button
					type="button"
					class="modal-button modal-button-primary"
					onclick={() => (checklistModal = null)}
				>
					OK
				</button>
			{:else}
				<button
					type="button"
					class="modal-button modal-button-secondary"
					onclick={() => (checklistModal = null)}
				>
					Cancel
				</button>
				<button
					type="button"
					class="modal-button modal-button-primary"
					disabled={!hasNewSelection(state)}
					onclick={() => void runImportSelected(state)}
				>
					Import selected
				</button>
			{/if}
		{/snippet}
	</Modal>
{/if}

<!--
	Scan-roots management modal (canvas-005b). Lists every registered scan
	root with its child-project count, plus a Rescan and Remove button per
	row. The cascade-remove confirmation stacks ON TOP of this modal — the
	one explicit exception to "one modal at a time".
-->
{#if manageModal}
	{@const manage = manageModal}
	<Modal onclose={() => (manageModal = null)}>
		{#snippet header()}
			Scan roots
		{/snippet}
		{#snippet body()}
			<ul class="manage-list">
				{#each manage.roots as entry (entry.root.id)}
					<li class="manage-row">
						<div class="manage-row-info">
							<span class="manage-row-path" title={entry.root.path}>
								{entry.root.path}
							</span>
							<span class="manage-row-count">
								{entry.childCount} project{entry.childCount === 1 ? '' : 's'}
							</span>
						</div>
						<div class="manage-row-actions">
							<button
								type="button"
								class="modal-button modal-button-secondary"
								onclick={() => void runRescanFlow(entry.root)}
							>
								Rescan
							</button>
							<button
								type="button"
								class="modal-button modal-button-secondary modal-button-destructive"
								onclick={() => openConfirmRemove(entry)}
							>
								Remove
							</button>
						</div>
					</li>
				{/each}
			</ul>
		{/snippet}
		{#snippet footer()}
			<button
				type="button"
				class="modal-button modal-button-secondary"
				onclick={() => (manageModal = null)}
			>
				Close
			</button>
		{/snippet}
	</Modal>
{/if}

<!--
	Cascade-remove confirmation dialog (canvas-005b). Renders ON TOP of the
	manage modal — the stack ordering is just the source order here; the
	confirmation's higher `z-index` (via the second Modal mount) wins.
	Communicates the ADR-013 retention exception: cascade hard-deletes tile
	state, NOT subject to ADR-005's 30-day window.
-->
{#if confirmRemoveModal}
	{@const c = confirmRemoveModal}
	<Modal maxWidth="480px" onclose={() => (confirmRemoveModal = null)}>
		{#snippet header()}
			Remove scan root
		{/snippet}
		{#snippet body()}
			<p class="confirm-body">
				Remove scan root <span class="confirm-path">{c.root.path}</span> and
				all {c.childCount} project{c.childCount === 1 ? '' : 's'} discovered
				under it? Tile state for those projects will not be retained.
			</p>
		{/snippet}
		{#snippet footer()}
			<button
				type="button"
				class="modal-button modal-button-secondary"
				onclick={() => (confirmRemoveModal = null)}
			>
				Cancel
			</button>
			<button
				type="button"
				class="modal-button modal-button-primary modal-button-destructive"
				onclick={() => void runRemoveScanRoot(c.root.id)}
			>
				Remove
			</button>
		{/snippet}
	</Modal>
{/if}

<!--
	Outside-click dismissal for the context menu (canvas-005a). The handler
	must run in the capture phase — before the canvas/tile pointerdown that
	may itself want to open a new menu. The sequence "user clicks elsewhere
	while a menu is open" therefore becomes (1) dismiss the current menu, (2)
	the canvas/tile handler optionally opens a new one (right-click) or just
	starts a pan/drag (left/middle-click). The capture-phase listener is
	added imperatively in `onMount`; Svelte's `<svelte:window>` is bubble-only.
-->


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

	/*
	 * Context menu — a screen-space overlay (ADR-003) positioned at click
	 * coordinates. Tokens drive every value (canvas-005a inlines the styling
	 * contract pending a possible follow-up STYLEGUIDE entry).
	 */
	.context-menu {
		position: absolute;
		min-width: 160px;
		background: var(--guppi-tile-fill);
		border: 1px solid var(--guppi-tile-border);
		border-radius: var(--guppi-radius-tile);
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		color: var(--guppi-tile-text);
		padding: var(--guppi-space-xs) 0;
		z-index: 10;
		user-select: none;
	}
	.context-menu-item {
		display: block;
		width: 100%;
		background: transparent;
		border: 0;
		text-align: left;
		padding: var(--guppi-space-sm) var(--guppi-space-md);
		font-family: inherit;
		font-size: inherit;
		color: inherit;
		cursor: pointer;
	}
	.context-menu-item:hover,
	.context-menu-item:focus {
		background: var(--guppi-canvas-bg-raised);
		outline: none;
	}

	/*
	 * Error toast (canvas-005a). Pinned to top-center, `statusMissing` border
	 * to signal refusal (not failure). Auto-dismisses after 3s — the JS timer
	 * just clears `toastMessage`.
	 */
	.error-toast {
		position: absolute;
		top: var(--guppi-space-lg);
		left: 50%;
		transform: translateX(-50%);
		background: var(--guppi-tile-fill);
		border: 1px solid var(--guppi-status-missing);
		border-radius: var(--guppi-radius-tile);
		color: var(--guppi-tile-text);
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		padding: var(--guppi-space-md) var(--guppi-space-lg);
		z-index: 11;
		pointer-events: none;
	}

	/*
	 * Modal-internal styling (canvas-005b). The `Modal.svelte` primitive
	 * owns the chrome (header/body/footer padding, backdrop, dismissal);
	 * these classes style the contents of each consumer's snippets. Every
	 * value is token-driven; no hard-coded colours/sizes/typography.
	 */

	/* Checklist modal header */
	.checklist-header {
		display: flex;
		flex-wrap: wrap;
		align-items: baseline;
		gap: var(--guppi-space-sm);
	}
	.checklist-header-path {
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-body);
		color: var(--guppi-tile-text);
		direction: rtl; /* truncate from the left for long paths */
		text-align: left;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		max-width: 100%;
	}
	.checklist-header-suffix {
		color: var(--guppi-tile-text-muted);
		font-weight: var(--guppi-weight-regular);
	}
	.checklist-header-controls {
		margin-left: auto;
		display: flex;
		gap: var(--guppi-space-md);
	}

	/* Checklist modal body */
	.checklist-empty {
		color: var(--guppi-tile-text-muted);
		margin: 0;
	}
	.checklist-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--guppi-space-xs);
	}
	.checklist-row {
		border-radius: var(--guppi-radius-badge);
	}
	.checklist-row:hover {
		background: var(--guppi-canvas-bg-raised);
	}
	.checklist-row.already-imported {
		opacity: 0.6;
	}
	.checklist-row.already-imported:hover {
		background: transparent; /* immune rows do not hover-highlight */
	}
	.checklist-row-label {
		display: flex;
		align-items: flex-start;
		gap: var(--guppi-space-md);
		padding: var(--guppi-space-sm) var(--guppi-space-md);
		cursor: pointer;
	}
	.checklist-row.already-imported .checklist-row-label {
		cursor: default;
	}
	.checklist-row-content {
		display: flex;
		flex-direction: column;
		gap: var(--guppi-space-xs);
		min-width: 0;
		flex: 1 1 auto;
	}
	.checklist-row-path {
		display: flex;
		align-items: center;
		gap: var(--guppi-space-sm);
		min-width: 0;
	}
	.checklist-row-pathtext {
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-body);
		color: var(--guppi-bc-text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}
	.checklist-row-badge {
		background: var(--guppi-status-idle);
		color: var(--guppi-status-text);
		border-radius: var(--guppi-radius-badge);
		padding: 0 var(--guppi-space-sm);
		font-size: var(--guppi-size-caption);
		font-weight: var(--guppi-weight-bold);
		font-family: var(--guppi-font-family);
		flex-shrink: 0;
	}
	.checklist-row-nickname {
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-caption);
		color: var(--guppi-bc-text-muted);
	}

	/* Manage-roots modal body */
	.manage-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--guppi-space-sm);
	}
	.manage-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--guppi-space-md);
		padding: var(--guppi-space-sm) var(--guppi-space-md);
		border-radius: var(--guppi-radius-badge);
	}
	.manage-row:hover {
		background: var(--guppi-canvas-bg-raised);
	}
	.manage-row-info {
		display: flex;
		flex-direction: column;
		gap: var(--guppi-space-xs);
		min-width: 0;
		flex: 1 1 auto;
	}
	.manage-row-path {
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-body);
		color: var(--guppi-tile-text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.manage-row-count {
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-caption);
		color: var(--guppi-tile-text-muted);
	}
	.manage-row-actions {
		display: flex;
		gap: var(--guppi-space-sm);
		flex-shrink: 0;
	}

	/* Confirm-remove modal body */
	.confirm-body {
		margin: 0;
		color: var(--guppi-tile-text);
		line-height: 1.5;
	}
	.confirm-path {
		font-family: var(--guppi-font-family-mono);
		color: var(--guppi-bc-text);
	}

	/*
	 * Buttons (canvas-005b). The styleguide does not yet codify a Modal
	 * button pattern; these classes inline the contract. If a third
	 * consumer surface lands, lift to `STYLEGUIDE.md` as a followup.
	 */
	.modal-button {
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		font-weight: var(--guppi-weight-medium);
		padding: var(--guppi-space-sm) var(--guppi-space-md);
		border-radius: var(--guppi-radius-badge);
		cursor: pointer;
		border: 1px solid transparent;
	}
	.modal-button-primary {
		background: var(--guppi-tile-border);
		color: var(--guppi-status-text);
		border-color: var(--guppi-tile-border);
	}
	.modal-button-primary:disabled {
		background: var(--guppi-canvas-bg-raised);
		color: var(--guppi-tile-text-muted);
		border-color: var(--guppi-canvas-bg-raised);
		cursor: not-allowed;
	}
	.modal-button-secondary {
		background: transparent;
		color: var(--guppi-tile-text);
		border: 1px solid var(--guppi-tile-border);
	}
	.modal-button-destructive {
		border-color: var(--guppi-status-missing);
	}
	.modal-button-destructive.modal-button-primary {
		background: var(--guppi-status-missing);
		color: var(--guppi-status-text);
	}

	/* Link-style buttons for the "Select all" / "Select none" header
	 * controls — chromeless, the styleguide's `tile-text` colour. */
	.modal-link-button {
		background: transparent;
		border: 0;
		padding: 0;
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		color: var(--guppi-tile-border);
		cursor: pointer;
		text-decoration: underline;
	}
</style>
