# Status indicator design

_Started 2026-05-16 08:37 UTC_

---

## User

<system-info comment="Only acknowledge these if relevant">
Project title is now "GUPPI"
Current date is now May 16, 2026
</system-info>

<pasted_text name="Pasted text (116 lines)">
Design the UI and UX for GUPPI — a single-user, native desktop mission-control
  app for managing AI-coding-agent (Claude Code) projects from one ambient surface.

  ## Who it's for
  One developer, on one machine, working across many small repos at once.
  He talks to it ("Bob, refine task 104 in image-gallery"), but also drives with
  mouse and keyboard. He sits in front of it for hours during deep work.

  ## The product in one sentence
  A Miro-like infinite canvas where every project is a frame, every project's
  bounded contexts are bubbles inside the frame, and live indicators show
  exactly which agents are running, idle, or stuck waiting for him to answer.

  ## The primary view: the canvas
  - An infinite pannable, zoomable, dark-mode world (think Miro / Figma / tldraw,
    but quieter and more focused — no toolbars cluttering the edges).
  - Each **project** is a rectangular **frame** with rounded corners and a thin
    border. The frame has:
    - A header bar across the top: drag handle, project name, status read-out
      (mini badges per state with counts), and a small monospace task-count row.
    - A body containing the project's bounded-context bubbles.
  - Each **bounded context (BC)** is a smaller rounded-rectangle bubble inside
    the frame, denser than the project frame, showing:
    - BC name (left)
    - A counts pill on the right (e.g. ▶3, ◆1) in mono
    - A status badge in the top-right corner (color + glyph)
  - Inside a frame, BCs are connected by typed edges showing their DDD
    relationship: customer-supplier (arrowhead), shared-kernel (no arrow),
    anti-corruption-layer (arrow + midpoint triangle notch), conformist (lighter
    arrow). Single neutral color, geometry distinguishes the type.
  - Frames auto-fit to their content. The user can drag frames around the canvas
    and drag BCs within their frame; positions persist.

  ## Status language (very important — this is the live pulse of the app)
  Every BC carries one of four states, shown as color + glyph (colorblind-safe):
  - `idle`     — grey-blue, hollow circle (nothing happening)
  - `running`  — bright blue, play triangle (an agent is working — slow ambient pulse)
  - `blocked`  — amber, diamond (agent is waiting on the user to answer a question)
  - `missing`  — magenta-pink, ✕ (expected artifact absent)

  When a BC is `blocked`, the actual question the agent is asking should be
  rendered as a small tethered callout near that BC on the canvas, so the user
  can see across the whole workspace which agents need them.

  ## Voice as a first-class modality
  - A wake word ("Bob") puts the app into listening mode.
  - A single ambient **voice-state indicator** in the bottom-right of the viewport
    (screen-space, doesn't pan) shows: idle / listening / muted. One small glyph,
    no chrome.
  - When the user speaks, show a transient transcript overlay that fades after
    the command is recognized.
  - The system also speaks back (TTS narration) — design a non-intrusive way to
    represent "the app is speaking to me right now".

  ## Other surfaces (secondary to the canvas)
  - **Project detail panel** — slides in from the side when a project is focused;
    renders markdown documents (vision, ADRs, research notes, BC READMEs) cleanly,
    optimized for long-form reading on a dark background.
  - **Emulated terminal panel** — a shallow view of a running Claude session's
    orchestrator + sub-agent communication. Looks like a terminal, but cleaner
    and more readable than a raw shell — colored agent labels, collapsible
    sub-agent threads, a clear "the agent is asking you something" affordance.
  - **Command palette** — keyboard-driven (Ctrl/Cmd+K), fuzzy command search,
    same vocabulary as voice commands. Designed so power-users never need a menu.
  - **New-project flow** — a quiet modal/flow for creating a new project
    (folder + git init + kick off a brainstorming session).
  - **Right-click context menus** on frames and BCs (project-level actions, BC
    actions). Native-feeling, not webby.

  ## Visual direction
  - **Dark mode only** for now. Backdrop in a deep, slightly cool charcoal
    (#16161c). Surfaces in slightly raised charcoals.
  - **Two warm/cool accent hues for hierarchy:** a periwinkle/violet for the
    project frame border (warm, anchor), a desaturated teal for BC borders
    (cool, secondary).
  - **Restrained motion budget.** Short eased transitions for camera moves
    (~320ms ease-out cubic). One slow ambient pulse (~1600ms) reserved for the
    "running" status — nothing else animates. The canvas should feel calm even
    with 50 frames and 200 BCs on screen.
  - **One font family** (Inter / Segoe UI stack) and one mono family (Cascadia
    Code / Consolas). Three sizes only: title 16, body 12, caption 10.
  - **4px spacing scale.**
  - **Geometry, not chrome, carries meaning.** No drop shadows. No gradients. No
    decorative borders. The only ornament is the focus ring (light periwinkle)
    around hovered or focused nodes.
  - Influences to draw from: tldraw's clarity, Linear's calm density, Figma's
    canvas affordances, Things 3's restraint, a Bloomberg-terminal-like
    information density without the ugliness. **Avoid:** generic SaaS "card with
    big illustration" aesthetics, glassmorphism, neon cyberpunk, anything that
    reads as "AI startup."

  ## What I want from you
  1. **A reference frame for the empty canvas at startup** — what does the
     workspace look like with one, three, ten projects? Show how density scales.
  2. **A single project frame in detail** — header bar, body with 4–6 BC bubbles,
     relationship edges between them, status badges, counts pills. This is the
     atomic unit of the app — get this right and the rest follows.
  3. **A "blocked-on-question" moment** — show how a question rendered at a BC's
     location looks on the canvas, and how it competes for attention against
     running and idle BCs nearby.
  4. **The voice-state indicator** in all three states, in situ on the canvas.
  5. **The project detail panel** — markdown rendering of a vision document on a
     dark surface, the kind of typography that lets the user read for an hour
     without fatigue.
  6. **The terminal panel** — a running Claude session with an orchestrator and
     two sub-agents, including the moment one of them needs an answer.
  7. **A focused tile transition** — show "zoomed out overview" vs. "zoomed in to
     one project" as two states; the camera transition between them is the
     product's most-used gesture.
  8. **A keyboard-driven command palette** — fuzzy-matching, same vocabulary as
     voice.

  For each, give me both a rationale (why this serves a single developer
  managing many agent sessions in parallel) and the pixel-level decisions
  (spacing, type sizes, exact hex values, motion timing). Optimize for **calm
  density** — many things visible, none of them shouting.
</pasted_text>

<!-- The user explicitly selected the following skills for this project, as attachments to their message. These are not optional context — they define how you work. Use them. -->
<attached-skill name="Hi-fi design">
Create a high-fidelity, polished design. Follow the instructions about design in your system prompt, particularly the 'How to do design work' section. Use the design_canvas starter component, or make a full-bleed prototype and offer options via Tweaks.
</attached-skill>

<attached-skill name="Interactive prototype">
Create a fully interactive prototype with realistic state management and transitions. Use React useState/useEffect for dynamic behavior. Include hover states, click interactions, form validation, animated transitions, and multi-step navigation flows. It should feel like a real working app, not a static mockup.
</attached-skill>

<attached-skill name="Ledger Design System (design system)">
[Design System] This project uses the **Ledger Design System** design system. This is a binding choice for visual style — every visual must follow it. Don't invent colors, type, spacing, or components not grounded here.

Scope: the design system is a visual style reference only. Its guide may describe example products, brands, or people that are unrelated to the user and unrelated to the subject of this conversation. Never treat anything in the design system as a fact about the user, their work, or the topic they asked about.

    Explore it to find what you need:
    - Always copy out the fonts and colors you need
    - For prototypes and designs, always copy out any relevant components
    - If the design system contains existing mocks of products, and you were asked to design or prototype something similar, copy and fork those mocks to start your design. This helps you make high-quality designs.
    
    Explore it quickly to find relevant UI kits (e.g. mocks of existing products you can copy and fork)

Full system at `/projects/a1b214e9-22c3-483c-909b-121fff556c89/`. Its guide is reproduced below so you don't need to fetch it:

<design-system-guide>
# Ledger Design System

A design system for **Ledger**, a local-only desktop budgeting app for contract workers on Windows 11.

> Project name "Ledger" was chosen during system creation — the brief did not specify a product name. Swap freely if you want a different one.

---

## Product context

Ledger is a single-user, locally-installed desktop budgeting app. It is built for a contract-driven income earner: bursts of income from contracts, a mix of fixed and variable expenses, and tax obligations on their own rhythm. It does not have authentication, signup, sync, or collaboration — it opens straight to a dashboard.

**Stack:** React + Tailwind, Windows 11 desktop shell.

**Top-level navigation** (left rail, in this order):

1. **Dashboard** — the headline view. Days until zero money, runway, recent activity.
2. **Income** — contracted vs paid, by client.
3. **Expenses** — fixed + variable categories, subscriptions.
4. **Taxes** — obligations and reserves.

**Vocabulary** is exact and non-negotiable:

- `contracted income` — money signed for, not yet received. (Not "expected".)
- `paid income` — money actually received. (Not "received" or "earned".)
- `variable category` — an expense bucket whose amount varies month to month.
- `subscription` — recurring fixed expense.
- `tax obligation` — money owed to a tax authority.
- `zero-money day` — the date at which projected balance reaches zero. THE headline metric.

**Design tone:** calm, financial, low-decoration. Information-dense without being busy. Monospace for numbers. Generous whitespace around headline figures. Muted palette with a single accent reserved for the zero-money-day date.

This system currently delivers the **shell + Dashboard**. Income, Expenses, Taxes are stubs — to be built in subsequent prompts.

---

## Sources

This design system was generated **from the brief alone** — no Figma, codebase, screenshots, or brand assets were attached. All decisions (type, color, layout, motif) are originals derived from the tone and constraints in the brief. When real assets land, they should override the placeholders.

Substitutions to flag:

- **Fonts:** No font files were provided. The system uses [Inter Tight](https://fonts.google.com/specimen/Inter+Tight) for UI and [JetBrains Mono](https://fonts.google.com/specimen/JetBrains+Mono) for numerics, both via Google Fonts. If you have a brand typeface (e.g. Söhne, Söhne Mono, Berkeley Mono, Fraktion Mono), drop the files into `fonts/` and update `colors_and_type.css`.
- **Logo:** A wordmark + glyph were drawn as part of the system (`assets/logo.svg`, `assets/glyph.svg`). They're functional but should be replaced with a real mark when one exists.
- **Iconography:** [Lucide](https://lucide.dev) is used via CDN as a stand-in. Strokes are 1.5px, calm and geometric — a good fit for the tone. Swap in a custom set if/when one exists.

---

## Content fundamentals

**Voice.** Quiet. Accurate. Plainspoken. The app speaks like a careful accountant, not a coach. It does not motivate, congratulate, or warn dramatically. It states. The user sees their money clearly; the app gets out of the way.

**Casing.** Sentence case for everything except proper nouns. Never title case.
- ✅ `Add expense`, `Zero-money day`, `Variable category`
- ❌ `Add Expense`, `Zero-Money Day`

**Person.** Avoid first and second person where possible. The app does not say "your money" or "I noticed". It says `Balance`, `Runway`, `Paid income`. When direct address is unavoidable (empty states, confirmations), use second person sparingly: `No expenses logged this month.`

**Numbers.** Tabular figures, monospaced. Always show currency with explicit sign for deltas (`+$1,200.00`, `−$340.00`). Use proper minus sign `−` (U+2212), not hyphen. Group with commas. Two decimal places everywhere money is shown — no rounding in summaries. Negative values render in muted red, not bright red.

**Dates.** ISO-adjacent for precision (`Apr 26, 2026`, `Apr 26`). Use `Today` and `Yesterday` only in activity feeds. The zero-money day has its own treatment — see Visual foundations.

**Empty states.** State the fact, then the action.
- ✅ `No paid income this month. Add income →`
- ❌ `Looks like there's nothing here yet! Why not add some income? 🎉`

**Warnings.** Never alarmist. The app may be one of the bearers of bad news; it must not editorialize.
- ✅ `Zero-money day is in 9 days.`
- ❌ `⚠️ Warning: You're running out of money soon!`

**No emoji.** Anywhere. Not in copy, not in empty states, not in success confirmations.

**No exclamation marks.** Anywhere.

**Examples of good copy:**

> Zero-money day · May 5, 2026 · 9 days
> Runway is shorter than last month.
> 3 contracts unpaid. $8,400.00 contracted.
> Q2 tax obligation due Jun 15. Reserve $4,200.00.
> No expenses logged today.

---

## Visual foundations

### Palette

The system is built on a **warm paper** light mode and a **cool slate** dark mode. Grays do almost all the work. There is exactly one chromatic accent, and it is reserved for the zero-money-day date.

- **Surface** — `#FAF8F4` (paper) → `#0F1115` (slate)
- **Foreground** — near-black `#1A1A1F` → near-white `#E8E8EC`
- **Hairlines** — 1px borders at low contrast (`#E5E1D8` light, `#22252C` dark). Hairlines, not heavy borders, define structure.
- **Accent — `#B8741A` (ochre).** This is the *only* chromatic color in the UI. It marks the zero-money-day date and nothing else.
- **Semantic** — `paid` (`#3F6B4A` muted green) and `obligation` (`#8C3A3A` muted red). Both desaturated. Used sparingly — usually only in number signs.

The palette is intentionally *quiet*. There are no purples, blues, or teals. No gradients. No glassmorphism. No translucent fills.

### Typography

- **UI** — Inter Tight, 13–15px body, 500 weight for headers, 400 for body. Letter-spacing tightens slightly at large display sizes (-0.02em at 48px+).
- **Numerics** — JetBrains Mono with `font-feature-settings: "tnum", "zero"`. Tabular figures everywhere. Slashed zero on. Numbers and currency symbols always monospace, even in body copy.
- **Display numbers** (zero-money-day count, balance) sit at 56–72px, weight 400, with 64px+ vertical breathing room.
- Headers are quiet: 13px uppercase, 0.06em letter-spacing, weight 500, foreground muted.

### Spacing

4px base unit. Common steps: `4, 8, 12, 16, 24, 32, 48, 64, 96`. The system leans into the larger steps — generous whitespace around headline figures, tight rhythm inside data tables.

### Layout

- **Left rail** is fixed-width `220px`, paper/slate surface, hairline divider on the right.
- **Content area** scrolls vertically, has a soft inner padding of `48px` horizontal, `40px` top.
- **Cards** are flat: 1px hairline border, no shadow, 8px corner radius. They never lift, hover, or animate. Cards are organizational, not decorative.
- **Tables** are dense: 32px row height, hairline row separators, no zebra striping.
- **Section headers** sit above their content with a 13px uppercase label and a hairline that runs to the right edge.

### Backgrounds

Flat surface color. No images, gradients, patterns, textures, or blurs. The single visual flourish is the **accent ochre** on the zero-money-day chip.

### Animation

Almost none.

- State changes (number updates, page transitions) cross-fade over **120ms**, easing `cubic-bezier(0.4, 0, 0.2, 1)`.
- The zero-money-day digit, when it changes, slides up `4px` and fades in over 180ms.
- **No bounces. No springs. No skeletons.** Loading states show "—" until data resolves.

### Hover & press

- **Hover** — foreground darkens by ~6% on text links, surface lifts to `#F5F2EB` light / `#1A1D24` dark on rows. No glow.
- **Press** — foreground at 70% opacity. No scale change.
- **Focus** — 1.5px ochre ring at 2px offset. Visible only on keyboard focus.

### Borders, shadows, radii

- **Borders** — 1px hairlines, low contrast. Always. Never 2px, never colored.
- **Shadows** — none in light mode. Dark mode uses one shadow on the active rail item: `0 0 0 1px rgba(255,255,255,0.04) inset`.
- **Radii** — `4px` (small: chips, inputs), `8px` (default: cards, buttons), `0` (tables, hairlines).

### Transparency & blur

Not used. The brief calls for calm and information-dense — translucency adds noise without adding meaning.

### Imagery

Not used. The app has no avatars, no illustrations, no photography. The data is the imagery.

### Cards

Flat, hairline-bordered, 8px radius, surface color = page background (no nested fill). They organize information without creating visual hierarchy of their own.

---

## Iconography

**Set:** [Lucide](https://lucide.dev), 1.5px stroke, 16–18px in UI chrome. Loaded via CDN — see `ui_kits/desktop/index.html`.

**Why Lucide:** geometric, calm, even stroke weights, no fills. Fits the tone better than Heroicons (rounder, friendlier) or Phosphor (more decorative).

**Usage rules:**
- Icons supplement text labels in the left rail and toolbars; they never replace text in a primary action.
- Foreground color matches surrounding text. Icons do not carry color of their own.
- No emoji. No unicode glyphs as icons. No icon fonts beyond Lucide.
- Stroke weight is uniform across the system; never mix Lucide's regular set with another set.

If a needed icon is missing from Lucide, the system specifies a fallback: a 14×14 square hairline with the first letter of the concept inside (monospace, 10px). This appears intentionally as a placeholder, not an attempt at a custom icon.

---

## Index — what's in this folder

| Path | Purpose |
|---|---|
| `README.md` | This file. |
| `SKILL.md` | Skill definition for using this system in Claude Code or Claude.ai. |
| `colors_and_type.css` | Single source of truth for color and type tokens. |
| `fonts/` | Webfont references (currently Google Fonts via `@import`). |
| `assets/` | Logo, glyph, any product imagery. |
| `preview/` | Cards displayed in the Design System tab — palette, type, spacing, components. |
| `ui_kits/desktop/` | The desktop app UI kit: shell + Dashboard, plus reusable JSX components. |

### UI kits

- **`ui_kits/desktop/`** — Windows 11 desktop app. Currently delivers: app shell (left rail, top chrome), Dashboard screen (zero-money-day headline, runway, recent activity, contracts unpaid, tax obligation). Income/Expenses/Taxes screens are stubbed.

</design-system-guide>

CSS tokens: the guide may describe tokens in prose, but the exact `--*` names are defined in the design system's stylesheet(s) (`/projects/a1b214e9-22c3-483c-909b-121fff556c89/colors_and_type.css`). The 56 custom properties it defines are:

--accent-ochre, --accent-ochre-soft, --accent-ochre-tint, --border-hairline, --border-strong, --duration-base, --duration-fast, --ease-base, --fg-1, --fg-2, --fg-3, --fg-4, --focus-ring, --font-mono, --font-ui, --hairline, --hairline-strong, --lh-base, --lh-snug, --lh-tight, --obligation, --obligation-soft, --paid, --paid-soft, --radius-md, --radius-none, --radius-sm, --space-1, --space-2, --space-3, --space-4, --space-5, --space-6, --space-7, --space-8, --space-9, --surface-0, --surface-1, --surface-2, --surface-inverse, --text-2xl, --text-3xl, --text-base, --text-display, --text-headline, --text-lg, --text-md, --text-sm, --text-xl, --text-xs, --tracking-base, --tracking-display, --tracking-label, --weight-medium, --weight-regular, --weight-semibold

Only use `var(--*)` names from that list. Never guess a token name — an unresolved `var()` silently falls back to the browser default.

For assets and UI kits beyond the guide: `read_file("/projects/a1b214e9-22c3-483c-909b-121fff556c89/<path>")` and `copy_files` to bring them into the current project.
</attached-skill>

## Assistant

_[tool: update_todos]_

The brief is extraordinarily detailed — you've already specified the exact palette, motion budget, type scale, and the eight artifacts you want. I'll skip questions and execute. Plan:

1. Build a design canvas with all 8 artifacts as side-by-side artboards (so you can compare and focus any one fullscreen).
2. Define a strict token set first (palette, type, status colors, motion) — then every artifact pulls from it.
3. Include per-artifact rationale + pixel decisions inline so you can read the reasoning alongside the design.

_[tool: copy_starter_component]_

_[tool: list_files]_

_[tool: read_file]_

_[tool: write_file]_

Now I'll build the GUPPI tokens, shared components, and then each artboard. Starting with the foundation.

_[tool: write_file]_

_[tool: write_file]_

Now the canvas density variants and zoom-transition artboards.

_[tool: write_file]_

_[tool: write_file]_

_[tool: write_file]_

_[tool: write_file]_

_[tool: grep]_

_[tool: write_file]_

Now the foundations artboard and the main HTML that wires it all together.

_[tool: write_file]_

_[tool: show_html]_

_[tool: show_html]_

_[tool: get_webview_logs]_

_[tool: str_replace_edit]_

_[tool: show_html]_

_[tool: get_webview_logs]_

_[tool: save_screenshot]_

Clean. Let me take a quick screenshot to verify the layout looks right.

_[tool: show_html]_

_[tool: update_todos]_

_[tool: done]_

_[tool: fork_verifier_agent]_

GUPPI is on the canvas — eight sections, side-by-side artboards, each design paired with a rationale plate (the why + exact pixels).

**What's there**
- **Foundations** — palette, type, 4px scale, status states, DDD edge geometry on one card
- **The canvas** at 1, 3, and 10 projects so you can read how density scales
- **Project frame** as the atomic unit — header bar, 8 BCs, all four edge types in one frame
- **Blocked moment** — tethered question callout, pulsing in sync with the ledger BC, with a secondary blocked notice elsewhere
- **Voice indicator** in idle, listening (with transcript), speaking/TTS, and muted
- **Project detail panel** — vision.md rendered for long-form reading (14px Inter, 1.65 line-height, max-width 75ch)
- **Terminal panel** — orchestrator + uploader + thumb-gen (blocked, with quick-answer chips)
- **Zoom transition** — overview at 38% vs focused at 110%
- **Command palette** — voice/keyboard share vocabulary; every row shows the spoken form

**Drag-to-explore**: grip any artboard label to reorder, kebab-menu to delete, click the expand arrow to fullscreen any single view. The rationale plates are themselves artboards — close any you don't want.

**Tokens** live in `guppi-tokens.css` if you want to rip a value. Status colors and the running pulse are the only chromatic motion in the system; everything else holds still.

Next steps when you want them: clickable prototype (zoom gesture, palette filtering), new-project flow, right-click context menus, or a light-theme variant.

## User

Verifier agent check completed

## Assistant

_[tool: str_replace_edit]_

_[tool: done]_

Fixed — added `box-sizing: border-box` to the `.guppi` root so the foundations card no longer overflows its 1240×720 frame. Hex codes and the DDD edges column now render in full.

## User

Please use my business colours as highlight colours.:
#25abfe and #ff8b00

Also the color for idle should be green, the color for blocked should be red and the color for missing should be grey.

There should also be an optional light mode.

## Assistant

_[tool: grep]_

Let me grep for hardcoded color references so I update everything consistently.

_[tool: grep]_

_[tool: write_file]_

Now I'll update tokens for the new accent colors, status colors, and add light mode support.

_[tool: str_replace_edit]_

_[tool: str_replace_edit]_

_[tool: str_replace_edit]_

_[tool: grep]_

_[tool: str_replace_edit]_

_[tool: str_replace_edit]_

_[tool: str_replace_edit]_

Now add the theme toggle.

_[tool: read_file]_

Let me make the foundations swatches reactive to the active theme so the displayed values stay accurate when toggling.

_[tool: str_replace_edit]_

_[tool: str_replace_edit]_

_[tool: grep]_

_[tool: grep]_

_[tool: str_replace_edit]_

_[tool: done]_

_[tool: fork_verifier_agent]_

Done. The system now uses your business colors:

**Accents** (replacing the old periwinkle + teal):
- **#ff8b00** — warm anchor, project frame borders, focus ring, active palette rail, blockquote rule, TTS speaking ring
- **#25abfe** — cool secondary, BC bubble borders, the running pulse

**Status colors** (per your request):
- idle → green
- running → blue (#25abfe)
- blocked → red
- missing → grey

**Light mode** — toggle pill top-right of the viewport, persists in localStorage. Every artboard (canvas density, project frame, blocked moment, voice indicator, detail panel, terminal, zoom, command palette) flips together. The foundations swatches read live CSS variables so the displayed hex values stay accurate as you toggle. The design canvas backdrop (paper) stays paper-colored — it's presentation chrome, not part of GUPPI's surface.

The rationale plates kept their paper color in both modes — they're notes about the design, not the design itself.

## User

Verifier agent check completed

## Assistant

All clear from the verifier — both themes render correctly, accent colors and status semantics match your spec.

## User

Change the status colors again. Idle should be gray. Missing should be orange.

## Assistant

_[tool: str_replace_edit]_

_[tool: done]_

Updated — idle is now grey (○), missing is now orange (✕). Running stays blue, blocked stays red. Both themes updated. The foundations card swatches read live tokens, so they'll show the new hex values automatically.

