# Windows desktop lifecycle P0 evidence — 2026-09-06

## Result

**Constrained and release-blocking for the actual desktop shell.** A deterministic Rust reference state machine covers intended launch, hidden startup, close-to-tray, tray restore/exit, repeated launch, abnormal termination, restart, tray recreation, and owned-resource counts. It is not a Tauri application and cannot prove real Windows process, tray, focus, startup-registration, sign-out, or orphan cleanup behavior.

The local build host reported Windows kernel `10.0.26200.0`, `rustc 1.95.0`, and `cargo 1.95.0`. The product target is Windows 11 x64; the minimum supported Windows 11 build and actual Tauri/WebView2 runtime versions remain unproven. Windows 10 is outside v1 scope.

## Reproduction

```powershell
cargo test --offline --locked --test windows_lifecycle -- --nocapture
cargo run --offline --locked --quiet --bin agentmeter-windows-p0 -- --fixture tests/fixtures/windows/lifecycle.json
```

The fixture result is reference-model evidence only. Its synthetic counters are not OS process observations.

## Required real-desktop gate

Build and package the minimal Tauri 2 shell, identify the build in its UI, and test it in a clean supported Windows 11 x64 VM. Capture process lists and tray/window behavior for normal launch, at least five repeated launches, close/show/focus, explicit exit, startup enable/disable and hidden login, sign-out, reboot, forced termination followed by relaunch, Explorer/tray recreation, and shutdown while owned background work is active. Verify exactly one tray icon, Collector, HTTP server, notifier, ADB mapping owner, and database writer, then record OS, Tauri, WebView2, and package versions.

ADR 0005 remains the selected architecture, but viability is not established until this gate passes.
