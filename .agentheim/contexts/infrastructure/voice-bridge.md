# Voice Bridge Contract

**Status:** Specified (not yet implemented — the `voice` BC is out of v1 scope)
**Owning context:** infrastructure (cross-cutting transport contract)
**Decided by:** [ADR-007](../../knowledge/decisions/ADR-007-voice-integration.md)
**Protocol version:** `1`

This document is the **versioned contract** between GUPPI and Whisperheim (and,
through Whisperheim, Utterheim). It specifies the transport, the port-discovery
mechanism, the JSON event shapes, and the reconnection / degradation semantics.

Both repos — GUPPI and Whisperheim — must agree on this document. A breaking
change to any shape below requires bumping `protocol_version` and a
coordinated release of both repos.

---

## 1. Transport

- **Whisperheim** runs a **WebSocket server** bound to **`127.0.0.1:<port>`**
  — loopback only. The server MUST NOT bind to a non-loopback interface.
- **GUPPI** connects as a **WebSocket client**. GUPPI is always the client;
  Whisperheim is always the server.
- A single connection is **bidirectional**: GUPPI receives `wake_word` and
  `transcript` events, and sends `speak` events on the same socket.
- All frames are **UTF-8 JSON text frames**. One JSON object per frame. No
  binary frames in protocol version `1`.
- Every JSON object carries a `type` field (string) identifying the event.
  Receivers MUST ignore unknown `type` values rather than erroring — this
  keeps the protocol forward-compatible within a major version.

## 2. Port discovery

The port is **not hard-coded**. Whisperheim chooses a free loopback port at
startup and advertises it via a config file.

- **Path (Windows):** `%APPDATA%\whisperheim\bridge.json`
- The file is written by Whisperheim **after** the WebSocket server is
  listening, and is **deleted (or its `port` cleared) on clean shutdown**.
- GUPPI reads this file to discover the endpoint. If the file is missing,
  unreadable, or stale (see §5), GUPPI treats voice as unavailable.

### `bridge.json` shape

```json
{
  "protocol_version": 1,
  "host": "127.0.0.1",
  "port": 53117,
  "pid": 18244,
  "started_at": "2026-05-14T09:21:03Z"
}
```

| field              | type   | notes                                                              |
|--------------------|--------|--------------------------------------------------------------------|
| `protocol_version` | int    | Bridge protocol version. GUPPI refuses to connect on mismatch.     |
| `host`             | string | Always `127.0.0.1` in version `1`.                                 |
| `port`             | int    | TCP port the WebSocket server is listening on.                     |
| `pid`              | int    | Whisperheim's process id — lets GUPPI detect a stale file.         |
| `started_at`       | string | ISO-8601 UTC. When the server started listening.                   |

- On `protocol_version` mismatch, GUPPI MUST NOT connect; it logs the mismatch
  and marks voice unavailable.
- GUPPI MAY watch `bridge.json` for changes (e.g. via the FS-observation
  machinery) so it can connect promptly when Whisperheim starts after GUPPI.

## 3. Event protocol

### 3.1 Inbound to GUPPI

#### `wake_word`

Emitted by Whisperheim when the wake word is detected. This is GUPPI's signal
to enter "listening" mode.

```json
{
  "type": "wake_word",
  "word": "Bob",
  "at": "2026-05-14T09:22:41.118Z"
}
```

| field  | type   | notes                                              |
|--------|--------|----------------------------------------------------|
| `type` | string | Literal `"wake_word"`.                             |
| `word` | string | The detected wake word. `"Bob"` in version `1`.    |
| `at`   | string | ISO-8601 UTC timestamp of detection.               |

#### `transcript`

Emitted by Whisperheim for each captured utterance span. After a `wake_word`,
GUPPI consumes `transcript` events until silence or a timeout (the timeout is
GUPPI-side policy, owned by the `voice` BC — not part of this transport
contract).

```json
{
  "type": "transcript",
  "text": "refine task 104 in image-gallery",
  "confidence": 0.93,
  "started_at": "2026-05-14T09:22:42.500Z",
  "ended_at": "2026-05-14T09:22:45.020Z"
}
```

