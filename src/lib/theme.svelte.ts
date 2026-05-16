// design-system-004 — central theme state.
//
// A single `theme` `$state` rune is the source of truth for the active theme
// across the frontend. The PixiJS canvas subscribes (via Svelte 5 effects)
// and re-renders on flip; the HTML overlay layer responds automatically as
// CSS custom properties flip under the `[data-theme="light"]` attribute on
// `<html>`.
//
// Persistence (per ADR-004 + Marco's 2026-05-16 sign-off) lives in SQLite —
// the new v5 `preferences (key, value)` table. The reads + writes go through
// the IPC commands `get_preference` / `set_preference`; the latter publishes
// `PreferenceChanged { key, value }` on the event bus (ADR-009) so any
// subscriber — including the canvas itself, in case the theme is flipped
// from a sibling surface in the future — can react without polling.
//
// Initialisation order on app mount (`Canvas.svelte.onMount`):
//   1. await `initTheme()` — reads `preference('theme')` from SQLite,
//      defaults to `dark` if absent, applies the active palette + HTML
//      attribute *before* the PixiJS Application boots so the first frame
//      already paints in the right palette.
//   2. The canvas runs as normal.
//   3. `setTheme('light' | 'dark')` is called from the toggle UI; it writes
//      via IPC, mutates the active palette, sets the HTML attribute, and
//      triggers a `renderScene()` via the `$effect` that watches `theme`.

import { applyPalette, type Theme } from './design/tokens';
import { getPreference, setPreference } from './ipc';

/** Reactive source of truth — the canvas reads this in an `$effect` and
 *  re-renders when it flips. The default-active palette is `dark` (matches
 *  the SQLite migration's `('theme','dark')` seed row). */
export const themeState = $state<{ value: Theme }>({ value: 'dark' });

/** Post-flip subscribers — called AFTER the active palette has been mutated
 *  and the HTML `data-theme` attribute has been set. The PixiJS canvas
 *  subscribes here to re-call `renderScene()` (which re-reads the now-active
 *  `color.*` numerics) on theme change. Plain pub/sub rather than a Svelte
 *  `$effect` so consumers can register from inside an `onMount` async
 *  closure where `$effect` would be out of scope. */
type ThemeListener = (theme: Theme) => void;
const themeListeners = new Set<ThemeListener>();

/** Register a listener fired after each theme flip; returns an unregister
 *  thunk for cleanup on component teardown. Idempotent — registering the
 *  same listener twice has no extra effect. */
export function onThemeChange(listener: ThemeListener): () => void {
	themeListeners.add(listener);
	return () => themeListeners.delete(listener);
}

function fireThemeListeners(theme: Theme) {
	for (const listener of themeListeners) {
		try {
			listener(theme);
		} catch {
			// Listener failures must not poison the flip — each is
			// independent (the canvas's `renderScene` is the only one in
			// v1, but the contract is fan-out).
		}
	}
}

/** Idempotent: applies `theme` to the active token palette + sets the
 *  HTML `data-theme` attribute (dark omits the attribute entirely so the
 *  `:root` default-theme block wins) + fires registered post-flip
 *  listeners (PixiJS canvas re-renders). Pure local work; does NOT touch
 *  IPC. Exported so the initialiser can use it without going through the
 *  `setTheme` write path. */
export function applyThemeToDom(theme: Theme): void {
	applyPalette(theme);
	const root = document.documentElement;
	if (theme === 'light') {
		root.setAttribute('data-theme', 'light');
	} else {
		root.removeAttribute('data-theme');
	}
	fireThemeListeners(theme);
}

/** App-start initialiser — read the persisted theme from SQLite and apply
 *  it BEFORE the canvas mounts so the first paint is already in the right
 *  palette. Falls back to `dark` on any IPC error or invalid stored value,
 *  matching the migration's seed default. */
export async function initTheme(): Promise<Theme> {
	let next: Theme = 'dark';
	try {
		const stored = await getPreference('theme');
		if (stored === 'light' || stored === 'dark') {
			next = stored;
		}
	} catch {
		// Best-effort load; stay on the dark default if IPC fails.
	}
	themeState.value = next;
	applyThemeToDom(next);
	return next;
}

/** Flip the active theme — writes to SQLite via IPC (which fires
 *  `PreferenceChanged` on the event bus, ADR-009), mutates the active
 *  palette, and sets the HTML `data-theme` attribute. The reactive
 *  `themeState.value` change is what the canvas's `$effect` observes
 *  to re-call `renderScene()`. */
export async function setTheme(next: Theme): Promise<void> {
	if (themeState.value === next) return;
	// Mutate locally first so the UI flips immediately, even if the IPC
	// write is briefly delayed. A subsequent failure leaves the in-memory
	// state ahead of disk — acceptable for a view-only preference; the
	// frontend re-reads on next app start and reconciles.
	themeState.value = next;
	applyThemeToDom(next);
	try {
		await setPreference('theme', next);
	} catch {
		// Best-effort write; the failed promise is silently swallowed
		// (frontend ipc.ts itself rejects on failure). A future toast
		// surface could communicate the persistence failure.
	}
}

/** Convenience — `setTheme(other-of-current)`. The toggle UI binds to this. */
export function toggleTheme(): Promise<void> {
	return setTheme(themeState.value === 'dark' ? 'light' : 'dark');
}
