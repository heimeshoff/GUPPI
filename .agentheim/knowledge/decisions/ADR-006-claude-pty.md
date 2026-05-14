---
id: ADR-006
title: Claude session ownership & PTY — portable-pty with cwd-per-spawn and Job Objects
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-006-claude-pty, infrastructure-013-pty-spike]
---

# ADR-006: Claude session ownership & PTY — `portable-pty` with cwd-per-spawn and Job Objects

**Status:** Accepted
**Scope:** global

## Context

GUPPI must spawn `claude` *inside each target project's folder*. That
requires:

- a **PTY**, so `claude`'s TUI renders correctly;
- a **controlled `cwd`** — the child must start in the project folder;
- **environment inheritance** from GUPPI's process;
- **lifecycle management** — kill on GUPPI exit, detect crashes, capture
  stdout/stderr.

This is the **riskiest piece of the architecture**. ADR-001 already committed
to a Tauri 2 Rust core, which is what makes the best-in-class Rust PTY library
available here.

Two facts were open and have now been answered by Marco:

- **`claude.exe` runs as a native Windows process.** GUPPI spawns the native
  Windows `claude.exe` directly. There is **no WSL** in the picture and no
  WSL path translation is needed. The PTY architecture below — `portable-pty`
  over ConPTY, child spawned with `cwd = project.path` — applies directly.
- **Scope is Windows-only day one.** The decision is built and validated for
  **Windows 11**. macOS and Linux are kept architecturally possible (the
  chosen library is cross-platform) but are **not validated day one** —
  consistent with ADR-001's platform stance.

## Options considered

1. **`portable-pty` (Rust)** — used by WezTerm in production. Wraps ConPTY on
   Windows, `openpty` on Unix. Mature, actively maintained.
2. **`pty-process` (Rust)** — cleaner API, but Unix-only. Disqualified for a
   Windows-first product.
3. **`node-pty`** — only relevant if the runtime were Electron. ADR-001 chose
   Tauri 2, and `node-pty` has known Windows pain (native rebuilds, module
   loading). Out of scope.
4. **Roll our own around `windows-rs` ConPTY APIs** — rejected. ConPTY's edge
   cases are not worth re-discovering by hand.

## Decision

Use **`portable-pty`** in the Rust core.

Each spawned session is an **actor** (a Tokio task) that owns:

- The `PtyPair` — master + slave handles.
- A **child process** spawned with `cwd = project.path`, `command =
  claude.exe`, with arguments and environment inherited from GUPPI's process.
- A **read loop** pulling bytes from the master and emitting
  `SessionOutput { project_id, bytes }` events. ANSI/VT parsing is **deferred**
  until the terminal-panel feature lands — the read loop ships raw bytes until
  then.
- A **write channel** for input.
- A **resize channel** for terminal-size changes.
- **Cleanup on drop** — send `CTRL_C`, wait briefly, then kill the process
  tree.

