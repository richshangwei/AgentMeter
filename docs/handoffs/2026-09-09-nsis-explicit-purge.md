# NSIS explicit purge handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used and the installer/uninstaller was not executed.

## Completed in this batch

- Added `desktop-p0/nsis/installer-hooks.nsh` through Tauri's supported `installerHooks` seam.
- Made NSIS install mode explicitly `currentUser`, matching AgentMeter's HKCU startup registration and user-scoped data design.
- Added an explicit uninstaller `/PURGE` option. Only that flag or the existing interactive checkbox sets Tauri's generated `DeleteAppDataCheckboxState`; default and silent uninstall preserve data.
- The custom hook does not contain `RMDir`, `$APPDATA` or `$LOCALAPPDATA`. Tauri's generated, identifier-scoped uninstall code remains the sole deletion implementation.
- The hook deletes the legacy `AgentMeterP0` Run value only outside update mode, preventing an upgrade from disabling startup.
- Expanded bundle contract tests to require the hook path, current-user mode, explicit purge semantics, update guard and absence of direct data-deletion commands.

## Rebuilt package evidence

- Tauri CLI found and included the absolute hook path in generated `installer.nsi`.
- makensis compiled the hook and produced one x64 NSIS bundle successfully.
- Installer size: 1,985,964 bytes.
- Installer SHA-256: `54DDC8CD1D756D0CCB03F10289087D9661F00FCED6295FA08253A465D60FF15C`.
- Embedded executable SHA-256: `B683C5D832E4CDE0CD8B08CADEA945894D9594620FBA5C39D6596F5638D7533C`.
- Version: 0.1.0; Authenticode: unsigned.

## Verification and boundary

- Bundle contract: 4 tests passed.
- Desktop: 7 tests passed.
- NSIS rebuild and static inspection passed.
- Full local verifier passed: 105 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, formatting, syntax and warnings-denied Clippy checks.

Formal status remains **47/79 checked, 32 open**. The generated executable behavior is compiled but not run. A real install must still prove default preservation, interactive or `/PURGE` deletion, startup removal, repair/update behavior and absence of orphaned processes. No clean VM or installer execution approval is currently available, and the package remains unsigned.

Changes remain uncommitted in the existing dirty worktree.
