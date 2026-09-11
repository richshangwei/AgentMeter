# Tablet dynamic monitors, silent desktop collection, and signed updates — 2026-09-10

## Outcome

This batch implements the requested desktop/tablet workflow:

- Windows background quota collection now suppresses the nested ConPTY console window; the preparation script reapplies the pinned `node-pty` seam after dependency installation.
- The tablet dashboard is data-first, responsive, and touch-sized. It has a fullscreen button, compact quota cards, a monitor settings dialog, and provider-specific installation/login/retry guides.
- Monitor selection is persisted locally, accepts future provider IDs, supports add/remove at runtime, and always renders one empty “新增監控” slot. The tablet remains monitor-only; provider/account settings stay on desktop.
- Source-usage-only observations render their real value instead of appearing empty. Initial unavailable/idle cards expose the guided setup action even before the first successful collection.
- Startup update checks are advisory and non-blocking. A user must confirm before a signed Tauri update is downloaded and installed. Manual checking is available from desktop settings.
- Updater cleanup runs through Tauri's `on_before_exit` hook, so download, signature or extraction failures leave monitoring alive and preserve the checked update for retry.
- The long-lived Device Pair cookie is scoped to `/api/v1/session`. Runtime staging rebuilds `node_modules` from the pinned lock with scripts disabled before applying the reviewed ConPTY patch.

## Release configuration

The checked-in desktop build is intentionally unconfigured for a real release because this repository has no GitHub owner/repository or signing key. Release builders must provide `AGENTMETER_UPDATE_ENDPOINT`, `AGENTMETER_UPDATE_PUBLIC_KEY`, and `TAURI_SIGNING_PRIVATE_KEY`, then run `cargo tauri build --config tauri.updater.conf.json` from `desktop-p0`. The updater accepts only an HTTPS GitHub `releases/latest/download/latest.json` endpoint and verifies the signed artifact before installation. Normal local builds remain usable without those values and report “尚未設定更新來源”.

## Verification

- `./scripts/verify-local.ps1` — passed after review fixes: formatting, root and desktop Rust tests, 24 browser/contract tests, syntax checks, and warnings-denied Clippy.
- `node .scratch/verify-tablet-ui.cjs` — passed at 1024×768, 1280×800, and 768×1024: no horizontal overflow, five tiles including one add slot, 48px touch controls.
- Targeted tablet/background/update tests — passed (14 tablet/background tests, 3 desktop UI tests, 21 Tablet HTTP tests, updater unit test).
- The screenshot `.scratch/tablet-dashboard.png` is a visual reference only; physical tablet, signed release, authenticated provider and clean-VM installer acceptance remain separate gates.

## Installer artifact

The ordinary unsigned preview installer was rebuilt and statically inspected successfully:

- `desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- 71,656,790 bytes
- SHA-256 `3A073752C00508EFAA9317B0616AAB3EF3736B751C74E77A86630DF53AB46C55`
- Embedded product/file version `0.1.0`; NSIS archive, WebView download component, startup diagnostic markers and Copilot contract markers present.
- Authenticode status remains `NotSigned`; this artifact is for local preview and does not claim production-release trust.
- This artifact includes the explicit updater base configuration required for packaged startup; see [the startup-fix handoff](2026-09-10-updater-startup-config-fix.md).

## Rollback

Revert the focused files in this batch. No database migration, account change, credential change, or destructive cleanup is introduced. If a release check is not desired, omit the three release environment values; monitoring continues normally.
