# AgentMeter

AgentMeter is a Windows desktop and paired-tablet quota monitor. The default desktop workflow now uses the four live collection paths verified against this machine's accounts, rather than asking users for report paths or Billing configuration. Clean-VM installation, public release readiness and physical-tablet validation remain separate gates.

## Current desktop workflow

See [the implemented automatic-quota desktop and release evidence](docs/evidence/automatic-quota-desktop-2026-09-10.md) for the current build and verified scope.

Open AgentMeter after signing in to the official tools. It discovers them automatically and refreshes all four quota cards; background updates continue from the system tray. No quota source paths, account slugs or Billing endpoints are required.

- Codex: official App Server rate limits.
- Copilot: official CLI `account.getQuota`, with the active GitHub CLI account as a fallback. Premium interactions are not AI credits; reset time is not assumed trustworthy.
- Claude: automatic official `/usage` terminal reading. First use may require clicking **啟用 Claude 讀取**, which trusts only AgentMeter's dedicated workspace. No model tools are enabled. Terminal parsing remains experimental.
- Antigravity: bundled official CLI `/usage`. Text-format compatibility remains experimental.

The desktop and tablet share observations and refresh handling. Failed refreshes preserve the previous quota with an explicit stale marker; missing values are never replaced with a fabricated zero or full allowance. The tablet can select any currently advertised Provider cards, always keeps one empty add slot, and has a fullscreen control. Legacy experiment commands below remain for historical regression coverage, not the default UI.

### Signed GitHub updates

The desktop checks for a signed GitHub release at startup without blocking monitoring; installation always requires an explicit confirmation. A release build must provide the exact latest manifest URL and Tauri public key at compile time, then enable updater artifacts with the overlay config:

```powershell
$env:AGENTMETER_UPDATE_ENDPOINT = 'https://github.com/OWNER/REPOSITORY/releases/latest/download/latest.json'
$env:AGENTMETER_UPDATE_PUBLIC_KEY = '<Tauri updater public key>'
$env:TAURI_SIGNING_PRIVATE_KEY = '<private key or path>'
Push-Location desktop-p0
cargo tauri build --config tauri.updater.conf.json
Pop-Location
```

Publish the generated installer, signature and `latest.json` from that build to the GitHub release. Normal local builds remain unsigned when those release-only values are absent. Tauri's updater verifies the signed artifact before installation; see the [official updater guide](https://v2.tauri.app/plugin/updater/) and [GitHub Releases API documentation](https://docs.github.com/en/rest/releases/releases).

Build-time preparation (developers only; the packaged app includes its runtime):

```powershell
node scripts/prepare-quota-smoke.mjs # first preparation only; refuses overwrite
./scripts/prepare-desktop-quota.ps1
```

The bundled first-party Node/ConPTY helper is a deliberate implementation departure from ADR 0006's all-Rust collectors, following the verified live approach. It accepts only a fixed provider identifier and an app-owned workspace, not arbitrary user scripts or commands.

## Build and test

Run the complete local regression checks with `./scripts/verify-local.ps1` in PowerShell. This includes the separate desktop crate and browser logic tests; it does not replace real-account, physical-device or clean-VM acceptance.

```powershell
cargo build --offline --locked
cargo test --offline --locked
```

The dependencies are pinned in `Cargo.lock`. Remove `--offline` only when a dependency refresh is intentional.

Build the real Tauri P0 desktop lifecycle shell separately:

```powershell
cargo build --manifest-path desktop-p0/Cargo.toml --offline
```

The shell identifies build `0.1.0`, closes to the system tray, uses the single-instance plugin first, supports `--hidden` startup, and provides reversible `--startup-enable` / `--startup-disable` commands. Clean-VM installer validation is still a separate release gate.

## Run the Codex experiment

Use the current user's Codex installation:

```powershell
cargo run --offline --locked --bin agentmeter-p0 -- codex collect
```

Use a sanitized transcript to reproduce normalization without a live account:

```powershell
cargo run --offline --locked --bin agentmeter-p0 -- codex collect --fixture tests/fixtures/codex/success.jsonl
```

The command writes one versioned JSON collection report to standard output. A live run with exit code `0` means a trustworthy Observation was produced; a fixture run is explicitly marked `source.mode = fixture_replay`, `source.replay = true`, `data_quality = local_observed`, and `freshness = unknown`, so it is regression evidence rather than provider proof. Exit code `1` means collection failed and the JSON contains a distinct `failure_code`; `observation` remains `null`. Exit code `2` means the command arguments are invalid.

The collector masks account email addresses in successful output and never copies Codex credentials. `account/usage/read` is probed only to record whether the installed app-server supports it; an unsupported method is reported rather than retried.

## P0 completion state

