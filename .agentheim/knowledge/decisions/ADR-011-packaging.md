---
id: ADR-011-packaging
title: Packaging and install — Tauri bundler, unsigned MSI for Windows initially
status: Accepted
scope: global
bc: infrastructure
date: 2026-05-14
related_tasks: [infrastructure-011-packaging]
related_adrs: [ADR-001-desktop-runtime]
---

# ADR-011: Packaging and install — Tauri bundler, unsigned MSI for Windows initially

**Status:** Accepted
**Scope:** global

## Context

GUPPI is a single-user, single-machine tool with no distribution audience. Even so, it should install cleanly and update without re-cloning the repo. The runtime is Tauri 2 (ADR-001), and the validated platform day one is Windows 11 / WebView2.

## Decision

- **Bundler.** Use Tauri's built-in bundler (`tauri build`) targeting **MSI** on Windows. NSIS is the available alternative; MSI is fine and is the chosen target.
- **Signing posture — deferred-unsigned.** Do **not** code-sign the binary initially. SmartScreen will warn once on first install; click through. Signing can be revisited if SmartScreen becomes annoying — reversibility is trivial, so there is no reason to pay for a certificate before there is a felt need.
- **Updates.** Use Tauri's updater plugin pointed at a GitHub Release feed in the personal `guppi` repo. Self-hosted, no third party.
- **Install location.** Default per-user install at `%LOCALAPPDATA%\Programs\guppi\` — no admin rights required.

## Consequences

- (+) Installs and uninstalls like any normal Windows app.
- (+) Self-hosted updates via GitHub Releases keep everything local and under personal control.
- (+) Per-user install avoids UAC prompts entirely.
- (–) SmartScreen warning on first install (and after each unsigned update). Acceptable for a personal tool.
- (–) The update feed depends on the personal `guppi` GitHub repo remaining available.

## Reversibility

Trivial. Switching the signing posture to sign-now only requires obtaining a certificate and wiring it into the Tauri bundler config; switching MSI to NSIS is a one-line bundler target change.
