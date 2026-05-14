// Thin abstraction over Tauri's IPC — ADR-001 keeps the runtime behind a
// seam so the rest of the frontend never imports `@tauri-apps/api` directly.
// If the runtime ever changes (ADR-001's reversibility note), only this file
// moves.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { CameraState, DomainEvent, ProjectSnapshot, Point } from './types';

/** The single Tauri event name the core's frontend bridge emits on (ADR-009). */
const FRONTEND_EVENT = 'guppi://event';

/** Fetch the hard-coded project's snapshot from the core (ADR-005). */
export function getProject(): Promise<ProjectSnapshot> {
	return invoke<ProjectSnapshot>('get_project');
}

/** Persist the project tile's position after a drag (ADR-004). */
export function saveTilePosition(pos: Point): Promise<void> {
	return invoke('save_tile_position', { x: pos.x, y: pos.y });
}

/** Read back the persisted tile position, if any. */
export async function loadTilePosition(): Promise<Point | null> {
	const result = await invoke<[number, number] | null>('load_tile_position');
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
