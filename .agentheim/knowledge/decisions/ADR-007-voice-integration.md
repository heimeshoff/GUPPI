---
id: ADR-007
title: Voice integration — extend Whisperheim with a local WebSocket bridge; GUPPI subscribes
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-007-voice-integration]
---

# ADR-007: Voice integration — extend Whisperheim with a local WebSocket bridge; GUPPI subscribes

**Status:** Accepted
**Scope:** global

## Context

The vision specifies wake-word "Bob" with voice **input** via Whisperheim
(Silero VAD + Parakeet for STT) and voice **output** via Utterheim (TTS). The
`voice` bounded context owns the GUPPI-side behavior — wake-word reaction,
intent mapping, narration — but Whisperheim and Utterheim are **separate,
external services** with their own repos and lifecycles.

The vision's open foundation question is *where* the wake-word listener and
the STT pipeline run, and *how* GUPPI consumes Whisperheim's transcripts.
Marco's stated preference was to extend Whisperheim to expose an interface
GUPPI subscribes to, rather than embedding listening inside GUPPI.

This decision lives across two repos (GUPPI and Whisperheim). The architect's
open question — **is Marco willing to add a WebSocket bridge to Whisperheim
now** — has been answered: **yes.** The recommended cross-repo approach is
adopted; the in-process fallback (drafted as ADR-007a) is **not** promoted.

ADR-001 already committed to a Tauri 2 Rust core, and named "the voice IPC
client" as one of the Rust core's responsibilities. This ADR specifies what
that client connects to.

Note: the `voice` BC is **out of v1 scope** (v1 is canvas-only). This ADR is
made now anyway because the *boundary* — where the wake-word listener runs —
is a foundation choice that other infrastructure decisions must be coherent
with. The bridge client code lands post-v1; the contract is fixed now.

## Options considered

1. **Whisperheim exposes a local WebSocket server.** GUPPI connects as a
   client, receives `wake_word` and `transcript` events, and sends `speak`
   events for TTS. The always-on mic stays inside Whisperheim.
2. **In-process listening in GUPPI** — embed Silero VAD + Parakeet directly
   in GUPPI's Rust core via ONNX Runtime. Doable, but duplicates
   Whisperheim's pipeline inside GUPPI and couples voice-tech upgrades to
   GUPPI releases. Drafted as fallback ADR-007a; **not adopted.**
3. **Run Whisperheim as a child process of GUPPI** — GUPPI spawns it and
   communicates over stdio. Couples the two lifecycles without giving the
   isolation benefit of option 1; also makes Whisperheim un-shareable with
   other consumers.

## Decision

Adopt **Option 1.** Extend Whisperheim with a **local WebSocket bridge**, and
have GUPPI subscribe to it as a client.

- Whisperheim runs a WebSocket server bound to **`127.0.0.1:<port>`** (loopback
  only — never exposed off-machine).
- The port is **discovered**, not hard-coded: Whisperheim writes a small JSON
  config file at a known path (on Windows,
  `%APPDATA%\whisperheim\bridge.json`) containing the chosen port and the
  protocol version. GUPPI reads this file to learn where to connect.
- The wire protocol is **JSON events**. Inbound to GUPPI: `wake_word`
  (`{ word: "Bob", at }`) and `transcript`
  (`{ text, confidence, started_at, ended_at }`). Outbound from GUPPI to the
  bridge (forwarded to Utterheim): `speak` (`{ text }`).
- **Wake-word detection lives in Whisperheim.** GUPPI does not run VAD or
  listen to the mic. GUPPI reacts to a `wake_word` event by entering
  "listening" mode, then consumes subsequent `transcript` events until silence
  or a timeout.
- GUPPI connects on startup, **reconnects with exponential backoff** if
  Whisperheim is not running, and **degrades gracefully** when the bridge is
  unavailable — voice is simply marked unavailable on the canvas; the rest of
  GUPPI is unaffected.
- Utterheim is reached **through the same bridge** for TTS output, so GUPPI
  has a single voice transport to manage. The bridge is responsible for
  routing `speak` events to Utterheim.

The concrete wire format, port-discovery file shape, and reconnection
semantics are specified in the companion contract document
**`voice-bridge.md`** in the `infrastructure` bounded context. That document
is the versioned contract; this ADR is the rationale.

## Consequences

- (+) Whisperheim and GUPPI develop independently; voice-tech upgrades
  (a new VAD, a new STT model) do not require GUPPI changes.
- (+) The always-on mic lives in exactly one place (Whisperheim), which is
  correct from both a UX standpoint and an OS-permissions standpoint — one
  process owns the microphone grant.
- (+) Other consumers — a future TUI, a CLI experiment — can subscribe to the
  same bridge without re-implementing voice.
- (+) Graceful degradation is natural: no bridge means no voice, not a broken
  GUPPI.
- (–) Requires coordinated changes to Whisperheim. Marco owns both repos and
  has signed off, so this is accepted, not blocking — but the bridge server
  is now a Whisperheim deliverable that must exist before the `voice` BC's
  first task can be exercised.
- (–) The contract is now a **versioned artifact**. Breaking changes to the
  event protocol require both repos to move together; `voice-bridge.md`
  carries a protocol version for exactly this reason.
- (–) WebSocket-over-loopback adds a small amount of transport machinery
  (framing, reconnection) compared to in-process calls — accepted as the cost
  of repo independence.

## Reversibility

Medium. The contract surface is small and versioned. If Whisperheim's owner
(also Marco) later decides on a different transport — Unix-domain socket,
named pipe, in-process — only the `voice` BC's bridge client and
`voice-bridge.md` change; the rest of GUPPI talks to the `voice` BC's ACL in
GUPPI terms (`utterance`, `command intent`, `narration`) and is unaffected.
The in-process fallback (ADR-007a) remains documented as the escape hatch if
the cross-repo coordination ever becomes untenable.
