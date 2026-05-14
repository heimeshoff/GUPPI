---
id: ADR-013
title: Scan roots — persisted, rescannable project-discovery folders
status: Accepted
scope: bc
bc: project-registry
date: 2026-05-15
related_tasks:
  - project-registry-002a-scan-roots-and-walk
  - project-registry-002b-import-and-cascade-deregister
related_adrs: [ADR-004, ADR-005, ADR-008]
---

# ADR-013: Scan roots — persisted, rescannable project-discovery folders

**Status:** Accepted
**Scope:** bc (project-registry)

## Context

ADR-005 settled project discovery as an explicit registry with a *one-shot*
"Scan folder for projects…" command — pick a folder, walk it once, present a
checklist. The picked folder was never remembered. In practice the user keeps
their projects under a small, stable set of parent folders (`C:\src`, …) and
re-clones / re-creates projects there continually. Re-picking the same folder
every time is friction ADR-005 explicitly accepted but did not have to.

This ADR introduces **scan roots**: folders the user registers *once*, that
GUPPI can re-walk on demand.

Alternatives considered:

1. **Stay one-shot (ADR-005 as-is)** — zero new model, but the user re-picks
   the same folders forever.
2. **Persisted scan roots, no origin tracking** — remember the roots, but a
   discovered project is just a `projects` row like any other. Simple, but
   "remove this root and everything it brought in" is impossible — the link is
   lost.
3. **Persisted scan roots with origin tracking + app-driven cascade-
   deregister** — each discovered project records its originating root;
   removing a root removes its imports.

## Decision

Adopt **option 3**.

- A `scan_roots` table persists each registered root with a per-root
  `depth_cap` (default 3, from ADR-005).
- `projects` gains a nullable `scan_root_id` FK. NULL = manually added
  (ADR-005 "Add project…"); non-NULL = discovered under that root.
- The FK is `ON DELETE RESTRICT`. The cascade-deregister **cannot** be
  DB-level: each project's filesystem watcher (`WatcherSupervisor::remove`,
  ADR-008) must be torn down in application code, which SQLite cannot do.
  `RESTRICT` therefore turns the app-driven ordering (remove watchers + drop
  child rows *before* the root row) into a checked invariant rather than a
  convention.
- Adding or rescanning a root walks the subtree (depth-capped, junk-dir-
  pruned, paths canonicalised per ADR-005) and returns a **checklist** of
  candidates — faithful to ADR-005's "user picks which to import." A rescan
  re-presents anything not yet imported.
- Cascade-deregister on root removal **hard-deletes** child projects and their
  tile state. ADR-005's 30-day tile-state retention is scoped to the
  user-initiated single "Remove project" affordance (an undo window); a
  scan-root removal is a deliberate bulk discard and does not retain.

## Consequences

- (+) The user registers their project parent folders once; rediscovering new
  clones is one "rescan" click.
- (+) "Stop tracking everything under this folder" is a single coherent
  operation.
- (+) Origin tracking keeps manually-added projects (NULL origin) immune to
  any root's cascade.
- (–) Extends ADR-004's schema (new table + column) and ADR-005's discovery
  model. ADR-005 carries a reconciliation note.
- (–) The app must drive the cascade in the correct order; `ON DELETE
  RESTRICT` guards it but a buggy command will hard-error rather than silently
  corrupt — acceptable, and the integration tests cover the happy path.
- (–) Two removal semantics now coexist (retained single-remove vs.
  hard-delete bulk cascade); this is deliberate and documented.

## Reversibility

Moderate. The schema additions are additive over ADR-004. Dropping scan roots
and reverting to ADR-005's one-shot scan means dropping the table, the column,
and four IPC commands — contained, but the `scan.rs` walker would be kept (the
one-shot scan reuses it).
