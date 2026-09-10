# Desktop compact UI continuation — 2026-09-10

## Completed implementation
- Restored the unsaved compact UI described in the Claude handoff screenshot: one command bar, four columns at desktop width, responsive two/one columns, aligned refresh actions, amber failure chips and 640px default window height.
- Remaining-percent bars show exact finite values (24.7% => width 24.7%), threshold colors and explicit unknown handling; tested bounds and thresholds.
- Removed temporary quota_trace/quota_rendered instrumentation after actual WebView acceptance. Permanent startup diagnostics and verbatim-path normalization regression test remain.
- Resolved the existing collapsible_if warning in the path-normalization fix without changing behavior.

## Actual desktop acceptance
A normal-user launch of the existing path-fixed release executable completed real collection on this machine. PID35892 produced a published record with no provider failure codes and a real ui_rendered acknowledgement with ready=4 and window counts [3,2,1,4]. Sanitized records are preserved in desktop-webview-quota-2026-09-10.json. This was the real Tauri WebView, not a native CLI-only probe or mocked browser. The test instance exited normally via --request-exit before rebuild.

This live evidence precedes compact UI rebuild; the new layout was separately verified in Edge with representative data. Do not describe fixture screenshots as real account screenshots.

## Verification
- powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1: PASS after final diagnostic cleanup; root and desktop tests, formatting, JavaScript tests/syntax and warnings-denied Clippy.
- node .scratch/verify-compact-ui.cjs: PASS; representative 3/2/1/4 quota windows, success/stale/error, desktop row and no horizontal overflow at640/390. Heights at1080x640: ready570.19px, stale587.78px, error396.94px, Claude trust-required443.94px. Expanded tablet settings at640px scroll vertically without horizontal overflow.
- rg found no quota_trace, quota_rendered or DEBUG-quota-desktop in desktop-p0/src and desktop-p0/ui.
- NSIS build: PASS, cargo tauri build --target x86_64-pc-windows-msvc --bundles nsis -- --offline --locked (from desktop-p0). Static inspect-nsis.ps1: PASS. Version0.1.0; bundled Node, Antigravity and quota collector scripts confirmed. Installer size70,586,478 bytes (67.3MiB), unsigned. SHA256: 6E0E20695804212B1F86447494CE86F1AD568CE75A602D07D0303DB16B04515E. Inspection: compact-ui-nsis-inspection-2026-09-10.json.

## Scope and limits
No account trust, credentials, startup registration, tablet pairing, or installed application data changed. Existing dirty worktree preserved. Installer is a local test build; clean-VM install/upgrade/uninstall has not been exercised in this task.


