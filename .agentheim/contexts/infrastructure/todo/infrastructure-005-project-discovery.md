---
id: infrastructure-005-project-discovery
type: decision
status: todo
scope: global
depends_on: [infrastructure-004-persistence]
---

# Decision: Project discovery model

## Context

guppi needs to know which folders are AgentHeim projects. Options: scan disk for `.agentheim/`, maintain an explicit registry, or hybrid.

## Architect's recommendation

**Explicit registry primary** + a user-triggered "Scan folder for projects…" command. No unprompted disk-walking.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-005-project-discovery.md`
- [ ] UI affordances ("Add project…", "Scan folder…", "Remove project") captured in canvas BC's task list when modeling kicks in there

## Notes — architect's ADR draft

### ADR-005: Project discovery — explicit registry first, with a manual scan command

**Status:** Proposed
**Scope:** global

**Context.** guppi needs to know which folders are AgentHeim projects. Options: (a) scan disk for `.agentheim/` directories (slow, intrusive, may surface things the user doesn't want), (b) maintain a registry of paths the user added explicitly, (c) hybrid — explicit primary, on-demand scan as a convenience.

**Decision.** **Explicit registry primary**, persisted in the `projects` table from ADR-004. UI provides:

1. **"Add project…"** — opens a native folder picker; validates that the folder contains `.agentheim/`; adds to registry.
2. **"Scan folder for projects…"** — opens a folder picker, walks one directory level (or N, user-configurable, default 3) looking for `.agentheim/` subdirectories, shows results, user checks which to import. Never runs unprompted.
3. **"Remove project"** — drops from registry; tile state retained for 30 days in case of re-add.

Projects are tracked by **canonical absolute path**, normalised on entry. If a path stops existing on startup (deleted, drive unmounted), the tile shows a "missing" state but is not auto-removed.

**Consequences.**
- (+) Predictable, fast startup — no surprises.
- (+) Respects the principle that the user is in charge.
- (–) Discovering newly-cloned projects requires a manual action. Acceptable; this is a personal tool.

**Reversibility.** Trivial; the scanner can be made automatic later if it ever feels missing.
