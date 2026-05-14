# canvas — INDEX

Per-BC catalog. See `README.md` for purpose, classification, ubiquitous language.

## ADRs

<!-- adr-list:start -->
*(None yet.)*
<!-- adr-list:end -->

## Backlog

<!-- backlog-list:start -->
*(None.)*
<!-- backlog-list:end -->

## Todo

<!-- todo-list:start -->
*(None.)*
<!-- todo-list:end -->

## Doing

<!-- doing-list:start -->
*(None.)*
<!-- doing-list:end -->

## Done

<!-- done-list:start -->
- [canvas-001-targeted-canvas-updates](done/canvas-001-targeted-canvas-updates.md) — `type: feature`. Canvas patches its `ProjectSnapshot` in place from the fine-grained FS events (`task_moved`/`added`/`removed`, `bc_appeared`/`disappeared`) instead of re-fetching; coarse `AgentheimChanged` retired → renamed `ResyncRequired` (lag-only resync signal). ADR-009 amended in place. Commit `5fa7080`.
<!-- done-list:end -->

## Research

<!-- research-list:start -->
*(None yet.)*
<!-- research-list:end -->
