---
id: canvas-011
title: Command palette ⌘K — keyboard-driven, voice-vocabulary parity
status: backlog
type: feature
context: canvas
created: 2026-05-16
completed:
commit:
depends_on: [design-system-001, design-system-003, design-system-004]
blocks: []
tags: [command-palette, keyboard, voice-parity, v2]
related_adrs: [ADR-003]
related_research: []
prior_art: []
---

## Why

The vision lists "Command palette — keyboard-driven (Ctrl/Cmd+K), fuzzy
command search, same vocabulary as voice commands" under "Beyond v1".
The 2026-05-16 design pins it: §8 + `guppi-palette.jsx`'s
`ViewCommandPalette`.

The headline contract: **voice and keyboard share a vocabulary**. The
same string — "refine task 104 in image-gallery" — works as a spoken
command (via `voice` BC) or a typed one (via this palette). This is
the design rule that lets the dev fluently switch modalities without
re-learning. Each palette row shows the verbal form right-aligned so
voice fluency is built by reading.

Note: per `context-map.md`, command-routing is currently *not* a
separate BC (`command-router` was considered and deferred). The
palette UI lives in `canvas`; the intent-to-executor routing it
shares with voice is a candidate for its own BC if the surface grows.

## What

Spec in `references/claude-design-2026-05-16/project/guppi-palette.jsx`
+ §8 rationale in `GUPPI.html`. Pinned decisions:

**Layout / chrome:**

- 640px wide × ~480px tall, surface-1, 12px radius (`--g-r-panel`).
- Modal scrim: `--g-overlay` (`rgba(10,10,14,0.62)` dark /
  `rgba(20,22,30,0.32)` light) over the canvas — background stays
  visible so the user remembers where they are.
- 14px Inter input field with blinking 1px caret.
- 8/12 row padding, 6px row radius.
- Row icon: 22×22 square, surface-2 fill.
- Active row: surface-4 background + 2px brand-orange left rail.
- Row gloss / hint: 12px fg-3.
- Voice equivalent: 10px mono, right-aligned in fg-4.
- Keyboard shortcut, where applicable: hairline-strong pill, 10px mono.
- Footer: 10px mono, fg-3 — "⌘K to dismiss" + verbal hint.

**Behavior:**

- ⌘K (Ctrl+K on Windows) toggles open.
- Fuzzy match on command name + argument fragments.
- Up/Down arrow + Enter to select; ESC to dismiss.
- Same vocabulary parsable by `voice` — both inputs share an intent
  layer. (Routing layer location TBD; see context-map.md.)
- The footer's verbal hint reminds the user that any typed command
  is also a spoken command.

**Light theme:**

- Inherits all tokens. Scrim swaps to `--g-overlay` light.

## Acceptance criteria

- [ ] ⌘K / Ctrl+K opens the palette over the canvas.
- [ ] Fuzzy match works on command + argument substrings.
- [ ] Arrow/Enter/ESC keyboard handling works as specified.
- [ ] Active row renders the brand-orange rail and surface-4 fill.
- [ ] Voice-equivalent string renders right-aligned on every row.
- [ ] Keyboard-shortcut pill renders where applicable.
- [ ] Selecting a command executes via the same intent-to-executor
      path the `voice` BC uses.
- [ ] Light theme renders cleanly.

## Notes

**Status:** **backlog** — v2. The vision sequences this after voice
input lands (the parity contract assumes voice exists). Capture only;
do not promote.

**Hard upstream:** the intent-to-executor router (or whatever
performs voice-command-string → canvas/runner action mapping) must
exist or be designed alongside. Re-check context-map.md when this
task is refined — if the surface has grown enough to want a
`command-router` BC, that's a strategic question (escalate via
`strategic-modeler`).

**Open questions for refinement:**

- Built-in command catalogue at v1? (Hardcoded list of N actions?)
- Argument auto-complete (project-name fuzzy match, task-id fuzzy)?
- Custom user commands / aliases? (Probably v3.)
- History / recent commands?

**Bundle reference:** `references/claude-design-2026-05-16/project/guppi-palette.jsx`
+ `GUPPI.html` §8.
