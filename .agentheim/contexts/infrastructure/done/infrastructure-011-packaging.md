---
id: infrastructure-011-packaging
type: decision
status: done
scope: global
depends_on: [infrastructure-001-desktop-runtime]
completed: 2026-05-14
related_adrs: [ADR-011-packaging]
---

# Decision: Packaging and install

## Context

Single user, single machine, no distribution. Still want a clean install and the ability to update without re-cloning the repo.

## Architect's recommendation

**Tauri's bundler targeting MSI on Windows.** Unsigned initially (SmartScreen complains once; click through). Updates via Tauri's updater plugin pointed at a GitHub Release feed in the personal `guppi` repo. Per-user install (`%LOCALAPPDATA%\Programs\guppi\`, no admin).

## Acceptance criteria

- [x] ADR committed at `.agentheim/knowledge/decisions/ADR-011-packaging.md`
- [x] Decision on signing posture (deferred-unsigned vs sign-now)

## Outcome

Packaging decision recorded as **ADR-011** (Status: Accepted) at
`.agentheim/knowledge/decisions/ADR-011-packaging.md`. Choice: Tauri's built-in
bundler (`tauri build`) targeting **MSI** on Windows; **deferred-unsigned**
signing posture (SmartScreen warns once, click through — signing revisited only
if it becomes annoying); updates via Tauri's updater plugin against a GitHub
Release feed in the personal `guppi` repo; per-user install at
`%LOCALAPPDATA%\Programs\guppi\` with no admin required. The signing-posture
acceptance criterion resolved to **deferred-unsigned** per the architect's
recommendation, on the grounds that reversibility is trivial. No application
code in this task — implementation is left for a future build task.

## Notes — architect's ADR draft

### ADR-011: Packaging and install — Tauri bundler, unsigned MSI for Windows initially

**Status:** Proposed
**Scope:** global

**Context.** Single user, single machine, no distribution. Still want a clean install and the ability to update without re-cloning the repo.

**Decision.**
- Use Tauri's built-in bundler (`tauri build`) targeting **MSI** on Windows (NSIS is the alternative; MSI is fine).
- **Do not sign** the binary initially. SmartScreen will complain once; click through. If it becomes annoying, revisit signing.
- **Updates:** Tauri's updater plugin pointed at a GitHub Release feed in the personal `guppi` repo. Self-hosted, no third party.
- **Install location:** Default per-user (`%LOCALAPPDATA%\Programs\guppi\`) — no admin required.

**Consequences.**
- (+) Easy to install/uninstall like any normal app.
- (+) Self-hosted updates keep things local.
- (–) SmartScreen warning on first install. Acceptable.

**Reversibility.** Trivial.
