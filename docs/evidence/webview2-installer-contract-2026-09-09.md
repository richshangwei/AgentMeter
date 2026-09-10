# WebView2 installer and startup-diagnostic contract — 2026-09-09

## Built configuration

- Tauri bundler: locked `2.9.4` NSIS implementation.
- `desktop-p0/tauri.conf.json`: `webviewInstallMode.type = downloadBootstrapper`, `silent = true`.
- No `minimumWebview2Version` is configured because the P0 clean-VM evidence has not established an approved minimum runtime.
- The latest package contains `NSISdl.dll` and an embedded v0.1.0 AgentMeter executable.

## Exact generated-installer behavior

- Present runtime: the generated NSIS section reads the WebView2 Evergreen client version from x64/x86 HKLM and then HKCU. Any non-empty version skips bootstrapper download/install.
- Missing runtime: on a fresh install, NSIS downloads Microsoft's WebView2 bootstrapper from Tauri bundler's fixed Microsoft link, then executes it with `/silent /install`.
- Download unavailable: NSIS prints the localized download error and aborts with the Traditional Chinese WebView2-required/restart-installer guidance.
- Installer failure: a nonzero bootstrapper exit prints the exit code and aborts with the same actionable guidance.
- Existing old runtime: without an approved minimum version, it is currently treated as present and skipped. Automatic old-runtime update and application restart adoption remain AC-28 release gates. The earlier statement that every old runtime would update was not true for the built configuration and is superseded by this record.
- Update mode: a missing runtime is not installed in the template's update branch; the lifecycle/upgrade VM matrix must verify the supported old-package-to-new-package preconditions.

The current development host reports WebView2 Runtime `152.0.4191.66`. This is observed baseline information, not the minimum supported version.

## Application failure behavior

`desktop-p0/src/diagnostics.rs` replaces release-mode panic-only startup handling. A Tauri startup failure or startup-registration failure writes an app-owned bounded `startup-error.log` under `%LOCALAPPDATA%\com.agentmeter.p0\logs`, then shows a native Win32 error dialog containing a stable Failure Code, recovery guidance and the log path. The native dialog does not require WebView2. Control characters are collapsed and detail is capped at 2,048 characters.

The NSIS inspection step now fails unless the extracted executable contains both startup Failure Codes, the log name and the versioned diagnostic schema marker. This proves inclusion in the packaged executable; actual missing/outdated/download-failure execution still belongs to the disposable clean-VM gate.
