# Context map

## Contexts

### canvas
- **Purpose:** The Miro-like infinite surface that is guppi's primary view. Renders every AgentHeim project as a tile, with its bounded contexts as connected child nodes; supports pan, zoom, drag-to-reposition, click/keyboard focus-zoom, status badges, and the project-detail view (rendered markdown for `vision.md`, `research/*.md`, ADRs, BC READMEs). This is guppi's reason to exist as a UI; the v1 MVP is canvas-only.
- **Core language:** canvas, tile, node, connection, focus, zoom level, viewport, layout, status badge, detail view, markdown pane.
- **Classification:** **core** — guppi exists to provide this ambient overview surface. The "single Miro-like canvas" is the load-bearing differentiator vs. juggling terminals.
- **Key actors:** Marco (single user). Reads from `project-registry` (what tiles to draw), `agent-awareness` (what badges/indicators to show), and `voice` (focus commands like "zoom to image-gallery").

### project-registry
- **Purpose:** Discovers, lists, and creates AgentHeim projects on disk. Watches the filesystem for projects (folders containing `.agentheim/`), enumerates their bounded contexts (`contexts/<name>/`), counts tasks per state (backlog / doing / done) per BC, and owns the new-project flow (create folder, `git init`, invoke the `brainstorm` skill inside it). This is the canvas's data source for "what exists".
- **Core language:** project, AgentHeim project, bounded context (BC), task, task state (backlog/todo/doing/done), discovery, new-project flow, vision file, contexts directory.
- **Classification:** **supporting** — necessary scaffolding, but the differentiator is what guppi *does* with the discovered projects, not the discovery itself. The AgentHeim shape (vision → BCs → tasks) is project-specific enough that this isn't off-the-shelf generic, but it's not what makes guppi guppi.
- **Key actors:** Marco (when creating a new project). The filesystem (read-only watcher for v1; mutation only on new-project creation).

### claude-runner
- **Purpose:** Spawns and owns `claude` processes — always *inside the target project's folder*, never in guppi's own folder. This is the load-bearing architectural rule from the vision: the project's own CLAUDE.md, skills, hooks, and settings must apply. Owns PTY/stdio, the emulated terminal panel (shallow view of orchestrator + sub-agent comms), session lifecycle, and routing input to the right session. Guppi is the *primary* launcher, not the *exclusive* one — bare-terminal sessions exist and are accepted as observed-only.
- **Core language:** session, guppi-owned session, spawn, PTY, stdio, target project folder, terminal panel, session routing, orchestrator stream, sub-agent stream.
- **Classification:** **core** — the cross-project, in-folder-spawning model is what makes guppi an AgentHeim-aware controller rather than yet another terminal multiplexer. The rule "guppi never hosts claude in its own folder" is core architecture, not plumbing.
- **Key actors:** Marco (launches sessions, types into them). The `claude` CLI (external process, treated as an integration). The target project's filesystem (cwd of spawned process).

### agent-awareness
- **Purpose:** Aggregates "what's running, what's idle, what's blocked-on-question" across all observed sessions. Combines two sources: rich signals from `claude-runner` for guppi-owned sessions (orchestrator stream, prompt detection) and best-effort filesystem signals (tasks in `doing/`, hook outputs, mtime patterns) for sessions guppi didn't spawn. Surfaces per-BC indicators (blinking dot, question text rendered at the BC's location on the canvas).
- **Core language:** tile state, BC state, running, idle, blocked-on-question, observed session, owned session, indicator, question-at-location, signal source.
- **Classification:** **core** — the "live indicators of what's running and what needs attention" is one of guppi's two stated differentiators (the other being the canvas itself). The unified-state model that survives the runner-vs-filesystem source split is guppi-specific intelligence.
- **Key actors:** No direct human actor — it's a derivation layer. Reads from `claude-runner` and from the filesystem (via `project-registry`'s watch or its own observers — to be decided in foundation).

### voice
- **Purpose:** The bridge to the user's voice channel — both directions. Inbound: wake-word "Guppy", Whisperheim integration for STT (Silero VAD + Parakeet), fuzzy command parsing to map utterances to actions ("refine task 104 in image-gallery"). Outbound: Utterheim integration for TTS narration (notifications, "agent is asking you something"). The vision treats voice as a first-class modality and a differentiator — the pain "can't talk to my computer most of the time" is what voice resolves.
- **Core language:** wake word, utterance, transcript, transcriber (Whisperheim), narrator (Utterheim), command intent, fuzzy match, voice focus command.
- **Classification:** **core** — ambient voice control is one of guppi's headline value propositions. Even though Whisperheim and Utterheim are external (and treated as upstream services via ACL), the wake-word loop, the intent mapping, and the "voice as primary input" stance are guppi-specific.
- **Key actors:** Marco (speaks, is spoken to). Whisperheim (upstream STT service). Utterheim (upstream TTS service).

### infrastructure
- **Purpose:** Standing home for globally-true tech concerns — runtime, IPC, persistence of canvas state, packaging, file watching plumbing, event/pub-sub transport between contexts, OS integration (PTY, microphone access). Created unconditionally by the brainstorm skill's foundation pass; BC-local infra stays in the originating BC.
- **Core language:** runtime, process, IPC channel, event bus, state directory, persistence, package, watcher, transport.
- **Classification:** **generic** — solved-problem plumbing. None of these are guppi's differentiator; the choice of stack is significant (foundation decision) but the concerns themselves are off-the-shelf.
- **Key actors:** No domain actor. Operationally owned by Marco (single developer / single user).

