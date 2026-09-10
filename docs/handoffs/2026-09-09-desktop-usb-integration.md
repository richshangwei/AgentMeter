# Desktop selected-device USB integration handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used. No ADB executable, Android device, installer or uninstaller was run.

## Completed in this batch

- Wired the existing production ADB reverse command layer into the Tauri desktop bridge. The user must explicitly start the loopback TabletServer and provide an absolute ADB executable path, selected physical-device serial and non-zero device port.
- Added desktop controls and commands for connect, verified browser open, missing-only recovery and disconnect. The view records the selected serial, `usb` transport, separate device/host ports, owned/missing/changed/unavailable mapping state and browser-launch request state.
- Browser launch requires a currently owned exact reverse mapping. Recovery recreates only a proven-missing mapping; externally changed ownership is refused. Disconnect/full exit use `OwnedReverse` teardown, which refuses to remove an entry that no longer matches the mapping AgentMeter created.
- Kept ADB inspection out of the two-second activity poll. The light activity command reports only current TabletServer authorization; explicit status/action commands perform bounded ADB process checks.
- Corrected `authenticated_session_established` from sticky historical state to a live authorization result. Expired, revoked or pair-invalidated Sessions no longer show online.
- Every untrustworthy transport transition—new mapping generation, missing/changed/unavailable mapping, recovery and disconnect—revokes old Tablet Sessions and closes their SSE authorization. Durable Device Pairs survive a transport transition and can exchange a distinct Session only after a mapping is trusted again.
- Added a real loopback HTTP regression proving transport revocation makes the old Session return 401, clears online, preserves the Device Pair and issues a distinct replacement Session. Existing expiry/reset tests now assert online also clears, and the desktop static contract locks in mapping-change revocation.

## Verification

`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1` passed:

- Root Rust: 112 tests passed, including 18 real loopback TabletServer HTTP tests.
- Desktop Rust: 8 tests passed.
- Browser logic: 11 tests passed.
- PowerShell syntax, root/desktop formatting, both JavaScript syntax checks and warnings-denied Clippy for both crates passed.

Tauri release compilation, makensis packaging and `scripts/inspect-nsis.ps1` also passed.

## Rebuilt package evidence

- Package: `desktop-p0/target/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- Installer size: 2,057,403 bytes.
- Installer SHA-256: `F019358A23D9EA0B8A37E46E21382AE5D213A3C66006DC2B57972A3D409B0D3F`.
- Embedded executable size: 9,472,512 bytes.
- Embedded executable SHA-256: `A84428ED1AA125EC0DAA233556CD9D88C2987FCD48288B4AE592ADDFAB47FD76`.
- Product/file version: 0.1.0; WebView2 download component: present; installer and executable Authenticode status: `NotSigned`.

## Acceptance boundary

Formal status remains **47/79 checked, 32 open**. The desktop now has the selected-device USB implementation needed for an operator run, but this environment has not supplied an absolute ADB installation, an authorized physical Android serial, real browser/tablet screenshots, unplug/replug/ADB-restart/sleep evidence or clean-VM installer execution. Those compound criteria remain open rather than being inferred from fake-ADB and loopback tests.

Changes remain uncommitted in the existing dirty worktree.
