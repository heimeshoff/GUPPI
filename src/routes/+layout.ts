// ADR-002: single-page app, all client-side, no SSR. The static adapter needs
// prerendering on and SSR off for the SPA fallback.
export const prerender = true;
export const ssr = false;
