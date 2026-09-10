# USB recovery control handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used.

## Completed in this batch

- Added exact ADB reverse-mapping inspection with three safe states: `owned`, `missing` and `changed`.
- Extended host `status` output with the current mapping state. An ADB command failure is reported as `unavailable` with a bounded failure code/message rather than guessed state.
- Added the explicit `recover-usb` operator path. It performs no mutation when the mapping is already owned, recreates only a mapping proved missing, and refuses to replace or remove a mapping whose selected remote port points elsewhere.
- Recovery repeats explicit physical-device discovery and `--no-rebind` setup, so an offline, unauthorized, emulator, non-USB or conflicting device still fails closed.
- Added real child-process tests that delete the fake ADB mapping during a running host, observe `missing`, recover to `owned`, and then verify scoped removal on exit.
- Added a changed-mapping test that rewrites the selected port to a different host port, confirms there is no second no-rebind setup and no remove command, and leaves the external entry intact.

## Verification

`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1` passed:

- Root Rust: 101 tests passed, including 7 runnable-host tests, 3 ADB command tests and 16 tablet HTTP tests.
- Desktop Rust: 7 tests passed.
- Browser/Desktop JavaScript: 11 tests passed.
- Both Rust formatting checks, both JavaScript syntax checks, and root/desktop Clippy with warnings denied passed.

## Acceptance boundary and next work

Formal status remains **47/79 checked, 32 open**. The tests deterministically model mapping disappearance and external replacement using a real process double. They do not certify a physical unplug/replug, ADB daemon restart, Windows host restart, tablet sleep/wake or browser reconnection.

The operator recovery sequence is now: run `status`; reconnect/authorize the exact physical tablet or restart ADB if state is unavailable; run `recover-usb` only after state is `missing`; run `open` separately if needed; then require authenticated-activity evidence before treating the tablet path as observed. A `changed` mapping requires the operator to identify its owner instead of AgentMeter overwriting it.

Changes remain uncommitted in the existing dirty worktree.
