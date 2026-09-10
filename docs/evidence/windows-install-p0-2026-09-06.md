# Ticket 06 clean install and WebView2 P0 evidence

## Automated evidence boundary

`agentmeter-windows-install-p0` is a fixture evaluator, not an installer and not a clean-VM run. The evaluator now refuses to report a complete pass when any required lifecycle operation, user-data policy, or measurement is absent. The checked-in nominal fixture intentionally has null measurements and no restart observation, so it returns `needs-info` with the `VM-evidence-required` release gate. The WebView2 download-failure fixture returns `blocked` with an actionable WebView2 diagnostic.

Reproduce with:

```text
cargo test --offline --locked --test windows_install -- --nocapture
```

Result on 2026-09-06: 3 passed, 0 failed.

## Outcome

Windows distribution is **blocked for v1 publication** until a real Tauri package is exercised on a clean supported Windows VM. The approved design remains Tauri `downloadBootstrapper`; current-runtime must avoid an unnecessary download, missing-runtime must install or fail with repair guidance, and an outdated runtime must update with an explicit restart boundary.

## Required operator evidence

1. Record VM image/build, architecture, clean snapshot identity, package filename/SHA-256, package identity, WebView2 version, and whether network access is available.
2. From the clean snapshot, test runtime present/current, missing with successful download, missing with download denied, and outdated/update/restart. Capture installer/app logs and screenshots without secrets.
3. Exercise install, first launch, restart, repair, same-version reinstall, supported upgrade, and uninstall. Verify startup registration and absence of orphan AgentMeter processes after every transition.
4. Verify uninstall preserves `%APPDATA%/com.agentmeter.p0` and `%LOCALAPPDATA%/com.agentmeter.p0` by default and that the explicit data-removal checkbox removes both; record binaries and registrations before/after.
5. Measure installer download size, installed size, first-launch time, five-minute idle CPU, AgentMeter/WebView2 working sets, and prerequisites.

Do not replace these observations with populated fixture values; the fixture is only the validation schema for evidence captured on the VM.
