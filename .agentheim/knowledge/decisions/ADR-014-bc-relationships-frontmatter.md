---
id: ADR-014
title: BC relationships live in per-BC README YAML frontmatter
status: Accepted
scope: bc
bc: project-registry
date: 2026-05-16
related_tasks:
  - project-registry-004-bc-relationships-and-positions
  - canvas-007-project-as-frame
related_adrs: [ADR-004, ADR-008, ADR-009]
---

# ADR-014: BC relationships live in per-BC README YAML frontmatter

**Status:** Accepted
**Scope:** bc (project-registry)

## Context

`canvas-007-project-as-frame` reshapes the canvas so each project renders as a
frame containing its bounded contexts as bubbles, with edges between BCs drawn
by relationship type (customer-supplier, shared-kernel, anticorruption-layer,
…). For the canvas to draw these edges, the registry needs a place to put the
data — and a way to keep it current as Marco edits Agentheim files in the
target project's repo.

Three candidate locations were considered:

1. **Per-BC `README.md` YAML frontmatter** (chosen). Each BC's README is where
   the BC explains itself; relationships are part of self-description. The
   `relationships:` block sits at the very top of the file, above the prose.
2. **A separate top-level `relationships.yaml` (or `context-map.yaml`).**
   One file per project listing every BC↔BC edge.
3. **Parse `context-map.md` directly.** The existing human-readable narrative
   in `.agentheim/context-map.md` already lists the relationships in prose
   form; lift them from there.

## Decision

**Per-BC README YAML frontmatter.**

The shape:

```yaml
---
name: canvas
classification: core
relationships:
  - to: project-registry
    type: customer-supplier
    direction: upstream
  - to: infrastructure
    type: shared-kernel
---

# canvas
...
```

- `to` is the bare directory name of a sibling BC inside the **same** project.
  Cross-project references are dropped at parse time with a warning log line —
  the canvas only renders intra-project edges at v1.
- `type` is one of `customer-supplier`, `shared-kernel`, `partnership`,
  `anticorruption-layer`, `conformist` — the DDD strategic palette.
- `direction` is `upstream` or `downstream`, and is **required** for
  directional types (`customer-supplier`, `anticorruption-layer`, `conformist`)
  and **absent** for non-directional types (`shared-kernel`, `partnership`).
  The parser drops mismatched entries with a warning rather than silently
  accept a half-broken shape.

Each BC declares its **outgoing** relationships. Brainstorm/model will
eventually keep both sides in sync (Agentheim follow-up); the parser does not
enforce symmetry at v1 — one side declaring an edge is enough for the canvas
to draw it.

The implementation lives in `src-tauri/src/project.rs::parse_relationships`,
called by `read_bounded_contexts` at every `get_project`. A change-detection
cache in `src-tauri/src/watcher.rs::AgentheimWatcher` fires the ADR-009
`BcRelationshipsChanged` event **only** when the parsed relationships set
actually differs from the previous parse (deep-equal compare) — prose-only
edits don't trigger relayouts.

## Why not the alternatives

**A separate `relationships.yaml`** puts the truth one hop away from the prose
that explains it. A BC's README discusses its upstream and downstream
dependencies in its prose; co-locating the structured form keeps those two
descriptions next to each other and makes it harder for them to drift. It
also keeps "the BC explains itself, end-to-end" as the invariant — no extra
file to remember.

**Parsing `context-map.md`** was rejected as fragile. The context-map narrative
is human-readable prose ("**DDD label: customer-supplier**, with
`project-registry` upstream as supplier"); turning prose into structured edges
needs either a more constrained markdown sub-grammar or NLP. YAML in
frontmatter is the lowest-friction structured form that survives a `git diff`,
and parsers exist for it in every language Marco might touch (`serde_yaml`
here; `js-yaml` on the frontend; the Agentheim Python plugin already speaks
YAML for task frontmatter).

## Consequences

**Positive:**
- BC self-description is single-file. Reading one README tells you what the
  BC does and how it relates to its siblings.
- The change-detection cache + `BcRelationshipsChanged` event keep the canvas
  cheap: prose edits don't relayout.
- Graceful degradation: malformed frontmatter is dropped with a warning; the
  BC still enumerates with `relationships: []` so the canvas keeps drawing
  the bubble.
- Brainstorm/model can keep `context-map.md` and the per-BC frontmatter in
  sync going forward — they have the same source of truth (the prose body of
  the README) at hand.

**Negative:**
- Two places to look for relationships in any given BC: the prose body and
  the frontmatter. Brainstorm/model is responsible for not letting them
  drift.
- Hand-curation of the bootstrap is one-time work per project. Future
  projects' relationships come from brainstorm/model; GUPPI's seven READMEs
  were translated from `context-map.md` directly in `project-registry-004`.

## Out of scope (v1)

- **Cross-project relationships.** v1 only draws edges inside one project
  frame. The parser drops `to` references that don't name a sibling BC
  (e.g. `voice → Whisperheim` ACL edges) with a warning. Reconsider at v2+
  when the canvas knows how to position cross-project edges.
- **Voice ACL relationships** to external services (`Whisperheim`,
  `Utterheim`). These are real DDD relationships in the system but are
  cross-project / external-service edges, out of scope for v1 intra-project
  rendering.
- **Auto-generation of the frontmatter** from the prose body. That belongs in
  the Agentheim plugin's brainstorm/model skills; captured separately.
