---
id: infrastructure-007-voice-integration
type: decision
status: done
scope: global
depends_on: []
completed: 2026-05-14
related_adrs: [ADR-007-voice-integration]
---

# Decision: Voice integration architecture

## Context

Wake-word "Bob" with input via Whisperheim (Silero VAD + Parakeet) and output via Utterheim. Vision's open question: *where* does the wake-word listener live, and how does GUPPI consume Whisperheim's transcripts?

User stated preference: extend Whisperheim to expose an interface GUPPI can subscribe to. Fallback: in-process listening inside GUPPI.

## Architect's recommendation

**Extend Whisperheim with a local WebSocket bridge.** GUPPI subscribes to `{ wake_word, transcript }` events; emits `{ speak, text }` events to Utterheim similarly. Always-on mic stays in Whisperheim (correct from UX and OS-permissions perspective).

## Acceptance criteria

- [x] ADR committed at `.agentheim/knowledge/decisions/ADR-007-voice-integration.md`
- [x] `voice-bridge.md` contract document specced (event protocol, port discovery, reconnection semantics)
- [x] Decision recorded on whether to add the WebSocket bridge to Whisperheim now or fall back to in-process VAD

## Architect open question Marco must answer

**Willingness to add WebSocket bridge to Whisperheim.** This decision lives across two repos. If touching Whisperheim now is a non-starter, fallback ADR-007a (in-process VAD via `voice_activity_detector` crate + Parakeet subprocess) gets promoted.

## Notes — architect's ADR draft

### ADR-007: Voice integration — extend Whisperheim with a local WebSocket interface; GUPPI subscribes

**Status:** Proposed
**Scope:** global

**Context.** Vision specifies wake-word "Bob" with input via Whisperheim (Silero VAD + Parakeet) and output via Utterheim. The vision's open question is *where* the wake-word listener and Whisperheim run, with stated preference for an interface GUPPI subscribes to vs in-process listening.

**Options considered.**
1. **Whisperheim exposes a local WebSocket / Unix-socket / named-pipe server.** GUPPI connects, receives `{ type: "transcript", text, confidence, started_at, ended_at }` and `{ type: "wake_word", word: "Bob", at }` events. Sends `{ type: "speak", text }` to Utterheim similarly.
2. **In-process listening in GUPPI** — Embed Silero VAD + Parakeet directly. Doable in Rust via ONNX Runtime but duplicates Whisperheim's work.
3. **Run Whisperheim as a child process of GUPPI** — GUPPI spawns it, communicates over stdio. Coupling without isolation.

**Decision.** **Option 1.** Define a stable contract document `voice-bridge.md` in GUPPI's repo: a local WebSocket server on `127.0.0.1:<port>` (port read from a known config file written by Whisperheim, e.g. `%APPDATA%\whisperheim\bridge.json`), JSON event protocol. GUPPI connects on startup, reconnects with exponential backoff if Whisperheim isn't running, gracefully degrades (voice unavailable indicator on the canvas).

Wake-word detection lives in Whisperheim. GUPPI only reacts to a `wake_word` event by entering "listening" mode, then expects subsequent `transcript` events until silence or a timeout.

Utterheim gets a similar contract for TTS output.

**Consequences.**
- (+) Whisperheim and GUPPI develop independently; voice tech upgrades don't require GUPPI changes.
- (+) Always-on mic stays in one place (Whisperheim), which is correct from a UX and OS-permissions perspective.
- (+) Other consumers (a future TUI, a CLI experiment) can subscribe to the same bridge.
- (–) Requires coordinated changes to Whisperheim — if you don't want to touch Whisperheim now, this is blocked. The fallback ADR-007a (in-process VAD via `voice_activity_detector` crate + spawn-Parakeet-as-subprocess) is documented as an escape but **not recommended**.
- (–) The contract is now a versioned thing; breaking changes need both repos to move.

**Reversibility.** Medium. The contract is small; if Whisperheim's owner (also Marco) decides differently, we adapt.

## Outcome

ADR written and **Accepted** at `.agentheim/knowledge/decisions/ADR-007-voice-integration.md`.

Marco answered the architect's open question: **yes, willing to add the
WebSocket bridge to Whisperheim.** The recommended cross-repo approach
(Option 1) is adopted; the in-process fallback ADR-007a is documented as an
escape hatch but **not promoted**.

**Decision:** Extend Whisperheim with a local WebSocket bridge bound to
`127.0.0.1:<port>`. GUPPI connects as a client, subscribes to `wake_word` and
`transcript` events, and sends `speak` events (forwarded to Utterheim). The
always-on mic and wake-word detection stay in Whisperheim. Port is discovered
via a `bridge.json` config file Whisperheim writes; GUPPI reconnects with
exponential backoff and degrades gracefully (voice-unavailable indicator) when
the bridge is absent.

The versioned transport contract is specified concretely in
`voice-bridge.md` (placed in the `infrastructure` BC as a cross-cutting
transport contract): JSON event shapes, the `bridge.json` discovery file,
exponential-backoff reconnection with jitter, stale-file detection, and
graceful-degradation semantics. Protocol version `1`.

Note: the `voice` BC is out of v1 scope — no application code lands now. This
decision fixes the foundation boundary so other infrastructure choices stay
coherent.

Key files:
- `.agentheim/knowledge/decisions/ADR-007-voice-integration.md` — the decision.
- `.agentheim/contexts/infrastructure/voice-bridge.md` — the versioned
  transport contract.