| field        | type   | notes                                                       |
|--------------|--------|-------------------------------------------------------------|
| `type`       | string | Literal `"transcript"`.                                     |
| `text`       | string | The transcribed utterance text.                             |
| `confidence` | number | STT confidence in `[0.0, 1.0]`.                             |
| `started_at` | string | ISO-8601 UTC. Start of the utterance span.                  |
| `ended_at`   | string | ISO-8601 UTC. End of the utterance span.                    |

### 3.2 Outbound from GUPPI

#### `speak`

Sent by GUPPI when it wants Utterheim to narrate something. The bridge
forwards `speak` events to Utterheim; GUPPI does not talk to Utterheim
directly.

```json
{
  "type": "speak",
  "text": "Task 104 is refined. Want me to start it?"
}
```

| field  | type   | notes                                              |
|--------|--------|----------------------------------------------------|
| `type` | string | Literal `"speak"`.                                 |
| `text` | string | The text Utterheim should speak.                   |

In protocol version `1`, `speak` is **fire-and-forget**: no acknowledgement
frame is defined. Delivery is best-effort while the connection is up; `speak`
events sent while disconnected are dropped (see §5).

## 4. Connection lifecycle

1. **GUPPI startup** — GUPPI reads `bridge.json`. If present and the
   `protocol_version` matches, GUPPI opens a WebSocket connection to
   `ws://<host>:<port>`.
2. **Connected** — GUPPI listens for `wake_word` / `transcript` frames and may
   send `speak` frames. Voice is marked available on the canvas.
3. **Disconnected** — the socket closes, errors, or `bridge.json` is missing.
   GUPPI enters the reconnection loop (§5) and marks voice unavailable.
4. **Whisperheim restart** — `bridge.json` may now carry a new `port` /
   `pid`. GUPPI re-reads it before each reconnection attempt so it always
   targets the current endpoint.

## 5. Reconnection & graceful degradation

Voice is an **enhancement, never a dependency**. The rest of GUPPI MUST
function fully with the bridge absent.

### Reconnection — exponential backoff

When the connection is unavailable (no `bridge.json`, stale file, connect
failure, or a dropped socket), GUPPI retries with **exponential backoff and
jitter**:

- Initial delay: **1 s**.
- Multiplier: **2×** per failed attempt.
- Maximum delay: **30 s** (the backoff caps here and retries indefinitely at
  30 s intervals).
- **Jitter:** each delay is randomized within ±20 % to avoid lockstep retries.
- On a **successful connection**, the backoff resets to the initial 1 s delay.

### Stale `bridge.json` detection

`bridge.json` may be left behind by a crashed Whisperheim. Before connecting,
GUPPI treats the file as **stale** (and skips the connect attempt, staying in
the backoff loop) if either:

- the process named by `pid` is not alive, or
- a TCP connect to `host:port` is refused.

### Graceful degradation

While the bridge is unavailable:

- GUPPI shows a **"voice unavailable"** indicator on the canvas. No modal, no
  error dialog — voice being off is a normal state.
- `speak` events produced by GUPPI while disconnected are **dropped** (logged
  at debug level). Version `1` defines **no outbound queue** — narration is
  only meaningful in real time.
- No `wake_word` / `transcript` events arrive, so GUPPI simply never enters
  listening mode. All non-voice input paths are unaffected.

## 6. Versioning

- This contract is **`protocol_version: 1`**.
- Additive, backward-compatible changes (new optional fields, new `type`
  values) do **not** bump the version — receivers ignore what they don't
  recognize (§1).
- Any **breaking** change (renamed/removed field, changed semantics, changed
  discovery path) bumps `protocol_version` and requires a coordinated release
  of GUPPI and Whisperheim. On a version mismatch, GUPPI refuses to connect
  and marks voice unavailable.

## 7. Out of scope for version `1`

- **Authentication** — loopback-only binding is the trust boundary; no token
  or handshake auth in version `1`.
- **`speak` acknowledgements / outbound queueing** — `speak` is
  fire-and-forget; no delivery guarantee while disconnected.
- **Binary frames / audio streaming** — only JSON text frames. Raw audio
  never crosses the bridge; Whisperheim owns the mic and ships only
  transcripts.
- **Multiple wake words / per-consumer routing** — one wake word (`"Bob"`),
  one shared bridge. Multi-consumer fan-out is a Whisperheim-side concern, not
  part of this contract yet.
