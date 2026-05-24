// Characterisation tests for `bc-layout.ts` — the deterministic one-shot
// spring-electrical layout for BC bubbles inside a project frame
// (canvas-007-project-as-frame).
//
// These tests codify the load-bearing invariants from `canvas-007`'s
// Outcome section:
//   - deterministic: same input -> same output across runs
//   - pinned positions are sticky (frame-local coords are preserved)
//   - related BCs cluster (edge-length metric strictly lower than for an
//     equivalent unrelated graph)
//   - frame auto-fits to BC union (width/height grow with content)
//   - empty BC list returns the inner-min frame size with no positions

import { describe, it, expect } from 'vitest';
import { shape } from './design/tokens';
import { computeBcLayout, type BcPositionMap } from './bc-layout';
import type { BoundedContext } from './types';

/** Build a zero-count BC with the given name and relationships. */
function bc(name: string, relTo: string[] = []): BoundedContext {
	return {
		name,
		task_counts: { backlog: 0, todo: 0, doing: 0, done: 0 },
		tasks: [],
		relationships: relTo.map((to) => ({
			to,
			type: 'shared-kernel' as const,
			direction: null
		}))
	};
}

/** Compute the sum of squared edge lengths for a layout — lower means
 *  related BCs are sitting closer together (i.e. cluster more). */
function totalEdgeLength(
	bcs: BoundedContext[],
	positions: BcPositionMap
): number {
	let sum = 0;
	const seen = new Set<string>();
	for (const b of bcs) {
		const p = positions.get(b.name);
		if (!p) continue;
		for (const rel of b.relationships) {
			const other = positions.get(rel.to);
			if (!other) continue;
			const lo = b.name < rel.to ? b.name : rel.to;
			const hi = b.name < rel.to ? rel.to : b.name;
			const key = `${lo}-${hi}`;
			if (seen.has(key)) continue;
			seen.add(key);
			const dx = p.x - other.x;
			const dy = p.y - other.y;
			sum += Math.sqrt(dx * dx + dy * dy);
		}
	}
	return sum;
}

describe('computeBcLayout — empty input', () => {
	it('returns the minimum-size frame with no positions for an empty BC list', () => {
		const out = computeBcLayout([], new Map());
		expect(out.positions.size).toBe(0);
		expect(out.width).toBe(shape.frameMinInnerWidth + shape.framePadding * 2);
		expect(out.height).toBe(
			shape.frameMinInnerHeight +
				shape.frameHeaderHeight +
				shape.framePadding * 2
		);
	});
});

describe('computeBcLayout — determinism', () => {
	it('produces byte-identical output for byte-identical input across runs', () => {
		const bcs = [bc('alpha'), bc('beta', ['alpha']), bc('gamma', ['beta'])];
		const a = computeBcLayout(bcs, new Map());
		const b = computeBcLayout(bcs, new Map());
		expect(a.width).toBe(b.width);
		expect(a.height).toBe(b.height);
		for (const name of ['alpha', 'beta', 'gamma']) {
			expect(a.positions.get(name)).toEqual(b.positions.get(name));
		}
	});

	it('produces byte-identical output regardless of input BC array order (sorts internally by name)', () => {
		const bcs1 = [bc('alpha'), bc('beta', ['alpha']), bc('gamma', ['beta'])];
		const bcs2 = [bc('gamma', ['beta']), bc('alpha'), bc('beta', ['alpha'])];
		const a = computeBcLayout(bcs1, new Map());
		const b = computeBcLayout(bcs2, new Map());
		for (const name of ['alpha', 'beta', 'gamma']) {
			expect(a.positions.get(name)).toEqual(b.positions.get(name));
		}
	});
});

describe('computeBcLayout — pinned positions', () => {
	it('preserves a pinned BC at its exact saved frame-local position', () => {
		const bcs = [bc('alpha'), bc('beta', ['alpha']), bc('gamma')];
		const saved = new Map([['alpha', { x: 50, y: 80 }]]);
		const out = computeBcLayout(bcs, saved);
		expect(out.positions.get('alpha')).toEqual({ x: 50, y: 80 });
	});

	it('preserves multiple pins simultaneously without recentering', () => {
		const bcs = [bc('alpha'), bc('beta'), bc('gamma')];
		const saved = new Map([
			['alpha', { x: 100, y: 100 }],
			['gamma', { x: 400, y: 250 }]
		]);
		const out = computeBcLayout(bcs, saved);
		expect(out.positions.get('alpha')).toEqual({ x: 100, y: 100 });
		expect(out.positions.get('gamma')).toEqual({ x: 400, y: 250 });
	});

	it('still places unpinned BCs around the pins (unpinned positions are finite numbers)', () => {
		const bcs = [bc('alpha'), bc('beta'), bc('gamma')];
		const saved = new Map([['alpha', { x: 100, y: 100 }]]);
		const out = computeBcLayout(bcs, saved);
		const beta = out.positions.get('beta');
		const gamma = out.positions.get('gamma');
		expect(beta).toBeDefined();
		expect(gamma).toBeDefined();
		expect(Number.isFinite(beta!.x)).toBe(true);
		expect(Number.isFinite(beta!.y)).toBe(true);
		expect(Number.isFinite(gamma!.x)).toBe(true);
		expect(Number.isFinite(gamma!.y)).toBe(true);
	});
});

