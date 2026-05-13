---
id: infrastructure-007-voice-integration
type: decision
status: todo
scope: global
depends_on: []
---

# Decision: Voice integration architecture

## Context

Wake-word "Bob" with input via Whisperheim (Silero VAD + Parakeet) and output via Utterheim. Vision's open question: *where* does the wake-word listener live, and how does GUPPI consume Whisperheim's transcripts?

User stated preference: extend Whisperheim to expose an interface GUPPI can subscribe to. Fallback: in-process listening inside GUPPI.

## Architect's recommendation

**Extend Whisperheim with a local WebSocket bridge.** GUPPI subscribes to `{ wake_word, transcript }` events; emits `{ speak, text }` events to Utterheim similarly. Always-on mic stays in Whisperheim (correct from UX and OS-permissions perspective).

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-007-voice-integration.md`
- [ ] `voice-bridge.md` contract document specced (event protocol, port discovery, reconnection semantics)
- [ ] Decision recorded on whether to add the WebSocket bridge to Whisperheim now or fall back to in-process VAD

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
