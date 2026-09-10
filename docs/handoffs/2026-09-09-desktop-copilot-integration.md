# Handoff — Desktop Copilot Billing integration

Date: 2026-09-09 (Asia/Taipei)  
Mode: single-agent  
Checklist: 50/79 checked, 29 open (unchanged)

## Outcome

The GitHub Copilot desktop card is no longer a static placeholder. It now invokes the same live Billing collector used by the CLI and renders sanitized official usage while keeping remaining quota explicitly unknown. The rebuilt release application was launched for the user and the rebuilt NSIS package contains the new command and quota guard.

## Deep module and shared seam

The live GitHub process, validation, cancellation, timeout, output cap, failure classification, endpoint construction, response validation, and usage normalization now live in `src/copilot.rs`. Its small external interface is `UsageRequest::new(...)` plus `collect_usage(...)`. The CLI and Tauri desktop are callers of the same module; `gh` and `fake-gh` are the production and test adapters at the process seam.

This removes the risk of the desktop silently diverging from the already-tested CLI security and schema behavior.

## Desktop behavior

- The Copilot card accepts an absolute GitHub CLI path, Personal/Business/Enterprise context, account or organization slug, and AI Credits/Premium Requests meter.
- It never accepts a GitHub token. Authentication remains owned by the existing `gh` login.
- The Tauri command participates in the shared collector lifecycle; full application exit cancels a running GitHub CLI request and waits for ownership release.
- Only account kind/name, billing period, SKU, model, unit and net usage reach the browser view. Provider diagnostics, raw stderr, monetary amounts and unrelated response fields are stripped.
- Successful data is labeled official Billing usage / experimental collector. Remaining quota is always shown as unknown because the REST response does not expose allowance or reset.
- Authentication, permission, rate-limit, endpoint, timeout and schema failures produce sanitized status without retaining stale usage as fresh.

## Verification

The final local verifier passed:

```text
120 root Rust tests passed
11 desktop Rust tests passed
11 browser logic tests passed
PowerShell syntax, root/desktop formatting, JavaScript syntax and warnings-denied Clippy passed
```

Focused coverage includes 17 Copilot process/parser tests, 8 desktop bundle/contract tests and desktop view tests proving raw secrets and monetary fields do not cross into the browser view.

## Release artifact

- Path: `desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- Installer size: 2,088,432 bytes
- Installer SHA-256: `0083F6000BEFD7FCFEA016AE16D9C91988FAA7C8DBC70B0121ADC4B32A172BED`
- Installer Authenticode: `NotSigned`
- Embedded executable size: 9,555,456 bytes
- Embedded executable SHA-256: `BF9394E78F166651D0B01B5482DEE4AE9420A18E235998DF8BB826DF75BF75D0`
- Embedded executable Authenticode: `NotSigned`
- Product/file version: `0.1.0`
- WebView2 download component: present
- Startup diagnostic markers: present
- Copilot Billing command/API/quota-guard markers: present

The release application was started visibly from the rebuilt executable as PID 8532 and remained responsive. A second sandbox-context readiness probe reproduced the known cross-desktop single-instance isolation and showed its startup-failure dialog; that exact probe PID was closed, leaving only PID 8532. This is not counted as new cross-desktop lifecycle acceptance evidence.

## Formal acceptance boundary

No formal checkbox closes. GitHub CLI is absent and no authenticated Personal, Organization, or Enterprise account was supplied, so the two #03 real-provider criteria remain open. The desktop implementation is operator-ready but does not substitute process-double evidence for a live account response.

The formal count remains **50/79 checked, 29 open**.

Superseded package note: the later [persistent settings handoff](2026-09-09-desktop-copilot-settings.md) records the current rebuilt artifact and adds v1-compatible source persistence. The hash in this earlier handoff identifies only its own batch artifact.
