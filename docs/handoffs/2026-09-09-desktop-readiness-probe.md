# Desktop readiness-probe handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used. The installer and desktop executable were built and inspected but not executed.

## Completed in this batch

- Added a side-effect-free `--probe-ready` command to the Tauri single-instance callback. When an existing instance's IPC target accepts the command, the probe exits successfully without showing the window or changing state.
- Extended the licensed local Windows single-instance patch so a readiness probe with no owner exits with code 3 and cannot accidentally become the primary resident application.
- Updated `instance-probe.ps1` with a `ready` expectation and added a source-contract test for both the callback and no-owner boundary.
- Replaced the lifecycle runner's fixed two-second launch assumption with bounded IPC-readiness polling for initial launch and post-reinstall restart.
- The runner now defaults to a five-minute idle sample and records duration, CPU percentage normalized by logical processor count, total processor time, working set and private memory. Operators can choose 30–600 seconds explicitly.
- Failure cleanup now requests graceful exit for each runner-owned desktop process, waits, and only then force-stops an owned process before uninstall cleanup.

## Rebuilt package evidence

- Package: `desktop-p0/target/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- Installer size: 1,971,046 bytes.
- Installer SHA-256: `0AB9A96DCA519D005158A9EB770E91971426FE3FF46649DD211573DCDB34901A`.
- Embedded executable size: 9,121,792 bytes.
- Embedded executable SHA-256: `FFE69873DFB1A104CD2C601F9A0DB72D7D440BBEAD32196ACDABE01FE8D0B6F5`.
- Product/file version: 0.1.0; NSIS download component: present; installer and executable Authenticode status: `NotSigned`.

## Verification and boundary

- Lifecycle/desktop source contracts: 6 tests passed.
- Desktop Rust tests: 7 passed.
- Both PowerShell scripts parse successfully.
- Tauri release compilation, makensis packaging and non-executing NSIS inspection passed.
- Full local verifier passed: 108 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, both PowerShell syntax checks, formatting, JavaScript syntax and warnings-denied Clippy for both crates.
- The rebuilt executable was not launched, so runtime readiness and clean-VM measurements remain unproven until an approved execution environment runs the lifecycle script.

Formal status remains **47/79 checked, 32 open**. This batch makes the first-launch, restart and idle-resource evidence measurable without inventing readiness, but it does not close a real-runtime acceptance tick.

Changes remain uncommitted in the existing dirty worktree.
