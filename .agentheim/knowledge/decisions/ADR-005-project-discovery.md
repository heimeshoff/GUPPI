---
id: ADR-005
title: Project discovery — explicit registry first, with a manual scan command
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-005-project-discovery]
related_adrs: [ADR-001, ADR-004]
---

# ADR-005: Project discovery — explicit registry first, with a manual scan command

**Status:** Accepted
**Scope:** global

## Context

GUPPI needs to know which folders on disk are Agentheim projects — i.e. which
folders contain a `.agentheim/` directory. This is the set of tiles GUPPI
renders on its canvas. The question is *how* GUPPI learns about those folders.

Three options were on the table:

1. **Scan disk for `.agentheim/` directories.** GUPPI walks the filesystem
   (or a configured set of roots) on startup and surfaces everything it finds.
   This is slow on large trees, intrusive, and may surface projects the user
   does not want on their canvas — experiments, archived work, vendored copies.
2. **Explicit registry.** GUPPI only knows about folders the user added
   deliberately. Predictable and fast, but discovering a newly-cloned project
   requires a manual step.
3. **Hybrid.** Explicit registry as the source of truth, with an *on-demand*
   scan offered as a convenience — never run unprompted.

The runtime is **Tauri 2** with a Rust core (ADR-001); project discovery is a
Rust-core responsibility reached over IPC. Persistence is **SQLite** in the OS
user-config directory (ADR-004), and the `projects` table is already part of
that schema sketch — so a registry has a home with no new storage decision
required. This ADR builds directly on ADR-004.

## Decision

Use **option 3: explicit registry primary, with a user-triggered scan
command.** No unprompted disk-walking.

The registry *is* the `projects` table from ADR-004
(`projects(id, path, nickname, added_at, last_seen_at)`). The Rust core exposes
three operations over IPC:

1. **"Add project…"** — opens a native folder picker; validates that the chosen
   folder contains a `.agentheim/` directory; on success, inserts a row into
   `projects`. Rejects folders that are not Agentheim projects with a clear
   message.
2. **"Scan folder for projects…"** — opens a folder picker, then walks that
   folder's subtree looking for `.agentheim/` subdirectories. Walk depth is
   user-configurable with a **default of 3 levels**. Results are presented as a
   checklist; the user picks which to import. The scan is *never* run
   unprompted — it is always an explicit user action against an explicit root.
3. **"Remove project"** — drops the project from the registry. The associated
   tile state (position, cluster membership) is **retained for 30 days** so an
   accidental removal followed by a re-add restores the tile in place; after 30
   days the orphaned tile state is garbage-collected.

Projects are tracked by **canonical absolute path**, normalised on entry
(resolved, symlinks collapsed, case-normalised on Windows). This makes the path
the natural unique key and prevents the same project being registered twice
under two spellings.

On startup, GUPPI checks each registered path. If a path no longer exists
(folder deleted, drive unmounted, network share offline), the tile is shown in
a **"missing" state** — it is *not* auto-removed. The user decides whether to
remove it or whether the path will come back.

## Consequences

- (+) **Predictable, fast startup.** GUPPI reads the registry and renders;
  there is no filesystem walk on the hot path and no surprise tiles.
- (+) **Respects user agency.** The canvas only ever shows what the user
  deliberately put there. This matches GUPPI's guiding principle that the user
  is in charge of their own view of the world (ADR-004).
- (+) **No new storage decision.** The registry is the existing `projects`
  table; this ADR consumes ADR-004 rather than extending it.
- (+) **Convenience without intrusion.** The scan command covers the
  "I just cloned five repos" case without GUPPI ever walking disk on its own.
- (+) **Resilient to moved/missing folders.** The "missing" state degrades
  gracefully instead of silently dropping projects.
- (–) **Discovering newly-created or newly-cloned projects requires a manual
  action** ("Add project…" or "Scan folder…"). Accepted: GUPPI is a personal
  tool for a single user on a single machine, and the manual step is cheap.
- (–) Tile-state retention introduces a small garbage-collection
  responsibility (the 30-day sweep) on the persistence layer.

## Downstream

The user-facing affordances — **"Add project…"**, **"Scan folder for
projects…"**, and **"Remove project"** — are UI concerns that belong to the
**canvas bounded context**. They are *not* modelled here. When modeling work
begins in the canvas BC, these three affordances (folder pickers, the scan
result checklist, the "missing" tile state, and the command-palette entries
for each) must be captured as canvas BC tasks. This ADR defines only the
discovery *model* and the Rust-core IPC surface; it deliberately does not reach
across the BC boundary to create those tasks.

## Reversibility

High. The decision is additive over ADR-004 and touches no other ADR. If
automatic discovery ever feels missing, the scan walker already exists — making
it run automatically on startup (against a configured set of roots) is a small,
contained change. Conversely, dropping the scan command and keeping only
explicit "Add project…" is equally trivial.

## Reconciliation

**2026-05-15 — superseded-in-part by ADR-013 (scan roots).** ADR-005's "Scan
folder for projects…" was a *one-shot* command: the picked folder was walked
once and never remembered. ADR-013 evolves this into **persisted, rescannable
scan roots** with origin tracking and cascade-deregister. ADR-005 still holds
for: the explicit-registry-primary stance, "Add project…", canonical-path
tracking, the "missing" tile state, and the BC placement of user-facing
affordances in the canvas BC.

One clarification ADR-013 depends on: ADR-005's **30-day tile-state retention**
rule applies specifically to the **user-initiated single "Remove project"**
affordance — an undo window for an accidental click. It does **not** apply to
ADR-013's scan-root cascade-deregister, which is a deliberate bulk discard and
hard-deletes the child projects' tile state.
