---
id: infrastructure-013-pty-spike
type: spike
status: backlog
scope: global
depends_on: [infrastructure-012-walking-skeleton]
related_adrs: [ADR-006-claude-pty]
---

# Spike: PTY end-to-end on Windows — `portable-pty` + Job Object + `claude.exe`

## Context

ADR-006 decided GUPPI spawns the native Windows `claude.exe` through
`portable-pty` (ConPTY), one actor per session, each child wrapped in a
Windows Job Object for orphan-free cleanup. PTY is **the riskiest piece of the
architecture**.

ADR-006 called for a one-day hands-on spike to prove the stack works
end-to-end, but that spike could not run when the ADR was written: there was
no Rust/Tauri scaffold in the repo. The walking-skeleton task
(`infrastructure-012-walking-skeleton`) creates the first code and explicitly
defers the PTY spike to run separately. This task is that deferred spike.

**This spike MUST run before any v1.x feature depends on PTY.** If it fails,
ADR-006 — and possibly the wider stack story — must be revisited.

## Goal

A hands-on proof, on real Windows 11 hardware, that `portable-pty` + a Windows
Job Object + native `claude.exe` works end-to-end with a long-running session.

## Scope (in)

1. In the Rust core (Tauri scaffold from the walking skeleton), spawn
   `claude.exe` through `portable-pty` with `cwd` set to a hardcoded project
   folder and environment inherited from GUPPI.
2. Wrap the child in a Windows Job Object configured with
   `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
3. Drive a read loop off the PTY master; confirm `claude`'s TUI output streams
   back (raw bytes are fine — no VT parsing required for the spike).
4. Send input through a write channel; confirm `claude` reacts.
5. Send a resize; confirm no crash.
6. Run the session long enough to be confident it is stable (not a
   one-shot) — minutes, with idle and active periods.
7. **Orphan check:** kill GUPPI (and separately, force-crash it) and confirm
   via Task Manager / `tasklist` that no `claude.exe` survives.

## Scope (out)

- VT/ANSI parsing, terminal panel UI — deferred per ADR-006.
- macOS / Linux paths — Windows-only day one.
- Multi-session orchestration, session registry — later feature work.

## Definition of done (acceptance criteria)

- [ ] `claude.exe` spawns in the correct `cwd` and its TUI output streams to
      the Rust core.
- [ ] Input and resize round-trip without crashing the session.
- [ ] A long-running session stays alive and responsive across idle/active
      periods.
- [ ] Killing GUPPI normally leaves zero orphan `claude.exe` processes.
- [ ] Force-crashing GUPPI leaves zero orphan `claude.exe` processes (Job
      Object `KILL_ON_JOB_CLOSE` verified).
- [ ] Result recorded back into ADR-006: spike PASSED (move "Empirical spike"
      section from DEFERRED to done), or spike FAILED with findings and a
      follow-up decision task to revisit the PTY choice.

## Risks retired

- ConPTY behaviour with a real long-lived `claude.exe` TUI under `portable-pty`.
- Job Object orphan-free cleanup on both clean exit and crash.
- `cwd`-per-spawn and env inheritance actually behave as ADR-006 assumes.

## Estimated effort

~1 day for someone comfortable with the Rust core, once the walking skeleton
exists.
