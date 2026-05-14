---
id: canvas-005-project-discovery-affordances
type: feature
status: backlog
scope: bc
depends_on:
  - project-registry-002b-import-and-cascade-deregister
  - design-system-001-styleguide
related_adrs:
  - ADR-005
  - ADR-013
related_research: []
prior_art: []
---

# Project discovery affordances — add/scan folder, checklist import, remove

## Why

ADR-005 is explicit that the user-facing discovery affordances live in the
**canvas BC**, not project-registry. `project-registry-002a/002b` ship the
backend scan/import/cascade capability and its IPC surface, but nothing the
user can actually click. v1's "real way to get projects in" is not complete
until the canvas exposes it.

## What

The canvas-BC UI for ADR-005's three affordances, invoking the IPC commands
delivered by `project-registry-002a/002b`:

- **"Add project…"** — native folder picker; validates `.agentheim/` presence;
  registers the single folder (ADR-005, manually-added → NULL `scan_root_id`).
- **"Scan folder for projects…"** — native folder picker → `add_scan_root` →
  the **discovery checklist modal** (multi-select list of `ScanCandidate`s,
  with already-imported candidates greyed/pre-marked) → `import_scanned_projects`
  with the user's picks. Plus a rescan affordance on an existing scan root.
- **"Remove project"** and **remove scan root** — the removal affordances
  (`remove_scan_root` drives the cascade-deregister).
- The ADR-005 **"missing" tile state** — a registered project whose path no
  longer exists on disk renders as "missing" rather than being dropped.

## Acceptance criteria

*(To be refined — this is a captured stub. Refinement should pin down the
checklist modal interaction, where the affordances live in the canvas chrome,
keyboard paths, and the "missing" tile treatment against `STYLEGUIDE.md`.)*

## Notes

Surfaced during the refinement of `project-registry-002` (2026-05-15), which
split the backend into `002a` (schema + walk) and `002b` (import + cascade) and
deferred the UI here per the ADR-005 BC seam.

Frontend gate: built against `contexts/design-system/STYLEGUIDE.md` — hence the
`design-system-001` dependency.

`canvas-002`'s auto-placement criterion already handles "a newly registered
project appears as a tile" (it reacts to the `ProjectAdded` domain event from
`WatcherSupervisor::add`). This task only needs the *trigger* + the *checklist*,
not tile placement.

Under-refined — needs a REFINE pass before promotion to `todo/`.
