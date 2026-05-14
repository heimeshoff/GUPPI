---
id: infrastructure-016-readme-resync-required-rename
type: chore
status: done
completed: 2026-05-14
commit: 7af6f4c
scope: bc
depends_on:
  - canvas-001-targeted-canvas-updates
related_adrs:
  - ADR-009
---

# Update infrastructure README — `AgentheimChanged` → `ResyncRequired`

## Why

`canvas-001` retired the coarse `AgentheimChanged` domain event: the watcher
now publishes only the fine-grained filesystem events on the normal path, and
the lag-resync signal was renamed `AgentheimChanged` → `ResyncRequired` (in-place
ADR-009 amendment). The infrastructure BC README still documents
`AgentheimChanged` under its event-taxonomy section as a live "compatibility
seam retired by `canvas-001`" — that retirement has now happened, so the README
is stale.

This was surfaced by the `canvas-001` worker, which could not edit the
infrastructure BC's README (cross-BC scope).

## What

In `.agentheim/contexts/infrastructure/README.md`, replace the
`AgentheimChanged` bullet (currently around lines 70–75) with a `ResyncRequired`
bullet describing its post-`canvas-001` role: the lag-only resync signal,
emitted **only** by `lib.rs`'s `Lagged` arm (never by the watcher), the single
event that triggers a full `get_project` re-fetch in the frontend. Mention that
the fine-grained `TaskMoved` / `TaskAdded` / `TaskRemoved` / `BCAppeared` /
`BCDisappeared` events are the normal-path events.

## Acceptance criteria

- [ ] The infrastructure README no longer mentions `AgentheimChanged` as a live
      event.
- [ ] A `ResyncRequired` entry exists describing the lag-only resync role.
- [ ] The fine-grained taxonomy is described as the normal-path events.

## Notes

Surfaced by `canvas-001-targeted-canvas-updates` during implementation on
2026-05-14. Pure documentation — no code change. The code and ADR-009 were
already updated by `canvas-001`.

## Outcome

Replaced the stale `AgentheimChanged` bullet in the infrastructure README's
event-taxonomy section with a `ResyncRequired` bullet describing its
post-`canvas-001` role: the lag-only resync signal emitted solely by `lib.rs`'s
`Lagged` arm, the single event that triggers a full `get_project` re-fetch. The
fine-grained FS domain events bullet is now labelled the normal-path events
(`infrastructure-014` / `canvas-001`). README now matches ADR-009's current
`DomainEvent` taxonomy.

Key file: `.agentheim/contexts/infrastructure/README.md` (Ubiquitous language —
skeleton additions section).
