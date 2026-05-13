---
id: infrastructure-006-claude-pty
type: decision
status: todo
scope: global
depends_on: [infrastructure-001-desktop-runtime]
---

# Decision: Claude session ownership & PTY library

## Context

GUPPI must spawn `claude` *inside each target project's folder*. PTY (so `claude`'s TUI works), controlled `cwd`, env inheritance, lifecycle management (kill on GUPPI exit, detect crashes, capture stdout/stderr). Must work cleanly on Windows 11. **This is the riskiest piece of the architecture.**

## Architect's recommendation

**`portable-pty`** (Rust) — wraps ConPTY on Windows, openpty on Unix. Each spawned session is a Tokio actor owning the PTY + child + read/write/resize channels. **Wrap each child in a Job Object on Windows** so GUPPI crashing leaves no orphans.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-006-claude-pty.md`
- [ ] Spike result attached: a one-day proof that `portable-pty` + Job Object + `claude.exe` on Windows works end-to-end with a long-running session. **If the spike fails, the whole stack story changes.**

## Architect open questions Marco must answer

1. **`claude.exe` native Windows vs WSL.** ADR assumes native Windows. If `claude` actually runs in WSL today, the PTY story changes significantly.
2. **Multi-OS scope.** Vision says "must work on Windows cleanly." Are macOS and Linux nice-to-have, or actually required day one?

## Notes — architect's ADR draft

### ADR-006: Claude session ownership — `portable-pty` (Rust) with explicit cwd-per-spawn

**Status:** Proposed
**Scope:** global

**Context.** GUPPI must spawn `claude` *inside each target project's folder*. This means a PTY (so `claude`'s TUI works), a controlled `cwd`, environment inheritance, and process lifecycle management (kill on GUPPI exit, detect crashes, capture stdout/stderr). Must work on Windows 11. This is the riskiest piece of the architecture.

**Options considered.**
1. **`portable-pty` (Rust)** — Used by WezTerm in production. Wraps ConPTY on Windows, openpty on Unix. Mature, actively maintained.
2. **`pty-process` (Rust)** — Cleaner API, but Unix-only. Disqualified.
3. **`node-pty`** — Only relevant if we picked Electron. Works but has known Windows pain (rebuilds, native module loading).
4. **Roll our own around `windows-rs` ConPTY APIs** — Don't. ConPTY's edge cases are not worth re-discovering.

**Decision.** Use **`portable-pty`** in the Rust core. Each spawned session is an actor (Tokio task) that owns:

- The `PtyPair` (master + slave handles).
- A child process handle with `cwd = project.path`, `command = claude`, with arguments and env inherited from GUPPI's environment.
- A read loop pulling from the master, parsing ANSI as needed (defer parsing until terminal-panel feature lands), and emitting `SessionOutput { project_id, bytes }` events.
- A write channel for input.
- A resize channel for terminal-size changes.
- Cleanup on drop: send SIGINT/CTRL_C, wait briefly, kill the process tree.

**Cross-platform note (Windows specifically).** ConPTY (Windows 10 1809+) is the real PTY on Windows and `portable-pty` uses it. Known sharp edges:

- Process-tree kill on Windows isn't built in; use `taskkill /F /T /PID` or the `windows` crate's `TerminateJobObject` with a Job Object wrapping the child. **Wrap each child in a Job Object** so GUPPI crashing leaves no orphans.
- ConPTY emits UTF-8 with some quirks; the terminal panel will eventually need a real VT parser (`vte` crate) — defer.
- ANSI cursor sequences and bracketed paste mostly work. Test early.
- WSL — if `claude` runs in WSL, that's a different beast; assume native Windows `claude.exe` for v1.

**Consequences.**
- (+) Solid cross-platform abstraction without re-implementing ConPTY.
- (+) Each session is an independent actor — failure isolation is natural.
- (–) ANSI/VT parsing comes later but the architecture must leave room.
- (–) Windows process-group cleanup needs deliberate Job Object wiring; not auto.

**Reversibility.** Low-cost. The actor boundary insulates the rest of the system from the PTY library.

**Flag:** This is the only place I'd recommend a small dedicated spike *before* committing — spend a day proving that `portable-pty` + Job Object + `claude.exe` on Windows actually works end-to-end with a long-running session.