## Relationships

### canvas ← project-registry (customer-supplier; canvas is downstream)
The canvas asks "what projects exist and what's in them?" and `project-registry` answers. The registry publishes a stable model of projects, BCs, and task counts; the canvas conforms to that shape to render tiles and nodes. **DDD label: customer-supplier**, with `project-registry` upstream as supplier.

### canvas ← agent-awareness (customer-supplier; canvas is downstream)
The canvas reads tile state (running / idle / blocked) and the current question text from `agent-awareness` to render badges and the question-at-BC-location overlay. The canvas does not derive state itself. **DDD label: customer-supplier**, `agent-awareness` upstream.

### agent-awareness ← claude-runner (customer-supplier; agent-awareness is downstream)
For guppi-owned sessions, `claude-runner` is the rich source of truth — orchestrator stream, prompt-waiting detection, session lifecycle events. `agent-awareness` subscribes to these signals and lifts them into its unified state model. **DDD label: customer-supplier with event publication** — `claude-runner` publishes session events; `agent-awareness` is one subscriber. (Other subscribers later: the terminal panel inside canvas.)

### agent-awareness ← project-registry / filesystem (conformist)
For bare-terminal sessions guppi didn't spawn, `agent-awareness` reads filesystem state directly (tasks in `doing/`, hook output files, mtimes). It has no influence on what AgentHeim writes — it conforms to whatever shape the AgentHeim plugin produces. **DDD label: conformist** to the AgentHeim-on-disk shape. If that shape ever shifts in a way that breaks observation, the fix lives here, not in AgentHeim.

### claude-runner → target project filesystem (the load-bearing rule)
Not a relationship between guppi contexts, but the most important external relationship in the system: `claude-runner` always spawns `claude` with cwd set to the *target project's folder*. The runner has no business logic of its own that depends on the project's contents — it's a faithful pass-through. This rule is enumerated in the vision's non-goals ("Not a host for `claude`").

### voice → Whisperheim, voice → Utterheim (anticorruption layer, both directions)
Whisperheim (STT) and Utterheim (TTS) are external services with their own vocabularies and lifecycles. The `voice` context wraps both behind an ACL so that the rest of guppi speaks in guppi terms (`utterance`, `command intent`, `narration`) and never directly in Whisperheim/Utterheim terms. This insulates guppi from changes in either service and keeps the option open to swap implementations. **DDD label: anticorruption layer.**

### voice → canvas, voice → project-registry, voice → claude-runner (publisher/subscriber, v2+)
v1 is read-only; voice has no command path to execute. Beyond v1, recognized command intents from `voice` will fan out to the appropriate executor context (focus commands → canvas; new-project → project-registry; "refine task X" → claude-runner). This is intentionally left unspecified at v1 — see Open questions. **No DDD label yet** — the routing layer between intent and executor is a candidate for its own context (`command-router`) if the variety grows.

### everything → infrastructure (open host / shared kernel)
The infrastructure BC publishes a small set of cross-cutting services (event bus, state persistence API, file-watcher protocol, packaging conventions). Every other BC consumes them. **DDD label: open host service**, with the published language being the IPC/event contracts decided in the foundation pass. Some thin slice may end up as a **shared kernel** (e.g., the `Project`/`BoundedContext` value types if they're identical across registry, canvas, and awareness) — that's a foundation decision, not a strategic one.

## Notes on what's intentionally *not* a separate BC

- **document-viewer** (candidate #6) folded into `canvas`. Rendering markdown in the detail pane has no distinct language, no distinct actor, and no distinct rate of change from the canvas itself. Markdown rendering is a generic library concern that lives inside the canvas BC's adapter layer.
- **command-router** considered and deferred. With v1 read-only and command volume low at v2, routing intent-to-executor can live inside `voice` (for voice-originated commands) and inside `canvas` (for mouse/keyboard commands). If the command surface grows or a third input modality appears, revisit and split.
- **event-bus / notification** considered and folded into `infrastructure`. Pub/sub between BCs is a transport concern, not a domain concern.

## Open questions (strategic)

- **Wake-word listener location** — in-process inside `voice`, or extend Whisperheim to expose a subscription interface? Affects whether `voice` owns audio capture or delegates it. Resolution: foundation pass.
- **Filesystem watching ownership** — `project-registry` watches for project discovery; `agent-awareness` watches for session state signals. Are these one watcher with two consumers, or two independent watchers? Foundation pass — likely the event-bus in `infrastructure` mediates a single watcher.
- **Command routing at v2** — does a dedicated `command-router` BC emerge once voice + keyboard + mouse all need to issue the same actions, or does each input-side BC route directly? Revisit when v2 features land.
- **Terminal panel ownership** — the emulated terminal panel renders inside the canvas but is fed by `claude-runner`. Is the rendering component part of `canvas` (a tile-detail mode) or part of `claude-runner` (a view over its sessions)? Lean: canvas owns rendering, runner owns the stream. Confirm during walking-skeleton.
