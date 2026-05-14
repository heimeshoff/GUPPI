---
id: ADR-012
title: ClaudeSession teardown — release the PTY master before joining the read loop, bounded best-effort join
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-013-pty-spike]
---

# ADR-012: `ClaudeSession` teardown — release the PTY master before joining the read loop, bounded best-effort join

**Status:** Accepted
**Scope:** global

## Context

ADR-006 specifies a `ClaudeSession` actor whose `Drop` does "send `CTRL_C`,
wait briefly, then kill the process tree". The empirical PTY spike
(`infrastructure-013-pty-spike`) implemented that actor and found a concrete
teardown-ordering hazard that ADR-006 did not call out.

The actor's read loop runs on its own OS thread, blocked in a `read()` on a
cloned PTY-master reader. The natural `Drop` is: signal the loop to stop, kill
the child, then `join()` the read-loop thread for a clean shutdown.

On Windows that `join()` **hangs**. The ConPTY reader does not observe EOF when
the *child* exits — it observes EOF only when the **PTY master handles
close**. Because Rust drops struct fields *after* `Drop::drop` returns, the
master was still alive during the join, so the reader stayed parked and the
join blocked until the OS lazily tore the pseudoconsole down (observed: 60+
seconds). A session teardown that can hang for a minute is unacceptable — it
would freeze GUPPI on app exit or on every "end session".

## Decision

`ClaudeSession::drop` tears down in this explicit order:

1. Signal the read loop to stop; kill the child and `wait()` it.
2. **Drop the PTY master and the writer** (held in `Option` fields so `Drop`
   can `take()` them). This closes the master handles, which is what makes the
   ConPTY reader see EOF and return.
3. **Bounded best-effort join** of the read-loop thread: the join runs on a
   watchdog thread and the drop waits at most ~2s for it. If the reader is
   still parked, the thread is **detached** — it dies on its own once ConPTY
   fully closes.
4. Let the Job Object handle drop — `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
   guarantees the OS reaps the whole process tree regardless.

The join is *tidiness*, not *correctness*. The correctness guarantee — no
orphan processes — is the Job Object (ADR-006), not the thread join.

## Options considered

1. **Detach the read-loop thread entirely; never join.** Simplest, never
   hangs. Rejected as the *sole* mechanism: a clean join, when it is quick, is
   worth having for deterministic shutdown in tests and logs. Kept as the
   fallback.
2. **Join first, then drop the master** (the naive order). Rejected — this is
   the bug: it hangs on Windows.
3. **Release the master first, then bounded best-effort join.** Chosen — fast
   in the common case, and structurally cannot hang GUPPI.
4. **Use a non-blocking / timeout-capable reader.** More invasive, and
   `portable-pty`'s reader does not offer it portably. Not worth it when the
   Job Object already guarantees cleanup.

## Consequences

- (+) Session teardown is bounded — GUPPI never freezes on app exit or
  end-session, even if a ConPTY reader is slow to unblock.
- (+) The common case still gets a clean, deterministic join.
- (+) Reinforces ADR-006's principle that the Job Object — not thread
  bookkeeping — is the real orphan-free-cleanup guarantee.
- (–) `master` and `writer` live behind `Option`, so `write`/`resize` carry a
  cheap "session is being torn down" guard. Minor ergonomic cost, contained in
  the `pty` module.
- (–) On rare slow teardowns a detached reader thread outlives the
  `ClaudeSession` briefly. It holds only a soon-dead reader handle and exits on
  the next failed `read()`; harmless.

## Reversibility

High. This is an internal teardown detail of the `pty` module, entirely behind
ADR-006's actor boundary. Nothing outside `pty.rs` observes it.
