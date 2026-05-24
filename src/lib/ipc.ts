// Thin abstraction over Tauri's IPC — ADR-001 keeps the runtime behind a
// seam so the rest of the frontend never imports `@tauri-apps/api` directly.
// If the runtime ever changes (ADR-001's reversibility note), only this file
// moves.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
	AddScanRootResult,
	BcRollup,
	CameraState,
	DomainEvent,
	Point,
	ProjectSnapshot,
	ScanCandidate,
	ScanRootRow,
	TaskAgentState
} from './types';

/** The single Tauri event name the core's frontend bridge emits on (ADR-009). */
const FRONTEND_EVENT = 'guppi://event';

/** Fetch every registered project's snapshot from the core
 * (`project-registry-001`). A row whose `.agentheim/` is missing is skipped
 * by the core, not returned as an error. Called on mount and on
 * `resync_required` (ADR-009 lag escape hatch). */
export function listProjects(): Promise<ProjectSnapshot[]> {
	return invoke<ProjectSnapshot[]>('list_projects');
}

/** Fetch one registered project's snapshot by id (`project-registry-001`).
 * Used for the per-project resync path (`resync_required { project_id }`) so
 * the canvas does not re-fetch every tile when only one was affected. */
export function getProject(projectId: number): Promise<ProjectSnapshot> {
	return invoke<ProjectSnapshot>('get_project', { projectId });
}

/** Register a single Agentheim folder manually (ADR-005 "Add project…",
 * `project-registry-003`). The canvas's right-click → folder-picker flow
 * (`canvas-005a`) hands this the absolute path the user picked. On success the
 * core fires `ProjectAdded` and the existing live-add path renders the tile.
 *
 * On rejection the backend returns the **exact** string
 * `"not an Agentheim project"` — the canvas renders this in an error toast.
 * The string is part of the IPC contract; do not rephrase it. */
export function registerProject(path: string): Promise<number> {
	return invoke<number>('register_project', { path });
}

/** Soft-delete a registered project (ADR-005 single "Remove project"
 * affordance, `project-registry-003`). The `tile_positions` row is preserved
 * for the 30-day retention window; re-adding via `registerProject` revives the
 * tile at its old spot. The core fires `ProjectRemoved`; the canvas drops the
 * tile through its `project_removed` event handler. */
export function removeProject(projectId: number): Promise<void> {
	return invoke('remove_project', { projectId });
}

/** Register a folder as a scan root and walk it for candidate Agentheim
 * projects (ADR-013, `project-registry-002a`). The root is canonicalised +
 * persisted FIRST so an empty subtree still leaves a rescannable root behind —
 * the checklist modal (`canvas-005b`) opens in either case. */
export function addScanRoot(
	path: string,
	depthCap?: number
): Promise<AddScanRootResult> {
	return invoke<AddScanRootResult>('add_scan_root', { path, depthCap });
}

/** Re-walk an already-registered scan root (ADR-013, `project-registry-002a`).
 * Returns a fresh candidate checklist; previously-imported candidates carry
 * `already_imported: true` so the modal can grey them out. */
export function rescanScanRoot(scanRootId: number): Promise<ScanCandidate[]> {
	return invoke<ScanCandidate[]>('rescan_scan_root', { scanRootId });
}

/** List every registered scan root (ADR-013). Drives the "Manage scan roots…"
 * menu item's visibility (`canvas-005b`): hidden when this returns empty. */
export function listScanRoots(): Promise<ScanRootRow[]> {
	return invoke<ScanRootRow[]>('list_scan_roots');
}

/** Import the user's checklist picks from a scan root's walk into the
 * registry (ADR-013, `project-registry-002b`). The backend re-verifies each
 * path against a fresh walk before importing; out-of-set paths are skipped.
 * Returns the imported project ids in input order. Each imported project
 * triggers a `ProjectAdded` event; the canvas-006 live-add chain serialises
 * the burst so each tile lands in a distinct spiral slot. */
export function importScannedProjects(
	scanRootId: number,
	paths: string[]
): Promise<number[]> {
	return invoke<number[]>('import_scanned_projects', { scanRootId, paths });
}

/** Remove a scan root, cascade-deregistering every project discovered under
 * it (ADR-013, `project-registry-002b`). The cascade hard-deletes child
 * projects (NOT subject to ADR-005's 30-day retention window); the
 * confirmation dialog (`canvas-005b`) communicates that to the user. Each
 * cascaded child fires `ProjectRemoved`; the canonical handler in
 * `Canvas.svelte` (`canvas-005a`) drops the tiles. */
export function removeScanRoot(scanRootId: number): Promise<void> {
	return invoke('remove_scan_root', { scanRootId });
}

/** List every live project id stamped with the given scan root
 * (`canvas-005b`). The frontend takes `.length` for the per-row child count
 * in the scan-roots management modal. Soft-deleted children are filtered out
 * by the DB layer. */
export function listProjectsByScanRoot(scanRootId: number): Promise<number[]> {
	return invoke<number[]>('list_projects_by_scan_root', { scanRootId });
}

/** Persist a project tile's position after a drag (ADR-004). Takes
 * `projectId` explicitly — the registry no longer rides on the core's
 * `AppState` (`project-registry-001`). */
export function saveTilePosition(projectId: number, pos: Point): Promise<void> {
	return invoke('save_tile_position', { projectId, x: pos.x, y: pos.y });
}

