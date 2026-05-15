---
id: canvas-005b-scan-flow-and-scan-root-management
type: feature
status: done
completed: 2026-05-15
commit: 4979e27
scope: bc
depends_on:
  - canvas-006-live-add-race-on-concurrent-project-added
  - project-registry-003-manual-add-remove-and-missing-projects
  - canvas-005a-single-shot-discovery-affordances
  - design-system-001-styleguide
related_adrs:
  - ADR-003
  - ADR-005
  - ADR-013
related_research: []
prior_art:
  - canvas-002-render-multiple-project-tiles
---

# Scan flow + scan-root management — the multi-step discovery surface

## Why

ADR-005 + ADR-013's scan-root model is fully implemented backend-side
(`project-registry-002a/002b`), but no user-facing UI exists. This task ships
the **scan-folder flow** (one-shot: pick a folder → walk it → checklist modal
→ import picks) and the **scan-root management surface** (list, rescan,
cascade-remove with confirmation). `canvas-005a` separately ships the
single-shot Add / Remove / Missing affordances and the right-click context-
menu shell this task extends.

The hard `depends_on` on `canvas-006` is **structural**:
`import_scanned_projects` fires N back-to-back `ProjectAdded` events, which
the unserialised live-add path mis-handles — every imported project collides
at the same spiral slot and N-1 are lost from the in-memory array.
canvas-005b's import flow would immediately re-demonstrate that bug;
canvas-006's serialisation fix removes it. `project-registry-003` is
depended on for the new `project_removed` event that `remove_scan_root`'s
cascade now emits — canvas-005b's UI must drop tiles cleanly when the cascade
fires (the handler itself lives in canvas-005a; this task uses it). The soft
ordering hint: canvas-005a's menu shell is the seam this task extends;
working canvas-005a first reduces merge friction but is not strictly required
(`depends_on` declared for clarity of intent).

## What

### Two new entries in the empty-canvas right-click menu

The menu shell is established by `canvas-005a`. This task appends:

- **"Scan folder for projects…"** — always shown.
- **"Manage scan roots…"** — shown **only when `list_scan_roots()` returns
  a non-empty list**. The modal never appears in an empty state; instead, the
  menu item is hidden. Refresh the visibility on mount, after every
  successful `add_scan_root`, and after every successful `remove_scan_root`.

### Discovery checklist modal

A new modal pattern in the codebase. Centered overlay (screen-space, ADR-003)
above a dimmed backdrop. Three entry points all open this same modal:
- "Scan folder for projects…" → after `add_scan_root` resolves.
- "Rescan" button on a scan-root row in the management modal → after
  `rescan_scan_root` resolves.
- (Future: voice command; out of scope here.)

**Shape:**
- **Header:** scan-root path (mono font, truncate from the left if too long) +
  candidate count, e.g. `"C:\src — 6 projects found"`. If the path is the
  result of a *rescan*, append `" (rescan)"` so the user knows.
- **Body:** scrollable list of `ScanCandidate` rows. Each row:
  - Checkbox on the left.
  - Path (mono font, `--guppi-bc-text`).
  - Nickname suggestion (regular font, `--guppi-bc-text-muted`, on the line
    below the path).
  - **For `already_imported: true`:** row body at 60% opacity; checkbox
    **pre-checked AND disabled** (cannot toggle); a small "imported" badge
    after the path, styled as a `statusIdle` pill (`#6f7585` background,
    `statusText` foreground, `radiusBadge` 6px). The badge answers the
    "wait, where did I already import that?" question.
  - **For `already_imported: false`:** row togglable; checkbox unchecked by
    default; row hover highlights with `--guppi-canvas-bg-raised`.
- **Header controls:** "Select all" / "Select none" — operate only on the
  togglable rows (already-imported are immune). If there are zero togglable
  rows, hide these.
- **Footer:** "Import selected" primary button + "Cancel" secondary.
  Primary is **disabled when zero new candidates are ticked** (the
  already-imported pre-ticks do not count). Clicking it invokes
  `import_scanned_projects(scan_root_id, picked_paths_of_new_candidates)`
  and closes the modal on success. Tiles arrive via N `ProjectAdded` events;
  canvas-006's serialised live-add gives them distinct spiral slots.
