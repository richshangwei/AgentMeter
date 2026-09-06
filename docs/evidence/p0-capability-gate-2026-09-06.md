# AgentMeter P0 capability matrix and P1 gate — 2026-09-06

## Decision summary

P0 is **constrained; P1 production implementation is a no-go until the real-world gates below are completed**. The repository contains runnable, fixture-driven vertical slices for all nine experiments, but no fixture is treated as proof of provider, Windows packaging, physical-device, or end-to-end tablet viability. No capability is classified as fully `supported` for v1 on the evidence currently available. The constrained results define a safe v1 shape: fail closed, preserve unknowns, expose diagnostics, and keep experimental collectors and the tablet monitor boundary out of release claims until their real gates pass.

### Classification criteria

- **Supported**: the acceptance behavior is demonstrated with reproducible evidence in the target environment and prerequisites, with no unresolved release-blocking gap.
- **Constrained**: the protocol/design seam and negative behavior are demonstrated, but a target-environment, real-account, device, packaging, or measurement gate remains. It may proceed only behind the stated limitation and setup requirement.
- **Blocked**: a required boundary is either not demonstrated even by the fixture experiment, or an unresolved safety/security decision prevents exposing it in v1. A blocked capability is not enabled by a mock pass.

## Capability matrix

| Capability | Result | Reproducible evidence | Environment / prerequisites | Quality and maturity | v1 limitation and next gate |
|---|---|---|---|---|---|
| Codex app-server quota observation | **Constrained** | [Codex evidence](codex-p0-2026-09-06.md); `cargo test --offline --locked` and `cargo run --offline --locked --bin agentmeter-p0 -- codex collect --timeout-ms 5000` | Windows 11 build 26200; Codex CLI `0.151.0-alpha.7.1`; authenticated account session required | Fixture path: local-observed / experimental. Live run: unknown quota, needs-login / experimental | `account/read` worked, but rate-limit and usage reads required authentication; do not show a healthy quota card. Repeat with a real authenticated app-server session and record methods/fields.
| Claude statusLine quota | **Constrained** | `cargo test --offline --locked --test claude_statusline -- --nocapture`; isolated `agentmeter-claude-p0` fixture lifecycle | Claude installation, version, account, and existing statusLine config are required | Fixture structured source: measured / experimental; log fallback: degraded / experimental | Only opt-in, reversible integration; preserve user config and label log fallback degraded. Capture real before/after config, events, coexistence, and cleanup.
| GitHub Copilot quota boundaries | **Constrained** | `cargo test --offline --locked --test copilot_quota -- --nocapture`; isolated `agentmeter-copilot-p0` fixtures | Personal, Business, and Enterprise accounts plus API/SDK version and required billing permissions | Fixture authoritative shapes: local-observed / experimental; preview SDK remains experimental | Do not conflate AI Credits with legacy premium requests. Ship only contexts confirmed by real scoped requests; record untested plans/roles and permission failures.
| Antigravity structured/fallback sources | **Constrained** | `cargo test --offline --locked --test antigravity_quota -- --nocapture`; isolated `agentmeter-antigravity-p0` fixtures | Real authenticated Antigravity installation and explicit product version | Structured fixture: local-observed / experimental; text fallback: estimated / experimental | Structured source has precedence. Text fallback must remain version-bounded and visibly degraded; capture real schema, localization, version, and release approval before enabling.
| Windows desktop lifecycle | **Constrained** | `cargo test --offline --locked --test windows_lifecycle -- --nocapture`; `agentmeter-windows-p0` fixture | Supported Windows VM and actual Tauri shell/runtime | Fixture: local-observed / experimental | Single-instance/tray/startup behavior is modeled, not proven in Tauri. Validate sign-out, restart, abnormal termination, repeated launch, process cleanup, and supported OS/runtime versions.
| Clean install and WebView2 | **Constrained** | `cargo test --offline --locked --test windows_install -- --nocapture`; `agentmeter-windows-install-p0` fixture | Clean Windows VM with no project toolchain; package identity and WebView2 runtime | Fixture: local-observed / experimental; no clean-VM measurements | Installer cannot be called supported. Run install/repair/reinstall/upgrade/uninstall and WebView2 present/missing/outdated/download-failure cases; measure size, first launch, idle resources, and record VM/package evidence.
| Selected-device USB loopback | **Constrained** | [USB evidence](usb-p0-2026-09-06.md); `cargo test --offline --locked --test usb_loopback -- --nocapture` | Authorized physical Android device, explicit serial, ADB, loopback bind, no-rebind reverse mapping | Fixture: local-observed / experimental | Browser launch is never health. Operator must capture `adb devices`, `adb reverse --list`, authenticated endpoint probe, reconnect/sleep/host restart, and owned teardown.
| Device Pair and Tablet Session security | **Constrained** | [Pairing evidence](device-pairing-p0-2026-09-06.md); `cargo test --offline --locked --test pairing_security -- --nocapture` | Real tablet over selected USB; protected Windows secret persistence | Fixture negative/positive paths: local-observed / experimental | Keep pairing and session distinct; reject replay, guessed codes, bad origin/CSRF, and revoked sessions. Validate real reconnect, storage boundary, attack cases, and cleanup before production exposure.
| Dashboard Snapshot, SSE, async refresh | **Constrained** | [Dashboard evidence](dashboard-p0-2026-09-06.md); `cargo test --offline --locked --test dashboard_streaming -- --nocapture` | Authenticated paired tablet and measured host/device channel | Fixture: local-observed / experimental | Snapshot/revision/heartbeat/refresh seam is viable, but real rendering, reconnect recovery, latency, resource use, and monitor-only enforcement remain unverified. No configuration or credential mutation routes in v1.

