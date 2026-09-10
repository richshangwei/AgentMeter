# Windows single-instance repair handoff — 2026-09-08

Single-agent batch using the diagnosing-bugs workflow. No independent review is claimed.

## Confirmed cause

The same compiled executable launched from two execution environments in Windows session 3 could see the same `com.agentmeter.p0-sim` mutex (valid handle, ERROR_ALREADY_EXISTS 183). Only the first environment could find `com.agentmeter.p0-sic` / `com.agentmeter.p0-siw`. The second could not see that IPC window. Upstream 2.4.3 continued startup when the mutex already existed but FindWindowW returned null, creating another resident process. Waiting for the fully initialized primary did not resolve the mismatch.

## Delivered

Vendored the already-cached tauri-plugin-single-instance 2.4.3 under desktop-p0/vendor with its licenses. Cargo uses an explicit local patch and updated lockfile; no registry cache files were changed. Windows now rejects failed mutex creation, waits at most two seconds for the owner's IPC window, rejects an unreachable owner, and bounds message dispatch to two seconds. An exit-only invocation without an owner now exits without starting a desktop application.

This intentionally preserves Windows desktop isolation. Cross-desktop activation is not supported: the second launch fails closed instead of starting another collector/writer. A normal same-desktop launch still forwards to the existing instance.

## Verification

- Regression script: `desktop-p0/tests/instance-probe.ps1`.
- Against old executable, isolated launch failed the script: secondary PID 57452 remained resident beyond 8 seconds. The test terminated its own secondary process.
- Against patched executable, isolated launch exited with 101 within approximately 2.7 seconds while primary PID 43816 remained alive.
- Same-environment repeat launch exited 0, with primary retained.
- Same-environment `--request-exit` exited 0 and primary terminated within the asserted deadline.
- `--request-exit` with no primary exited 0 without staying resident.
- Desktop unit tests: 3 passed; offline locked build and all-targets Clippy with warnings denied passed.

Original reproduction processes 83056 and 41340 were terminated only after checking their paths against this project's debug executable. No user account settings or credentials changed. No preview is intentionally left running from this batch.

## Remaining

The second-launch acceptance checkbox stays open because this batch proves process coordination but does not visually verify activation/focus, and isolated desktops correctly reject rather than reveal the first window. Physical tray exit, collector-child cleanup, sign-out/restart, and clean-VM validation remain separate gates. Formal count remains 47/79 complete, 32 open. Continue source configuration persistence, authenticated Provider integration and tablet client work. Changes remain uncommitted.
