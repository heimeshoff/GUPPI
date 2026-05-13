---
id: infrastructure-002-frontend-framework
type: decision
status: todo
scope: global
depends_on: [infrastructure-001-desktop-runtime]
---

# Decision: Frontend framework

## Context

Once the desktop runtime is chosen (ADR-001), pick the frontend framework that mounts inside its WebView. Constraints: no SSR, all client-side, small bundles, doesn't fight a canvas-heavy app.

## Architect's recommendation

**Svelte 5 with SvelteKit (`@sveltejs/adapter-static`).**

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-002-frontend-framework.md`
- [ ] Justification matches architect's draft or Marco's amended version
- [ ] `scope: global` in frontmatter

## Architect open question Marco must answer

**Svelte vs React vs Solid.** Pure framework-preference call. Architect picked Svelte 5 because it pairs well with Tauri's lean ethos. React has the largest ecosystem (every canvas library has a wrapper). Solid is the closest-to-Svelte ergonomics with React-style ecosystem. Decide before the ADR is committed.

## Notes — architect's ADR draft

### ADR-002: Frontend framework — Svelte 5 + SvelteKit (static adapter)

**Status:** Proposed
**Scope:** global

**Context.** Need a frontend framework that pairs well with Tauri (no SSR, all client-side), produces small bundles, and doesn't fight a canvas-heavy app. Marco's preference between Svelte/React/Solid is unknown — flagging for confirmation.

**Decision.** Use **Svelte 5** with SvelteKit configured for `@sveltejs/adapter-static`. The frontend is a single-page app shipped as static assets inside the Tauri bundle.

**Consequences.**
- (+) Tiny runtime, fast updates, fewer hooks/effects ceremonies in canvas code where renders are tight.
- (+) Excellent integration with Tauri (no SSR concerns).
- (–) Smaller ecosystem than React. If a critical canvas library is React-only, we'd need to wrap it.
- (–) Svelte 5 runes are still maturing.

**Flag for user:** Pure framework-preference question. If you'd rather use React (largest ecosystem, every canvas library has a wrapper) or Solid (closest to Svelte's ergonomics with React's ecosystem), say so and this ADR changes — nothing downstream depends on the choice except the styleguide task.

**Reversibility.** Low cost early, high cost late. Decide once and live with it.
