# voice

## Purpose

The bridge to Marco's voice channel, both directions.

**Inbound:** wake-word "Guppy" detection, Whisperheim integration for STT (Silero VAD + Parakeet), fuzzy command parsing to map an utterance to an action (e.g. "Guppy, refine task 104 in image-gallery" → a structured command intent the rest of guppi can execute).

**Outbound:** Utterheim integration for TTS narration — notifications, "agent is asking you something", confirmations.

The vision treats voice as a first-class input modality and a differentiator. The pain "can't talk to my computer most of the time; to use voice I have to focus a specific terminal first" is what this BC resolves: voice is ambient, the wake word is always listening, no focus required.

## Classification

**Core.** Ambient voice control is one of guppi's headline value propositions. Whisperheim and Utterheim are external services, but the wake-word loop, intent mapping, and the "voice is the primary input" stance are guppi-specific behavior.

## Out of v1 scope

v1 is canvas-only. This BC's code does not ship in v1 — but its boundary is fixed now so foundation choices (e.g., where the wake-word listener runs) can be made coherently. Its first task lands post-v1.

## Ubiquitous language (seed)

- **Wake word** — "Guppy". The trigger that begins a voice command.
- **Utterance** — a captured span of speech from the user.
- **Transcript** — the text produced by Whisperheim from an utterance.
- **Transcriber** — Whisperheim; the STT pipeline (Silero VAD + Parakeet).
- **Narrator** — Utterheim; the TTS channel guppi speaks through.
- **Command intent** — a structured representation of what the user asked for: verb + target + arguments (e.g. `focus(project=image-gallery)`).
- **Fuzzy match** — the tolerant mapping from a transcript to a command intent, accommodating speech variation.
- **Voice focus command** — a special class of intent that drives `canvas` focus ("zoom to image-gallery").
- **Narration** — outbound TTS speech directed at Marco.

## Anticorruption layer (both directions)

Whisperheim and Utterheim have their own vocabularies and lifecycles. This BC wraps both behind an ACL so the rest of guppi only ever sees guppi terms (`utterance`, `command intent`, `narration`). If either service is swapped, the change is contained here.

## Upstream / downstream

- **Downstream of:** Whisperheim (STT, via ACL), Utterheim (TTS, via ACL).
- **Upstream of (v2+):** `canvas` (focus commands), `project-registry` (new-project), `claude-runner` (task/command actions). Routing layer between intent and executor is intentionally unspecified at v1 — see context-map.md open questions about a possible `command-router` BC.

## Open questions

- Where does the wake-word listener run? In-process inside this BC, or extended into Whisperheim as a subscription interface? Vision flags this as a foundation question.
- Fuzzy command parsing — heuristic local code, or LLM-mediated intent extraction? Foundation pass.
- Audio device ownership (mic in, speaker out) — directly here, or via an infrastructure-provided audio service? Lean: directly here for v2; revisit if a second consumer appears.
