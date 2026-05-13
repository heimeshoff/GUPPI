---
id: infrastructure-001-desktop-runtime
type: decision
status: todo
scope: global
depends_on: []
---

# Decision: Desktop runtime

## Context

guppi is a native desktop app for a single user on a single machine. Primary platform: Windows 11. Cross-platform desirable but not strictly required day one.

Requirements:
- Render an infinite canvas with many tiles smoothly (PixiJS / WebGL works inside WebViews).
- Spawn and manage long-lived `claude` PTY sessions across OSes — riskiest piece.
- Watch many filesystem subtrees in parallel.
- Eventually host an emulated terminal panel inside the UI.
- Ship as a single installer.

## Architect's recommendation

**Tauri 2** with a Rust core and a web-tech frontend.

## Acceptance criteria

- [ ] ADR committed at `.agentheim/knowledge/decisions/ADR-001-desktop-runtime.md`
- [ ] Justification matches the architect's draft below (or Marco's amended version)
- [ ] `scope: global` recorded in frontmatter
- [ ] No code change required (this is a decision-only task)

## Architect open question Marco must answer

**Tauri vs Electron.** Architect prefers Tauri 2 (best PTY library exists in Rust via `portable-pty`, ~10–20MB bundles, fast startup). Electron is the alternative: TS-everywhere, larger ecosystem, but Node's PTY story on Windows is more fragile. Decide before the ADR is committed.

## Notes — architect's ADR draft

### ADR-001: Desktop runtime — Tauri 2

**Status:** Proposed
**Scope:** global

**Context.** guppi is a native desktop app for a single user on a single machine, primary platform Windows 11, want Mac/Linux to remain possible. The app must (a) render an infinite canvas with hundreds of tiles smoothly, (b) spawn and manage long-lived `claude` PTY sessions across OSes, (c) watch many filesystem subtrees in parallel, (d) eventually host an emulated terminal panel inside the UI, (e) ship as a single installer. Two languages is acceptable; a heavy 200MB runtime is not preferred but not disqualifying.

**Decision.** Use **Tauri 2** with a Rust core and a web-tech frontend. The Rust side owns: PTY, filesystem watching, sqlite, voice IPC client, project discovery. The frontend owns: canvas rendering, tile UI, command palette, terminal panel UI. Communication via Tauri's `invoke` (commands) and `emit` (events).

**Consequences.**
- (+) Rust gives us the best-in-class PTY library (`portable-pty`) and excellent FS-watcher (`notify`).
- (+) Bundles are ~10–20MB, startup is fast, memory footprint modest.
- (+) Strong type boundaries between core and UI; the "core" is reusable if we ever build a TUI or CLI escape hatch.
- (–) Two languages. Marco has to be comfortable in Rust for the backend, or accept a learning ramp.
- (–) Tauri 2 plugin ecosystem is younger than Electron's; expect to write our own integrations occasionally.
- (–) WebView differs across OSes (WebView2 on Windows, WKWebView on macOS) — possible canvas rendering quirks. Test the canvas early on Windows.

**Reversibility.** Medium. Switching to Electron later means rewriting the Rust core in Node, but the frontend would survive mostly intact if we keep IPC behind a thin abstraction.
