# ADB command layer handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used.

## Completed in this batch

- Added `src/usb.rs` as a production command boundary around an explicitly supplied absolute ADB executable.
- Device discovery requires the exact requested serial to appear once in `adb devices -l`, be authorized, be in the `device` state, and carry a physical `usb:` transport marker. Emulator and non-USB transports fail closed.
- Reverse forwarding uses the selected serial and the exact command `reverse --no-rebind tcp:<device-port> tcp:<host-port>`; device and host ports remain distinct values.
- Setup verifies the exact serial/remote/local tuple from `reverse --list`. Failed verification attempts best-effort cleanup of only the requested remote port.
- The returned `OwnedReverse` checks the mapping again before teardown. If another process changed it, AgentMeter reports `mapping_ownership_lost` and refuses removal.
- Every ADB process has null stdin, captured output capped at 64 KiB per stream, a caller-supplied timeout capped at 60 seconds, forced termination on timeout, and no raw device output in public failures.
- Added `fake-adb` as an isolated process double and `tests/usb_command.rs` to exercise actual child-process invocation, exact argument order, no-rebind conflicts, a hung ADB process, verified teardown and changed-mapping refusal.

## Verification

`powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1` passed:

- Root Rust: 96 tests passed.
- Desktop Rust: 7 tests passed.
- Browser/Desktop JavaScript: 11 tests passed.
- Root and desktop formatting checks passed.
- Tablet and desktop client syntax checks passed.
- Root and desktop Clippy passed with warnings denied.

The targeted `cargo test --offline --locked --test usb_command -- --nocapture` run passed all three integration tests. A temporary 2-second normal-command threshold was found flaky under concurrent Windows process startup; normal fake-ADB cases now use a 5-second test allowance, while the dedicated hung-command case still proves a 100-millisecond timeout and completes within three seconds.

## Acceptance boundary and next work

Formal status remains **47/79 checked, 32 open**. The command layer is real, but this machine has no usable ADB executable or authorized physical Android tablet. Therefore this batch does not claim an installed reverse mapping, tablet reachability, unplug/replug recovery, sleep recovery or authenticated device traffic.

Next, wire this command layer into the runnable tablet host controller, keep browser launch separate from authenticated health, and expose actionable lifecycle state. Final USB acceptance still requires a selected physical-device run covering mapping evidence, authenticated reachability, restart/unplug/sleep recovery and scoped teardown.

Changes remain uncommitted in the existing dirty worktree.
