# Runnable USB host controller handoff — 2026-09-09

Single-agent implementation and verification. This continues the ADB command-layer batch; no multi-agent work was used.

## Completed in this batch

- Extended `agentmeter-tablet-host` with an optional all-or-nothing USB configuration: `--adb ABSOLUTE_PATH --serial SERIAL --device-port PORT`.
- The server binds its loopback listener first, then installs `tcp:<device-port> -> tcp:<actual-host-port>` against the exact selected physical USB serial through the production ADB command layer.
- Readiness output records selected serial, transport, device port, host port and mapping verification. It deliberately reports `browser_launch: not_requested` and `authenticated_health: not_observed`.
- Added the separate stdin command `open`. It asks Android's activity manager to open `http://127.0.0.1:<device-port>/`, contains no pair code or credential, and only reports that launch was requested; it never changes reachability or authenticated-health state.
- The owned reverse mapping lives for the host process and is reverified before removal on normal exit, EOF or an error path. A changed mapping is not removed.
- Partial USB configuration fails before any readiness announcement.
- Expanded the fake ADB process double and host CLI integration tests to verify the complete devices/no-rebind/list/browser/list/remove sequence and mapping-file disappearance after exit.

## Verification

`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1` passed after the integration:

- Root Rust: 98 tests passed, including 3 ADB command tests and 5 runnable-host tests.
- Desktop Rust: 7 tests passed.
- Browser/Desktop JavaScript: 11 tests passed.
- Root and desktop formatting, JavaScript syntax checks, and root/desktop Clippy with warnings denied all passed.

## Acceptance boundary and next work

Formal status remains **47/79 checked, 32 open**. All USB execution in this batch used a real child process that behaves as a controlled ADB double. This proves process ownership and command construction but does not prove an installed ADB distribution, a physical Android USB transport, an actual browser activity, authenticated tablet traffic, or unplug/replug/sleep/restart recovery.

The next locally actionable gap is exposing authenticated tablet activity as an observable host transport signal and defining recovery state transitions without confusing browser launch with online status. Final issue acceptance still requires operator evidence from a selected physical tablet.

Changes remain uncommitted in the existing dirty worktree.
