# P0 checklist supervisor audit — 2026-09-06

## Verdict

The formal checklist scope is `.scratch/p0-feasibility`: 10 issue files and 79 acceptance ticks. Skill templates and examples under `.agents/` are not project work items.

P0 validation is **not complete**. After checking each tick against implementation, tests, and target-environment evidence, 30 of 79 ticks are supported and 49 remain open. Ticket 10 is complete only as publication of the current capability matrix; it does not declare tickets 01–09 or production release complete.

| Ticket | Supported | Total | Remaining gate |
|---|---:|---:|---|
| 01 Codex app-server | 5 | 7 | Successful authenticated quota/usage run and sanitized real-account evidence |
| 02 Claude statusLine | 6 | 8 | Complete real failure matrix and real installation/account/config coexistence evidence |
| 03 Copilot boundaries | 5 | 8 | Real Personal/Business/Enterprise access, authoritative response, and versioned real evidence |
| 04 Antigravity sources | 3 | 8 | Real structured account/window semantics, real version/localization fallback matrix, and versioned evidence |
| 05 Windows lifecycle | 1 | 8 | Actual Tauri shell and Windows process/tray/startup/restart evidence |
| 06 Clean install/WebView2 | 1 | 8 | Installable Tauri package on a clean Windows VM, lifecycle logs, and measurements |
| 07 Selected-device USB | 1 | 8 | Real loopback listener, ADB executable, authorized physical device, reconnect, and targeted teardown |
| 08 Pair/session security | 0 | 8 | HTTP middleware, DPAPI persistence, negative attack cases, and real tablet evidence |
| 09 Dashboard delivery | 0 | 8 | Real HTTP/SSE implementation, tablet rendering, reconnect, refresh, latency, and resource evidence |
| 10 Capability publication | 8 | 8 | Publication complete; capability validation remains constrained |

## Corrections made under supervision

- Reopened prior checks that only had fixture state-machine evidence but claimed actual Tauri/Windows lifecycle or real Provider Account behavior.
- Made Claude statusLine coexistence reversible and conflict-safe, preserved prior output, and aligned output to the canonical status axes.
- Added distinct Codex optional-usage failure diagnostics without discarding a valid quota Observation or inventing Source usage.
- Added Copilot failure classes and explicit preview-SDK stability, permissions, and Billing API comparison.
- Added Antigravity quota-map support, unknown preservation, exact fallback version gating, and distinct authentication/command diagnostics.
- Prevented null clean-VM measurements or missing lifecycle operations from passing the installer evaluator.
- Corrected per-device ADB reverse ownership/no-rebind/teardown semantics and added multi-device and unauthorized cases.
- Corrected pairing to eight digits, 120 seconds, and five attempts; separated Forget Account, Clear Pairing, and Reset AgentMeter; made route authorization and missing DPAPI evidence explicit.
- Corrected Dashboard Snapshot account-quota deduplication for multiple Sources and tightened stream identity, heartbeat, reconnect, asynchronous 202 refresh, canonical signals, and monitor-only routes.
- Reconciled Ticket 10 as a publication-only task, linked every evidence package, recorded version gaps, and documented that no requirements or ADR change was silently adopted.

## Automated verification

Run from the repository root:

```powershell
cargo fmt --all -- --check
cargo clippy --offline --locked --all-targets -- -D warnings
cargo test --offline --locked
git diff --check
```

Final result: formatting passed, Clippy passed with warnings denied, all 50 integration tests passed, and the diff whitespace check passed.

## Required human/target-environment sequence

1. Provide authenticated test contexts for Codex, Claude, GitHub Copilot Personal/Business/Enterprise as available, and Antigravity. Run the evidence steps in each Provider evidence file without sharing credentials or billing secrets.
2. Build the actual Tauri 2 shell/package and supply a clean supported Windows 11 x64 VM. Run lifecycle, WebView2, repair/reinstall/upgrade/uninstall, and measurement sequences.
3. Supply an authorized physical Android tablet and ADB. Run selected-serial reverse mapping, reconnect, pairing/session attack, DPAPI, and targeted cleanup sequences.
4. Run the real HTTP/SSE tablet dashboard, capture monitor-only behavior and measurements, then repeat this audit before promoting any capability to `supported`.

Fixtures remain regression evidence only. They must not be used to check a tick whose wording requires a real Provider Account, clean VM/package, operating-system behavior, HTTP route, DPAPI boundary, ADB device, or tablet rendering.
