---
id: ADR-001
title: Desktop runtime — Tauri 2
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-001-desktop-runtime]
---

# ADR-001: Desktop runtime — Tauri 2

**Status:** Accepted
**Scope:** global

## Context

GUPPI is a native desktop app for a single user on a single machine. The
primary — and, for day one, the **only** validated — platform is **Windows
11**. macOS and Linux are kept architecturally possible but are explicitly
**not** a day-one requirement: they are untested and unvalidated.

The runtime must:

- (a) render an infinite canvas with hundreds of tiles smoothly (PixiJS /
  WebGL works inside WebViews);
- (b) spawn and manage long-lived `claude` PTY sessions — the riskiest piece;
- (c) watch many filesystem subtrees in parallel;
- (d) eventually host an emulated terminal panel inside the UI;
- (e) ship as a single installer.

Two languages is acceptable. A heavy ~200MB runtime is not preferred but
would not be disqualifying on its own.

The runtime choice came down to **Tauri 2** versus **Electron**. Electron
offers TypeScript everywhere and a larger, more mature ecosystem, but Node's
PTY story on Windows is more fragile. Tauri 2 pairs a Rust core with a
web-tech frontend, gives access to the best-in-class PTY library
(`portable-pty`), produces ~10–20MB bundles, and starts fast.

Marco reviewed both and signed off on the architect's recommendation.

## Decision

Use **Tauri 2** with a **Rust core** and a **web-tech frontend**.

- The **Rust core** owns: PTY, filesystem watching, sqlite persistence, the
  voice IPC client, and project discovery.
- The **frontend** owns: canvas rendering, tile UI, command palette, and the
  terminal panel UI.
- **IPC** between the two uses Tauri's `invoke` (commands) and `emit`
  (events).

The IPC surface is kept behind a thin abstraction on the frontend side so the
runtime is not deeply entangled with Tauri-specific APIs.

## Consequences

- (+) Rust gives us the best-in-class PTY library (`portable-pty`) and an
  excellent FS-watcher (`notify`) — directly de-risking the two hardest
  requirements (b) and (c).
- (+) Bundles are ~10–20MB, startup is fast, and the memory footprint is
  modest.
- (+) Strong type boundaries between core and UI; the core is reusable if we
  ever build a TUI or CLI escape hatch.
- (–) Two languages. Marco must be comfortable in Rust for the backend, or
  accept a learning ramp.
- (–) Tauri 2's plugin ecosystem is younger than Electron's; expect to write
  our own integrations occasionally.
- (–) WebViews differ across OSes (WebView2 on Windows, WKWebView on macOS).
  Since only Windows 11 / WebView2 is validated day one, any canvas rendering
  quirks on other WebViews are an **accepted unknown**, not a solved problem.
  The canvas must still be tested early on Windows.
- (–) Cross-platform support is **kept possible, not validated**. Choosing a
  cross-platform-capable runtime preserves the option, but no macOS/Linux
  build is exercised or guaranteed until explicitly scoped as future work.

## Reversibility

Medium. Switching to Electron later would mean rewriting the Rust core in
Node, but the frontend would survive mostly intact because the IPC layer is
kept behind a thin abstraction.
