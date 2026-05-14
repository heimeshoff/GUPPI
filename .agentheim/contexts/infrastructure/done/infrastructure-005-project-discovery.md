---
id: infrastructure-005-project-discovery
type: decision
status: done
completed: 2026-05-14
scope: global
depends_on: [infrastructure-004-persistence]
related_adrs: [ADR-005]
commit: 48e95b3
---

# Decision: Project discovery model

## Context

GUPPI needs to know which folders are Agentheim projects. Options: scan disk for `.agentheim/`, maintain an explicit registry, or hybrid.

## Architect's recommendation

**Explicit registry primary** + a user-triggered "Scan folder for projects…" command. No unprompted disk-walking.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-005-project-discovery.md`
- [ ] UI affordances ("Add project…", "Scan folder…", "Remove project") captured in canvas BC's task list when modeling kicks in there

## Notes — architect's ADR draft

### ADR-005: Project discovery — explicit registry first, with a manual scan command

**Status:** Proposed
**Scope:** global

**Context.** GUPPI needs to know which folders are Agentheim projects. Options: (a) scan disk for `.agentheim/` directories (slow, intrusive, may surface things the user doesn't want), (b) maintain a registry of paths the user added explicitly, (c) hybrid — explicit primary, on-demand scan as a convenience.

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

## Outcome

Decided: explicit registry primary (the `projects` table from ADR-004) plus a
user-triggered "Scan folder for projects…" command — no unprompted
disk-walking. Projects tracked by canonical absolute path; missing paths show a
"missing" tile state rather than being auto-removed; removed projects' tile
state is retained 30 days.

ADR written and Accepted: `.agentheim/knowledge/decisions/ADR-005-project-discovery.md`.

The canvas BC UI affordances ("Add project…", "Scan folder…", "Remove
project") are recorded as a Downstream note in the ADR — they are canvas BC
modeling work and were intentionally not created as tasks here (BC boundary).
