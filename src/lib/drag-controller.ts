// Pure drag-controller state machine for the canvas (canvas-012).
//
// The canvas hosts three drag kinds — empty-canvas pan, project-frame drag
// (claimed by the frame's header bar), and BC-bubble drag — that previously
// spread their state across four variables in two scopes (`dragProjectId`,
// `dragBcName`, `dragOriginX/Y` at module scope; `panning` at function scope
// inside the empty-canvas pointerdown closure). That spread is what made
// `canvas-012`'s stuck-pan bug invisible to the eye: `pointerup`'s clear of
// `panning` lived inside an `if (panning) { … }` branch reached only after
// two earlier early-returns for the BC- and frame-drag branches, so a
// pointer sequence that left `panning` set without taking the pan branch
// at pointerup survived as live drag-state.
//
// This module consolidates the four variables into a single
// discriminated-union `DragState` and exposes pure transition functions for
// the five DOM events (`pointerdown`, `pointermove`, `pointerup`,
// `pointercancel`, `pointerleave`). The two terminal events
// (`pointercancel`, `pointerleave`) unconditionally land in `idle`; they
// were not wired at all in the pre-canvas-012 controller and are required
// for touch-interruption and pointer-leaves-window cases where `pointerup`
// never arrives.
//
// Right-click (`button === 2`) is a no-op on the controller: the caller
// handles menu open and never enters a drag state for it. This matches
// the pre-canvas-012 behaviour at all three claim sites.
//
// Deliberately a stand-alone module with **no** imports from
// `./Canvas.svelte`, `./camera.svelte`, `pixi.js`, or Svelte. Same
// "verification surface" stance as `tile-layout.ts` / `bc-layout.ts` /
// `snapshot-patch.ts`. Frontend test infrastructure lands in
// `infrastructure-017`; a follow-up backfill task will then wire this
// module's unit tests. The pure shape is the end-state.

/** Active drag-state. `idle` is the resting state; the other three variants
 *  each carry exactly the data their move-handler branch needs. There is
 *  no overlap between the four variants and no state can be in two of
 *  them simultaneously — the single-variable replacement of the previous
 *  four-variable spread is the structural fix for canvas-012. */
export type DragState =
	| { kind: 'idle' }
	| { kind: 'panning'; lastX: number; lastY: number }
	| { kind: 'frame'; projectId: number; originX: number; originY: number }
	| {
			kind: 'bc';
			projectId: number;
			bcName: string;
			originX: number;
			originY: number;
	  };

/** Pointerdown target descriptor — built by the call site at the moment of
 *  the down event. `empty` is the canvas background (the default pan
 *  claim); `frameHeader` is the project frame's header bar (the frame
 *  drag handle — the frame BODY is intentionally pass-through per
 *  `canvas-007`, so a down on the body falls through to `empty`);
 *  `bcBubble` is a BC bubble surface inside its frame. */
export type Target =
	| { kind: 'empty' }
	| { kind: 'frameHeader'; projectId: number }
	| { kind: 'bcBubble'; projectId: number; bcName: string };

/** Resting state — exported so callers don't reach into the union literal. */
export const IDLE: DragState = { kind: 'idle' };

/** Movement delta emitted by `onPointerMove`. The caller applies it; the
 *  controller only computes it. `pan` carries SCREEN-space delta (the
 *  consumer feeds it to `camera.panBy` directly). `frame` and `bc` carry
 *  WORLD-space delta (already divided by `zoom`) ready to add to an
 *  `entry.pos` or BC frame-local position. The `kind` field exists for
 *  the caller's branch — same shape as the state's `kind`, minus
 *  `idle`. */
export type DragDelta =
	| { kind: 'pan'; dx: number; dy: number }
	| { kind: 'frame'; projectId: number; dx: number; dy: number }
	| { kind: 'bc'; projectId: number; bcName: string; dx: number; dy: number };

/** Persistence intent emitted by `onPointerUp`. `tile` triggers
 *  `saveTilePosition` for `projectId`; `bc` triggers `saveBcPosition` for
 *  `projectId` + `bcName`; `camera` triggers `saveCamera`. Absence
 *  (`undefined`) means: nothing to persist — e.g., the gesture was
 *  cancelled or never actually entered a drag-affecting state. */
export type Persist =
	| { kind: 'tile'; projectId: number }
	| { kind: 'bc'; projectId: number; bcName: string }
	| { kind: 'camera' };

