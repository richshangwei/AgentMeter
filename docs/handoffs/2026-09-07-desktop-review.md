# Desktop lifecycle review handoff — 2026-09-07

## Disposition

The chief supervisor required reopening #05.4: same-shell launch/exit passes do not resolve a cross-launch-environment run that left two resident processes. See `../evidence/desktop-lifecycle-p0-2026-09-07.md`. No desktop code changed in this batch.

## Investigation completed

Read the installed `tauri-plugin-single-instance 2.4.3` Windows source. The coordination names derive from the configured application identifier, not executable path casing. The plugin calls `CreateMutexW`, then on `ERROR_ALREADY_EXISTS` searches a hidden window and sends `WM_COPYDATA`. If the mutex exists but `FindWindowW` returns no window, setup returns success without forwarding or exiting. This is a concrete unhandled branch worth testing; it is not proof of the earlier anomaly's cause. Windows session, desktop, access-token/integrity and sandbox boundaries were not measured during that anomalous run.

## Next reproducible work

Capture session/desktop/token context for both launch methods, test rapid simultaneous launches and the existing-mutex/missing-window condition, then add a fail-closed bounded coordination path if reproduced. Do not change OS security settings or bypass isolation. Verify direct tray Show/Exit plus owned service shutdown before closing #05.3. Clean-VM installer, reboot/sign-out and runtime/resource evidence remain open.

## Cleanup

The earlier two exact-path test processes were terminated; subsequent same-shell exit tests recorded zero residents. No processes were launched or terminated in this review batch. Startup configuration was not changed in this batch. Changes remain uncommitted.
