# Desktop repeated-process lifecycle handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used. The current project debug executable was launched and exited; no installer, registry, startup entry, Provider credential, ADB mapping or user data was changed.

## Completed in this batch

- Added `scripts/test-desktop-process-lifecycle.ps1`, an explicit `-AllowProcessLaunch`-gated real-process runner. It refuses a pre-existing matching process or wrong executable filename, uses bounded readiness/secondary/full-exit waits, and force-cleans only the exact primary PID it created if a failure leaves it resident.
- The success record includes schema version, executable path/hash/product version, OS version, primary PID, every secondary exit/result, final resident count and the supported desktop-isolation boundary. An optional evidence path must already have a parent directory.
- Added an offline Rust source-contract test and included PowerShell parsing in the complete local verifier.
- Executed the runner against `desktop-p0/target/debug/agentmeter-desktop-p0.exe` on Microsoft Windows NT 10.0.26200.0. IPC readiness succeeded; five secondary launches each exited 0 and left exactly one process with the original primary PID; the app's `--request-exit` path exited 0 and left zero matching processes.
- Extended the same run with an abnormal-termination phase: a fresh ready primary was force-terminated by its exact owned PID, zero matching residents remained, a new primary reached readiness, a secondary reused that PID, and the new primary then completed graceful full exit with zero residents.
- The result is saved as `docs/evidence/desktop-process-lifecycle-2026-09-09.json`. Combined with the earlier direct observation that a same-desktop second launch restores the hidden window, this re-closes issue #05 criterion 4.
- Scope is explicit: same-interactive-desktop dispatch is supported. If another Windows desktop sees the mutex but cannot see the IPC window, the local plugin returns an error and refuses to create a second AgentMeter instance. It does not claim cross-desktop focus/elevation relay.

## Verification

- Real process runner: success, five repeated launches, one resident PID each time, zero after full exit.
- Full local verifier: 113 root Rust tests, 8 desktop Rust tests and 11 browser logic tests passed.
- Both crate formatting checks, three PowerShell syntax checks, two JavaScript syntax checks and warnings-denied Clippy for both crates passed.

## Acceptance boundary

Formal status is now **48/79 checked, 31 open**. Direct tray Show/Exit could not be newly captured because the Windows Computer Use helper failed initialization twice with a missing kernel-assets path. Abnormal prior termination/relaunch is now proven, but the compound lifecycle criterion remains open for sign-out, reboot and Explorer/tray recreation. Packaged clean-VM lifecycle, authenticated Providers and physical-tablet behavior also remain open.

Changes remain uncommitted in the existing dirty worktree.
