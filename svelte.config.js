import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// ADR-002: ship as static assets inside the Tauri bundle — no SSR,
		// no server. The static adapter produces the asset bundle Tauri serves
		// (frontendDist = "../build" in tauri.conf.json).
		adapter: adapter({
			fallback: 'index.html'
		})
	}
};

export default config;
