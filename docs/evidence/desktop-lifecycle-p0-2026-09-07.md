# Tauri desktop lifecycle production-path evidence — 2026-09-07

## Build

The standalone `desktop-p0` package is a real Tauri desktop shell, not the earlier lifecycle fixture. It pins Tauri `2.11.5`, `tauri-plugin-single-instance` `2.4.3`, and `tauri-build` `2.6.3`; it compiles offline on Windows with Rust/Cargo `1.95.0`.

```powershell
cargo build --manifest-path desktop-p0/Cargo.toml --offline
cargo clippy --manifest-path desktop-p0/Cargo.toml --offline --all-targets -- -D warnings
```

Both commands passed. The host kernel was `10.0.26200.0`. The exact WebView2 runtime version used by this process was not isolated from other running WebView2 processes and remains an evidence gap.

## Observed lifecycle

- The executable opened one WebView window titled `AgentMeter P0 — build 0.1.0`; accessibility inspection returned the build identity and `Desktop shell is running` content.
- Closing the Windows close button removed the visible window while the AgentMeter process remained resident and the tray icon had been created successfully during setup.
- A second launch while visible exited immediately and left one resident process. A second launch while hidden exited immediately, restored the existing window, and still left one resident process.
- The startup command created `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\AgentMeterP0` with the quoted executable plus `--hidden`. Running that exact hidden form produced one resident process and no targetable AgentMeter window; the exit probe then returned the resident count to zero. Disable removed the exact registry value and a follow-up read confirmed it was absent.
- The shared full-exit function was invoked through a second-instance `--request-exit` probe: the request process exited and the resident-process count changed from one to zero. The tray Exit menu invokes the same function, but a direct tray-menu click was not captured.
- The observed AgentMeter process working set was approximately 42.01 MB on this development host. This is not a clean-VM or packaged idle-resource measurement.

## Remaining gate

Additional repeatability finding: a later launch through the Windows Computer Use helper followed by a direct shell `--request-exit` invocation left two resident test processes. The cause was not established. Both exact-path test processes were stopped. Repeating launch and exit with `Start-Process` from the same shell then passed twice (second launch exited with one resident instance; exit request left zero). The same-shell success does not resolve the cross-launch-environment anomaly; broader single-instance viability requires investigation.

Do not close the tray-menu criterion until Show and Exit are directly exercised from the Windows notification area. Sign-out, reboot, forced termination/relaunch, Explorer/tray recreation, actual owned Collector/server/database cleanup, clean-VM OS support, WebView2 version, and packaged runtime measurements also remain required.

## 2026-09-09 repeated-process acceptance

The bounded `scripts/test-desktop-process-lifecycle.ps1` runner was executed against the current real Tauri debug executable on Microsoft Windows NT 10.0.26200.0. IPC readiness succeeded. Each of five secondary `--hidden` launches exited with code 0 while process inspection found exactly one resident process with the original primary PID. A final `--request-exit` exited 0, the primary exited within the deadline, and the matching resident count became zero. The versioned JSON record, including executable SHA-256 and product version, is in [desktop-process-lifecycle-2026-09-09.json](desktop-process-lifecycle-2026-09-09.json).

The same runner then created a fresh ready primary, force-terminated that exact owned PID to model abnormal prior termination, and observed zero matching residents. A new launch reached readiness, one secondary launch reused the new primary PID, and explicit full exit again left zero. This closes those subcases but not the compound sign-out/reboot/Explorer-recreation requirement.

Together with the previously observed hidden-window reveal and the Windows plugin patch that rejects an IPC-inaccessible cross-desktop owner without continuing startup, this closes the second-launch/single-instance criterion for the supported same-interactive-desktop scope. It does not close direct tray-menu, sign-out, reboot, abnormal termination, Explorer/tray recreation or clean-VM gates.

The current-machine WebView2 registry probe found runtime `152.0.4191.66` under the x64 HKLM EdgeUpdate client key. With the existing Windows 11 25H2/build 26200 baseline, pinned Tauri/plugin versions, executable version/hash, JSON PID observations and documented same-desktop/direct-tray/restart limitations, the lifecycle experiment's environment-and-evidence record is complete. This is a tested baseline, not a minimum-supported-build determination.