The auditable capability matrix is in `docs/evidence/p0-capability-gate-2026-09-06.md`. Production release remains gated on real Provider Account runs, a clean Windows/Tauri package run, and an authenticated physical Android tablet run. Each experiment has a matching evidence file under `docs/evidence/` with its reproduction command and remaining operator steps.

## Work handoffs

The [guided setup and Claude integration handoff](docs/handoffs/2026-09-09-claude-setup-flow.md) records automatic local setup checks, preview/backup/enable/restore actions, the built-in receiver, verification and the remaining installer/real-account limits. It follows the [automatic source discovery batch](docs/handoffs/2026-09-09-source-discovery.md).

The [startup diagnostics and WebView2 contract handoff](docs/handoffs/2026-09-09-startup-diagnostics-webview.md) records native startup-failure reporting, packaged diagnostic markers, exact installer runtime branches and two newly completed checklist items.

The [dynamic tablet monitors, silent collection and signed updates handoff](docs/handoffs/2026-09-10-tablet-dynamic-monitors-and-silent-updates.md) records the new fullscreen/data-first tablet view, expandable monitor slots, invisible background collection and release signing boundary.

The [packaged updater startup fix handoff](docs/handoffs/2026-09-10-updater-startup-config-fix.md) records the reproduced `desktop_startup_failed`, its non-WebView root cause, release-process proof and replacement installer hash.

The [desktop repeated-process lifecycle handoff](docs/handoffs/2026-09-09-desktop-process-lifecycle.md) records five real second-launch dispatches, one resident primary, bounded full exit, cross-desktop fail-closed scope and the reclosed single-instance tick.

The [live Provider readiness handoff](docs/handoffs/2026-09-09-live-provider-readiness.md) records the latest real Codex app-server handshake/authentication result and the local absence of runnable Claude, Copilot and Antigravity CLIs without changing credentials or settings.

The [desktop selected-device USB integration handoff](docs/handoffs/2026-09-09-desktop-usb-integration.md) records Tauri ADB controls, mapping ownership/recovery behavior, transport-generation Session revocation, current online semantics and the rebuilt NSIS artifact.

The [desktop TabletServer integration handoff](docs/handoffs/2026-09-09-desktop-tablet-integration.md) records explicit loopback lifecycle controls, DPAPI-backed pairing, ephemeral desktop code display, full-exit teardown and the rebuilt NSIS artifact.

The [tablet secret-surface handoff](docs/handoffs/2026-09-09-tablet-secret-surface.md) records query rejection, server-side no-store responses, sentinel non-disclosure tests and the remaining real-browser evidence boundary.

The [installer upgrade-runner handoff](docs/handoffs/2026-09-09-installer-upgrade-runner.md) records the all-or-none two-package upgrade contract, pre-mutation identity/version guards and still-unexecuted real upgrade boundary.

The [desktop readiness-probe handoff](docs/handoffs/2026-09-09-desktop-readiness-probe.md) records the side-effect-free single-instance readiness signal, five-minute resource sampling contract and rebuilt package evidence.

The [Claude test-isolation handoff](docs/handoffs/2026-09-09-claude-test-isolation.md) records the reproduced Windows temporary-path collision, atomic uniqueness fix, parallel regression test and 30-run post-fix proof.

The [installer lifecycle-runner handoff](docs/handoffs/2026-09-09-installer-lifecycle-runner.md) records the hash-pinned, mutation-gated clean-VM preserve/purge runner, its versioned JSON evidence and the still-unexecuted acceptance boundary.

The [NSIS explicit-purge handoff](docs/handoffs/2026-09-09-nsis-explicit-purge.md) records current-user packaging, opt-in `/PURGE`, update-safe legacy cleanup and the rebuilt installer.

The [startup/uninstall contract handoff](docs/handoffs/2026-09-09-startup-uninstall-contract.md) records the fixed Run-key mismatch, legacy cleanup, identifier-based data policy and rebuilt NSIS evidence.

The [NSIS static-inspection handoff](docs/handoffs/2026-09-09-nsis-static-inspection.md) records non-executing archive/version/hash/signature checks and the safe reusable inspection script.

The [NSIS bundle handoff](docs/handoffs/2026-09-09-nsis-bundle.md) records the first generated x64 setup artifact, hash/size, official Tauri CLI version, unsigned status and remaining clean-VM gate.

The [desktop bundle-preflight handoff](docs/handoffs/2026-09-09-desktop-bundle-preflight.md) records active NSIS/WebView2 configuration, downgrade protection, offline contract tests and the remaining missing Tauri CLI/clean-VM gate.

The [USB recovery-control handoff](docs/handoffs/2026-09-09-usb-recovery-control.md) records owned/missing/changed inspection, safe missing-only recovery and external-mapping refusal tests.

