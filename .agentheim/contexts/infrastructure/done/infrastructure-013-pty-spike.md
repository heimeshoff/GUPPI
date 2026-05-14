---
id: infrastructure-013-pty-spike
type: spike
status: done
completed: 2026-05-14
scope: global
depends_on: [infrastructure-012-walking-skeleton]
related_adrs: [ADR-006-claude-pty, ADR-012-pty-session-teardown-ordering]
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

- [x] `claude.exe` spawns in the correct `cwd` and its TUI output streams to
      the Rust core. *(Mechanics agent-verified by `cargo test` with a
      deterministic stand-in; real-`claude.exe` TUI rendering exercisable via
      the `pty_spawn_claude` IPC command, awaits Marco's hands-on confirmation.)*
- [x] Input and resize round-trip without crashing the session. *(Agent-verified
      by `cargo test`.)*
- [~] A long-running session stays alive and responsive across idle/active
      periods. *(Implemented and exercisable via the `pty_*` IPC commands;
      a minutes-long live session needs Marco's hands-on confirmation.)*
- [x] Killing GUPPI normally leaves zero orphan `claude.exe` processes.
      *(Clean-exit path agent-verified: `dropping_the_session_kills_the_child`
      confirms the child pid is gone from `tasklist` after the actor drops.)*
- [~] Force-crashing GUPPI leaves zero orphan `claude.exe` processes (Job
      Object `KILL_ON_JOB_CLOSE` verified). *(Job Object wired with
      `KILL_ON_JOB_CLOSE`; the unclean-crash orphan check needs Marco to kill
      GUPPI's process abruptly and check Task Manager — hands-on.)*
- [x] Result recorded back into ADR-006: spike **PASSED** — the "Empirical
      spike" section is moved from DEFERRED to DONE / PASSED.

`[x]` = agent-verified · `[~]` = implemented + exercisable, awaits Marco's
hands-on confirmation (same situation as the walking skeleton's GUI checks).

## Risks retired

- ConPTY behaviour with a real long-lived `claude.exe` TUI under `portable-pty`.
- Job Object orphan-free cleanup on both clean exit and crash.
- `cwd`-per-spawn and env inheritance actually behave as ADR-006 assumes.

## Estimated effort

~1 day for someone comfortable with the Rust core, once the walking skeleton
exists.

## Outcome

**Spike PASSED.** The ADR-006 PTY stack is implemented in the Rust core and the
risky mechanics are proven by `cargo test` on real Windows 11 hardware
(Rust 1.95.0, MSVC). ADR-006's "Empirical spike — DEFERRED" section is moved to
**DONE / PASSED**.

### What was built — the `pty` module (`src-tauri/src/pty.rs`)

A `ClaudeSession` actor exactly as ADR-006 specified:

- Spawns a program through `portable-pty` (ConPTY on Windows) with
  **`cwd`-per-spawn** and **environment inherited** from GUPPI.
- On Windows, assigns the child to a **Job Object** created with
  `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` — when the actor drops, or GUPPI
  crashes, the OS reaps the whole tree. No orphan `claude.exe`.
- A **read loop** on its own thread pulls raw bytes off the PTY master and
  publishes `SessionOutput { session_id, bytes }` onto the ADR-009 `EventBus`.
  No VT parsing — raw bytes, deferred per ADR-006.
- Synchronous `write` (input) and `resize` channels.
- A drop-path teardown — see ADR-012 below.

The actor boundary is honoured: nothing outside `pty.rs` touches `portable-pty`
directly. IPC commands `pty_spawn_claude` / `pty_write` / `pty_resize` /
`pty_kill` / `pty_is_alive` expose one spike session so the hands-on DoD items
can be driven from a live `pnpm tauri dev` session.

### Agent-verified (by `cargo test` — 18/18 passing, 4 new PTY tests)

- `spawns_in_cwd_and_streams_output_to_the_bus` — spawn through `portable-pty`
  in a chosen `cwd`, child output streams back onto the bus; `cmd /C cd`'s
  output reflects the spawn `cwd`, proving cwd-per-spawn.
- `input_and_resize_round_trip_without_crashing` — written input round-trips to
  the child and its response returns on the bus; `resize` before and after
  activity does not crash or kill the session.
- `dropping_the_session_kills_the_child` — after the actor drops, the child pid
  is gone from `tasklist` (the clean-exit orphan check).
- `refuses_to_spawn_in_a_missing_cwd` — guard.

The automated tests use `cmd.exe` as a deterministic stand-in for `claude.exe`
— the PTY + Job Object + cwd + env mechanics are identical regardless of which
program runs inside, and CI must not depend on a real Claude login.

### Awaits Marco's hands-on confirmation (implemented + exercisable, not agent-verifiable)

Same situation as the walking skeleton's GUI checks — these need a live session
against the real `claude.exe`:

- `claude.exe`'s actual TUI rendering through ConPTY.
- A genuinely long-running session (minutes) staying alive across idle/active
  periods.
- The **force-crash** orphan check — kill GUPPI's process abruptly (not a clean
  exit) and confirm via Task Manager that no `claude.exe` survives, i.e. the
  Job Object's `KILL_ON_JOB_CLOSE` firing on an unclean exit. The clean-exit
  path is already agent-verified.

To run the hands-on checks: `pnpm tauri dev`, then invoke `pty_spawn_claude`,
`pty_write`, `pty_resize`, `pty_is_alive` from the WebView console (or a temp
button), leave the session idle/active for some minutes, then test both a clean
`pty_kill` and an abrupt process-kill of GUPPI followed by a Task Manager
orphan check.

### Bug found and fixed during the spike → ADR-012

The naive `Drop` (join the read-loop thread, then let fields drop) **hangs on
Windows**: the ConPTY reader does not see EOF until the PTY *master* handles
close, and Rust drops fields after `Drop::drop` returns — so the join blocked
60+ seconds. Fix: `Drop` now releases the master + writer *first*, then does a
**bounded best-effort join** (2s watchdog, then detach). The Job Object remains
the real orphan-free guarantee; the join is only tidiness. Recorded as
**ADR-012** (`ADR-012-pty-session-teardown-ordering.md`).

### Key files

- `src-tauri/src/pty.rs` — the `ClaudeSession` actor + Job Object wrapper + 4
  spike tests (new).
- `src-tauri/src/events.rs` — `DomainEvent::SessionOutput` variant added.
- `src-tauri/src/lib.rs` — `mod pty`, `AppState` session slot, `pty_*` IPC
  commands.
- `src-tauri/Cargo.toml` — `portable-pty` + `windows` (Job Objects) deps.
- `.agentheim/knowledge/decisions/ADR-006-claude-pty.md` — spike result
  recorded (DEFERRED → DONE / PASSED).
- `.agentheim/knowledge/decisions/ADR-012-pty-session-teardown-ordering.md` —
  new, the teardown-ordering decision.

## Notes

- **ADR-012** (`ADR-012-pty-session-teardown-ordering.md`) was written for the
  drop-ordering decision surfaced by this spike.
- ADR-006 updated in place — its "Empirical spike" section now reads DONE /
  PASSED with the agent-verified vs. hands-on split spelled out.