- **Empty-candidates case** (`add_scan_root` returned an empty `candidates`
  array — e.g. an empty folder, or every candidate was already imported and
  the user filtered them mentally): the modal still opens. Header reads
  `"<path> — no Agentheim projects found"`. Body shows a friendly empty
  state ("Nothing to import. Re-running the scan later will pick up new
  clones."). Footer has a single **"OK"** button. The scan root **is**
  persisted regardless (002a persists the root FIRST before walking) — a
  future "Manage scan roots…" → "Rescan" will pick up new candidates.

**Dismissal:** "Cancel" / "OK" button, click on the backdrop, `Escape`.

**Modal styling contract** (new pattern, inlined; same tokens as the
context menu from canvas-005a):
- **Backdrop:** `var(--guppi-canvas-bg)` (`#16161c`) at 70% opacity over the
  canvas; covers the full viewport; click-to-dismiss.
- **Modal body:** centered, max-width 720px, max-height 80% of viewport.
  Background `var(--guppi-tile-fill)`, border 2px `var(--guppi-tile-border)`,
  radius `var(--guppi-radius-tile)` (12px). Drop shadow optional and not
  required.
- **Header / body / footer padding:** `var(--guppi-spacing-lg)` (16px).
- **Row padding:** `var(--guppi-spacing-sm)` (8px) vertical,
  `var(--guppi-spacing-md)` (12px) horizontal.
- **Primary button:** `var(--guppi-tile-border)` background, `statusText`
  foreground, `radiusBadge`. Disabled state: `var(--guppi-canvas-bg-raised)`
  background, `var(--guppi-tile-text-muted)` foreground, `cursor: not-allowed`.
- **Secondary button:** transparent background, 1px
  `var(--guppi-tile-border)` border, `var(--guppi-tile-text)` foreground.

### Scan-roots management modal

Same modal pattern. Centered overlay; dimmed backdrop; same styling tokens.

- **Header:** "Scan roots".
- **Body:** scrollable list. One row per scan root from `list_scan_roots()`.
  Each row:
  - Path (mono).
  - Child-project count (live rows only — soft-deleted rows already excluded
    by project-registry-003's `deleted_at` filter).
  - **"Rescan"** button (secondary style) → invokes `rescan_scan_root(id)` →
    opens the checklist modal pre-populated with results (same modal
    component instance is fine; the rescan flag in the header
    differentiates).
  - **"Remove"** button (secondary style, but with the `statusMissing` border
    colour so the destructive action reads as such) → opens a
    cascade-confirmation dialog (next section).
- **Footer:** "Close" secondary button.
- **Empty state:** does not apply — the menu item is hidden when the list is
  empty (see "Two new entries").

**The per-row child count.** `Db::list_projects_by_scan_root(scan_root_id)`
already exists in 002b; it is **not** exposed as an `#[tauri::command]` yet.
The worker adds a thin `list_projects_by_scan_root(scan_root_id) ->
Result<Vec<i64>, String>` (or equivalent shape — the frontend only needs the
count) wrapper in `src-tauri/src/lib.rs`. This is acceptable cross-BC scope
creep for a one-line backend addition; if it grows past one wrapper, escalate
to a `project-registry-004` rather than ballooning here.

### Cascade-remove confirmation dialog

A small two-button confirmation modal that opens when the user clicks
"Remove" on a scan-root row.

- **Body text:** `"Remove scan root <path> and all N projects discovered
  under it? Tile state for those projects will not be retained."` (The
  retention caveat is important — ADR-013 makes the cascade hard-delete,
  not subject to ADR-005's 30-day window. Communicate it.)
- **Buttons:** "Cancel" (secondary) and "Remove" (primary, but with
  `statusMissing` border styling).
- **Confirm:** invoke `remove_scan_root(scan_root_id)` → backend cascades:
  emits N `project_removed` events (project-registry-003), tears down N
  watchers, hard-deletes N child rows, then deletes the scan-root row. The
  frontend drops tiles via the canvas-005a `project_removed` handler
  (already wired). The scan-roots management modal updates its row list
  (re-fetch `list_scan_roots()` to refresh) — if zero scan roots remain,
  close the modal AND hide the "Manage scan roots…" menu item on next
  right-click.
- **Cancel / backdrop / Escape:** close the confirmation only, leaving the
  management modal open behind it.

### `project_removed` handling

Already established by `canvas-005a`. This task does NOT re-wire the
listener. The cascade fan-out from `remove_scan_root` produces N events of
the same `project_removed` variant, and canvas-005a's handler is the single
canonical processor (drops tile + position cache + layout entry per event).

## Scope (in)

- `src/lib/Canvas.svelte`: append "Scan folder for projects…" and "Manage
  scan roots…" entries to the right-click menu shell (data shape established
  by canvas-005a). Compute "Manage scan roots…" visibility from
  `list_scan_roots()` — refresh on mount, after `add_scan_root` resolves,
  after `remove_scan_root` resolves.
- The **discovery checklist modal** — a new component
  (`src/lib/ChecklistModal.svelte` or inline in `Canvas.svelte`; worker's
  call).
- The **scan-roots management modal** — new component, same pattern. If two
  modals share enough boilerplate, extracting a generic `Modal.svelte` is
  recommended (DRY).
- The **cascade-remove confirmation dialog** — a third small modal.
- `src/lib/ipc.ts`: wrappers for `addScanRoot(path)`, `rescanScanRoot(id)`,
  `listScanRoots()`, `importScannedProjects(scanRootId, paths)`,
  `removeScanRoot(id)`, and `listProjectsByScanRoot(id)` (count source).
- `src-tauri/src/lib.rs`: thin `#[tauri::command] fn list_projects_by_scan_root`
  wrapper around the existing `Db::list_projects_by_scan_root` so the
  frontend can render the per-row child count. Register it in
  `invoke_handler`. (One-line cross-BC bleed, called out in Scope (out)'s
  escalation rule.)

## Scope (out)

- Single-shot Add / Remove / Missing (canvas-005a).
- The right-click context-menu shell itself (canvas-005a).
- The `project_removed` event handler (canvas-005a — this task uses it
  unchanged).
- Any further backend changes beyond the `list_projects_by_scan_root` IPC
  wrapper. If something else turns out to be missing during work, escalate
  to a `project-registry-004` rather than expanding scope.
- A "Show in Explorer" / "Open in IDE" affordance on tiles or rows (future,
  not v1).
- Inline progress UI for walk / import. The calls are fast at v1 sizes; if
  `add_scan_root` exceeds ~500ms on a real workload in practice, a spinner
  is a follow-up task.
- Frontend test runner. Same `pnpm check`-only verification surface as
  the other canvas tasks.
- A `STYLEGUIDE.md` entry for the Modal pattern. If a third consumer
  appears, codify then.

## Acceptance criteria

- [ ] Right-click on the empty canvas shows "Scan folder for projects…"
      alongside whatever canvas-005a contributed ("Add project…").
      "Manage scan roots…" appears in the menu only when at least one scan
      root is registered; otherwise hidden. Visibility refreshes after
      successful add / remove operations.
- [ ] Selecting "Scan folder for projects…" → native folder picker →
      backend `add_scan_root` → discovery checklist modal opens with the
      returned `{scan_root_id, candidates}`.
- [ ] In the checklist modal: rows with `already_imported: true` render at
      60% opacity, checkbox pre-checked **and disabled**, with an "imported"
      badge after the path. Rows with `already_imported: false` render at
      full opacity, checkbox unchecked by default, togglable. "Select all"
      / "Select none" header controls operate on togglable rows only and
      are hidden when there are zero togglable rows.
- [ ] "Import selected" is disabled when zero new candidates are ticked.
      Clicking it invokes `import_scanned_projects` with the picked paths
      and closes the modal. After it returns, tiles for the picked projects
      appear at distinct spiral positions (canvas-006's serialised live-add
      makes this true; the import flow is the smoke-test for that fix).
- [ ] Empty-candidates case: the modal opens with the "no Agentheim
      projects found" empty state, a single "OK" button, and the scan root
      is persisted (verifiable via `list_scan_roots()` returning the new
      root after the modal closes).
- [ ] "Manage scan roots…" opens a modal listing every scan root with its
      path, child-project count, "Rescan" button, and "Remove" button.
- [ ] Clicking "Rescan" on a row opens the discovery checklist modal
      pre-populated with the rescan's results. The header indicates this is
      a rescan.
- [ ] Clicking "Remove" on a row opens a confirmation dialog that names
      the scan-root path AND the child-project count, and explicitly states
      that tile state will not be retained. Confirming → backend cascades →
      tiles for the cascaded children drop via N `project_removed` events
      (handler in canvas-005a). The management modal's row list refreshes;
      if zero scan roots remain, the modal closes and "Manage scan roots…"
      hides on next right-click.
- [ ] All colours / sizes / typography pulled from
      `src/lib/design/tokens.ts` and `--guppi-*` CSS custom properties; no
      hard-coded values introduced.
- [ ] All modals dismiss on backdrop click + `Escape` + their explicit
      cancel/close buttons. Only one modal is open at a time (the
      confirmation modal stacks **on top of** the management modal — the
      management modal stays mounted behind it but does not respond to
      clicks; this is the explicit exception to "one at a time").
- [ ] `pnpm check` is clean — `0 errors, 0 warnings`.

## Notes

- **canvas-006 MUST land first.** Without it, importing N projects
  re-creates the spiral-collision bug — the very thing canvas-006 fixes.
  The task ordering is a hard `depends_on`.
- **project-registry-003 MUST land first** for the `project_removed` event
  the cascade now fires.
- The `count_projects_by_scan_root` need: a thin `#[tauri::command]`
  wrapper around the existing `Db::list_projects_by_scan_root`. Acceptable
  cross-BC scope creep for one line; if more backend turns out to be
  missing, escalate to a `project-registry-004`.
- **Modal extraction is the worker's call.** Three modals (checklist,
  manage-roots, confirmation) is enough to justify a `Modal.svelte`
  primitive (header / body / footer slots, backdrop, dismiss wiring) —
  recommendation, not requirement.
- The rescan flow could optionally show "N new" on the rescan button if
  the candidate set has any non-`already_imported` rows. Nice-to-have, not
  required for acceptance.
- **No ADR.** Same reasoning as canvas-005a — modal patterns and
  scan-flow UI sit inside ADR-003 + the styleguide's token vocabulary, plus
  ADR-013's already-established cascade semantics. The "hide Manage scan
  roots when empty" decision is a UX choice; recorded inline.

## Coordination

- **canvas-005a** ships the menu shell, the `project_removed` handler, and
  the missing-tile rendering. This task extends the shell with two more
  items and consumes the handler.
- **canvas-006** ships the live-add serialisation that makes the N-arrivals
  case (the *raison d'être* of `import_scanned_projects`) correct.
- **project-registry-003** ships `register_project` / `remove_project` /
  `ProjectRemoved` / `missing: bool`. The cascade in `remove_scan_root`
  starts firing `project_removed` events as part of that task — without
  which canvas-005b's cascade-remove UI would leave stale tiles.

## Outcome

Shipped the multi-step discovery surface that ADR-005 + ADR-013 named: the
**scan-folder flow** (pick → walk → checklist → import), the **scan-roots
management** modal (list / rescan / cascade-remove), and the **cascade-remove
confirmation** that names the retention exception. The three modals share a
new generic `src/lib/Modal.svelte` primitive (header / body / footer
snippets, `Escape` + backdrop-click dismissal, token-driven chrome) —
extraction triggered because the inline boilerplate would have duplicated
three times otherwise. The patterns are component-internal pending a fourth
consumer; codifying the Modal + button vocabulary as a `STYLEGUIDE.md` entry
is a backlog candidate (cross-BC scope; not attempted here).

Pieces:

- **Three new IPC wrappers + one new backend wrapper.** Frontend
  `src/lib/ipc.ts` gained `addScanRoot` / `rescanScanRoot` / `listScanRoots`
  / `importScannedProjects` / `removeScanRoot` / `listProjectsByScanRoot`.
  Backend `src-tauri/src/lib.rs` gained the one-line
  `#[tauri::command] fn list_projects_by_scan_root` over the existing
  `Db::list_projects_by_scan_root`; registered in `invoke_handler`. Returns
  `Vec<i64>` — the frontend takes `.length` for the per-row count
  (documented in the BC README's Discovery affordances section). This is
  the one explicit cross-BC bleed the task scoped in.
- **Empty-canvas menu extended.** `openEmptyCanvasMenu` now produces three
  items: "Add project…" (canvas-005a), "Scan folder for projects…" (always
  shown), and "Manage scan roots…" (hidden when the cached `scanRootsCount`
  is zero). `refreshScanRootsCount` runs on mount, after `addScanRoot`
  resolves, and after the management modal's `refreshManageModal` re-poll.
- **`Modal.svelte` primitive.** Centered overlay above a 70% backdrop of
  `--guppi-canvas-bg`. `header` / `body` / `footer` snippet slots. `Escape`
  and click-on-backdrop both fire the `onclose` callback. `maxWidth` prop
  overrides the 720px default for the narrower cascade-confirm dialog.
- **Discovery checklist modal.** Renders rows from a `ScanCandidate[]`;
  `already_imported: true` rows go to 60% opacity with the checkbox pre-
  checked AND `disabled`, plus a `statusIdle`-coloured "imported" pill
  after the path. Togglable rows hover-highlight against
  `--guppi-canvas-bg-raised`. The header path uses `direction: rtl` to
  truncate from the left, the rescan flag appends `" (rescan)"`. Header
  controls "Select all" / "Select none" operate only on togglable rows
  and hide when there are zero togglable rows. "Import selected" disables
  until at least one new (not already-imported) candidate is ticked — the
  imported pre-checks do NOT count toward enabling it; the same filter
  applies to the actual `importScannedProjects(scan_root_id, picks)` call.
  Empty-candidates case: same modal, "no Agentheim projects found"
  empty-state body, single "OK" footer button; the scan root is persisted
  by the backend regardless (002a persists before walking) so a future
  rescan picks up new clones.
- **Scan-roots management modal.** Loads via `listScanRoots()` +
  per-row `listProjectsByScanRoot(id)`. Each row carries the path
  (truncated mono), "{N} projects" count, "Rescan" (re-runs the walk +
  reopens the checklist with `isRescan: true`), and "Remove" (opens the
  cascade-confirm). "Close" footer button. Empty state never renders —
  the menu item is hidden when zero roots exist.
- **Cascade-remove confirmation.** Narrower (480px) modal that stacks ON
  TOP of the management modal (the explicit exception to one-at-a-time —
  Svelte's render order plus the second backdrop's `z-index: 20` produces
  the stacking automatically; the lower modal stays mounted behind it and
  its backdrop blocks pointer events). Body verbatim: *"Remove scan root
  &lt;path&gt; and all N projects discovered under it? Tile state for those
  projects will not be retained."* — the ADR-013 hard-delete caveat is
  communicated. Confirming calls `removeScanRoot`; cascade events flow
  through the canvas-005a `project_removed` handler (the one canonical
  listener — canvas-005b does NOT re-subscribe). Post-cascade
  `refreshManageModal` re-polls; if zero roots remain it closes the
  management modal and `scanRootsCount` drops to 0 so the menu item hides
  next time.
- **Button vocabulary.** `modal-button` + `modal-button-primary` /
  `modal-button-secondary` / `modal-button-destructive`. Primary uses
  `--guppi-tile-border` background + `--guppi-status-text` foreground,
  disabled goes to `--guppi-canvas-bg-raised` + `--guppi-tile-text-muted`
  with `cursor: not-allowed`. Destructive flips the border (or background,
  on primary-destructive) to `--guppi-status-missing` to signal a
  destructive action. Every value token-driven; no hard-coded literals
  outside the one documented backdrop RGB which mirrors
  `--guppi-canvas-bg` at 70% alpha (single edit point if the canvas-bg
  token shifts).

**No ADR written.** All decisions sit inside ADR-003 (PixiJS + overlays),
ADR-005 (manual + scan-root discovery), ADR-013 (cascade semantics), and
the styleguide. The Modal extraction is a component-internal v1 pattern;
the per-row count's `Vec<i64>` shape choice is documented in the BC
README rather than promoted to an ADR (no future maintainer would ask
"why this, not a count endpoint?" — the answer is in the README, and the
shape is trivially reversible if a count endpoint is ever preferred).

**No new backlog items created.** The "fourth consumer triggers a
STYLEGUIDE.md entry for Modal + buttons" rule from the task is captured in
the BC README so a future modal-using task will surface it naturally; not
worth a backlog ticket yet.

**Verification:** `pnpm check` → `0 errors, 0 warnings, 0 files with
problems` (938 files). `cargo check --manifest-path src-tauri/Cargo.toml`
clean. The fully wired flow (native picker, three modals, end-to-end
cascade) is a `pnpm tauri dev` hands-on exercise — the same posture as
canvas-005a / canvas-001 / canvas-002 / canvas-006.

**Files:**
- `src/lib/Canvas.svelte` — three new modals (checklist / manage-roots /
  cascade-confirm), two new empty-canvas menu items, scan-root state +
  flows, `refreshScanRootsCount` priming on mount.
- `src/lib/Modal.svelte` — new generic Modal primitive.
- `src/lib/ipc.ts` — six new IPC wrappers (`addScanRoot`, `rescanScanRoot`,
  `listScanRoots`, `importScannedProjects`, `removeScanRoot`,
  `listProjectsByScanRoot`).
- `src/lib/types.ts` — `ScanRootRow`, `ScanCandidate`, `AddScanRootResult`
  interfaces mirroring the Rust shapes.
- `src-tauri/src/lib.rs` — one-line `list_projects_by_scan_root`
  `#[tauri::command]` wrapper + `invoke_handler` registration.
- `.agentheim/contexts/canvas/README.md` — Modal / discovery-checklist /
  scan-roots-management / cascade-remove-confirmation entries in the
  ubiquitous language; Discovery affordances section extended with the
  scan-folder + management flows.
