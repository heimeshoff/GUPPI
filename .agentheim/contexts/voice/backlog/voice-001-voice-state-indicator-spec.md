---
id: voice-001
title: Voice-state indicator — screen-space ambient pill, 4 states
status: backlog
type: feature
context: voice
created: 2026-05-16
completed:
commit:
depends_on: [design-system-001, design-system-003, design-system-004]
blocks: []
tags: [voice, screen-space, ambient-indicator, v2]
related_adrs: [ADR-007]
related_research: []
prior_art: []
---

## Why

The vision treats voice as a first-class modality: "A wake word ("Bob")
puts the app into listening mode... A single ambient voice-state
indicator in the bottom-right of the viewport (screen-space, doesn't
pan) shows: idle / listening / muted." The 2026-05-16 design pins it:
§4 of the artboard set + `guppi-voice-views.jsx` (`ViewVoiceIdle`,
`ViewVoiceListening`, `ViewVoiceSpeaking`, `ViewVoiceMuted`).

Voice is for context-free, ambient control — "Bob, refine task 104" —
while the dev's hands are on the keyboard. The indicator must be
findable without searching, but it must NEVER pull attention away
from the canvas. One small pill, bottom-right, doesn't move when you
pan.

## What

Spec in `references/claude-design-2026-05-16/project/guppi-voice-views.jsx`
+ §4 rationale in `GUPPI.html`. Pinned decisions:

**Position / chrome:**

- Pinned to viewport bottom-right (offset 16,16 — screen-space, not
  canvas-space).
- Pill, hairline border (`--g-hairline-strong`), surface-1 fill.
- 7×12 inner padding.
- Does NOT pan or zoom with the canvas.

**Four states:**

1. **Idle** — dot at `--g-status-idle` grey, hollow. No motion. The
   subtle "yes, voice is available" signal.
2. **Listening** — dot at `--g-status-running` brand blue. Slow
   subtle pulse (`--g-pulse` 1600ms). Transcript overlay (see below).
3. **Speaking (TTS narration)** — dot at `--g-periwinkle` brand
   orange. 1px dashed ring around the dot (`--g-periwinkle` opacity
   0.55). 3-bar visualizer animates at 1.4–1.8s period (2px wide bars).
4. **Muted** — dot at `--g-fg-2` neutral. 1.4px diagonal slash through
   the dot.

**Transcript overlay (listening state):**

- 12px Inter, line-height 1.5.
- Hairline-strong border.
- Fades after recognition (~600ms).
- "Confidence" indicator: 10px mono, fg-4.

**Light theme:**

- Inherits all tokens from `design-system-004`. Speaking-state dashed
  ring colour adjusts via the light palette.

## Acceptance criteria

- [ ] Indicator pinned screen-space to bottom-right; verified not to
      pan/zoom with the canvas.
- [ ] All four states render with the design's specs (dot colour,
      motion, slash).
- [ ] Listening state pulses on `--g-pulse` cycle.
- [ ] Transcript overlay renders in listening state and fades on
      recognition.
- [ ] Speaking state animates 3-bar visualizer at the specified period.
- [ ] State transitions are immediate (no janky cross-fade between
      modes).
- [ ] Light theme renders cleanly.
- [ ] Integrates with `voice` BC's Whisperheim/Utterheim ACL — the
      indicator state mirrors the bridge's state, not a duplicate
      one inside this UI.

## Notes

**Status:** **backlog** — v2 (per vision: "Beyond v1 — Voice input:
'Bob, ...' wake word..."). Capture only.

**Hard upstream:** `voice` BC's Whisperheim/Utterheim bridge must exist
(ADR-007 specifies the WebSocket contract). The indicator subscribes
to the bridge's state events.

**Open questions for refinement:**

- Where does the wake-word lifecycle live — inside `voice` BC (extends
  Whisperheim with a wake-word listener) or in-process (GUPPI listens
  to a hot mic and forwards on wake)?
- "Speaking" state needs the TTS playback signal — Utterheim emits
  start/end events. Confirm contract.
- Click-to-toggle-mute behaviour — does clicking the indicator mute
  the mic? Confirm with Marco at refinement.
- Multiple-listening states (e.g. "wake-word detected, awaiting
  command") — is that a distinct sub-state?

**Bundle reference:** `references/claude-design-2026-05-16/project/guppi-voice-views.jsx`
+ `GUPPI.html` §4.