The [authenticated transport status handoff](docs/handoffs/2026-09-09-authenticated-transport-status.md) records the safe separation between browser launch and actual authenticated Tablet Session/request evidence.

The [runnable USB host controller handoff](docs/handoffs/2026-09-09-usb-host-controller.md) records the optional selected-device ADB integration, separate browser launch state and ownership-safe mapping teardown.

The [ADB command-layer handoff](docs/handoffs/2026-09-09-adb-command-layer.md) records the real bounded ADB process boundary, explicit physical-device selection, no-rebind verification and ownership-safe teardown tests.

The [authenticated browser end-to-end handoff](docs/handoffs/2026-09-09-browser-e2e.md) records actual mock-only pairing, four-card rendering, SSE revision advance, reload recovery and host-disconnect behavior.

The [consolidated verification handoff](docs/handoffs/2026-09-09-local-verification.md) records the current combined local checks and their acceptance limits.

The [real-process cancellation handoff](docs/handoffs/2026-09-08-cancellation-process-proof.md) records an actual idle subprocess cancellation and Windows resource-release test.

The [collector cancellation handoff](docs/handoffs/2026-09-08-collector-cancellation.md) records cancellable response waits and coordinated explicit desktop exit, with remaining process-ownership limits.

The [actual browser smoke handoff](docs/handoffs/2026-09-08-browser-smoke.md) records real page loading, validation and reload checks, and explicitly excludes authenticated-flow certification.

The [browser pair recovery handoff](docs/handoffs/2026-09-08-browser-pair-recovery.md) covers persistent cookie attributes, session restoration and remaining real-browser verification.

The [stable tablet origin handoff](docs/handoffs/2026-09-08-stable-tablet-origin.md) documents explicit host port selection and restart/conflict tests.

The [tablet stream resilience handoff](docs/handoffs/2026-09-08-tablet-stream-resilience.md) records watchdog, snapshot ordering and fragmented-event parsing tests.

The [tablet browser client handoff](docs/handoffs/2026-09-08-tablet-client.md) covers the pairing/Dashboard page, authenticated POST read routes, SSE recovery implementation and outstanding browser/device verification.

The [runnable tablet host handoff](docs/handoffs/2026-09-08-tablet-host.md) provides the mock-provider host command and real-process startup/shutdown test evidence; its browser client remains unfinished.

The [full regression audit](docs/handoffs/2026-09-08-full-regression-audit.md) records all 32 remaining gates, missing host/client implementation and the 80-test root-suite result after correcting test deadline races.

The [source settings handoff](docs/handoffs/2026-09-08-source-settings.md) covers explicit Claude path persistence and restore/forget behavior with storage tests.

The [single-instance repair handoff](docs/handoffs/2026-09-08-single-instance-fix.md) records the confirmed cross-desktop IPC cause, local dependency patch and real-process regression results.

The [Claude monitoring handoff](docs/handoffs/2026-09-08-claude-watch.md) records automatic local-report monitoring, cancellation/retry tests and a reproduced Windows single-instance failure.

The [desktop source integration handoff](docs/handoffs/2026-09-08-desktop-sources.md) covers Codex executable discovery, manual Claude report import, verification and the remaining authentication/UI gates.

The [desktop Dashboard handoff](docs/handoffs/2026-09-08-desktop-dashboard.md) documents the new four-card desktop preview, actual Codex refresh integration, UI verification and remaining live-source work. This batch was completed in single-agent mode.

Completed work batches have handoffs under [docs/handoffs](docs/handoffs). The latest [2026-09-09 Copilot live-evidence runner handoff](docs/handoffs/2026-09-09-copilot-live-runner.md) records the opt-in PowerShell runner, six-request matrix coverage, sanitized evidence contract, and honest 50/79 checked / 29 open boundary. The preceding [2026-09-09 persistent Copilot settings handoff](docs/handoffs/2026-09-09-desktop-copilot-settings.md) records schema-v2 save/restore/forget behavior, v1 migration, the current rebuilt NSIS identity, and direct launch. The [desktop Copilot integration handoff](docs/handoffs/2026-09-09-desktop-copilot-integration.md) records the shared collector module and functional card; its package hash is historical. The [Copilot live-contract handoff](docs/handoffs/2026-09-09-copilot-live-contract.md) records the six-endpoint process seam and Antigravity fail-closed decision. The [startup/WebView handoff](docs/handoffs/2026-09-09-startup-diagnostics-webview.md) records the two most recently closed acceptance ticks. The [2026-09-07 batch handoff](docs/handoffs/2026-09-07-p0-batch.md) retains the broader original work map. Write a new dated handoff after each subsequent batch and link its verification evidence.
