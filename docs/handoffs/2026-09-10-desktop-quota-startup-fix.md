# AgentMeter desktop quota / startup handoff

## User request

Finish the desktop quota integration. The acceptance bar is the real AgentMeter
window showing actual values for all four Providers: `4/4`. Do not accept a CLI
probe alone as proof.

## Current truth

- Workspace: `D:\WorkSpace\AgentMeter`
- The user reported the actual window showed `0/4` for Codex, Claude Code,
  GitHub Copilot, and Antigravity.
- The user then saw a startup error. The diagnostic file was:
  `C:\Users\richs\AppData\Local\com.agentmeter.p0\logs\startup-error.log`
- Its cause was:
  `failed to initialize plugin single-instance: AgentMeter is already running but its IPC window is unavailable in this Windows desktop`
- This was a stale/duplicate process issue during testing. After ending the old
  process, the current release process was responsive.
- The earlier successful native-quota evidence was misleading as a product
  acceptance test. It used a different process path/environment and did not
  verify the WebView rendered values.

## Root cause found and fix already applied

The desktop Tauri resource directory can be returned as a Windows verbatim local
path such as `\\?\C:\...`. The bundled Node runtime cannot load its entrypoint or
use that working directory with this prefix; it exits with no stdout and Node
reports `EISDIR` while resolving `D:`. This caused all four Providers to fail
before collection started.

`desktop-p0/src/auto_quota.rs` now normalizes only verbatim local-drive paths at
the controlled Node subprocess boundary. Verbatim UNC paths remain unchanged.
The change has a regression unit test.

The exact diagnostic and minimized repro are documented in:
`docs/evidence/desktop-quota-path-diagnosis-2026-09-10.md`

## What has been verified

- Desktop Rust tests: 19 passed, including the new path-normalization test.
- Release build: passed with `cargo build --manifest-path desktop-p0/Cargo.toml --release --offline --locked` after closing the old process.
- The old diagnostic run before the fix showed `node_exists=true`,
  `entry_exists=true`, child exit `1`, stdout `0`, and WebView `ready=0`.
- A later native CLI probe after rebuilding failed to find some Provider tools in
  that execution environment. Do not claim this is an account failure without
  reproducing from the actual desktop process.
- Current release process was last observed responsive at PID `51748`; its PID is
  not stable. Check before acting.

## Remaining work, in order

1. Ensure no old `agentmeter-desktop-p0.exe` process is holding the release binary.
   Ask the user to close AgentMeter if needed; use only a narrowly targeted
   process action, never a broad kill.
2. Start the release executable from the normal user desktop, not a hidden or
   alternate desktop, so the single-instance IPC plugin can initialize.
3. Wait for the initial collection to finish. Inspect the UI and/or use the
   existing readiness mechanism. The final acceptance must be actual WebView DOM
   values or a user-visible screenshot showing `4/4`, not only
   `scripts/test-native-quota.mjs`.
4. If it fails, run the already built-in opt-in diagnostics with
   `--quota-diagnostics` and inspect only:
   `C:\Users\richs\AppData\Local\com.agentmeter.p0\logs\quota-diagnostic-<pid>.jsonl`.
   It is designed to exclude credentials and raw Provider output.
5. Remove the temporary `DEBUG-quota-desktop` instrumentation and its UI
   acknowledgement after the original scenario is green. Do not remove the
   regression test. Re-run `scripts/verify-local.ps1`.
6. Rebuild the NSIS installer only after the real WebView `4/4` acceptance passes.

## Important code locations

- Collector subprocess and path normalization: `desktop-p0/src/auto_quota.rs`
- Bundled Node entrypoint: `scripts/quota-desktop.mjs`
- Shared Provider collectors: `scripts/quota-smoke.mjs`
- Desktop startup and single-instance handling: `desktop-p0/src/main.rs`
- UI rendering and refresh: `desktop-p0/ui/dashboard.js`
- Temporary diagnostics: `desktop-p0/src/diagnostics.rs`
- UI test: `desktop-p0/tests/auto-quota-ui.test.cjs`

## Guardrails

- Never use Billing endpoints or ask the user for source paths for quota.
- Never print or persist tokens, account identity, raw CLI output, or full
  Provider responses.
- Keep the four Provider order: `codex`, `claude`, `copilot`, `antigravity`.
- A `PASS` from the native helper is not sufficient; require the desktop UI
  acceptance described above.
- Preserve unrelated dirty worktree changes. Do not reset or clean the repo.
