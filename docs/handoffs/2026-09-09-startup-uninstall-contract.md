# Startup and uninstall contract handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used and the installer was not executed.

## Completed in this batch

- Inspected the exact Tauri 2.11.4 NSIS template used by the installed bundler.
- Confirmed Tauri's generated uninstaller already presents an unchecked-by-default “delete app data” checkbox. When selected outside update mode it removes `%APPDATA%/com.agentmeter.p0` and `%LOCALAPPDATA%/com.agentmeter.p0`; otherwise those directories are preserved.
- Found a real startup-cleanup defect: AgentMeter wrote the Run value `AgentMeterP0`, while the generated uninstaller deletes the product-name value `AgentMeter P0`.
- Changed the application startup value to `AgentMeter P0`, matching `tauri.conf.json` and the generated NSIS script.
- Preserved migration cleanup: enabling or disabling startup also removes the legacy `AgentMeterP0` value. Registry failures remain actionable rather than silently ignored.
- Added a bundle contract test that locks the application constant to `productName` and requires legacy cleanup.
- Corrected the clean-install evidence document to name the actual Tauri identifier-based roaming/local data directories.

## Rebuilt package evidence

The NSIS rebuild completed successfully. Its generated script contains:

- `DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "${PRODUCTNAME}"`, where `PRODUCTNAME` is `AgentMeter P0`.
- Data removal only under `DeleteAppDataCheckboxState = 1` and outside update mode.
- `RmDir /r "$APPDATA\${BUNDLEID}"` and `RmDir /r "$LOCALAPPDATA\${BUNDLEID}"`, where `BUNDLEID` is `com.agentmeter.p0`.

Latest artifact:

- Size: 1,985,637 bytes.
- SHA-256: `DD2DE8631F0E9F019E896BBD0F22EAC8AF57E35B33BFA0224B8498F06C36212E`.
- Embedded executable SHA-256: `33D6873554662992E1F5A70CD1883695A6B6BE012AA1EAC347A86C7C60F3874C`.
- Product/file version: 0.1.0.
- Authenticode: unsigned.

## Verification and boundary

- NSIS rebuild passed.
- Static installer inspection passed.
- Full local verification passed: 104 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, formatting, syntax and warnings-denied Clippy checks.

Formal status remains **47/79 checked, 32 open**. The generated script proves intended installer behavior, but the installer/uninstaller was not executed. Clean-VM install, startup cleanup, preserve/delete choices, repair, upgrade and process cleanup still require real lifecycle evidence. The unsigned package remains unsuitable for public distribution.

Changes remain uncommitted in the existing dirty worktree.