/** Pointerdown transition. Returns the next state. Right-click
 *  (`button === 2`) is a no-op: state is unchanged. The caller still
 *  handles menu-open externally — the controller only owns drag-state.
 *
 *  Note that an in-flight drag is replaced by the new claim. This
 *  matches the pre-canvas-012 behaviour (a fresh pointerdown
 *  unconditionally overwrites `dragProjectId` / `dragBcName` /
 *  `dragOriginX/Y` at the claim site); it also makes "right-click during
 *  an in-flight left-drag" a no-op — the right-click does not enter a
 *  drag, the left-drag's terminal `pointerup` still fires and clears
 *  state cleanly. */
export function onPointerDown(
	state: DragState,
	target: Target,
	x: number,
	y: number,
	button: number
): DragState {
	void state; // unused — pointerdown unconditionally re-claims (see header)
	if (button === 2) {
		// Right-button: caller opens a context menu; controller stays
		// where it is. Crucially we do NOT clear an in-flight drag here —
		// the in-flight drag's own terminal event (pointerup / cancel /
		// leave) is what clears it.
		return state;
	}
	switch (target.kind) {
		case 'empty':
			return { kind: 'panning', lastX: x, lastY: y };
		case 'frameHeader':
			return {
				kind: 'frame',
				projectId: target.projectId,
				originX: x,
				originY: y
			};
		case 'bcBubble':
			return {
				kind: 'bc',
				projectId: target.projectId,
				bcName: target.bcName,
				originX: x,
				originY: y
			};
	}
}

/** Pointermove transition. Returns the next state plus an optional delta
 *  for the caller to apply. State persists across moves (origins/last-
 *  coords advance with the cursor); the controller never decides to
 *  leave an active drag mid-move. `zoom` is read for `frame` and `bc`
 *  kinds to convert the screen-space delta into world-space; `panning`
 *  ignores it (camera pan is screen-space). */
export function onPointerMove(
	state: DragState,
	x: number,
	y: number,
	zoom: number
): { next: DragState; delta?: DragDelta } {
	switch (state.kind) {
		case 'idle':
			return { next: state };
		case 'panning': {
			const dx = x - state.lastX;
			const dy = y - state.lastY;
			return {
				next: { kind: 'panning', lastX: x, lastY: y },
				delta: { kind: 'pan', dx, dy }
			};
		}
		case 'frame': {
			const dx = (x - state.originX) / zoom;
			const dy = (y - state.originY) / zoom;
			return {
				next: {
					kind: 'frame',
					projectId: state.projectId,
					originX: x,
					originY: y
				},
				delta: { kind: 'frame', projectId: state.projectId, dx, dy }
			};
		}
		case 'bc': {
			const dx = (x - state.originX) / zoom;
			const dy = (y - state.originY) / zoom;
			return {
				next: {
					kind: 'bc',
					projectId: state.projectId,
					bcName: state.bcName,
					originX: x,
					originY: y
				},
				delta: {
					kind: 'bc',
					projectId: state.projectId,
					bcName: state.bcName,
					dx,
					dy
				}
			};
		}
	}
}

/** Pointerup transition. Always lands in `idle`. Emits an optional
 *  `persist` intent so the caller can fire the matching IPC save (frame
 *  position, BC frame-local position, or camera snapshot). `idle → idle`
 *  emits no intent — a stray pointerup with no in-flight drag is a
 *  no-op. */
export function onPointerUp(state: DragState): { next: DragState; persist?: Persist } {
	switch (state.kind) {
		case 'idle':
			return { next: IDLE };
		case 'panning':
			return { next: IDLE, persist: { kind: 'camera' } };
		case 'frame':
			return {
				next: IDLE,
				persist: { kind: 'tile', projectId: state.projectId }
			};
		case 'bc':
			return {
				next: IDLE,
				persist: { kind: 'bc', projectId: state.projectId, bcName: state.bcName }
			};
	}
}

/** Pointercancel transition. Unconditional terminal — cancelled drags do
 *  NOT persist. Wired at `window` per canvas-012; covers touch
 *  interruption and OS-level pointer hijack. */
export function onPointerCancel(_state: DragState): { next: DragState } {
	void _state;
	return { next: IDLE };
}

/** Pointerleave transition. Same unconditional-terminal contract as
 *  cancel. Wired at `window` per canvas-012; covers the cursor leaving
 *  the window mid-drag (the next move never arrives, so without this
 *  the state would stick). */
export function onPointerLeave(_state: DragState): { next: DragState } {
	void _state;
	return { next: IDLE };
}