**Windows process-tree cleanup — Job Objects.** Process-tree kill on Windows
is not built in. Each child is **wrapped in a Windows Job Object** configured
with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`. If GUPPI crashes or exits, the OS
tears the job down and **no orphan `claude.exe` processes are left behind**.
This is the deliberate wiring that replaces the implicit process-group
semantics Unix would give for free.

## Cross-platform notes (kept possible, not validated day one)

ConPTY (Windows 10 1809+) is the real PTY on Windows and `portable-pty` uses
it. Known sharp edges to watch on Windows 11 — the validated target:

- **Process-tree kill** — handled via the Job Object wiring above. A
  `taskkill /F /T /PID` fallback is acceptable but the Job Object is the
  primary mechanism.
- **ConPTY UTF-8 quirks** — ConPTY emits UTF-8 with some quirks; the terminal
  panel will eventually need a real VT parser (`vte` crate). Deferred.
- **ANSI cursor sequences and bracketed paste** — mostly work; must be tested
  early.

macOS (`openpty`) and Linux are reachable through the same `portable-pty`
abstraction, but **no non-Windows path is exercised or guaranteed** until
explicitly scoped as future work.

## Empirical spike — DONE / PASSED (`infrastructure-013-pty-spike`, 2026-05-14)

ADR-006 originally called for a one-day hands-on spike proving
`portable-pty` + Job Object + `claude.exe` works end-to-end on Windows 11 with
a long-running session. That spike was **DEFERRED** when the ADR was written
because there was no Rust/Tauri scaffold in the repository. The
walking-skeleton task (`infrastructure-012-walking-skeleton`) created that
scaffold; the deferred spike then ran as `infrastructure-013-pty-spike`.

**Result: PASSED.** The ADR-006 stack is implemented in the Rust core as the
`pty` module — a `ClaudeSession` actor that owns a `portable-pty` `PtyPair`, a
child spawned with `cwd`-per-spawn and inherited environment, a raw-bytes read
loop publishing `SessionOutput` onto the ADR-009 `EventBus`, synchronous
`write` / `resize`, and an ADR-006 drop-path teardown. On Windows the child is
assigned to a Job Object created with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.

What was proven, and how:

- **Agent-verified by `cargo test` on Windows 11 (Rust 1.95.0, MSVC):**
  - `claude.exe`-equivalent spawns through `portable-pty` (ConPTY) in a
    chosen `cwd` with inherited env, and the read loop streams the child's
    output back onto the bus as `SessionOutput` — proven against `cmd /C cd`,
    whose output reflects the spawn `cwd`.
  - Input written to the PTY master round-trips to the child and its
    response returns on the bus; `resize` before and after activity does not
    crash or kill the session.
  - Dropping the `ClaudeSession` kills the child — the child pid is gone from
    `tasklist` after the actor is dropped (the programmatic half of the
    clean-exit orphan check).
  - The automated tests use a deterministic stand-in (`cmd.exe`) for
    `claude.exe`: the PTY + Job Object + cwd + env mechanics are identical
    regardless of which program runs inside, and CI must not depend on a real
    Claude login.
- **Confirmed hands-on by Marco against the real `claude.exe` (2026-05-14)** —
  driven via the `pty_spawn_claude` / `pty_write` / `pty_resize` / `pty_kill` /
  `pty_is_alive` IPC commands in a live `pnpm tauri dev` session:
  - `claude.exe`'s TUI rendering through ConPTY end-to-end — `SessionOutput`
    events carried real VT redraw sequences.
  - A long-running session stayed alive and responsive across idle/active
    periods (several minutes idle, then `pty_write` still produced output).
  - The force-crash orphan check passed: GUPPI's process was abruptly
    End-Task'd from Task Manager and no `claude.exe` survived — the Job
    Object's `KILL_ON_JOB_CLOSE` fired on an *unclean* exit. The clean-exit
    path (`pty_kill`) was confirmed too.

A teardown bug found and fixed during the spike: the drop path must release
the PTY master + writer *before* joining the read-loop thread — on Windows the
ConPTY reader does not observe EOF until the master handles close, so joining
first would hang. The fix drops master/writer first, then does a bounded,
best-effort join (2s) with the Job Object as the real cleanup guarantee. This
is recorded in ADR-012.

**ADR-006's residual risk is fully retired.** Both the automated mechanics and
the real-`claude.exe` hands-on items are confirmed — this is no longer "the
whole stack story might change". The `portable-pty` + Job Object +
cwd-per-spawn decision stands, validated end-to-end on Windows 11.

## Consequences

- (+) Solid PTY abstraction without re-implementing ConPTY; production-proven
  by WezTerm.
- (+) Each session is an independent actor — failure isolation is natural; one
  crashed `claude` session does not take down the others.
- (+) Job Object wiring gives deterministic orphan-free cleanup on Windows
  crash/exit.
- (–) ANSI/VT parsing is deferred — the architecture must leave room for a
  `vte`-based parser when the terminal panel lands.
- (–) Windows process-group cleanup needs deliberate Job Object wiring; it is
  not automatic.
- (–) The empirical Windows validation is sequenced after the walking
  skeleton — the decision carries a known, tracked residual risk until
  `infrastructure-013-pty-spike` runs.

## Reversibility

Low-cost. The actor boundary insulates the rest of the system from the PTY
library: the rest of GUPPI talks to a session actor via input/output/resize
channels, not to `portable-pty` directly. Swapping the PTY library — or
reacting to a failed spike — is contained behind that boundary.
