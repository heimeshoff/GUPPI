// Thin abstraction over Tauri's IPC — ADR-001 keeps the runtime behind a
// seam so the rest of the frontend never imports `@tauri-apps/api` directly.
// If the runtime ever changes (ADR-001's reversibility note), only this file
// moves.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CameraState, DomainEvent, ProjectSnapshot, Point } from './types';

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
