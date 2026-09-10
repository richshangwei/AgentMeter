# Desktop bundle preflight handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used and no dependency was downloaded or installed.

## Completed in this batch

- Changed the Tauri desktop configuration from `bundle.active: false` to an active `nsis` target. The previous value would have prevented installer output even if the Tauri CLI were available.
- Retained the approved Windows WebView2 `downloadBootstrapper` strategy with silent bootstrapper execution.
- Set `allowDowngrades: false` so an older installer cannot silently replace a newer installation.
- Added two offline bundle-contract tests covering active NSIS output, WebView2 mode, downgrade policy, explicit package identity/product/version and the checked-in frontend directory.
- Built the optimized desktop executable successfully without network access.

Local release executable observation:

- Path: `desktop-p0/target/release/agentmeter-desktop-p0.exe`
- Size: 9,122,304 bytes (8.70 MiB)
- SHA-256: `EA0CE062B9C211E469F1CBBCF3C1EA8D6FBD9DC4ADA5AB1E24D40ECD1D90F3B3`

This executable is a build artifact under the ignored target directory. It is not an installer-size measurement and is not clean-machine evidence.

## Verification

- `cargo test --offline --locked --test desktop_bundle_config -- --nocapture`: 2 passed.
- `cargo test --manifest-path desktop-p0/Cargo.toml --offline --locked`: 7 passed.
- `cargo build --manifest-path desktop-p0/Cargo.toml --offline --locked --release`: passed.
- `powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1`: passed with 103 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, both formatting checks, both syntax checks and both Clippy checks.

## Acceptance boundary and next work

Formal status remains **47/79 checked, 32 open**. `cargo tauri --version` reports that the command is not installed, so this batch did not create an NSIS file. It also did not install, repair, upgrade or uninstall anything and did not exercise WebView2 on a clean VM.

The next packaging step requires an approved Tauri CLI installation or an already provisioned build environment, followed by NSIS generation and clean-VM testing. That external step must record package hash/size, WebView2 branches, lifecycle behavior, local-data policy, diagnostics, timings and process/resource observations before any installer tick closes.

Changes remain uncommitted in the existing dirty worktree.
