// Characterisation tests for `tile-layout.ts` — the deterministic outward
// square-spiral auto-placement of project tiles (canvas-002).
//
// These tests codify the load-bearing invariants from `canvas-002`'s Outcome
// section. If any of them break silently the canvas will overlap tiles or
// place new projects far from their neighbours — both visually catastrophic.

import { describe, it, expect } from 'vitest';
import { shape } from './design/tokens';
import { spiralPosition, spiralPositions } from './tile-layout';

const CELL_W = shape.tileWidth + shape.bcOrbitRadius * 2;
const CELL_H = shape.tileHeight + shape.bcOrbitRadius * 2;

describe('spiralPosition', () => {
	it('places index 0 at the world origin', () => {
		expect(spiralPosition(0)).toEqual({ x: 0, y: 0 });
	});

	it('places index 1 one cell to the right of origin (first leg = right)', () => {
		expect(spiralPosition(1)).toEqual({ x: CELL_W, y: 0 });
	});

	it('walks the square-spiral leg pattern: right, down, left*2, up*2, right*3', () => {
		// Cell-coords of indices 0..7 following the documented spiral:
		//   0: (0,0)              -- origin
		//   1: right -> (1, 0)
		//   2: down  -> (1, 1)
		//   3: left  -> (0, 1)
		//   4: left  -> (-1, 1)
		//   5: up    -> (-1, 0)
		//   6: up    -> (-1, -1)
		//   7: right -> (0, -1)
		const expectedCells: Array<[number, number]> = [
			[0, 0],
			[1, 0],
			[1, 1],
			[0, 1],
			[-1, 1],
			[-1, 0],
			[-1, -1],
			[0, -1]
		];
		for (let i = 0; i < expectedCells.length; i++) {
			const [cx, cy] = expectedCells[i];
			expect(spiralPosition(i)).toEqual({ x: cx * CELL_W, y: cy * CELL_H });
		}
	});

	it('keeps consecutive indices adjacent (one axis moves by exactly one cell, the other stays)', () => {
		let prev = spiralPosition(0);
		for (let i = 1; i < 40; i++) {
			const curr = spiralPosition(i);
			const dx = Math.abs(curr.x - prev.x);
			const dy = Math.abs(curr.y - prev.y);
			// Exactly one axis moves by exactly one cell width/height per step.
			const movedX = dx === CELL_W && dy === 0;
			const movedY = dy === CELL_H && dx === 0;
			expect(movedX || movedY).toBe(true);
			prev = curr;
		}
	});

	it('is pure: same input always yields the same output', () => {
		for (const i of [0, 1, 5, 17, 42]) {
			expect(spiralPosition(i)).toEqual(spiralPosition(i));
		}
	});

	it('produces no duplicate positions for the first 50 indices (injective spiral)', () => {
		const seen = new Set<string>();
		for (let i = 0; i < 50; i++) {
			const p = spiralPosition(i);
			const key = `${p.x},${p.y}`;
			expect(seen.has(key)).toBe(false);
			seen.add(key);
		}
	});

	it('returns origin for negative / non-finite indices (defensive fallback)', () => {
		expect(spiralPosition(-1)).toEqual({ x: 0, y: 0 });
		expect(spiralPosition(Number.NaN)).toEqual({ x: 0, y: 0 });
		expect(spiralPosition(Number.POSITIVE_INFINITY)).toEqual({ x: 0, y: 0 });
	});
});

describe('spiralPositions', () => {
	it('returns the first N positions in spiral order', () => {
		const list = spiralPositions(5);
		expect(list.length).toBe(5);
		for (let i = 0; i < 5; i++) {
			expect(list[i]).toEqual(spiralPosition(i));
		}
	});

	it('returns an empty array for count = 0', () => {
		expect(spiralPositions(0)).toEqual([]);
	});
});
