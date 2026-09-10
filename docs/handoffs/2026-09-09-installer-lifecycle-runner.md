# Installer lifecycle runner handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used. The lifecycle runner was parsed and tested but was **not executed**, so this batch made no installer, registry, startup or application-data changes.

## Completed in this batch

- Added `scripts/test-nsis-lifecycle.ps1` as an explicit clean-Windows lifecycle runner for the generated NSIS package.
- The runner refuses to proceed unless the operator supplies `-AllowSystemMutation`, an exact 64-digit installer SHA-256 and an existing evidence directory.
- Baseline guards reject a host with a running AgentMeter process, an existing install/uninstall registration, either startup value, or either AgentMeter application-data directory.
- All launched installer, application and uninstaller processes have time limits; the application is tested through hidden launch and its explicit `--request-exit` path.
- Added the side-effect-free `--probe-ready` single-instance command. It succeeds only after an existing desktop IPC target accepts the message; with no owner it exits with code 3 and never becomes a resident application.
- Startup enable/disable verifies the current `AgentMeter P0` value and rejects the legacy `AgentMeterP0` value.
- A second hash-identical silent install verifies same-version reinstall behavior, retained application data and startup registration, followed by another hidden launch and explicit exit.
- `preserve` verifies that default silent uninstall retains both probe markers. `purge` passes `/PURGE` and verifies that both identifier-scoped data locations are removed.
- Evidence records installer and installed size, initial-install and reinstall duration, measured first-launch/restart IPC readiness, a configurable 30–600 second idle CPU/working/private-memory sample (default 300 seconds), and WebView2 versions before and after.
- Successful runs and operational failures after preflight write `agentmeter.windows-install-lifecycle/v1` JSON evidence. Reported preservation and purge fields are derived from observed markers rather than the requested policy; rejected preflight inputs make no system change and report directly to the operator.
- Failure cleanup is limited to the runner's GUID-named system-temp install directory and its own marker files.
- Added a Rust source-contract test and made the complete local verifier parse the PowerShell runner without executing it.

## Clean-VM operator commands

Copy the installer and runner onto a disposable clean supported Windows VM, create the evidence directory, and run one policy at a time from PowerShell:

```powershell
.\test-nsis-lifecycle.ps1 -InstallerPath '.\AgentMeter P0_0.1.0_x64-setup.exe' -ExpectedSha256 '626906D75DC5045FB299F67AE7FF281403CD8BE40FEB8521147BF914DB4BDCD0' -DataPolicy preserve -EvidencePath '.\evidence\preserve.json' -AllowSystemMutation

.\test-nsis-lifecycle.ps1 -InstallerPath '.\AgentMeter P0_0.1.0_x64-setup.exe' -ExpectedSha256 '626906D75DC5045FB299F67AE7FF281403CD8BE40FEB8521147BF914DB4BDCD0' -DataPolicy purge -EvidencePath '.\evidence\purge.json' -AllowSystemMutation
```

The mutation switch is deliberately mandatory. These commands install and uninstall software and must only be run on an approved disposable VM.

## Verification and boundary

- PowerShell parser: passed under the repository's Windows PowerShell runtime.
- Lifecycle source-contract test: 1 passed.
- Full local verifier: 106 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, both formatting checks, both JavaScript syntax checks and warnings-denied Clippy for both crates passed.
- No installer or uninstaller was executed and no clean VM was available in this batch.

Formal status remains **47/79 checked, 32 open**. The runner removes repeatability and evidence-format gaps, but only an actual approved clean-VM run can close installer acceptance criteria. Repair semantics beyond same-version reinstall, cross-version upgrade, runtime failure branches, screenshots and signing also remain open.

Changes remain uncommitted in the existing dirty worktree.
