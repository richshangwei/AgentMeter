# Packaged updater startup configuration fix — 2026-09-10

## Reproduction and root cause

The installed preview displayed `desktop_startup_failed`. The preserved diagnostic at `%LOCALAPPDATA%\com.agentmeter.p0\logs\startup-error.log` identified the actual boundary:

`failed to initialize plugin updater: plugins.updater was null; expected Config`

This was not a WebView2 failure. `tauri-plugin-updater` requires a deserializable base configuration during plugin initialization, before AgentMeter's runtime update builder supplies its endpoint and public key. The desktop configuration did not contain a `plugins.updater` object, so the packaged process failed before the main window and single-instance exit handling were available.

## Fix

- Added an explicit `plugins.updater` object with an empty base `pubkey` to `desktop-p0/tauri.conf.json`.
- Runtime updates remain fail-closed: an empty base key cannot enable downloads; the compile-time GitHub endpoint and real public key are still required by `desktop-p0/src/updater.rs`.
- Added `tests/updater_boot_config.test.cjs` to the full verifier. It failed before the config change and passes after it.

## Verification

- Rebuilt the release executable and launched it with `--hidden` against the same local application-data directory.
- After three seconds the process remained alive and the preserved startup-error log timestamp was unchanged (`2026-09-10T14:16:57.0793433Z`).
- The repaired process accepted `--request-exit` and terminated with no remaining AgentMeter process.
- `./scripts/verify-local.ps1` passed, including root/desktop tests, updater startup contract, browser contracts, formatting and warnings-denied Clippy.
- Rebuilt NSIS package passed static archive, version, WebView download-component and embedded-marker inspection.

## Artifact

- `desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- 71,656,790 bytes
- SHA-256 `3A073752C00508EFAA9317B0616AAB3EF3736B751C74E77A86630DF53AB46C55`
- Product/file version `0.1.0`; Authenticode status `NotSigned`.

The previous installer hash `76CC976B89899A676CE936D0C10F80640EDCF17E8D1A2C85112418657A5C1BB9` is superseded and must not be used.
