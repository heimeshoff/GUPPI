import { defineConfig } from 'vitest/config';

// Minimal Vitest configuration for the pure modules under `src/lib/`
// (`infrastructure-017-frontend-test-infrastructure`).
//
// Scope-in (per ADR-002): only the three Svelte/Pixi-free modules
// (`snapshot-patch.ts`, `tile-layout.ts`, `bc-layout.ts`) plus any future
// pure-module siblings. These import nothing from Svelte, SvelteKit, or
// PixiJS, so this config deliberately skips the SvelteKit pipeline — no
// `@sveltejs/kit/vite` plugin, no preprocessing — to keep the test surface
// fast and free of browser-runtime entanglement.
//
// Tests live next to the module they cover (`src/lib/<name>.test.ts`) and
// the production code is reachable via plain relative imports.
//
// Run with: `pnpm test`.
export default defineConfig({
	test: {
		include: ['src/lib/**/*.test.ts'],
		environment: 'node',
		// The pure modules have no module-level side effects; isolation per-file
		// is unnecessary and slows the suite. Each test imports the SUT fresh
		// via ESM's normal cache semantics.
		isolate: false,
		reporters: 'default'
	}
});