## Cross-experiment decisions and scope

1. A provider account is identified separately from a source and quota window. Source precedence is structured provider output first; text/log fallbacks are explicitly degraded and never silently promoted.
2. Missing, stale, unsupported, paused, authentication-failed, malformed, and schema-changed states remain distinct. Unknown values stay unknown; no collector fabricates percentages, reset times, or account identity.
3. The desktop owns the collector, server, tray, startup registration, and persistence lifecycles. The tablet is monitor-only and reaches the host through loopback plus a selected-device ADB reverse mapping.
4. Pairing is a short-lived single-use exchange; a Device Pair is durable; a Tablet Session is separately revocable and expiring. Every route requires authorization and state-changing routes retain origin/CSRF protection.
5. Dashboard updates are complete revisioned snapshots over SSE with heartbeat and reconnect-to-latest behavior. Refresh is asynchronous, coalesced/throttled, and must not block on a slow collector.
6. P0 fixture binaries are evidence tools, not production integrations. Their `experimental` maturity is part of the result.

## P1 go / no-go gate

**Decision: NO-GO for production P1 release; GO for bounded P1 implementation and test work.** The architecture may proceed behind feature flags and fixture/regression tests, but release publication is gated on:

- one real authenticated evidence run for each provider, with version, account/role scope, representative sanitized fields, and cleanup/config coexistence results;
- a clean supported Windows VM/Tauri run covering lifecycle, install, WebView2, restart, upgrade, uninstall, and measured resource/latency data;
- one authorized physical Android device run covering selected serial, ADB reverse, authenticated reachability, reconnect/sleep/host restart, pairing/session revocation, and owned teardown;
- real tablet rendering and monitor-only authorization evidence for snapshot, SSE ordering/heartbeat, reconnect-to-latest, asynchronous refresh, and failure isolation;
- a final security review confirming no secrets in URLs/logs/fixtures and approved Windows secret protection; and
- an explicit release decision for the Antigravity text fallback and any provider context that remains permission- or plan-dependent.

Until these gates pass, v1 scope is limited to fixture-backed development, diagnostics, and explicitly labeled experimental adapters. A missing real-account/VM/device result is a release constraint, not evidence that the capability works in production.

## Prioritized follow-up

1. Run provider account gates (Codex, Claude, Copilot, Antigravity) and convert each row from constrained only when evidence is captured.
2. Validate Windows lifecycle and clean installation/WebView2 on a clean VM, then retain reproducible logs and measurements.
3. Validate USB selection/reverse/reconnect on a physical device; use that same channel for pairing/session security tests.
4. Exercise the real tablet dashboard against the authenticated channel and measure reconnect, latency, and idle resource behavior.
5. Re-run the complete matrix and perform the final release/security review before changing Ticket 10 to resolved.
