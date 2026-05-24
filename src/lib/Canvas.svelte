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
		getBcAgentRollup,
		getTaskAgentState,
		getProject,
		importScannedProjects,
		listProjects,
		listProjectsByScanRoot,
		listScanRoots,
		loadBcViewStates,
		loadCamera,
		loadTilePosition,
		onDomainEvent,
		registerProject,
		removeProject,
		removeScanRoot,
		rescanScanRoot,
		saveBcViewState,
		saveCamera,
		saveTilePosition,
		logToCore
	} from './ipc';
	import type { BcViewState } from './ipc';
	import type {
		BcRollup,
		BoundedContext,
		CameraState,
		Point,
		ProjectSnapshot,
		ScanCandidate,
		ScanRootRow,
		Task,
		TaskAgentState,
		TaskColumn
	} from './types';
	import { applyDomainEvent } from './snapshot-patch';
	import { spiralPosition } from './tile-layout';
	import {
		COLUMN_ORDER,
		bucketTasksByColumn,
		formatElapsed,
		frameSize,
		frameScreenAabb,
		shouldMountInterior,
		type FrameSize
	} from './frame-interior';
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

	// --- Task selection signal (canvas-021) ---------------------------
	// Clicking a task card SELECTS it and signals "open the detail panel for
	// this task" (the panel itself is canvas-022). This component owns the
	// selection state (so the selected card can render its §3.11 `selected`
	// border); the optional `onTaskSelected` callback prop is the outbound
	// signal a parent (canvas-022's panel host) subscribes to. v1 has no panel
	// consumer wired yet — the selection state + signal are the contract
	// canvas-022 plugs into.
	interface TaskRef {
		projectId: number;
		bc: string;
		taskId: string;
	}
	let { onTaskSelected }: { onTaskSelected?: (task: TaskRef) => void } = $props();

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
	// canvas-020 (ADR-017 pivot) reshapes the entry from canvas-007's
	// `{ id, snapshot, pos, bcLayout, bcPositions }` to
	// `{ id, snapshot, pos, size }`:
	//   - `pos`   : world-space top-left of the project's FRAME (unchanged).
	//   - `size`  : world-space frame-shell size. The retired `bc-layout.ts`
	//               grew the frame per-BC to fit a force-directed bubble
	//               cloud; the pivot makes the frame a fixed-size region whose
	//               kanban-accordion DOM interior scrolls within it
	//               (`frame-interior.frameSize`). The Pixi shell only ever
	//               draws this rect; the interior is a DOM overlay.
	// The BC bubble + intra-project edge interior (canvas-007 / ADR-015) and
	// `bc-layout.ts` are retired here (ADR-017): BCs render as DOM accordion
	// rows in the overlay, not Pixi bubbles, and BC↔BC edges are not drawn.
	interface ProjectEntry {
		id: number;
		snapshot: ProjectSnapshot;
		pos: Point;
		size: FrameSize;
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
	// canvas-020 (ADR-017): the frame SHELL is the only Pixi-rendered part of a
	// project. The BC-bubble (`BcDisplayObjects`) + intra-project-edge (`edges`,
	// `bcsRoot`, `bcs`) interior of canvas-007/canvas-015 is retired — the
	// accordion-kanban interior is a DOM overlay (see the markup + `#interiors`).
	// The empty-frame placeholder also moves to the DOM interior, so the Pixi
	// shell keeps only border + header + title + counts + focus-ring + missing
	// glyph.
	interface FrameDisplayObjects {
		container: Container;          // parent of the shell for one project; positioned at entry.pos
		body: Graphics;                // frame body (fill + border)
		header: Graphics;              // header fill + divider
		title: Text;                   // project title
		counts: Text;                  // total task count
		focusRing: Graphics;           // hover halo (toggled via .visible)
		missingGlyph: Text;            // missing-tile ✕ glyph (toggled via .visible)
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

	// --- canvas-020: kanban-accordion DOM interior state (ADR-017) ----------
	// The frame INTERIOR is a DOM overlay (ADR-017 hybrid substrate). These
	// `$state` stores back the reactive overlay markup; the Pixi side never
	// reads them. The overlay derives its mounted set + positions from the
	// camera runes + `projects` reactively (`mountedInteriors` $derived).

	// Camera runes are not `$state` on `this` component (they live on the
	// `Camera` instance), so a manual tick bumps `cameraVersion` whenever pan /
	// zoom changes to re-run the `mountedInteriors` $derived (the overlay must
	// re-position on every camera change, exactly the `worldToScreen` contract
	// ADR-016 / ADR-017 lean on). Pan/zoom/drag/resize handlers bump it.
	let cameraVersion = $state(0);
	let viewportW = $state(0);
	let viewportH = $state(0);

	// Per-BC accordion view-state, keyed `${projectId}:${bcName}` (canvas-023,
	// ADR-021). Holds the persisted collapse flag + drag-reorder `sortOrder`.
	// Batch-loaded from SQLite on frame paint (`primeBcViewState`); mutations
	// (`toggleAccordion`, `reorderBc`) optimistically update this map AND write
	// through to the DB via `saveBcViewState`. A key absent from the map means
	// "use the defaults": expanded, and the stable BC-name order. `collapsed`
	// inverts the old in-memory `accordionExpanded` (default expanded).
	let bcViewState = $state<Map<string, BcViewState>>(new Map());
	function accordionKey(projectId: number, bcName: string): string {
		return `${projectId}:${bcName}`;
	}
	function isExpanded(projectId: number, bcName: string): boolean {
		const v = bcViewState.get(accordionKey(projectId, bcName));
		// Absent ⇒ default expanded; present ⇒ inverse of the collapse flag.
		return v === undefined ? true : !v.collapsed;
	}
	/** The persisted drag-reorder index for a BC, or `null` when unset (the row
	 *  falls back to the stable BC-name order). */
	function bcSortOrder(projectId: number, bcName: string): number | null {
		return bcViewState.get(accordionKey(projectId, bcName))?.sortOrder ?? null;
	}
	/** Toggle a BC row collapsed/expanded, persisting the new collapse flag (and
	 *  preserving its current `sortOrder`). Optimistic: the map updates
	 *  immediately so the accordion animates without waiting on the round-trip;
	 *  a failed write is logged best-effort (the in-memory state still reflects
	 *  the user's intent for this session). */
	function toggleAccordion(projectId: number, bcName: string) {
		const key = accordionKey(projectId, bcName);
		const prev = bcViewState.get(key);
		const next: BcViewState = {
			collapsed: !(prev ? prev.collapsed : false),
			sortOrder: prev?.sortOrder ?? null
		};
		const map = new Map(bcViewState);
		map.set(key, next);
		bcViewState = map;
		void saveBcViewState(projectId, bcName, next).catch((e) =>
			logToCore('warn', `save_bc_view_state (collapse) failed for ${key}: ${e}`)
		);
	}

	/** Order a project's BCs for accordion display (canvas-023): BCs with a
	 *  persisted `sortOrder` sort by it; BCs without one keep the snapshot's
	 *  stable BC-name order and sort *after* any explicitly-ordered rows. The
	 *  comparator is a pure function of `bcViewState` + the snapshot, so the
	 *  `$derived` recomputes whenever either changes. Does not mutate the
	 *  snapshot array. */
	function orderBcs(projectId: number, bcs: BoundedContext[]): BoundedContext[] {
		// `nameIndex` preserves the snapshot's (BC-name) order as the stable
		// tiebreaker and the fallback for rows with no explicit `sortOrder`.
		const nameIndex = new Map<string, number>();
		bcs.forEach((bc, i) => nameIndex.set(bc.name, i));
		const rank = (bc: BoundedContext): number => {
			const so = bcSortOrder(projectId, bc.name);
			// Explicitly-ordered rows occupy the front band (their sortOrder);
			// unset rows fall to a band after every possible explicit order,
			// keeping their snapshot BC-name order via the nameIndex tiebreak.
			return so ?? Number.MAX_SAFE_INTEGER;
		};
		return [...bcs].sort((a, b) => {
			const ra = rank(a);
			const rb = rank(b);
			if (ra !== rb) return ra - rb;
			return (nameIndex.get(a.name) ?? 0) - (nameIndex.get(b.name) ?? 0);
		});
	}

	/** Batch-load a project's persisted BC view-state (collapse + order) and
	 *  merge it into `bcViewState` (canvas-023). Called on frame paint
	 *  (`refresh` / `refreshOne`). Best-effort: a failed load leaves the frame
	 *  on its defaults (all expanded, BC-name order). */
	async function primeBcViewState(projectId: number) {
		try {
			const states = await loadBcViewStates(projectId);
			const map = new Map(bcViewState);
			for (const [bcName, st] of states) {
				map.set(accordionKey(projectId, bcName), st);
			}
			bcViewState = map;
		} catch (e) {
			void logToCore('warn', `load_bc_view_states failed for ${projectId}: ${e}`);
		}
	}

	/** Drag-reorder: move `bcName` to sit at `targetIndex` within the project's
	 *  currently-displayed BC order, then renumber every BC's `sortOrder`
	 *  densely (0..n-1) and persist each (canvas-023). Renumbering the whole
	 *  frame keeps the stored order total + gap-free so a later insert is
	 *  unambiguous. Optimistic: `bcViewState` updates immediately; each write is
	 *  best-effort. No-op if the move does not change the order. */
	function reorderBc(projectId: number, bcName: string, targetIndex: number) {
		const entry = findProject(projectId);
		if (!entry) return;
		const ordered = orderBcs(projectId, entry.snapshot.bcs).map((b) => b.name);
		const from = ordered.indexOf(bcName);
		if (from === -1) return;
		const clamped = Math.max(0, Math.min(targetIndex, ordered.length - 1));
		if (from === clamped) return;
		ordered.splice(from, 1);
		ordered.splice(clamped, 0, bcName);

		const map = new Map(bcViewState);
		ordered.forEach((name, i) => {
			const key = accordionKey(projectId, name);
			const prev = map.get(key);
			const next: BcViewState = { collapsed: prev?.collapsed ?? false, sortOrder: i };
			map.set(key, next);
			void saveBcViewState(projectId, name, next).catch((e) =>
				logToCore('warn', `save_bc_view_state (reorder) failed for ${key}: ${e}`)
			);
		});
		bcViewState = map;
	}

	// --- accordion header drag-to-reorder (canvas-023) ----------------------
	// HTML5 drag-and-drop on the accordion header (the drag handle). A genuine
	// drag suppresses the click-toggle (`headerDragMoved`) so dragging a row
	// does not also collapse it. `headerDrag` carries the in-flight drag's
	// source `(projectId, bcName)`; the row sections are the drop targets,
	// keyed by their display index.
	let headerDrag = $state<{ projectId: number; bcName: string } | null>(null);
	let headerDragMoved = $state(false);
	function onHeaderDragStart(projectId: number, bcName: string, ev: DragEvent) {
		headerDrag = { projectId, bcName };
		headerDragMoved = false;
		if (ev.dataTransfer) {
			ev.dataTransfer.effectAllowed = 'move';
			// Required for Firefox to start a drag; payload is unused (we read
			// `headerDrag` directly).
			ev.dataTransfer.setData('text/plain', bcName);
		}
	}
	function onHeaderDragOver(projectId: number, ev: DragEvent) {
		// Only allow a drop within the same frame (reorder is per-project).
		if (headerDrag && headerDrag.projectId === projectId) {
			ev.preventDefault();
			headerDragMoved = true;
			if (ev.dataTransfer) ev.dataTransfer.dropEffect = 'move';
		}
	}
	function onHeaderDrop(projectId: number, targetIndex: number, ev: DragEvent) {
		ev.preventDefault();
		const drag = headerDrag;
		headerDrag = null;
		if (!drag || drag.projectId !== projectId) return;
		reorderBc(projectId, drag.bcName, targetIndex);
	}
	function onHeaderDragEnd() {
		headerDrag = null;
	}
	/** Click handler for the accordion header that suppresses the toggle when
	 *  the click was the tail of a drag-reorder gesture (canvas-023). */
	function onHeaderClick(projectId: number, bcName: string) {
		if (headerDragMoved) {
			headerDragMoved = false;
			return;
		}
		toggleAccordion(projectId, bcName);
	}

	// Per-BC live agent roll-up (`active / blocked / idling`) from
	// agent-awareness-002, keyed `${projectId}:${bcName}`. Fetched lazily by the
	// interior on mount and refreshed on `task_agent_state_changed` / task
	// events. Absent ⇒ the accordion row shows the static count only until the
	// fetch lands.
	let bcRollups = $state<Map<string, BcRollup>>(new Map());

	// --- canvas-021: per-task live agent state (agent-awareness-002) --------
	// The per-card live-agent indicator's data, keyed `${projectId}:${bc}:${taskId}`.
	// Distinct from `bcRollups` (the accordion-header aggregate) — this is the
	// per-task read model `get_task_agent_state` returns. Fetched lazily for a
	// BC's in-flight (DOING) tasks on prime, and patched in place from the
	// `task_agent_state_changed` bus event (no resync). Absent / `idle` ⇒ no
	// indicator line (the §3.11 line shows only for running / blocked).
	let taskAgentStates = $state<Map<string, TaskAgentState>>(new Map());
	function taskKey(projectId: number, bc: string, taskId: string): string {
		return `${projectId}:${bc}:${taskId}`;
	}

	// A local once-per-second clock the live-agent indicator times "waiting
	// 2m 14s" against (`formatElapsed(since, nowMs)`). agent-awareness-002
	// supplies the `since` transition timestamp ONCE; the visible elapsed value
	// advances from this local tick, NOT a per-second event (ADR-018). The
	// interval is owned for the whole canvas session — one timer, not one per
	// card — and torn down on unmount.
	let nowMs = $state(Date.now());
	let elapsedTimer: ReturnType<typeof setInterval> | null = null;

	// True when at least one task carries a running / blocked live state — i.e.
	// at least one indicator line is timing. The clock only ticks while this is
	// true, so an idle canvas does no per-second work (the styleguide's one
	// sanctioned ambient loop — §5 Q3 — earns its keep only when live).
	const hasLiveAgent = $derived.by<boolean>(() => {
		for (const s of taskAgentStates.values()) {
			if (s.activity !== 'idle') return true;
		}
		return false;
	});

	$effect(() => {
		if (hasLiveAgent && elapsedTimer === null) {
			nowMs = Date.now();
			elapsedTimer = setInterval(() => {
				nowMs = Date.now();
			}, 1000);
		} else if (!hasLiveAgent && elapsedTimer !== null) {
			clearInterval(elapsedTimer);
			elapsedTimer = null;
		}
		return () => {
			if (elapsedTimer !== null) {
				clearInterval(elapsedTimer);
				elapsedTimer = null;
			}
		};
	});

	// The currently-selected task — the card whose detail panel is open
	// (canvas-022). Drives the §3.11 `selected` blue border. Cleared when its
	// project/BC/task leaves the model.
	let selectedTask = $state<TaskRef | null>(null);

	/** Select a task card: set the selected state AND emit the outbound
	 *  "open detail panel for this task" signal (canvas-022 consumes the
	 *  callback). Idempotent — re-clicking the open card keeps it open. */
	function selectTask(projectId: number, bc: string, taskId: string) {
		selectedTask = { projectId, bc, taskId };
		onTaskSelected?.({ projectId, bc, taskId });
	}

	/** Is this the currently-selected card? */
	function isSelected(projectId: number, bc: string, taskId: string): boolean {
		const s = selectedTask;
		return (
			s !== null && s.projectId === projectId && s.bc === bc && s.taskId === taskId
		);
	}

	/** The live agent state for a task, or `null` if none / idle. The §3.11
	 *  indicator line renders only for `running` / `blocked_on_question`. */
	function liveAgentState(
		projectId: number,
		bc: string,
		taskId: string
	): TaskAgentState | null {
		const s = taskAgentStates.get(taskKey(projectId, bc, taskId));
		if (!s || s.activity === 'idle') return null;
		return s;
	}

	/** The rendered live-agent indicator line for a card, or `null` to omit it.
	 *  "orchestrator · waiting 2m 14s" — the agent label + the locally-timed
	 *  elapsed string from the supplied `since` timestamp (§3.11). */
	function agentLine(
		projectId: number,
		bc: string,
		taskId: string
	): { text: string; blocked: boolean } | null {
		const s = liveAgentState(projectId, bc, taskId);
		if (!s) return null;
		const blocked = s.activity === 'blocked_on_question';
		const label = s.agent_label ?? 'agent';
		const verb = blocked ? 'waiting' : 'working';
		const elapsed = s.since != null ? ` ${formatElapsed(s.since, nowMs)}` : '';
		return { text: `${label} · ${verb}${elapsed}`, blocked };
	}

	/** Re-fetch the per-task agent state for every in-flight (DOING) task in a
	 *  BC (`agent-awareness-002`, ADR-018). Best-effort: a failed IPC leaves the
	 *  card without an indicator. Only DOING tasks can carry a live indicator
	 *  (§3.11), so we skip the other three columns. */
	async function refreshTaskAgentStates(entry: ProjectEntry, bcName: string) {
		const bc = entry.snapshot.bcs.find((b) => b.name === bcName);
		if (!bc) return;
		for (const t of bc.tasks) {
			if (t.column !== 'doing') continue;
			try {
				const st = await getTaskAgentState(entry.id, bcName, t.id);
				const next = new Map(taskAgentStates);
				next.set(taskKey(entry.id, bcName, t.id), st);
				taskAgentStates = next;
			} catch (e) {
				void logToCore(
					'warn',
					`get_task_agent_state failed for ${entry.id}/${bcName}/${t.id}: ${e}`
				);
			}
		}
	}

	/** Find a project entry by id; null if not currently rendered. */
	function findProject(id: number): ProjectEntry | null {
		return projects.find((p) => p.id === id) ?? null;
	}

	/** Re-fetch one BC's agent roll-up (`agent-awareness-002`, ADR-018) and
	 *  store it for the accordion-row header slot. Best-effort: a failed IPC
	 *  leaves the row on its static count. The roll-up's `idling` denominator
	 *  is the BC's current task total, so we pass the live snapshot count. */
	async function refreshBcRollup(entry: ProjectEntry, bcName: string) {
		const bc = entry.snapshot.bcs.find((b) => b.name === bcName);
		if (!bc) return;
		const total =
			bc.task_counts.backlog +
			bc.task_counts.todo +
			bc.task_counts.doing +
			bc.task_counts.done;
		try {
			const rollup = await getBcAgentRollup(entry.id, bcName, total);
			const next = new Map(bcRollups);
			next.set(accordionKey(entry.id, bcName), rollup);
			bcRollups = next;
		} catch (e) {
			void logToCore('warn', `get_bc_agent_rollup failed for ${entry.id}/${bcName}: ${e}`);
		}
	}

	/** One frame's DOM interior view-model: where to anchor it on screen and at
	 *  what zoom scale. `cameraVersion` is read so this recomputes on every
	 *  camera change (pan / zoom / frame-drag / resize). */
	interface InteriorView {
		id: number;
		name: string;
		left: number;
		top: number;
		width: number;
		height: number;
		zoom: number;
		bcs: BoundedContext[];
		missing: boolean;
	}

	// The set of frame interiors to MOUNT this frame, per the ADR-017 cost
	// governors (viewport culling + zoom-floor LOD). Off-screen / zoomed-out
	// frames are NOT in this list — they render the cheap Pixi shell only.
	// Reading `cameraVersion`, `viewportW/H`, and `projects` makes this a pure
	// reactive function of the camera + model; Svelte's keyed `{#each}` then
	// reconciles the actual DOM nodes (only frames crossing the cull boundary
	// mount/unmount — ADR-017).
	const mountedInteriors = $derived.by<InteriorView[]>(() => {
		void cameraVersion; // re-run on any camera change (worldToScreen moved)
		const z = camera.zoom;
		const viewport = { w: viewportW, h: viewportH };
		const views: InteriorView[] = [];
		for (const entry of projects) {
			const aabb = frameScreenAabb(entry.pos, entry.size, camera);
			if (!shouldMountInterior(aabb, viewport, z)) continue;
			const screen = camera.worldToScreen(entry.pos.x, entry.pos.y);
			views.push({
				id: entry.id,
				name: entry.snapshot.name,
				left: screen.x,
				top: screen.y,
				width: entry.size.width,
				height: entry.size.height,
				zoom: z,
				bcs: orderBcs(entry.id, entry.snapshot.bcs),
				missing: entry.snapshot.missing
			});
		}
		return views;
	});

	/** The roll-up pills to show in an accordion-row header: one capsule per
	 *  non-zero live-agent state (active = running, blocked, idling), each with
	 *  its colourblind-safe glyph (§3.9). Absent roll-up ⇒ no pills (the static
	 *  count still shows). */
	function rollupPills(
		projectId: number,
		bcName: string
	): { state: TaskState; glyph: string; count: number; color: number }[] {
		const r = bcRollups.get(accordionKey(projectId, bcName));
		if (!r) return [];
		const pills: { state: TaskState; glyph: string; count: number; color: number }[] = [];
		if (r.active > 0)
			pills.push({ state: 'running', glyph: statusGlyph.running, count: r.active, color: statusColor.running });
		if (r.blocked > 0)
			pills.push({ state: 'blocked', glyph: statusGlyph.blocked, count: r.blocked, color: statusColor.blocked });
		if (r.idling > 0)
			pills.push({ state: 'idle', glyph: statusGlyph.idle, count: r.idling, color: statusColor.idle });
		return pills;
	}

	/** Total task count for a BC (the accordion-row right-aligned read-out). */
	function bcTotal(bc: BoundedContext): number {
		const c = bc.task_counts;
		return c.backlog + c.todo + c.doing + c.done;
	}

	/** A hex `#rrggbb` string from a tokens numeric, for inline DOM styling of
	 *  the status-coloured roll-up glyph (the only place we need a runtime hex —
	 *  the colour is data-driven per status, not a static token class). */
	function hexColor(n: number): string {
		return '#' + n.toString(16).padStart(6, '0');
	}

	/** The kanban column label (uppercase) for a `TaskColumn`. */
	function columnLabel(col: TaskColumn): string {
		return col.toUpperCase();
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
			// canvas-020: prime the reactive viewport size for the DOM interior
			// overlay's cull test (ADR-017). Kept current by the resize listener.
			syncViewport();

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
				// canvas-020: repaint runs on every zoom change (wheel, eased
				// zoom-to-fit tick, theme flip). Bump the camera version so the
				// DOM interior overlay re-positions + re-applies `scale(z)` and
				// the LOD gate re-evaluates (ADR-017). Pan does not call repaint
				// — it bumps `cameraVersion` directly in the pointermove handler.
				cameraVersion++;
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

			// --- canvas-015 / canvas-020: persistent project frame SHELL ----
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
				container.addChild(body);
				container.addChild(header);
				container.addChild(title);
				container.addChild(counts);
				container.addChild(missingGlyph);
				container.addChild(focusRing);

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
					missingGlyph
				};
			}

			function updateFrameDisplayObjects(
				entry: ProjectEntry,
				obj: FrameDisplayObjects,
				z: number
			) {
				const fw = entry.size.width;
				const fh = entry.size.height;
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
					// canvas-020: bump the camera version so the DOM interior
					// overlay re-positions to the new pan (ADR-017 worldToScreen
					// + transform contract). Cheap: the $derived restyles ≤9
					// mounted interior roots, never the hundreds of card nodes.
					cameraVersion++;
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
					// canvas-020: the frame's DOM interior tracks the shell — its
					// `entry.pos` mutation is `$state`-reactive, but bump the
					// camera version too so the position $derived recomputes now.
					cameraVersion++;
					return;
				}
				// delta.kind === 'bc' — RETIRED by canvas-020 (ADR-017). BCs are
				// no longer draggable Pixi bubbles; they are DOM accordion rows
				// inside the interior overlay. No Pixi hit-area starts a BC drag,
				// so this delta never fires. The controller's three-kind union is
				// preserved (ADR-017), so the branch stays as a defensive no-op.
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
					// RETIRED by canvas-020 (ADR-017) — BC drag no longer
					// originates (BCs are DOM accordion rows, not Pixi bubbles).
					// Defensive no-op: the controller union still carries the
					// kind, but nothing emits it.
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
						// Clear a selection / live-agent state belonging to the
						// removed project so a stale card reference does not linger
						// (canvas-021).
						if (selectedTask?.projectId === event.project_id) {
							selectedTask = null;
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
					case 'task_agent_state_changed': {
						// `agent-awareness-002` (ADR-018) — a task's live agent
						// state changed. Two reactive surfaces consume it, both
						// off the separate live-agent read model (NOT the
						// snapshot), so we patch in place rather than resync:
						//   (1) the accordion-header roll-up (`active / blocked /
						//       idling`) — re-fetched via `get_bc_agent_rollup`;
						//   (2) the per-card live-agent indicator (canvas-021) —
						//       the event payload IS the new per-task state, so we
						//       patch `taskAgentStates` straight from it (no IPC).
						// No `renderScene()` — both surfaces are reactive DOM.
						const entry = findProject(event.project_id);
						if (!entry) return;
						const next = new Map(taskAgentStates);
						next.set(taskKey(event.project_id, event.bc, event.task_id), {
							activity: event.state,
							agent_label: event.agent_label,
							since: event.since,
							question: event.question
						});
						taskAgentStates = next;
						void refreshBcRollup(entry, event.bc);
						return;
					}
					default: {
						// A fine-grained filesystem-observation event:
						// `task_moved` / `task_added` / `task_removed` /
						// `task_changed` / `bc_appeared` / `bc_disappeared`.
						// Route by id; if the project is not rendered, ignore
						// (it is a project the canvas does not have — or the
						// live-add race, in which case the matching
						// `project_added` will arrive and trigger a fresh fetch
						// that already reflects the change).
						const entry = findProject(event.project_id);
						if (!entry) return;
						applyDomainEvent(entry.snapshot, event, (msg) =>
							void logToCore('warn', msg)
						);
						// `entry.snapshot` is part of Svelte 5 `$state` (deeply
						// reactive). The kanban-accordion DOM interior derives
						// from `projects` reactively, so a card add/move/remove/
						// edit and a BC appear/disappear re-render the overlay
						// with no imperative call. `renderScene()` repaints the
						// Pixi shell (the header total-task count ticks). A
						// task event that changes a BC's task count also nudges
						// that BC's roll-up "idling" denominator, so refresh it.
						if (
							event.kind === 'task_added' ||
							event.kind === 'task_moved' ||
							event.kind === 'task_removed' ||
							event.kind === 'bc_appeared'
						) {
							void refreshBcRollup(entry, event.bc);
							// A card entering / moving within DOING may now carry a
							// live agent; re-prime the BC's per-task indicators
							// (canvas-021). The agent state is a separate read model,
							// so the snapshot patch above does not carry it.
							void refreshTaskAgentStates(entry, event.bc);
						}
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
				if (disposed) return;
				syncViewport();
				renderScene();
			});
		})();

		/** Mirror the canvas host's CSS-px size into the reactive viewport runes
		 *  + bump the camera version. The DOM interior overlay's cull test
		 *  (`shouldMountInterior`) reads these CSS-px dimensions — NOT
		 *  `app.renderer.width/height`, which are DPR-multiplied device px. */
		function syncViewport() {
			viewportW = host.clientWidth;
			viewportH = host.clientHeight;
			cameraVersion++;
		}

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
				maxX = Math.max(maxX, entry.pos.x + entry.size.width);
				maxY = Math.max(maxY, entry.pos.y + entry.size.height);
			}
			return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
		}

		/** Build a `ProjectEntry` for `snapshot`: restore its saved frame
		 * position if any, otherwise pick the next spiral slot and persist
		 * it immediately so it is stable across restarts even if never
		 * dragged. `spiralIndex` is the registration-order index used when no
		 * saved frame position exists.
		 *
		 * canvas-020 (ADR-017): the frame size is the deterministic default
		 * (`frameSize`) — the kanban-accordion DOM interior scrolls within it.
		 * The retired `bc-layout.ts` per-BC autofit + `loadBcPositions`
		 * round-trip are gone; the accordion-row roll-ups are fetched lazily
		 * by the interior overlay (`refreshBcRollup`). */
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

			return {
				id: snapshot.id,
				snapshot,
				pos,
				size: frameSize(snapshot.bcs.length)
			};
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
				// canvas-020: prime each BC's accordion-header agent roll-up
				// (agent-awareness-002). Lazy + best-effort; the interior shows
				// the static count until each fetch lands.
				for (const entry of entries) {
					// canvas-023: hydrate persisted accordion collapse + order.
					void primeBcViewState(entry.id);
					for (const bc of entry.snapshot.bcs) {
						void refreshBcRollup(entry, bc.name);
						void refreshTaskAgentStates(entry, bc.name);
					}
				}
			} catch (e) {
				status = `error: ${e}`;
				logToCore('error', `list_projects failed: ${e}`);
			}
		}

		/** Re-fetch exactly one project's snapshot (`resync_required` —
		 * ADR-009 — and `bc_relationships_changed`). Preserves the entry's
		 * existing world-space position so the frame does not jump on a
		 * refresh. canvas-020 (ADR-017): the frame size is the deterministic
		 * default; the kanban-accordion DOM interior re-derives from the fresh
		 * snapshot, and each BC's roll-up is re-fetched. */
		async function refreshOne(id: number) {
			try {
				const fresh = await getProject(id);
				const idx = projects.findIndex((p) => p.id === id);
				if (idx === -1) return;
				const existing = projects[idx];
				projects[idx] = {
					id,
					snapshot: fresh,
					pos: existing.pos,
					size: frameSize(fresh.bcs.length)
				};
				renderScene();
				// canvas-023: re-merge persisted view-state (a newly-appeared BC
				// may carry its own collapse/order if it was seen before).
				void primeBcViewState(id);
				for (const bc of fresh.bcs) {
					void refreshBcRollup(projects[idx], bc.name);
					void refreshTaskAgentStates(projects[idx], bc.name);
				}
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
				for (const bc of entry.snapshot.bcs) void refreshBcRollup(entry, bc.name);
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

		/** Toggle one project frame's focus ring without rebuilding the
		 *  scene (canvas-015 AC #9). The ring's geometry was already laid
		 *  down by `updateFrameDisplayObjects` at last render, so a pure
		 *  `.visible` flip is enough. */
		function toggleProjectFocusRing(id: number, visible: boolean) {
			const obj = frameObjects.get(id);
			if (!obj) return;
			obj.focusRing.visible = visible;
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

<!--
	Kanban-accordion frame INTERIOR (canvas-020, ADR-017). The hybrid substrate:
	the Pixi shell (border + header + drag) lives on the WebGL canvas above; this
	DOM overlay carries each on-screen frame's accordion-of-BCs → kanban board.
	Each `.frame-interior` is absolutely positioned at `worldToScreen(frame.pos)`
	and `transform: scale(z)` matches the Pixi zoom, so the interior tracks the
	shell exactly (layout computed once at zoom-1, zoom is a compositor scale —
	never a reflow). Only frames that pass the ADR-017 cull + LOD gate
	(`mountedInteriors`) are here; off-screen / zoomed-out frames render the
	cheap Pixi shell only. The keyed `{#each}` reconciles card nodes across pan.

	`pointer-events` are off on the wrapper so empty space + the header still
	reach the Pixi pan/drag hit-areas underneath; the interior body re-enables
	them so scroll + accordion clicks work.
-->
<div id="interiors" class="interiors-layer" aria-hidden={false}>
	{#each mountedInteriors as view (view.id)}
		<div
			class="frame-interior"
			style="left: {view.left}px; top: {view.top}px; width: {view.width}px;
				height: {view.height}px; transform: scale({view.zoom});
				transform-origin: top left;"
			data-project-id={view.id}
		>
			<!-- The interior sits below the Pixi header band; offset by the
			     header height so it fills the frame body region. The layout is
			     computed once at zoom-1 px; `transform: scale(z)` on this root is
			     a compositor move, not a reflow (ADR-017). -->
			<div class="frame-interior-body">
				{#if view.missing}
					<p class="interior-empty">Project directory missing on disk.</p>
				{:else if view.bcs.length === 0}
					<p class="interior-empty">
						No bounded contexts yet — add a <code>contexts/&lt;bc&gt;/</code> directory.
					</p>
				{:else}
					<div class="accordion" role="list">
						{#each view.bcs as bc, bcIndex (bc.name)}
							{@const expanded = isExpanded(view.id, bc.name)}
							{@const pills = rollupPills(view.id, bc.name)}
							{@const dragging =
								headerDrag?.projectId === view.id && headerDrag?.bcName === bc.name}
							<!-- The whole row is a drop target at its display index;
							     the header button is the drag handle (canvas-023). -->
							<section
								class="accordion-row"
								class:expanded
								class:dragging
								role="listitem"
								ondragover={(ev) => onHeaderDragOver(view.id, ev)}
								ondrop={(ev) => onHeaderDrop(view.id, bcIndex, ev)}
							>
								<button
									type="button"
									class="accordion-header"
									aria-expanded={expanded}
									draggable={true}
									onclick={() => onHeaderClick(view.id, bc.name)}
									ondragstart={(ev) => onHeaderDragStart(view.id, bc.name, ev)}
									ondragend={onHeaderDragEnd}
								>
									<span class="accordion-chevron" class:open={expanded} aria-hidden="true">▶</span>
									<span class="accordion-bc-name">{bc.name}</span>
									<span class="accordion-rollup">
										{#each pills as pill (pill.state)}
											<span class="rollup-pill" style="color: {hexColor(pill.color)};">
												<span class="rollup-glyph" aria-hidden="true">{pill.glyph}</span>
												<span class="rollup-count">{pill.count}</span>
											</span>
										{/each}
									</span>
									<span class="accordion-total">{bcTotal(bc)} task{bcTotal(bc) === 1 ? '' : 's'}</span>
								</button>
								{#if expanded}
									{@const buckets = bucketTasksByColumn(bc.tasks)}
									<div class="kanban-board">
										{#each COLUMN_ORDER as col (col)}
											<div class="kanban-column">
												<div class="kanban-column-header">{columnLabel(col)}</div>
												<div class="kanban-column-stack">
													{#if buckets[col].length === 0}
														<div class="kanban-empty">—</div>
													{:else}
														{#each buckets[col] as t (t.id)}
															{@const line = agentLine(view.id, bc.name, t.id)}
															{@const selected = isSelected(view.id, bc.name, t.id)}
															<!-- Task card (canvas-021): id + 2-line title + tag
															     chips + the live-agent indicator line; click
															     selects it and signals the detail panel
															     (canvas-022). States: default / hover (CSS) /
															     selected / blocked (§3.11). -->
															<div
																class="task-card"
																class:selected
																class:blocked={line?.blocked}
																aria-pressed={selected}
																onclick={() => selectTask(view.id, bc.name, t.id)}
																onkeydown={(e) => {
																	if (e.key === 'Enter' || e.key === ' ') {
																		e.preventDefault();
																		selectTask(view.id, bc.name, t.id);
																	}
																}}
																role="button"
																tabindex="0"
															>
																<div class="task-card-id">{t.id}</div>
																<div class="task-card-title">{t.title}</div>
																{#if t.tags.length > 0}
																	<div class="task-card-tags">
																		{#each t.tags as tag (tag)}
																			<span class="task-card-tag">{tag}</span>
																		{/each}
																	</div>
																{/if}
																{#if line}
																	<div
																		class="task-card-agent"
																		class:running={!line.blocked}
																		class:blocked={line.blocked}
																	>
																		<span class="task-card-agent-glyph" aria-hidden="true"
																			>{line.blocked ? '◆' : '▶'}</span
																		>
																		<span class="task-card-agent-text">{line.text}</span>
																	</div>
																{/if}
															</div>
														{/each}
													{/if}
												</div>
											</div>
										{/each}
									</div>
								{/if}
							</section>
						{/each}
					</div>
				{/if}
			</div>
		</div>
	{/each}
</div>

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

	/* ===== Kanban-accordion frame interior (canvas-020, ADR-017) =========
	 * The DOM half of the hybrid substrate: each on-screen frame's interior
	 * (BC accordion → kanban board → task cards) rendered as a DOM overlay
	 * positioned at `worldToScreen(frame.pos)` + `transform: scale(z)`. The
	 * Pixi frame shell (border + header + drag) sits on the WebGL canvas
	 * beneath. Every value is a `--guppi-*` token (styleguide §3.9–3.11). */
	.interiors-layer {
		position: absolute;
		inset: 0;
		overflow: hidden;
		/* Off so empty space + the Pixi header band keep receiving pan / drag
		   pointer events; re-enabled on the scrollable interior body. */
		pointer-events: none;
		z-index: 2;
	}
	.frame-interior {
		position: absolute;
		/* width / height / left / top / transform set inline per-frame. The
		   layout is computed once at this zoom-1 size; the inline
		   `transform: scale(z)` is a compositor move, never a reflow. */
		box-sizing: border-box;
	}
	.frame-interior-body {
		position: absolute;
		/* The Pixi header band owns the top strip; the interior fills the body
		   region below it. */
		top: var(--guppi-frame-header-height);
		left: 0;
		right: 0;
		bottom: 0;
		box-sizing: border-box;
		padding: var(--guppi-frame-padding);
		overflow: hidden;
		pointer-events: auto;
	}
	.interior-empty {
		margin: 0;
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		color: var(--guppi-frame-empty-text);
		text-align: center;
	}
	.interior-empty code {
		font-family: var(--guppi-font-family-mono);
	}

	/* --- BC accordion (§3.9) --- */
	.accordion {
		display: flex;
		flex-direction: column;
		gap: var(--guppi-accordion-row-gap);
		height: 100%;
		overflow-y: auto;
	}
	.accordion-row {
		flex: 0 0 auto;
		display: flex;
		flex-direction: column;
		background: var(--guppi-accordion-row-fill);
		border-radius: var(--guppi-accordion-row-radius);
		overflow: hidden;
	}
	.accordion-row.expanded {
		/* Let an expanded row take a share of the interior height so its board
		   scrolls vertically rather than pushing siblings off. */
		flex: 1 1 auto;
		min-height: 0;
	}
	.accordion-row.dragging {
		/* Lift affordance while a header is being drag-reordered (canvas-023). */
		opacity: 0.5;
	}
	.accordion-header {
		display: flex;
		align-items: center;
		gap: var(--guppi-space-sm);
		height: var(--guppi-accordion-row-header-height);
		padding: 0 var(--guppi-accordion-row-padding);
		background: var(--guppi-accordion-row-header-fill);
		border: 0;
		width: 100%;
		/* `grab` signals the header doubles as a drag-reorder handle
		   (canvas-023); it still click-toggles the accordion. */
		cursor: grab;
		font-family: var(--guppi-font-family);
		text-align: left;
		color: var(--guppi-accordion-row-text);
	}
	.accordion-header:active {
		cursor: grabbing;
	}
	.accordion-header:hover {
		outline: 1px solid var(--guppi-focus-ring);
		outline-offset: -1px;
	}
	.accordion-chevron {
		flex: 0 0 auto;
		width: var(--guppi-accordion-chevron-size);
		font-size: var(--guppi-accordion-chevron-size);
		line-height: 1;
		color: var(--guppi-accordion-chevron);
		transition: transform var(--guppi-duration-accordion) var(--guppi-ease-panel);
	}
	.accordion-chevron.open {
		transform: rotate(90deg);
	}
	.accordion-bc-name {
		flex: 0 1 auto;
		font-size: var(--guppi-size-body);
		font-weight: var(--guppi-weight-medium);
		color: var(--guppi-accordion-row-text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.accordion-rollup {
		flex: 1 1 auto;
		display: flex;
		align-items: center;
		gap: var(--guppi-space-xs);
	}
	.rollup-pill {
		display: inline-flex;
		align-items: center;
		gap: var(--guppi-space-xs);
		height: var(--guppi-accordion-rollup-pill-height);
		padding: 0 var(--guppi-space-sm);
		border-radius: var(--guppi-accordion-rollup-pill-radius);
		background: var(--guppi-accordion-rollup-pill-fill);
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-caption);
		/* the per-state status colour is applied inline (data-driven) */
	}
	.rollup-glyph {
		line-height: 1;
	}
	.accordion-total {
		flex: 0 0 auto;
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-caption);
		color: var(--guppi-accordion-row-text-muted);
	}

	/* --- Kanban board + column (§3.10) --- */
	.kanban-board {
		display: flex;
		gap: var(--guppi-kanban-column-gap);
		padding: var(--guppi-accordion-row-padding);
		border-top: 1px solid var(--guppi-accordion-row-divider);
		overflow-x: auto;
		overflow-y: hidden;
		flex: 1 1 auto;
		min-height: 0;
	}
	.kanban-column {
		display: flex;
		flex-direction: column;
		flex: 1 0 var(--guppi-kanban-column-min-width);
		min-width: var(--guppi-kanban-column-min-width);
		max-width: var(--guppi-kanban-column-max-width);
		background: var(--guppi-kanban-column-fill);
		border-radius: var(--guppi-kanban-column-radius);
		padding: var(--guppi-kanban-column-padding);
		min-height: 0;
	}
	.kanban-column-header {
		flex: 0 0 auto;
		height: var(--guppi-kanban-column-header-height);
		display: flex;
		align-items: center;
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-caption);
		font-weight: var(--guppi-weight-medium);
		letter-spacing: 0.08em;
		text-transform: uppercase;
		color: var(--guppi-kanban-column-header-text);
		border-bottom: 1px solid var(--guppi-kanban-column-divider);
	}
	.kanban-column-stack {
		display: flex;
		flex-direction: column;
		gap: var(--guppi-card-gap);
		padding-top: var(--guppi-card-gap);
		overflow-y: auto;
		flex: 1 1 auto;
		min-height: 0;
	}
	.kanban-empty {
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-caption);
		color: var(--guppi-kanban-column-empty-text);
		text-align: center;
		padding: var(--guppi-space-sm) 0;
	}

	/* --- Task card (§3.11) — STRUCTURE only; live-agent line + selection
	 *     are canvas-021 / canvas-022. --- */
	.task-card {
		flex: 0 0 auto;
		min-height: var(--guppi-card-min-height);
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		gap: var(--guppi-space-xs);
		padding: var(--guppi-card-padding);
		background: var(--guppi-card-fill);
		border: var(--guppi-card-border-width) solid var(--guppi-card-border);
		border-radius: var(--guppi-card-radius);
		cursor: pointer;
	}
	.task-card:hover {
		background: var(--guppi-card-fill-hover);
	}
	.task-card-id {
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-caption);
		color: var(--guppi-card-id-text);
	}
	.task-card-title {
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-body);
		font-weight: var(--guppi-weight-medium);
		color: var(--guppi-card-title-text);
		/* wrap to 2 lines then ellipsis (§3.11 / Q11d); full title in panel */
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.task-card-tags {
		display: flex;
		flex-wrap: wrap;
		gap: var(--guppi-space-xs);
	}
	.task-card-tag {
		display: inline-flex;
		align-items: center;
		height: var(--guppi-card-tag-height);
		padding: 0 var(--guppi-space-xs);
		border-radius: var(--guppi-card-tag-radius);
		background: var(--guppi-card-tag-fill);
		color: var(--guppi-card-tag-text);
		font-family: var(--guppi-font-family);
		font-size: var(--guppi-size-caption);
	}
	/*
	 * §3.11 card states. `blocked` and `selected` can co-occur; the selected
	 * blue border wins (selector order below), while the red ◆ glyph + agent
	 * line keep blocked legible by glyph, not colour alone. The accent border
	 * (2px) is laid in via `border-width` + colour so the 1px default never
	 * shifts the card's box on selection (border-box sizing absorbs the extra px).
	 */
	.task-card.blocked {
		border-width: var(--guppi-card-border-width-accent);
		border-color: var(--guppi-card-border-blocked);
	}
	.task-card.selected {
		border-width: var(--guppi-card-border-width-accent);
		border-color: var(--guppi-card-border-selected);
	}
	.task-card:focus-visible {
		outline: none;
		border-width: var(--guppi-card-border-width-accent);
		border-color: var(--guppi-card-border-selected);
	}
	/* §3.11 live-agent indicator line — a single mono caption with a leading
	 * status glyph; running reads brand-blue, blocked reads status-red. */
	.task-card-agent {
		display: flex;
		align-items: center;
		gap: var(--guppi-space-xs);
		font-family: var(--guppi-font-family-mono);
		font-size: var(--guppi-size-caption);
		line-height: 1.2;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.task-card-agent.running {
		color: var(--guppi-card-agent-line-running);
	}
	.task-card-agent.blocked {
		color: var(--guppi-card-agent-line-blocked);
	}
	/* The one sanctioned ambient loop (§2.6 / §5 Q3): a running agent line
	 * breathes at `durationPulse`. Blocked is static (a blocked agent is not
	 * working). Honoured only when the user has not asked for reduced motion. */
	.task-card-agent.running .task-card-agent-glyph {
		animation: card-agent-pulse var(--guppi-duration-pulse) var(--guppi-ease-pulse)
			infinite;
	}
	@keyframes card-agent-pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.45;
		}
	}
	@media (prefers-reduced-motion: reduce) {
		.task-card-agent.running .task-card-agent-glyph {
			animation: none;
		}
	}
	.task-card-agent-text {
		overflow: hidden;
		text-overflow: ellipsis;
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
