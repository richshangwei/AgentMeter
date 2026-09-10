# 05: Prove the Windows desktop lifecycle

**What to build:** A minimal Windows desktop slice that demonstrates AgentMeter's real runtime lifecycle: one application instance, a tray-controlled window, explicit exit, and optional hidden startup without leaving duplicate or orphaned background processes.

**Blocked by:** None.

**Status:** needs-info

- [x] Launching AgentMeter creates one functional desktop instance and exposes the minimal shell needed to identify the running build.
- [x] Closing the main window hides it while leaving the application available from the system tray.
- [ ] The tray can show the window and perform an explicit full exit that stops all AgentMeter-owned background work.
- [x] A second launch activates or reveals the existing instance instead of creating another collector, server, tray icon, or database writer.
- [x] Optional startup registration launches the application hidden and can be enabled and disabled without leaving stale startup entries.
- [ ] Sign-out, restart, abnormal prior termination, repeated launch, and tray recreation are exercised and checked for duplicate or orphaned processes.
- [x] The experiment records supported Windows versions, runtime versions, process observations, known lifecycle limitations, and reproducible evidence.
- [x] The outcome states whether the selected Rust core and Tauri desktop shell are viable for v1 or identifies a release-blocking lifecycle issue.

## Comments

- 2026-09-08 repair: Confirmed that both execution environments see the same mutex but only one can see the IPC window; upstream single-instance 2.4.3 continued startup when that window was missing. Local licensed vendor patch now rejects unreachable owners and bounds dispatch, with a real-process red/green regression. Same-environment reuse and full-exit command pass; no-owner exit is a no-op. Cross-desktop activation remains intentionally unavailable and focus/tray evidence remains outstanding, so criterion 4 is not reclosed. See `docs/handoffs/2026-09-08-single-instance-fix.md`.

- 2026-09-07 supervisor follow-up: Reopened criterion 4 after a Computer Use launch followed by a direct shell exit request left two resident test processes. Same-shell repeat tests passed, but the cross-environment anomaly remains unresolved. The exact-path test processes were cleaned up; investigate launch environment and single-instance coordination before reclosing this criterion.

- 2026-09-06: Added an isolated fixture-driven Windows lifecycle experiment covering single-instance reuse, tray hide/show/exit, owned-resource cleanup, restart/abnormal termination, and reversible startup registration. Status remains `needs-info` pending validation in a real supported Windows VM/Tauri shell.
- 2026-09-06 audit: Reopened the four previously checked desktop behavior ticks. `agentmeter-windows-p0` is a deterministic reference state machine, not a functional Tauri desktop, so synthetic counters cannot prove OS process, tray, focus, startup, or cleanup behavior. `docs/evidence/windows-lifecycle-p0-2026-09-06.md` records the local baseline and exact clean-VM/Tauri gate. ADR 0005 remains selected, but actual shell viability is release-blocked until that gate passes.
- 2026-09-07 production-path follow-up: Added and built the real Tauri `desktop-p0` shell with Tauri `2.11.5` and single-instance plugin `2.4.3`. Windows UI inspection confirmed build identity and functional WebView rendering; closing hid the window while the tray-owning process remained; a second launch exited and restored the existing window with exactly one resident process. The HKCU startup entry was created with `--hidden`, read back, removed, and confirmed absent. The tray Exit handler shares the verified full-exit function, but the tray menu itself was not directly clicked, so criterion 3 stays open. Sign-out/reboot/forced termination/tray recreation and clean-VM version evidence also stay open. See `docs/evidence/desktop-lifecycle-p0-2026-09-07.md`.

- 2026-09-09 readiness-probe follow-up: Added a side-effect-free `--probe-ready` command through the existing single-instance IPC path. It succeeds only when the owner accepts the message; with no owner the local Windows patch exits with code 3 rather than starting a resident application. The clean-VM lifecycle runner now uses this signal for first-launch and restart timing. Source contracts and a rebuilt NSIS pass, but the executable was not run in this batch, so no lifecycle tick is closed. See `docs/handoffs/2026-09-09-desktop-readiness-probe.md`.

- 2026-09-09 repeated-process acceptance: Added and executed `scripts/test-desktop-process-lifecycle.ps1` against the real Tauri debug executable on Windows 10.0.26200. Five successive secondary launches each dispatched to the ready primary, exited 0 and left exactly the original PID resident; the explicit full-exit request then left zero matching processes. This resolves the previously reproduced duplicate-process failure when combined with the local plugin's cross-desktop fail-closed rule: the supported same-interactive-desktop path reveals the owner, while an IPC-inaccessible desktop is rejected without starting another instance. Criterion 4 is reclosed with that documented scope. Direct tray-menu clicks, sign-out/reboot/forced termination and tray recreation remain open. See `docs/evidence/desktop-process-lifecycle-2026-09-09.json` and `docs/handoffs/2026-09-09-desktop-process-lifecycle.md`.

- 2026-09-09 abnormal-relaunch follow-up: Extended and reran the real-process runner. It force-terminated only the exact ready PID it created, observed zero remaining matching processes, launched a new primary, obtained readiness, dispatched another secondary to that new PID, and completed a graceful full exit with zero residents. This proves the abnormal-prior-termination and repeated-launch portions of criterion 6, but the compound item remains open pending sign-out, OS restart and Explorer/tray recreation evidence.

- 2026-09-09 environment-record acceptance: The lifecycle evidence now records the Windows 11 25H2 kernel baseline `10.0.26200.0`, Tauri `2.11.5`, single-instance plugin `2.4.3`, current WebView2 Runtime `152.0.4191.66`, executable product version/hash, primary/secondary PIDs, readiness, normal and abnormal exits, exact process counts, reproducible commands and all remaining lifecycle limitations. Criterion 7 is closed as an evidence-record requirement; it does not establish a minimum supported Windows build or replace the clean-VM matrix in issue 06. See `docs/evidence/desktop-lifecycle-p0-2026-09-07.md` and `docs/handoffs/2026-09-09-startup-diagnostics-webview.md`.
