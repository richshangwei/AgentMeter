# Installer upgrade-runner handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used. No installer was executed because the repository currently has no lower-version signed-off package to use as an upgrade baseline.

## Completed in this batch

- Extended `scripts/test-nsis-lifecycle.ps1` with an optional base-to-upgrade package sequence.
- The base package now has an explicit expected product version, verified after initial install and same-version reinstall.
- Upgrade path, SHA-256 and expected version are an all-or-none group. Before any installation, the runner verifies both files and hashes, parses both versions, requires the upgrade version to be greater, and rejects identical package hashes.
- The optional upgrade installs into the same owned temporary location and verifies the new embedded product version, executable/uninstaller presence, application-data markers, current startup registration and absence of the legacy startup value.
- The upgraded application must answer the IPC readiness probe and complete explicit exit before startup disable and final uninstall.
- Version, path, hash, size, elapsed time, readiness time and `supported_upgrade` observation are included in the versioned success/failure evidence.
- Cleanup now includes a possible upgraded runner-owned process.

## Operator form

The normal one-package preserve/purge commands remain valid. Once an approved lower-version package exists, add all three upgrade arguments:

```powershell
.\test-nsis-lifecycle.ps1 `
  -InstallerPath '.\AgentMeter OLD_x64-setup.exe' `
  -ExpectedSha256 '<OLD-64-DIGIT-SHA256>' `
  -ExpectedVersion '<OLD-VERSION>' `
  -UpgradeInstallerPath '.\AgentMeter NEW_x64-setup.exe' `
  -UpgradeExpectedSha256 '<NEW-64-DIGIT-SHA256>' `
  -UpgradeExpectedVersion '<NEW-VERSION>' `
  -DataPolicy preserve `
  -EvidencePath '.\evidence\upgrade-preserve.json' `
  -AllowSystemMutation
```

This command is intentionally illustrative and contains non-runnable placeholders. It must only be populated from release artifacts and run on an approved disposable clean VM.

## Verification and boundary

- PowerShell syntax passed under the repository's Windows PowerShell runtime.
- Lifecycle runner contracts: 2 tests passed, including source-order proof that identity, upgrade and clean-baseline guards precede the first installer invocation.
- Full local verifier passed: 109 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, both PowerShell syntax checks, formatting, JavaScript syntax and warnings-denied Clippy for both crates.
- No lower-version artifact or clean VM was available, so the upgrade branch has not been executed and cannot be reported as accepted.

Formal status remains **47/79 checked, 32 open**. The supported-upgrade operator path is now implemented, but the real upgrade acceptance tick stays open until two approved versioned artifacts are exercised.

Changes remain uncommitted in the existing dirty worktree.