/** Read back a project's persisted tile position, if any. */
export async function loadTilePosition(projectId: number): Promise<Point | null> {
	const result = await invoke<[number, number] | null>('load_tile_position', {
		projectId
	});
	return result ? { x: result[0], y: result[1] } : null;
}

/** One BC's persisted accordion view-state inside a project frame
 * (`canvas-023`, ADR-021): whether the kanban-accordion row is collapsed, and
 * the user's drag-reordered position. `sortOrder` is `null` when unset — the
 * canvas falls back to the stable BC-name order for that row. Mirrors the Rust
 * `BcViewStateRow`. */
export interface BcViewState {
	collapsed: boolean;
	sortOrder: number | null;
}

/** Persist a BC's accordion view-state inside its project frame
 * (`canvas-023`, ADR-021). The accordion interaction fires this on
 * collapse/expand toggle and on drag-reorder end. Preserved through ADR-005
 * soft-delete (the 30-day retention window covers `bc_view_state` the same way
 * it covers `tile_positions`). */
export function saveBcViewState(
	projectId: number,
	bcName: string,
	state: BcViewState
): Promise<void> {
	return invoke('save_bc_view_state', {
		projectId,
		bcName,
		collapsed: state.collapsed,
		sortOrder: state.sortOrder
	});
}

/** Read back one BC's persisted view-state, if any (`canvas-023`). `null` for
 * a BC the user has never collapsed or reordered. */
export async function loadBcViewState(
	projectId: number,
	bcName: string
): Promise<BcViewState | null> {
	const result = await invoke<{ collapsed: boolean; sort_order: number | null } | null>(
		'load_bc_view_state',
		{ projectId, bcName }
	);
	return result ? { collapsed: result.collapsed, sortOrder: result.sort_order } : null;
}

/** Batch-load every persisted BC view-state for a project — the project-frame
 * paint's single round-trip on mount (`canvas-023`, ADR-021). Returns a
 * `Map<bc_name, BcViewState>`. BCs without saved view-state are absent from the
 * result; the canvas falls back to its defaults (expanded; BC-name order). */
export async function loadBcViewStates(
	projectId: number
): Promise<Map<string, BcViewState>> {
	const result = await invoke<Record<string, { collapsed: boolean; sort_order: number | null }>>(
		'load_bc_view_states',
		{ projectId }
	);
	const out = new Map<string, BcViewState>();
	for (const [name, row] of Object.entries(result)) {
		out.set(name, { collapsed: row.collapsed, sortOrder: row.sort_order });
	}
	return out;
}

/** Persist the camera (pan + zoom) as a JSON blob in `app_state` (ADR-004). */
export function saveCamera(camera: CameraState): Promise<void> {
	return invoke('save_camera', { camera: JSON.stringify(camera) });
}

/** Read back the persisted camera, if any. */
export async function loadCamera(): Promise<CameraState | null> {
	const raw = await invoke<string | null>('load_camera');
	if (!raw) return null;
	try {
		return JSON.parse(raw) as CameraState;
	} catch {
		return null;
	}
}

/** Read one cross-session user preference by key from the v5 SQLite
 *  `preferences` table (`design-system-004-light-theme`). Returns `null`
 *  when the key has never been set — the frontend defaults in that case.
 *  Theme is the first inhabitant; future preferences (font scale,
 *  reduced-motion override, etc.) reuse the same call. */
export async function getPreference(key: string): Promise<string | null> {
	const result = await invoke<string | null>('get_preference', { key });
	return result;
}

/** Upsert a cross-session user preference (`design-system-004-light-theme`).
 *  The backend persists into SQLite and publishes a `PreferenceChanged`
 *  domain event on the bus so any subscriber (the PixiJS canvas, the HTML
 *  overlay layer) can react without polling. */
export function setPreference(key: string, value: string): Promise<void> {
	return invoke('set_preference', { key, value });
}

/** Read one task's live agent state (`agent-awareness-002`, ADR-018). Returns
 *  the unified read-model shape the canvas folds into a card's live-agent
 *  indicator (canvas-021 owns the indicator CONTENT; canvas-020 only wires the
 *  read). Idle for a task with no live signal. */
export function getTaskAgentState(
	projectId: number,
	bc: string,
	taskId: string
): Promise<TaskAgentState> {
	return invoke<TaskAgentState>('get_task_agent_state', { projectId, bc, taskId });
}

/** Read a BC's agent roll-up (`active / blocked / idling`) for the accordion-row
 *  header slot (`agent-awareness-002`, ADR-018). `totalTasks` is the BC's task
 *  count from the project snapshot; the core derives `idling = total − active −
 *  blocked` so tasks with no live signal count as idling without a per-task
 *  projection entry. Re-fetched on `task_agent_state_changed` for the BC. */
export function getBcAgentRollup(
	projectId: number,
	bc: string,
	totalTasks: number
): Promise<BcRollup> {
	return invoke<BcRollup>('get_bc_agent_rollup', { projectId, bc, totalTasks });
}

/** Forward a frontend log line into the core's tracing log file (ADR-010). */
export function logToCore(
	level: 'info' | 'warn' | 'error' | 'debug',
	message: string
): Promise<void> {
	return invoke('log_from_frontend', { level, message });
}

/**
 * Subscribe to core domain events. The frontend never polls — it learns about
 * state changes by being told (ADR-009). Returns an unlisten function.
 */
export function onDomainEvent(handler: (event: DomainEvent) => void): Promise<UnlistenFn> {
	return listen<DomainEvent>(FRONTEND_EVENT, (e) => handler(e.payload));
}
