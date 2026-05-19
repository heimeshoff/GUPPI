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
		loadBcPositions,
		loadCamera,
		loadTilePosition,
		onDomainEvent,
		registerProject,
		removeProject,
		removeScanRoot,
		rescanScanRoot,
		saveBcPosition,
		saveCamera,
		saveTilePosition,
		logToCore
	} from './ipc';
	import type {
		BoundedContext,
		CameraState,
		Point,
		ProjectSnapshot,
		Relationship,
		ScanCandidate,
		ScanRootRow
	} from './types';
	import { applyDomainEvent } from './snapshot-patch';
	import { spiralPosition } from './tile-layout';
	import { computeBcLayout, type BcFrameLayout, type BcPositionMap } from './bc-layout';
	import {
		IDLE,
		onPointerDown as dragOnPointerDown,
		onPointerMove as dragOnPointerMove,
		onPointerUp as dragOnPointerUp,
		onPointerCancel as dragOnPointerCancel,
		onPointerLeave as dragOnPointerLeave,
		type DragState
	} from './drag-controller';
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
	import {
		themeState,
		initTheme,
		setTheme,
		onThemeChange
	} from './theme.svelte';

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
	//
	// `canvas-007` extends the entry shape from `{ id, snapshot, pos }` to
	// `{ id, snapshot, pos, bcLayout, bcPositions }`:
	//   - `pos`         : world-space top-left of the project's FRAME
	//                     (replaces the orbit-baseline `pos` of the project
	//                     tile, semantically the same anchor).
	//   - `bcLayout`    : deterministic force-directed positions of the
	//                     BCs inside the frame, plus the auto-fit frame
	//                     width/height. Recomputed on BC add / remove /
	//                     relationship-change and on BC drag end.
	//   - `bcPositions` : per-BC manual-drag overrides in frame-local
	//                     coords. Persisted via `saveBcPosition` and
	//                     batch-loaded via `loadBcPositions` at project
	//                     paint. BCs present here are pinned during
	//                     force-directed re-layout (`bc-layout.ts`).
	interface ProjectEntry {
		id: number;
		snapshot: ProjectSnapshot;
		pos: Point;
		bcLayout: BcFrameLayout;
		bcPositions: BcPositionMap;
	}
	let projects = $state<ProjectEntry[]>([]);

	// --- Persistent scene-graph display objects (canvas-015) ----------
	// One `FrameDisplayObjects` per rendered project, kept across renders
	// and updated in place. Instantiated on `project_added` or initial
	// `refresh`; removed on `project_removed`. Children live in world
	// coordinates — `world.position` and `world.scale` carry the camera
	// transform on the parent container so pan/zoom never rebuild children.
	//
	// `Map<number, FrameDisplayObjects>` keyed by `entry.id`. Kept OUTSIDE
	// `$state` — these are imperative Pixi handles, not reactive data.
	interface BcDisplayObjects {
		container: Container; // BC bubble container (frame-local coords)
		body: Graphics;       // bubble body (fill + border)
		focusRing: Graphics;  // hover focus ring (toggled via .visible)
		pillBg: Graphics;     // counts pill background
		pillText: Text;       // counts label
		title: Text;          // BC name
		badge: BadgeDisplayObjects;
	}
	interface BadgeDisplayObjects {
		container: Container;
		body: Graphics;
		glyph: Text;
	}
	interface FrameDisplayObjects {
		container: Container;          // parent of everything for one project; positioned at entry.pos
		body: Graphics;                // frame body (fill + border)
		header: Graphics;              // header fill + divider
		title: Text;                   // project title
		counts: Text;                  // total task count
		focusRing: Graphics;           // hover halo (toggled via .visible)
		missingGlyph: Text;            // missing-tile ✕ glyph (toggled via .visible)
		emptyText: Text;               // "No bounded contexts yet" placeholder (toggled via .visible)
		edges: Graphics;               // all intra-project edges in one Graphics (cleared+redrawn on layout change)
		bcsRoot: Container;            // parent of BC bubbles
		bcs: Map<string, BcDisplayObjects>; // BC name -> display objects
	}
	const frameObjects = new Map<number, FrameDisplayObjects>();

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

	/** Recompute one project's BC layout in place. Called on BC add /
	 *  remove / relationship-change and on BC drag end. Deterministic and
	 *  one-shot — no requestAnimationFrame loop. */
	function recomputeBcLayout(entry: ProjectEntry) {
		entry.bcLayout = computeBcLayout(entry.snapshot.bcs, entry.bcPositions);
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
	export function deriveBcStatus(bc: BoundedContext): TaskState {
		const c = bc.task_counts;
		const total = c.backlog + c.todo + c.doing + c.done;
		if (total === 0) return 'missing';
		if (c.doing > 0) return 'running';
		if (c.backlog > 0 && c.todo === 0) return 'blocked';
		return 'idle';
	}

	/**
	 * Screen-space text-floor counter-scale (ADR-003 Extension 2026-05-19
	 * invariant #6 — "BC text floor — no hiding").
	 *
	 * Under canvas-015's camera-as-stage-transform model, on-screen CSS-px
	 * size of a `Text` child of `world` is `fontSize * world.scale`. To
	 * honour "every BC always shows its name (if you squint)" we counter-
	 * scale a title `Text` upward in local space when the nominal screen
	 * size would fall below `floorPx`. Returns the multiplier to apply via
	 * `t.scale.set(s)`; returns 1 at default zoom and above so there is no
	 * cost in the common case.
	 *
	 * Pure / side-effect-free. Sites that apply it:
	 *   - project frame title          (fontSize = typography.sizeTitle)
	 *   - project missing-tile glyph   (fontSize = typography.sizeTitle)
	 *   - BC bubble title              (fontSize = typography.sizeBody)
	 *   - BC counts pill text          (fontSize = typography.sizeCaption)
	 *
	 * truncateTextToWidth must run AFTER applying this scale so the
	 * binary-search measures `t.width` (which is `baselineWidth * s`)
	 * against the world-space maxWidth budget.
	 */
	function screenSpaceTitleScale(fontSize: number, z: number, floorPx = 8): number {
		const nominal = fontSize * z;
		if (nominal >= floorPx) return 1;
		return floorPx / nominal;
	}

	/**
	 * Truncate a Pixi `Text`'s displayed string so it fits within
	 * `maxWidth` pixels. Binary-searches the largest prefix of `fullText`
	 * whose rendered width (with ellipsis appended) is <= maxWidth. If
	 * even the ellipsis alone doesn't fit, renders an empty string.
	 *
	 * canvas-013 (revised 2026-05-19) — used by both the project frame
	 * title (`updateFrameDisplayObjects`) and the BC bubble title
	 * (`updateBcDisplayObjects`) to keep titles inside their respective
	 * frames at every zoom.
	 */
	function truncateTextToWidth(t: Text, fullText: string, maxWidth: number) {
		if (maxWidth <= 0) {
			t.text = '';
			return;
		}
		t.text = fullText;
		if (t.width <= maxWidth) return;
		const ellipsis = '…';
		t.text = ellipsis;
		if (t.width > maxWidth) {
			t.text = '';
			return;
		}
		let lo = 0;
		let hi = fullText.length;
		while (lo < hi) {
			const mid = (lo + hi + 1) >> 1;
			t.text = fullText.slice(0, mid) + ellipsis;
			if (t.width <= maxWidth) lo = mid;
			else hi = mid - 1;
		}
		t.text = fullText.slice(0, lo) + ellipsis;
	}

	onMount(() => {
		let app: Application | null = null;
		let unlistenEvent: (() => void) | null = null;
		let unlistenTheme: (() => void) | null = null;
		let disposed = false;

		// PixiJS scene graph: world container holds everything; camera maps it.
		const world = new Container();
		let renderScene: () => void = () => {};

		// Eased camera transition state (motion budget): when set, the ticker
		// lerps the camera toward `cameraTarget` and clears it when close.
		let cameraTarget: CameraState | null = null;
		let cameraAnimStart = 0;

		// --- shared drag controller (canvas-002, extracted in canvas-012) --
		// One set of `window` pointer listeners for *all* drag targets. The
		// drag-state machine itself — formerly four sibling variables
		// (`dragProjectId`, `dragBcName`, `dragOriginX/Y` here plus
		// `panning` in the empty-canvas pointerdown closure) — is now a
		// single discriminated union owned by the pure `drag-controller.ts`
		// module. Canvas.svelte holds the current `DragState` and delegates
		// every pointer event's transition to the module. Persistence
		// intents (`saveTilePosition` / `saveBcPosition` / `saveCamera`)
		// fire from this scope so the controller stays Svelte/Pixi/IPC-free
		// (verification surface, same pattern as `tile-layout.ts` /
		// `bc-layout.ts` / `snapshot-patch.ts`).
		let dragState: DragState = IDLE;

		(async () => {
			// --- restore persisted theme BEFORE the canvas boots ---------
			// design-system-004: read the v5 `preferences.theme` row from
			// SQLite and apply the active palette + HTML `data-theme`
			// attribute *before* PixiJS reads `color.canvasBg` below. The
			// first paint then lands in the correct palette without a
			// visible flash from dark to light. Failures fall back to dark
			// (the migration's default seed) without throwing.
			try {
				await initTheme();
			} catch (e) {
				logToCore('warn', `could not restore persisted theme: ${e}`);
			}

			app = new Application();
			await app.init({
				resizeTo: host,
				background: color.canvasBg,
				antialias: true,
				// canvas-013 AC #1 — Crispness invariant.
				// Without these two, PixiJS v8 defaults to
				// `resolution = 1` regardless of the display's actual DPR;
				// every Text and Graphics is rasterized at 1× and the GPU
				// upscales to device pixels, producing a bilinear-smear
				// blur halo at zoom-out and a soft edge everywhere. Setting
				// `resolution = devicePixelRatio` + `autoDensity = true`
				// tells Pixi to render at the framebuffer's real resolution
				// and downscale via CSS, the standard HiDPI idiom.
				// Coordinate inputs (positions, line widths) stay in CSS
				// pixels — Pixi multiplies by `resolution` internally — so
				// the rest of the scene math is unchanged by this flip.
				// (ADR-003 extension 2026-05-19.)
				resolution: window.devicePixelRatio || 1,
				autoDensity: true
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
			// `canvas-007`: each project renders as a FRAME (rounded rect
			// border + header bar carrying name/counts) containing its BCs
			// as interior bubbles (`bcLayout.positions`), with intra-project
			// edges drawn between BCs by relationship type. Project->BC
			// orbit edges are retired; containment (BC inside frame)
			// replaces the line.
			// canvas-015: persistent scene graph + camera as stage transform.
			// The world container's `position` and `scale` carry the camera
			// transform — pan moves `world.position`, zoom multiplies
			// `world.scale`, and the GPU rasterises every child once per
			// frame against that transform. Children are drawn in WORLD
			// coordinates and instantiated once per project (kept across
			// renders in `frameObjects`); pan does not allocate. Stroke
			// widths are pre-divided by zoom so the on-screen border stays
			// constant CSS px (ADR-003 Extension 2026-05-19, invariant #4)
			// — `repaint(z)` updates them on zoom change.
			renderScene = () => {
				if (!app) return;
				const z = camera.zoom;

				// Camera-as-stage-transform: world.position is the pan in
				// screen pixels, world.scale is the zoom. With these set,
				// every child rendered in world coordinates appears at the
				// right place on screen WITHOUT a per-render rebuild.
				world.position.set(camera.pan_x, camera.pan_y);
				world.scale.set(z);

				// Add the screen-space voice indicator to the stage (not
				// world, so it does not pan/zoom). Instantiated once at
				// mount; only its content updates here.
				ensureVoiceIndicator();
				updateVoiceIndicator();

				// Reconcile per-project display objects against `projects`.
				// 1) Drop frames that no longer exist (project_removed).
				const liveIds = new Set(projects.map((p) => p.id));
				for (const id of Array.from(frameObjects.keys())) {
					if (!liveIds.has(id)) {
						const obj = frameObjects.get(id)!;
						world.removeChild(obj.container);
						obj.container.destroy({ children: true });
						frameObjects.delete(id);
					}
				}

				// 2) Create or update each project's persistent objects.
				// Geometry and text are updated in place; per-zoom stroke
				// widths are derived from `z` so the screen-space invariant
				// holds.
				for (const entry of projects) {
					let obj = frameObjects.get(entry.id);
					if (!obj) {
						obj = createFrameDisplayObjects(entry);
						frameObjects.set(entry.id, obj);
						world.addChild(obj.container);
					}
					updateFrameDisplayObjects(entry, obj, z);
				}
			};

			/** Repaint all persistent geometry that depends on zoom-derived
			 *  values (stroke widths in world-space) and on the active
			 *  palette. Called on theme flip and on zoom-change; does NOT
			 *  re-instantiate any Pixi object. */
			function repaint() {
				if (!app) return;
				const z = camera.zoom;
				for (const entry of projects) {
					const obj = frameObjects.get(entry.id);
					if (!obj) continue;
					updateFrameDisplayObjects(entry, obj, z);
				}
				updateVoiceIndicator();
			}

			// --- theme flip → repaint persistent geometry (design-system-004) -
			// PixiJS objects hold colour numerics at instantiation, so a
			// palette flip after the scene is built has no visible effect
			// unless we redraw. `applyPalette()` has already mutated the
			// active `color` / `statusColor` / `glow` objects by the time
			// this listener fires; `repaint()` clears+restrokes/refills the
			// persistent Graphics with the new palette in place (NOT a
			// re-instantiation — canvas-015 invariant). We also update the
			// WebGL renderer's clear colour so the canvas backdrop flips
			// alongside the world contents.
			unlistenTheme = onThemeChange(() => {
				if (!app) return;
				try {
					// `Renderer.background.color` is the same WebGL clear
					// colour that `app.init({ background: ... })` set.
					// PixiJS v8's `Color` setter accepts the 0xrrggbb
					// numeric directly.
					app.renderer.background.color = color.canvasBg;
				} catch {
					// Best-effort — older renderer versions or a partial
					// init shouldn't block the scene redraw.
				}
				repaint();
			});

			/** World-space center of one BC inside its project frame. The
			 *  layout's `positions` map carries frame-local coords; we add
			 *  the frame's world-space origin (`entry.pos`). Edges now live
			 *  in world space, so no camera projection is applied —
			 *  `world.scale` carries the on-screen zoom (canvas-015). */
			function bcCenterWorld(
				entry: ProjectEntry,
				bcName: string
			): Point | null {
				const local = entry.bcLayout.positions.get(bcName);
				if (!local) return null;
				return {
					x: entry.pos.x + local.x + shape.bcInsideWidth / 2,
					y: entry.pos.y + local.y + shape.bcInsideHeight / 2
				};
			}

			/** Draw a filled triangular arrowhead at `to`, pointing from
			 *  `from -> to`. Coordinates are in WORLD space; geometry that
			 *  must read at constant screen-space CSS px (head length / head
			 *  width) is pre-divided by `z` so the parent `world.scale`'s
			 *  multiplication restores the token value on screen.
			 *  pullBack stays in world-space (`shape.bcInsideWidth / 2`)
			 *  because it is the distance from the BC bubble's edge — the
			 *  bubble itself lives in world space now (canvas-015). */
			function drawArrowhead(
				g: Graphics,
				from: Point,
				to: Point,
				z: number,
				col: number
			) {
				const dx = to.x - from.x;
				const dy = to.y - from.y;
				const len = Math.sqrt(dx * dx + dy * dy);
				if (len < 0.0001) return;
				const ux = dx / len;
				const uy = dy / len;
				// canvas-015: world-space arrowhead size = screen-px / z so the
				// world.scale = z multiplication makes the on-screen size match
				// the design token. pullBack is the world-space distance from
				// the bubble edge — bubble is now drawn in world coords, so
				// pullBack is a world-space token value (no `* z` and no `/ z`).
				const headLen = shape.arrowheadLength / z;
				const headW = shape.arrowheadWidth / z;
				const pullBack = shape.bcInsideWidth / 2;
				const tipX = to.x - ux * pullBack;
				const tipY = to.y - uy * pullBack;
				const baseX = tipX - ux * headLen;
				const baseY = tipY - uy * headLen;
				// Perpendicular for the arrowhead's width axis.
				const px = -uy;
				const py = ux;
				const leftX = baseX + px * (headW / 2);
				const leftY = baseY + py * (headW / 2);
				const rightX = baseX - px * (headW / 2);
				const rightY = baseY - py * (headW / 2);
				g.moveTo(tipX, tipY)
					.lineTo(leftX, leftY)
					.lineTo(rightX, rightY)
					.lineTo(tipX, tipY)
					.fill(col);
			}

			/** Draw the ACL notch — a small filled triangle at the midpoint
			 *  pointing toward the upstream end of the edge. World-space
			 *  coordinates with size pre-divided by `z` for the same reason
			 *  as `drawArrowhead`. */
			function drawAclNotch(
				g: Graphics,
				upstream: Point,
				downstream: Point,
				z: number,
				col: number
			) {
				const dx = downstream.x - upstream.x;
				const dy = downstream.y - upstream.y;
				const len = Math.sqrt(dx * dx + dy * dy);
				if (len < 0.0001) return;
				const ux = dx / len;
				const uy = dy / len;
				const midX = (upstream.x + downstream.x) / 2;
				const midY = (upstream.y + downstream.y) / 2;
				// canvas-015 — world-space notch size = screen-px / z.
				const size = shape.aclNotchSize / z;
				// Tip points toward upstream.
				const tipX = midX - ux * (size / 2);
				const tipY = midY - uy * (size / 2);
				const baseX = midX + ux * (size / 2);
				const baseY = midY + uy * (size / 2);
				const px = -uy;
				const py = ux;
				const leftX = baseX + px * (size / 2);
				const leftY = baseY + py * (size / 2);
				const rightX = baseX - px * (size / 2);
				const rightY = baseY - py * (size / 2);
				g.moveTo(tipX, tipY)
					.lineTo(leftX, leftY)
					.lineTo(rightX, rightY)
					.lineTo(tipX, tipY)
					.fill(col);
			}

			// --- canvas-015: persistent project frame lifecycle --------------
			// `createFrameDisplayObjects` instantiates each Pixi object ONCE
			// per project (body, header, title, counts, focus ring, missing
			// glyph, empty-state text, edges Graphics, BC bubbles parent).
			// `updateFrameDisplayObjects` updates them in place every render
			// — geometry (.clear() + redraw) + text contents + visibility
			// flags. NO `world.removeChildren()`; NO per-render `new
			// Graphics()`. Pan path skips this function entirely (camera
			// transform is on `world` directly); zoom and topology-change
			// paths run through it.

			function createFrameDisplayObjects(entry: ProjectEntry): FrameDisplayObjects {
				const container = new Container();
				const body = new Graphics();
				const header = new Graphics();
				const focusRing = new Graphics();
				focusRing.visible = false;
				const edges = new Graphics();
				const bcsRoot = new Container();
				const missingGlyph = new Text({
					text: statusGlyph.missing,
					style: {
						fill: color.statusMissing,
						fontFamily: typography.fontFamily,
						fontSize: typography.sizeTitle,
						fontWeight: String(typography.weightBold) as '700'
					}
				});
				missingGlyph.anchor.set(1, 0);
				missingGlyph.visible = false;
				const title = new Text({
					text: entry.snapshot.name,
					style: {
						fill: color.frameTitleText,
						fontFamily: typography.fontFamily,
						fontSize: typography.sizeTitle,
						fontWeight: String(typography.weightBold) as '700'
					}
				});
				const counts = new Text({
					text: '',
					style: {
						fill: color.frameTitleTextMuted,
						fontFamily: typography.fontFamilyMono,
						fontSize: typography.sizeCaption
					}
				});
				counts.anchor.set(1, 0.5);
				const emptyText = new Text({
					text: 'No bounded contexts yet',
					style: {
						fill: color.frameEmptyText,
						fontFamily: typography.fontFamily,
						fontSize: typography.sizeBody
					}
				});
				emptyText.anchor.set(0.5, 0.5);
				emptyText.visible = false;

				// Edges live ABOVE the body/header so they aren't occluded by
				// the body fill, but BELOW BC bubbles so the bubble bodies
				// cover the edge endpoints.
				container.addChild(body);
				container.addChild(header);
				container.addChild(title);
				container.addChild(counts);
				container.addChild(missingGlyph);
				container.addChild(emptyText);
				container.addChild(focusRing);
				container.addChild(edges);
				container.addChild(bcsRoot);

				// Header bar drag + hover wiring. The hit area is updated in
				// `updateFrameDisplayObjects` whenever the frame size changes
				// (BC topology change re-runs layout).
				attachFrameHeaderInteractivity(container, entry.id);

				return {
					container,
					body,
					header,
					title,
					counts,
					focusRing,
					missingGlyph,
					emptyText,
					edges,
					bcsRoot,
					bcs: new Map()
				};
			}

			function updateFrameDisplayObjects(
				entry: ProjectEntry,
				obj: FrameDisplayObjects,
				z: number
			) {
				const fw = entry.bcLayout.width;
				const fh = entry.bcLayout.height;
				const headerH = shape.frameHeaderHeight;
				const isMissing = entry.snapshot.missing;
				const borderCol = isMissing ? color.statusMissing : color.frameBorder;

				// Position the per-frame container at the entry's world
				// coords. The parent `world` carries the camera transform.
				obj.container.position.set(entry.pos.x, entry.pos.y);
				obj.container.alpha = isMissing ? 0.5 : 1;

				// canvas-013 / canvas-015 stroke-width policy: world-space
				// stroke widths are pre-divided by z so the parent's
				// `world.scale = z` multiplication restores the constant
				// CSS-pixel value on screen.
				const strokeFrame = Math.max(1 / z, shape.borderWidthFrame / z);
				const strokeFocus = Math.max(1 / z, shape.borderWidthFocus / z);

				// --- Body (frame fill + border) ---
				obj.body.clear();
				obj.body
					.roundRect(0, 0, fw, fh, shape.radiusFrame)
					.fill(color.frameFill)
					.stroke({ width: strokeFrame, color: borderCol });

				// --- Header bar (top-rounded band) + divider line ---
				obj.header.clear();
				obj.header
					.roundRect(0, 0, fw, headerH, shape.radiusFrame)
					.fill(color.frameHeaderFill);
				obj.header
					.rect(0, headerH / 2, fw, headerH / 2)
					.fill(color.frameHeaderFill);
				obj.header
					.moveTo(0, headerH)
					.lineTo(fw, headerH)
					.stroke({ width: strokeFrame, color: color.frameHeaderDivider });

				// --- Header content: total task counts + project title ---
				// Text scales with zoom via `world.scale` (canvas-015). Font
				// sizes are the world-space (zoom-1) token values; truncation
				// budget is computed against world-space widths, then
				// truncateTextToWidth measures with `width / scale.x === 1`
				// directly — but the width of the Text changes with font
				// size, which is fixed at the token value here, so a
				// world-space width budget is correct.
				const totalTasks = entry.snapshot.bcs.reduce(
					(acc, b) =>
						acc +
						b.task_counts.backlog +
						b.task_counts.todo +
						b.task_counts.doing +
						b.task_counts.done,
					0
				);
				obj.counts.text = `${totalTasks} task${totalTasks === 1 ? '' : 's'}`;
				obj.counts.style.fill = color.frameTitleTextMuted;
				obj.counts.position.set(
					fw - shape.framePadding,
					headerH / 2
				);

				const titleX = shape.framePadding;
				const titleGap = shape.framePadding * 0.5;
				const titleMaxW = Math.max(
					0,
					fw - shape.framePadding * 2 - obj.counts.width - titleGap
				);
				obj.title.style.fill = color.frameTitleText;
				// ADR-003 invariant #6 (text floor): counter-scale upward when
				// nominal on-screen size would drop below the 8 CSS-px floor.
				// At default zoom and above, scale is 1 (no cost). Apply BEFORE
				// truncate so the binary search measures against the actually
				// rendered width.
				obj.title.scale.set(screenSpaceTitleScale(typography.sizeTitle, z));
				truncateTextToWidth(obj.title, entry.snapshot.name, titleMaxW);
				obj.title.position.set(
					titleX,
					(headerH - typography.sizeTitle) / 2
				);

				// --- Missing-tile glyph ---
				obj.missingGlyph.visible = isMissing;
				if (isMissing) {
					obj.missingGlyph.style.fill = color.statusMissing;
					// ADR-003 invariant #6 — text floor counter-scale.
					obj.missingGlyph.scale.set(
						screenSpaceTitleScale(typography.sizeTitle, z)
					);
					obj.missingGlyph.position.set(
						fw - spacing.sm,
						spacing.sm
					);
				}

				// --- Focus ring (canvas-015 AC #9: geometry persistent,
				// toggled via .visible). The ring is ALWAYS drawn with
				// current geometry so a subsequent pointerover can just
				// flip `.visible = true` without re-running this update.
				obj.focusRing.clear();
				obj.focusRing
					.roundRect(-2, -2, fw + 4, fh + 4, shape.radiusFrame + 2)
					.stroke({ width: strokeFocus, color: color.focusRing });
				obj.focusRing.visible = hoveredKey === `project:${entry.id}`;

				// --- Intra-project edges ---
				// All edges live in ONE persistent Graphics — clear+redraw on
				// every update. The Graphics's local coords are RELATIVE to
				// the frame container, so we offset world coords by entry.pos
				// before drawing. (Edges still use `bcCenterWorld` which
				// returns absolute world coords; we adjust for the frame
				// container origin here.)
				drawIntraProjectEdgesIntoFrame(entry, obj.edges, z);

				// --- Empty-state placeholder ---
				const isEmpty = entry.snapshot.bcs.length === 0 && !isMissing;
				obj.emptyText.visible = isEmpty;
				if (isEmpty) {
					obj.emptyText.style.fill = color.frameEmptyText;
					obj.emptyText.position.set(fw / 2, headerH + (fh - headerH) / 2);
				}

				// --- BC bubbles (per entry) ---
				// Reconcile: drop BC display objects that no longer exist;
				// create or update each currently-present BC.
				const liveBcNames = new Set(entry.snapshot.bcs.map((b) => b.name));
				for (const name of Array.from(obj.bcs.keys())) {
					if (!liveBcNames.has(name)) {
						const bcObj = obj.bcs.get(name)!;
						obj.bcsRoot.removeChild(bcObj.container);
						bcObj.container.destroy({ children: true });
						obj.bcs.delete(name);
					}
				}
				for (const bc of entry.snapshot.bcs) {
					const local = entry.bcLayout.positions.get(bc.name);
					if (!local) continue;
					let bcObj = obj.bcs.get(bc.name);
					if (!bcObj) {
						bcObj = createBcDisplayObjects(bc);
						attachBcInteractivity(bcObj.container, entry.id, bc.name);
						obj.bcs.set(bc.name, bcObj);
						obj.bcsRoot.addChild(bcObj.container);
					}
					updateBcDisplayObjects(entry.id, bc, local, bcObj, z);
				}

				// --- Header hit area (CSS-px aware via world.scale) ---
				// Hit-test predicates run in WORLD space because we set
				// `hitArea.contains` against the container's local coords.
				// Pixi's hit-test traverses the scene graph with the inverse
				// transform of each container — so coordinates passed to our
				// predicate are in the container's LOCAL frame, which after
				// canvas-015 is the frame's world-space coord system (entry.pos
				// is just the container's translation). The predicate is a
				// frame-local rect (0..fw × 0..headerH) — no `z` factor needed.
				obj.container.hitArea = {
					contains: (x: number, y: number) =>
						x >= 0 && x <= fw && y >= 0 && y <= headerH
				};
			}

			/** Edges live in WORLD space, but rendered into a Graphics that is
			 *  a child of the per-frame container (translated by entry.pos).
			 *  Adjust the absolute world coords from `bcCenterWorld` by
			 *  subtracting the frame origin so the Graphics's local coord
			 *  system places lines correctly. */
			function drawIntraProjectEdgesIntoFrame(
				entry: ProjectEntry,
				g: Graphics,
				z: number
			) {
				g.clear();
				// Translate so the Graphics's local space is the frame's
				// local space. We do this by drawing in (worldX - entry.pos.x,
				// worldY - entry.pos.y) coordinates. Implementation: temporarily
				// shift `bcCenterWorld`'s output by `-entry.pos` inside this fn.
				const bcs = entry.snapshot.bcs;
				if (bcs.length < 2) return;

				const indexByName = new Map<string, number>();
				bcs.forEach((bc, i) => indexByName.set(bc.name, i));

				const seen = new Set<string>();
				for (const bc of bcs) {
					for (const rel of bc.relationships) {
						const otherIdx = indexByName.get(rel.to);
						if (otherIdx === undefined) continue;
						const ownIdx = indexByName.get(bc.name);
						if (ownIdx === undefined || ownIdx === otherIdx) continue;
						const lo = Math.min(ownIdx, otherIdx);
						const hi = Math.max(ownIdx, otherIdx);
						const key = `${lo}-${hi}`;
						if (seen.has(key)) continue;
						seen.add(key);

						const fromAbs = bcCenterWorld(entry, bc.name);
						const toAbs = bcCenterWorld(entry, rel.to);
						if (!fromAbs || !toAbs) continue;
						const fromLocal = {
							x: fromAbs.x - entry.pos.x,
							y: fromAbs.y - entry.pos.y
						};
						const toLocal = {
							x: toAbs.x - entry.pos.x,
							y: toAbs.y - entry.pos.y
						};
						drawRelationshipEdgeLocal(bc, rel, g, fromLocal, toLocal, z);
					}
				}
			}

			/** Frame-local version of `drawRelationshipEdge`: callers
			 *  pre-compute the from/to points in the frame's local coord
			 *  system. Stroke widths and arrowhead/notch sizes are still
			 *  pre-divided by `z` for the constant-screen-px invariant. */
			function drawRelationshipEdgeLocal(
				from: BoundedContext,
				rel: Relationship,
				g: Graphics,
				fromCenter: Point,
				toCenter: Point,
				z: number
			) {
				void from; // direction comes off `rel`; `from` is unused here
				const w = shape.edgeWeight / z;
				const wConf = shape.edgeWeightConformist / z;

				switch (rel.type) {
					case 'shared-kernel':
					case 'partnership': {
						g.moveTo(fromCenter.x, fromCenter.y).lineTo(toCenter.x, toCenter.y);
						g.stroke({
							width: Math.max(1 / z, w),
							color: color.edgeMutual
						});
						break;
					}
					case 'customer-supplier': {
						const downstream =
							rel.direction === 'upstream' ? fromCenter : toCenter;
						const upstream =
							rel.direction === 'upstream' ? toCenter : fromCenter;
						g.moveTo(upstream.x, upstream.y).lineTo(downstream.x, downstream.y);
						g.stroke({
							width: Math.max(1 / z, w),
							color: color.edgeUpstream
						});
						drawArrowhead(g, upstream, downstream, z, color.edgeUpstream);
						break;
					}
					case 'anticorruption-layer': {
						const downstream =
							rel.direction === 'upstream' ? fromCenter : toCenter;
						const upstream =
							rel.direction === 'upstream' ? toCenter : fromCenter;
						g.moveTo(upstream.x, upstream.y).lineTo(downstream.x, downstream.y);
						g.stroke({
							width: Math.max(1 / z, w),
							color: color.edgeACL
						});
						drawArrowhead(g, upstream, downstream, z, color.edgeACL);
						drawAclNotch(g, upstream, downstream, z, color.edgeACL);
						break;
					}
					case 'conformist': {
						const downstream =
							rel.direction === 'upstream' ? fromCenter : toCenter;
						const upstream =
							rel.direction === 'upstream' ? toCenter : fromCenter;
						g.moveTo(upstream.x, upstream.y).lineTo(downstream.x, downstream.y);
						g.stroke({
							width: Math.max(1 / z, wConf),
							color: color.edgeConformist
						});
						drawArrowhead(g, upstream, downstream, z, color.edgeConformist);
						break;
					}
				}
			}

			// --- camera interaction: pan (drag empty space) + zoom (wheel) -
			// Drag-state lives in the module-scope `dragState` declared
			// above and is owned by `drag-controller.ts`. Each listener
			// below is a 3–5-line shim: build a target descriptor or read
			// the current state, call the controller, apply the delta /
			// persistence intent, render. The previous four-variable
			// spread plus a function-scope `panning` flag — the structural
			// defect canvas-012 fixes — is gone.
			app.canvas.addEventListener('pointerdown', (e) => {
				// Right-button on empty canvas = open the empty-canvas context
				// menu at the click coordinates (canvas-005a). It is the only
				// way to start the "Add project…" flow. The window-level
				// capture-phase dismisser will have already nulled any
				// currently-open menu before this listener runs, so opening a
				// new one here works cleanly. The drag-controller treats
				// `button === 2` as a no-op (state unchanged), so passing
				// it through after the menu-open also works — but we return
				// early to keep the call site readable.
				if (e.button === 2) {
					e.preventDefault();
					openEmptyCanvasMenu(e.clientX, e.clientY);
					return;
				}
				// Tile / BC dragging is claimed by their own Pixi hit areas
				// (the frame's header bar and the BC bubble surface, both
				// `stopPropagation()`-ing); a pointerdown that reaches the
				// canvas is empty-space = pan claim. The frame BODY is
				// intentionally pass-through (canvas-007) so empty regions
				// inside a frame land here too.
				dragState = dragOnPointerDown(
					dragState,
					{ kind: 'empty' },
					e.clientX,
					e.clientY,
					e.button
				);
				cameraTarget = null; // a manual gesture cancels any eased transition
				// `menu` was already cleared by the capture-phase listener
				// above; no need to repeat it here.
			});
			// Suppress the browser's native context menu so our overlay can
			// own the right-click affordance.
			app.canvas.addEventListener('contextmenu', (e) => e.preventDefault());

			// One shared window-level pointermove + pointerup + pointercancel
			// + pointerleave for all three drag kinds — canvas-002's shared
			// drag controller, extended in canvas-007 to handle BC drag
			// inside a frame, and in canvas-012 with the terminal cancel/
			// leave listeners that cover touch-interruption and pointer-
			// leaves-window (cases where `pointerup` never arrives).
			window.addEventListener('pointermove', (e) => {
				const { next, delta } = dragOnPointerMove(
					dragState,
					e.clientX,
					e.clientY,
					camera.zoom
				);
				dragState = next;
				if (!delta) return;
				if (delta.kind === 'pan') {
					// canvas-015 pan path — AC #3: pointermove during pan
					// triggers ONLY a `world.position` update plus the GPU
					// draw. No `renderScene()` call here means no Pixi
					// object allocation, no `.clear()`+redraw, no scene
					// reconciliation: just the camera transform on the
					// world container. Stroke widths and text don't change
					// during pan, so no repaint is needed.
					camera.panBy(delta.dx, delta.dy);
					world.position.set(camera.pan_x, camera.pan_y);
					return;
				}
				if (delta.kind === 'frame') {
					// Frame drag: shift one project's world-space origin.
					// `entry.pos` mutation is what `updateFrameDisplayObjects`
					// reads to position the persistent container — call it
					// for just this one frame (NOT a full renderScene)
					// because nothing else changed.
					const entry = findProject(delta.projectId);
					if (!entry) return;
					entry.pos = { x: entry.pos.x + delta.dx, y: entry.pos.y + delta.dy };
					const obj = frameObjects.get(entry.id);
					if (obj) obj.container.position.set(entry.pos.x, entry.pos.y);
					return;
				}
				// delta.kind === 'bc'
				const entry = findProject(delta.projectId);
				if (!entry) return;
				const current = entry.bcPositions.get(delta.bcName);
				const layoutPos =
					current ?? entry.bcLayout.positions.get(delta.bcName);
				if (!layoutPos) return;
				const nextPos: Point = {
					x: layoutPos.x + delta.dx,
					y: layoutPos.y + delta.dy
				};
				// Pin the BC at its new position; mirror it into the
				// layout's positions Map so the render reads the updated
				// spot without a full re-layout on every mouse move (we do
				// recompute on drag END to allow other BCs to re-flow
				// around the new pin).
				entry.bcPositions.set(delta.bcName, nextPos);
				entry.bcLayout.positions.set(delta.bcName, nextPos);
				// canvas-015: update only the single BC bubble's container
				// position and the project's edges Graphics (so the moving
				// BC's edges follow). No full renderScene needed.
				const frame = frameObjects.get(entry.id);
				if (frame) {
					const bcObj = frame.bcs.get(delta.bcName);
					if (bcObj) bcObj.container.position.set(nextPos.x, nextPos.y);
					drawIntraProjectEdgesIntoFrame(entry, frame.edges, camera.zoom);
				}
			});
			window.addEventListener('pointerup', () => {
				const { next, persist } = dragOnPointerUp(dragState);
				dragState = next;
				if (!persist) return;
				if (persist.kind === 'tile') {
					const entry = findProject(persist.projectId);
					if (entry) {
						void saveTilePosition(entry.id, entry.pos);
					}
					return;
				}
				if (persist.kind === 'bc') {
					// Persist the dragged BC's new frame-local position
					// and re-run the one-shot layout so the rest of the
					// graph re-flows around the new pin.
					const entry = findProject(persist.projectId);
					if (entry) {
						const pos = entry.bcPositions.get(persist.bcName);
						if (pos) {
							void saveBcPosition(entry.id, persist.bcName, pos);
						}
						recomputeBcLayout(entry);
						renderScene();
					}
					return;
				}
				// persist.kind === 'camera'
				void saveCamera(camera.snapshot());
			});
			// canvas-012: terminal listeners that cover the pointerup-never-
			// arrives failure modes (OS-level pointer hijack / touch
			// interruption / pointer leaves the window mid-drag). Both
			// unconditionally land in `idle`; cancelled drags do NOT
			// persist (no IPC save).
			window.addEventListener('pointercancel', () => {
				dragState = dragOnPointerCancel(dragState).next;
			});
			window.addEventListener('pointerleave', () => {
				dragState = dragOnPointerLeave(dragState).next;
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
					// canvas-015 zoom path — AC #4: world.position + world.scale
					// are the only camera-transform writes. Stroke widths in
					// world space depend on z (pre-divided by z so the
					// on-screen width stays constant CSS-px after world.scale
					// multiplication), so `repaint()` rewrites them in place
					// — no Pixi object allocation. Pan path is untouched.
					world.position.set(camera.pan_x, camera.pan_y);
					world.scale.set(camera.zoom);
					repaint();
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
					case 'bc_relationships_changed': {
						// `canvas-007`: a BC README's `relationships:`
						// frontmatter changed. The event payload does NOT
						// carry the new relationships — the watcher emits
						// it as a scoped "go refresh" signal — so a per-
						// project `refreshOne` re-pulls the parsed set and
						// re-runs the BC layout once (force-directed
						// re-runs for that project; pinned-by-saved-
						// position BCs stay put). No full re-fetch.
						const entry = findProject(event.project_id);
						if (!entry) return;
						void refreshOne(event.project_id);
						return;
					}
					case 'preference_changed': {
						// `design-system-004-light-theme`. The IPC fires this
						// on every `set_preference`, including the canvas's
						// own `setTheme` write. Two reasons to still handle
						// it: (1) future sibling surfaces (voice "Bob, go
						// dark"; command palette) can flip the theme without
						// touching this component; (2) it makes the system
						// fan-out-symmetric (every consumer learns the new
						// value the same way).
						//
						// `setTheme` is idempotent — it early-returns when
						// `themeState.value === next` — so the local-set
						// path costs nothing here. Unknown keys are ignored.
						if (event.key === 'theme') {
							if (event.value === 'light' || event.value === 'dark') {
								void setTheme(event.value);
							}
						}
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
						// the explicit `renderScene()` call at the end of
						// this branch (canvas-014 retired the unconditional
						// ticker-tick rebuild that previously covered for
						// the dispatcher's missing render).
						//
						// BC topology changes (appear / disappear) require
						// a one-shot layout recompute so the frame auto-
						// fits and the new node finds a slot. Task-count
						// events leave the BC set untouched and need no
						// re-layout.
						if (
							event.kind === 'bc_appeared' ||
							event.kind === 'bc_disappeared' ||
							event.kind === 'task_added' ||
							event.kind === 'task_moved' ||
							event.kind === 'task_removed'
						) {
							// `task_*` events may lazily create a BC node
							// (`snapshot-patch.ts`'s `bcNode`), so they too
							// can change topology. Re-running the layout
							// is cheap on these tiny graphs and keeps the
							// frame coherent.
							recomputeBcLayout(entry);
						}
						// Repaint so the snapshot mutation (count tick, new
						// BC bubble, etc.) becomes visible. Before
						// canvas-014's ticker fix this was implicit on the
						// next animation frame; with the ticker dormant
						// we must drive the render explicitly.
						renderScene();
						return;
					}
				}
			});

			renderScene();

			// canvas-014: the previous implementation re-ran the FULL scene
			// graph rebuild (`renderScene` → `world.removeChildren()` +
			// rebuild every Graphics/Text) on every ticker tick (~60 Hz),
			// even when the camera was idle and no input was happening.
			// Combined with the fact that every interactive path (pan,
			// wheel, drag, hover) already calls `renderScene()` explicitly,
			// the ticker rebuild was pure waste and the dominant per-frame
			// cost. Switch the ticker to a conditional step that only fires
			// during an active eased camera transition; the resize listener
			// below covers the window-resize case the old comment named.
			app.ticker.add(() => {
				if (cameraTarget) {
					stepCameraTransition();
					// canvas-015: the eased camera transition is a
					// zoom+pan animation. Apply it via world.position +
					// world.scale (camera-as-stage-transform) and a
					// `repaint()` pass for the zoom-dependent stroke
					// widths. No per-tick reconcile / scene rebuild.
					world.position.set(camera.pan_x, camera.pan_y);
					world.scale.set(camera.zoom);
					repaint();
				}
			});

			// Re-render on window resize so the screen-space scene (and the
			// bottom-right voice indicator that reads `app.renderer.width/height`)
			// reflows. Previously the ticker's unconditional `renderScene` did
			// double duty here; an explicit `resize` listener is cheaper and
			// makes the contract obvious.
			window.addEventListener('resize', () => {
				if (!disposed) renderScene();
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
		// project frame (which already auto-fits to its interior BCs). Used
		// by zoom-to-fit ('f'). canvas-007: BC bubbles are inside the
		// frame, so the frame extent already covers them; no separate BC
		// orbit pass needed.
		function sceneWorldBounds(): { x: number; y: number; w: number; h: number } {
			if (projects.length === 0) {
				return {
					x: 0,
					y: 0,
					w: shape.frameMinInnerWidth + shape.framePadding * 2,
					h:
						shape.frameMinInnerHeight +
						shape.frameHeaderHeight +
						shape.framePadding * 2
				};
			}
			let minX = Number.POSITIVE_INFINITY;
			let minY = Number.POSITIVE_INFINITY;
			let maxX = Number.NEGATIVE_INFINITY;
			let maxY = Number.NEGATIVE_INFINITY;
			for (const entry of projects) {
				minX = Math.min(minX, entry.pos.x);
				minY = Math.min(minY, entry.pos.y);
				maxX = Math.max(maxX, entry.pos.x + entry.bcLayout.width);
				maxY = Math.max(maxY, entry.pos.y + entry.bcLayout.height);
			}
			return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
		}

		/** Build a `ProjectEntry` for `snapshot`: restore its saved frame
		 * position if any, otherwise pick the next spiral slot and persist
		 * it immediately so it is stable across restarts even if never
		 * dragged. Also batch-loads every persisted per-BC position for
		 * this project (`loadBcPositions` — one IPC round-trip per project
		 * paint) and computes the deterministic force-directed initial
		 * layout that will draw the BCs inside the frame. `spiralIndex` is
		 * the registration-order index used when no saved frame position
		 * exists. */
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
					// frame still lands in the same spot after a restart.
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

			// Batch-load every persisted BC position for this project —
			// the project-frame paint's single round-trip on mount
			// (`project-registry-004`, `canvas-007`). BCs without a saved
			// position fall through to the force-directed layout.
			let bcPositions: BcPositionMap = new Map();
			try {
				bcPositions = await loadBcPositions(snapshot.id);
			} catch (e) {
				logToCore(
					'warn',
					`could not load BC positions for project ${snapshot.id}: ${e}`
				);
			}
			const bcLayout = computeBcLayout(snapshot.bcs, bcPositions);
			return { id: snapshot.id, snapshot, pos, bcLayout, bcPositions };
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
		 * ADR-009 — and `bc_relationships_changed` — `canvas-007`).
		 * Preserves the entry's existing world-space position AND the
		 * persisted per-BC drag positions so the frame and its bubbles do
		 * not jump on a refresh. The BC layout is recomputed against the
		 * fresh `bcs` / `relationships` set (force-directed re-runs once;
		 * pinned-by-saved-position BCs stay where the user dragged them). */
		async function refreshOne(id: number) {
			try {
				const fresh = await getProject(id);
				const idx = projects.findIndex((p) => p.id === id);
				if (idx === -1) return;
				const existing = projects[idx];
				const bcLayout = computeBcLayout(fresh.bcs, existing.bcPositions);
				projects[idx] = {
					id,
					snapshot: fresh,
					pos: existing.pos,
					bcLayout,
					bcPositions: existing.bcPositions
				};
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

		// --- canvas-015: persistent BC bubble + ambient overlays ---------
		// Per the canvas-015 persistent-scene-graph rewrite, each BC bubble
		// is a persistent Container instantiated ONCE on first render and
		// updated in place. Its children — body Graphics (rect+focus ring),
		// counts pill, title Text, badge — all persist; only contents and
		// stroke widths refresh per-zoom/per-theme.

		/** Build one inside-the-frame BC bubble (§3.7). World-space size
		 *  driven by `bcInsideWidth` / `bcInsideHeight`. The BC's
		 *  `container.position` is set by `updateBcDisplayObjects` to its
		 *  frame-local coords on every update (frame-local coord system is
		 *  inherited from the parent frame container). */
		function createBcDisplayObjects(bc: BoundedContext): BcDisplayObjects {
			const container = new Container();
			const body = new Graphics();
			const focusRing = new Graphics();
			focusRing.visible = false;
			const pillBg = new Graphics();
			const pillText = new Text({
				text: '',
				style: {
					fill: color.bcInsideTextMuted,
					fontFamily: typography.fontFamilyMono,
					fontSize: typography.sizeCaption
				}
			});
			pillText.anchor.set(0.5);
			const title = new Text({
				text: bc.name,
				style: {
					fill: color.bcInsideText,
					fontFamily: typography.fontFamily,
					fontSize: typography.sizeBody,
					fontWeight: String(typography.weightMedium) as '500'
				}
			});
			const badge = createStatusBadge();
			container.addChild(body);
			container.addChild(focusRing);
			container.addChild(pillBg);
			container.addChild(pillText);
			container.addChild(title);
			container.addChild(badge.container);
			return { container, body, focusRing, pillBg, pillText, title, badge };
		}

		/** Update one BC bubble's geometry, text, and pill in place. Called
		 *  on initial render, on every zoom change (stroke widths), on
		 *  count ticks (pillText), and on theme flip (palette). No allocation. */
		function updateBcDisplayObjects(
			projectId: number,
			bc: BoundedContext,
			local: Point,
			obj: BcDisplayObjects,
			z: number
		) {
			void projectId; // key parts already wired up at creation time
			const w = shape.bcInsideWidth;
			const h = shape.bcInsideHeight;
			obj.container.position.set(local.x, local.y);

			const strokeBody = Math.max(1 / z, shape.borderWidth / z);
			const strokeFocus = Math.max(1 / z, shape.borderWidthFocus / z);

			// --- Body ---
			obj.body.clear();
			obj.body
				.roundRect(0, 0, w, h, shape.radiusBcInside)
				.fill(color.bcInsideFill)
				.stroke({ width: strokeBody, color: color.bcInsideBorder });

			// --- Focus ring (own Graphics, toggled via .visible — AC #9) ---
			obj.focusRing.clear();
			obj.focusRing
				.roundRect(-2, -2, w + 4, h + 4, shape.radiusBcInside + 2)
				.stroke({ width: strokeFocus, color: color.focusRing });
			obj.focusRing.visible = hoveredKey === `bc:${projectId}:${bc.name}`;

			// --- Counts pill ---
			const c = bc.task_counts;
			obj.pillText.text = `b${c.backlog} t${c.todo} d${c.doing} ✓${c.done}`;
			obj.pillText.style.fill = color.bcInsideTextMuted;
			// ADR-003 invariant #6 — text floor counter-scale. Apply BEFORE
			// reading `obj.pillText.width` so the pill background is sized to
			// the actually-rendered text (which grows when scaled up).
			obj.pillText.scale.set(
				screenSpaceTitleScale(typography.sizeCaption, z)
			);
			const pillTextPadX = 6;
			const pillH = shape.bcInsidePillHeight;
			const pillW = Math.max(
				shape.bcInsidePillMinWidth,
				obj.pillText.width + pillTextPadX * 2
			);
			const pillX = w - shape.framePadding * 0.5 - pillW;
			const pillY = shape.framePadding * 0.5;
			obj.pillBg.clear();
			obj.pillBg
				.roundRect(pillX, pillY, pillW, pillH, shape.bcInsidePillRadius)
				.fill(color.bcInsidePillFill);
			obj.pillText.position.set(pillX + pillW / 2, pillY + pillH / 2);

			// --- Title (BC name) ---
			const bcTitleX = shape.framePadding * 0.5;
			const bcTitleGap = shape.framePadding * 0.5;
			const bcTitleMaxW = Math.max(0, pillX - bcTitleX - bcTitleGap);
			obj.title.style.fill = color.bcInsideText;
			// ADR-003 invariant #6 — text floor counter-scale; apply BEFORE
			// truncate so the binary search measures the rendered width.
			obj.title.scale.set(screenSpaceTitleScale(typography.sizeBody, z));
			truncateTextToWidth(obj.title, bc.name, bcTitleMaxW);
			obj.title.position.set(bcTitleX, shape.framePadding * 0.5);

			// --- Status badge ---
			updateStatusBadge(obj.badge, w, deriveBcStatus(bc));

			// --- Hit area (frame-local; no `z` factor — world.scale handles
			// it; hit-test coordinates arrive in the container's local
			// space, which is the BC's local space here). ---
			obj.container.eventMode = 'static';
			obj.container.hitArea = {
				contains: (x: number, y: number) =>
					x >= 0 && x <= w && y >= 0 && y <= h
			};
		}

		// Status badge — coloured pill with the status glyph, pinned to a
		// bubble's top-right corner. Persistent across renders.
		function createStatusBadge(): BadgeDisplayObjects {
			const container = new Container();
			const body = new Graphics();
			const glyph = new Text({
				text: '',
				style: {
					fill: color.statusText,
					fontFamily: typography.fontFamily,
					fontSize: typography.sizeCaption,
					fontWeight: String(typography.weightBold) as '700'
				}
			});
			glyph.anchor.set(0.5);
			container.addChild(body);
			container.addChild(glyph);
			return { container, body, glyph };
		}

		function updateStatusBadge(
			badge: BadgeDisplayObjects,
			nodeW: number,
			state: TaskState
		) {
			const size = shape.badgeHeight;
			const bx = nodeW - size - 6;
			const by = 6;
			badge.body.clear();
			badge.body
				.roundRect(bx, by, size, size, shape.radiusBadge)
				.fill(statusColor[state]);
			badge.glyph.text = statusGlyph[state];
			badge.glyph.style.fill = color.statusText;
			badge.glyph.position.set(bx + size / 2, by + size / 2);
		}

		// --- Voice indicator (screen-space overlay on app.stage) -----------
		// Instantiated once on mount via `ensureVoiceIndicator`; updated in
		// place via `updateVoiceIndicator`. Lives on `app.stage` (NOT
		// `world`) so it ignores camera pan/zoom — screen-space affordance.
		let voiceIndicator: {
			container: Container;
			dot: Graphics;
			label: Text;
		} | null = null;

		function ensureVoiceIndicator() {
			if (voiceIndicator || !app) return;
			const container = new Container();
			const dot = new Graphics();
			const label = new Text({
				text: '',
				style: {
					fill: color.voiceIdle,
					fontFamily: typography.fontFamily,
					fontSize: typography.sizeCaption,
					fontWeight: String(typography.weightMedium) as '500'
				}
			});
			label.anchor.set(1, 0.5);
			container.addChild(dot);
			container.addChild(label);
			app.stage.addChild(container);
			voiceIndicator = { container, dot, label };
		}

		function updateVoiceIndicator() {
			if (!voiceIndicator || !app) return;
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
			voiceIndicator.dot.clear();
			voiceIndicator.dot.circle(cx, cy, r).fill(voiceColor);
			voiceIndicator.label.text =
				voiceState === 'listening'
					? 'mic'
					: voiceState === 'muted'
						? 'muted'
						: 'mic';
			voiceIndicator.label.style.fill = voiceColor;
			voiceIndicator.label.position.set(cx - r - 6, cy);
		}

		/** Wire a project frame's container as a drag handle for the header
		 *  bar — the hit area itself is a frame-local rect set by
		 *  `updateFrameDisplayObjects` so it tracks the current frame size.
		 *  Persistence on `pointerup` via the window-level handler.
		 *  canvas-015: this is instantiated ONCE per frame at create time;
		 *  drag and hover handlers are attached once and stay through every
		 *  subsequent zoom/topology update. */
		function attachFrameHeaderInteractivity(frame: Container, id: number) {
			frame.eventMode = 'static';
			frame.on('pointerover', () => {
				hoveredKey = `project:${id}`;
				// canvas-015 hover focus ring (AC #9): toggle the
				// persistent focusRing Graphics's `.visible` without
				// rebuilding the rest of the scene. Stroke width / rect
				// geometry are still right from the last
				// updateFrameDisplayObjects call.
				toggleProjectFocusRing(id, true);
			});
			frame.on('pointerout', () => {
				if (hoveredKey === `project:${id}`) {
					hoveredKey = null;
					toggleProjectFocusRing(id, false);
				}
			});
			frame.on('pointerdown', (e) => {
				e.stopPropagation();
				if (e.button === 2) {
					if (e.nativeEvent && 'stopImmediatePropagation' in e.nativeEvent) {
						e.nativeEvent.stopImmediatePropagation();
					}
					openTileMenu(e.global.x, e.global.y, id);
					return;
				}
				cameraTarget = null;
				dragState = dragOnPointerDown(
					dragState,
					{ kind: 'frameHeader', projectId: id },
					e.global.x,
					e.global.y,
					e.button
				);
			});
		}

		/** Wire one BC bubble's container into the shared drag controller +
		 *  hover state. canvas-015: instantiated once per BC at create time;
		 *  handlers persist across renders. */
		function attachBcInteractivity(
			bubble: Container,
			projectId: number,
			bcName: string
		) {
			const key = `bc:${projectId}:${bcName}`;
			bubble.on('pointerover', () => {
				hoveredKey = key;
				// canvas-015 hover focus ring (AC #9): redraw just this
				// one BC's body+ring in place. The body Graphics carries
				// both the bubble shape and the optional focus-ring
				// stroke; clear+restroke is allocation-free.
				toggleBcFocusRing(projectId, bcName, true);
			});
			bubble.on('pointerout', () => {
				if (hoveredKey === key) {
					hoveredKey = null;
					toggleBcFocusRing(projectId, bcName, false);
				}
			});
			bubble.on('pointerdown', (e) => {
				e.stopPropagation();
				if (e.button === 2) {
					if (e.nativeEvent && 'stopImmediatePropagation' in e.nativeEvent) {
						e.nativeEvent.stopImmediatePropagation();
					}
					return;
				}
				cameraTarget = null;
				dragState = dragOnPointerDown(
					dragState,
					{ kind: 'bcBubble', projectId, bcName },
					e.global.x,
					e.global.y,
					e.button
				);
			});
		}

		/** Toggle one project frame's focus ring without rebuilding the
		 *  scene (canvas-015 AC #9). The ring's geometry was already laid
		 *  down by `updateFrameDisplayObjects` at last render, so a pure
		 *  `.visible` flip is enough. */
		function toggleProjectFocusRing(id: number, visible: boolean) {
			const obj = frameObjects.get(id);
			if (!obj) return;
			obj.focusRing.visible = visible;
		}

		/** Toggle one BC bubble's focus ring without rebuilding the scene
		 *  (canvas-015 AC #9). */
		function toggleBcFocusRing(
			projectId: number,
			bcName: string,
			visible: boolean
		) {
			const frame = frameObjects.get(projectId);
			if (!frame) return;
			const bcObj = frame.bcs.get(bcName);
			if (!bcObj) return;
			bcObj.focusRing.visible = visible;
		}

		return () => {
			disposed = true;
			unlistenEvent?.();
			unlistenTheme?.();
			app?.destroy(true);
		};
	});
</script>

<div class="canvas-host" bind:this={host}></div>

<div class="status">{status}</div>

<!--
	Theme toggle (design-system-004). Pinned to the viewport top-right
	(16, 16) per the design's `references/.../GUPPI.html`. Pill chrome
	with hairline border + two inner buttons carrying sun/moon glyphs.
	State is read from the `themeState` rune; clicks call `setTheme`
	directly (which is idempotent if the value is already active). The
	preference is persisted via the v5 SQLite `preferences` table; a
	`PreferenceChanged` event is fired on the bus (ADR-009) so future
	sibling surfaces (voice command, command palette) can flip the
	same source.
-->
<div class="theme-toggle" role="tablist" aria-label="Theme">
	<button
		type="button"
		class="theme-toggle-button"
		class:active={themeState.value === 'dark'}
		role="tab"
		aria-selected={themeState.value === 'dark'}
		title="Dark theme"
		onclick={() => void setTheme('dark')}
	>
		<!-- moon glyph (design reference) -->
		<svg
			class="theme-toggle-glyph"
			width="12"
			height="12"
			viewBox="0 0 12 12"
			fill="none"
			stroke="currentColor"
			stroke-width="1.4"
			aria-hidden="true"
		>
			<path
				d="M10.5 7A4.5 4.5 0 1 1 5 1.5a3.5 3.5 0 0 0 5.5 5.5z"
				fill="currentColor"
				stroke="none"
			/>
		</svg>
		Dark
	</button>
	<button
		type="button"
		class="theme-toggle-button"
		class:active={themeState.value === 'light'}
		role="tab"
		aria-selected={themeState.value === 'light'}
		title="Light theme"
		onclick={() => void setTheme('light')}
	>
		<!-- sun glyph (design reference) -->
		<svg
			class="theme-toggle-glyph"
			width="12"
			height="12"
			viewBox="0 0 12 12"
			fill="none"
			stroke="currentColor"
			stroke-width="1.4"
			stroke-linecap="round"
			aria-hidden="true"
		>
			<circle cx="6" cy="6" r="2.4" fill="currentColor" stroke="none" />
			<path
				d="M6 1v1.5M6 9.5V11M1 6h1.5M9.5 6H11M2.5 2.5l1 1M8.5 8.5l1 1M2.5 9.5l1-1M8.5 3.5l1-1"
			/>
		</svg>
		Light
	</button>
</div>

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
	 * Theme toggle (design-system-004). Pinned to the viewport top-right
	 * (16, 16). A pill with a hairline border holding two buttons; the
	 * `active` button reads on the brand-orange tile-border tone so the
	 * affordance feels continuous with the rest of the canvas chrome.
	 * Tokens drive every value (no magic numbers).
	 */
	.theme-toggle {
		position: absolute;
		top: var(--guppi-space-lg);
		right: var(--guppi-space-lg);
		z-index: 9;
		display: inline-flex;
		align-items: center;
		gap: var(--guppi-space-xs);
		background: var(--guppi-tile-fill);
		border: 1px solid var(--guppi-hairline-strong);
		border-radius: 999px;
		padding: var(--guppi-space-xs);
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-caption);
		user-select: none;
	}
	.theme-toggle-button {
		display: inline-flex;
		align-items: center;
		gap: var(--guppi-space-xs);
		background: transparent;
		border: 0;
		padding: var(--guppi-space-xs) var(--guppi-space-md);
		font-family: inherit;
		font-size: inherit;
		font-weight: var(--guppi-weight-medium);
		color: var(--guppi-tile-text-muted);
		border-radius: 999px;
		cursor: pointer;
		line-height: 1;
		transition:
			background var(--guppi-duration-affordance) var(--guppi-ease-standard),
			color var(--guppi-duration-affordance) var(--guppi-ease-standard);
	}
	.theme-toggle-button:hover {
		color: var(--guppi-tile-text);
	}
	.theme-toggle-button.active {
		background: var(--guppi-tile-border);
		color: var(--guppi-status-text);
	}
	.theme-toggle-glyph {
		flex-shrink: 0;
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
