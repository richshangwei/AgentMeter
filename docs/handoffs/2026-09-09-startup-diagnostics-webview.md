# Startup diagnostics and WebView2 contract handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used. The installer was rebuilt and inspected but not executed.

## Completed in this batch

- Replaced release `.expect(...)` startup termination with a bounded app-owned diagnostic file and a native Win32 error dialog that works without WebView2. Desktop startup and startup-registration failures use distinct stable codes and actionable recovery text.
- Added unit coverage for schema, required fields, one-line control-character handling and the 2,048-character detail bound. A root contract test protects the release call sites and the WebView-independent dialog/log implementation.
- Extended NSIS inspection to reject an extracted executable missing any startup diagnostic marker; the rebuilt package passed with `startup_diagnostic_markers_present: true`.
- Audited the locked Tauri bundler 2.9.4 NSIS source and recorded the exact present, missing, download-failure, install-failure, update and old-runtime branches. Corrected the prior assumption about old runtimes: no minimum version is configured, so existing old versions are skipped and remain a release gate.
- Recorded the development host's current WebView2 Runtime `152.0.4191.66`, alongside the already versioned OS/Tauri/plugin/process evidence.
- Closed #05.7 (runtime/process/limitation/reproduction record) and #06.2 (approved bootstrapper strategy plus truthful branch documentation). No execution-only criterion was closed.

## Verification and package

- Full local verifier passed: 115 root Rust tests, 9 desktop Rust tests, 11 browser logic tests, four PowerShell syntax checks, both formatters, both JavaScript syntax checks and warnings-denied Clippy for both crates.
- Tauri release compilation and makensis passed.
- Package: `desktop-p0/target/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- Installer: 2,066,079 bytes; SHA-256 `626906D75DC5045FB299F67AE7FF281403CD8BE40FEB8521147BF914DB4BDCD0`; Authenticode `NotSigned`.
- Embedded executable: 9,480,704 bytes; SHA-256 `3BB2B811FE7DB5EBC1FBA6B58D749245F1B36AD6B2343813F6CC0B4F480F678F`; version 0.1.0; Authenticode `NotSigned`.
- WebView download component and packaged startup-diagnostic markers: present.

## Acceptance boundary

Formal status is now **50/79 checked, 29 open**. Missing/outdated WebView execution, download denial, repair, clean install, same-version reinstall, cross-version upgrade, uninstall policy, timing/resources and startup error presentation still require an approved disposable clean VM. The absence of an approved `minimumWebview2Version` remains explicit and must be resolved from target-environment evidence rather than guessed.

Changes remain uncommitted in the existing dirty worktree.
