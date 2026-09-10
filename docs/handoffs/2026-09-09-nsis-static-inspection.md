# NSIS static inspection handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used.

## Completed in this batch

- Added `scripts/inspect-nsis.ps1` as a reusable, non-executing package inspection step.
- The script requires an existing installer, locates 7-Zip explicitly, validates NSIS archive type and the expected embedded AgentMeter executable, extracts only that executable into a unique system temporary directory, and always validates the cleanup path before recursive removal.
- Output is versioned JSON containing installer and embedded sizes, SHA-256 hashes, Authenticode states, product/file versions, archive type and whether the NSIS download component is present.
- The current package passed with `download_component_present: true`; archive inspection identifies NSIS 3 Unicode and `$PLUGINSDIR/NSISdl.dll`.
- The embedded application is 9,151,488 bytes, version 0.1.0, SHA-256 `90A16BDD8B0536B18EA744C1166DF87327B4403078C5F8C3A10787A7D6FE6193`, and unsigned.
- Compared with the post-build application, the embedded file has the same size and differs by exactly three bytes at the Tauri bundle-type marker (`UNK` versus `NSS`), consistent with the bundler's reported NSIS metadata patch.

## Verification

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/inspect-nsis.ps1 `
  -InstallerPath 'desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe' `
  -ExpectedVersion '0.1.0'
```

The command exited successfully and left no inspection directory. The full local verifier also passed with 103 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, formatting, syntax and warnings-denied Clippy checks.

## Acceptance boundary

Formal status remains **47/79 checked, 32 open**. A scoped attempt to execute the installer for a reversible local smoke test was rejected by the execution approval layer; it was not retried or bypassed. Static extraction is materially safer but cannot prove install, launch, WebView2 download behavior, repair, upgrade, uninstall, registrations, process cleanup or user-data policy. No clean VM is available on this host.

The package remains unsigned and unsuitable for public release. Actual installer lifecycle work requires explicit approval to run the installer or access to an isolated supported Windows VM.

Changes remain uncommitted in the existing dirty worktree.