describe('computeBcLayout — related BCs cluster', () => {
	it('produces shorter total edge length for a connected graph than for an unrelated one', () => {
		// Same node set, two relationship graphs. The connected graph's edges
		// pull related nodes together; the empty graph has no attractive
		// force, so its nodes spread on pure repulsion. The connected version
		// must have *shorter* edges measured against itself than the
		// repulsion-only graph would, when measured against the *same*
		// relationship set.
		const connected = [
			bc('alpha', ['beta', 'gamma']),
			bc('beta', ['alpha', 'gamma']),
			bc('gamma', ['alpha', 'beta']),
			bc('delta'),
			bc('epsilon'),
			bc('zeta')
		];
		const unrelated = [
			bc('alpha'),
			bc('beta'),
			bc('gamma'),
			bc('delta'),
			bc('epsilon'),
			bc('zeta')
		];
		// Measure edge length using the connected relationship topology for
		// both layouts — the question is "how far apart did alpha/beta/gamma
		// end up?" in each case.
		const connectedOut = computeBcLayout(connected, new Map());
		const unrelatedOut = computeBcLayout(unrelated, new Map());
		const connectedEdgeLength = totalEdgeLength(
			connected,
			connectedOut.positions
		);
		const unrelatedEdgeLength = totalEdgeLength(
			connected,
			unrelatedOut.positions
		);
		expect(connectedEdgeLength).toBeLessThan(unrelatedEdgeLength);
	});
});

describe('computeBcLayout — frame auto-fit', () => {
	it('returns at least the minimum-size frame even for one small BC', () => {
		const out = computeBcLayout([bc('only')], new Map());
		expect(out.width).toBeGreaterThanOrEqual(
			shape.frameMinInnerWidth + shape.framePadding * 2
		);
		expect(out.height).toBeGreaterThanOrEqual(
			shape.frameMinInnerHeight +
				shape.frameHeaderHeight +
				shape.framePadding * 2
		);
	});

	it('grows the frame when a pinned BC is dragged far to the lower-right', () => {
		// Baseline: no pins, three BCs. Auto-fit hugs content.
		const bcs = [bc('alpha'), bc('beta'), bc('gamma')];
		const baseline = computeBcLayout(bcs, new Map());

		// Now pin gamma far away — the frame must grow to cover that corner
		// (+ framePadding), per the documented "pinned positions are
		// authoritative; the frame fits to them" contract.
		const farPin = new Map([['gamma', { x: 2000, y: 1500 }]]);
		const grown = computeBcLayout(bcs, farPin);

		expect(grown.width).toBeGreaterThan(baseline.width);
		expect(grown.height).toBeGreaterThan(baseline.height);
		// Concrete frame-fit invariant: width covers pinned BC's far edge plus padding.
		expect(grown.width).toBeGreaterThanOrEqual(
			2000 + shape.bcInsideWidth + shape.framePadding
		);
		expect(grown.height).toBeGreaterThanOrEqual(
			1500 + shape.bcInsideHeight + shape.framePadding
		);
	});

	it('keeps unpinned-only layouts inside their inner content rectangle (min-corner at framePadding offset)', () => {
		// With no pins the layout shifts everything so the content min-corner
		// sits at (framePadding, frameHeaderHeight + framePadding). All
		// positions must therefore be >= those offsets.
		const bcs = [bc('alpha'), bc('beta'), bc('gamma'), bc('delta')];
		const out = computeBcLayout(bcs, new Map());
		let minX = Number.POSITIVE_INFINITY;
		let minY = Number.POSITIVE_INFINITY;
		for (const p of out.positions.values()) {
			if (p.x < minX) minX = p.x;
			if (p.y < minY) minY = p.y;
		}
		// Allow a tiny floating-point slack — the offset uses arithmetic on
		// the simulation's positions, so exact equality is fragile.
		expect(minX).toBeGreaterThanOrEqual(shape.framePadding - 1e-6);
		expect(minY).toBeGreaterThanOrEqual(
			shape.frameHeaderHeight + shape.framePadding - 1e-6
		);
	});
});
