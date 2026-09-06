# 05: Prove the Windows desktop lifecycle

**What to build:** A minimal Windows desktop slice that demonstrates AgentMeter's real runtime lifecycle: one application instance, a tray-controlled window, explicit exit, and optional hidden startup without leaving duplicate or orphaned background processes.

**Blocked by:** None.

**Status:** needs-info

- [ ] Launching AgentMeter creates one functional desktop instance and exposes the minimal shell needed to identify the running build.
- [x] Closing the main window hides it while leaving the application available from the system tray.
- [x] The tray can show the window and perform an explicit full exit that stops all AgentMeter-owned background work.
- [x] A second launch activates or reveals the existing instance instead of creating another collector, server, tray icon, or database writer.
- [x] Optional startup registration launches the application hidden and can be enabled and disabled without leaving stale startup entries.
- [ ] Sign-out, restart, abnormal prior termination, repeated launch, and tray recreation are exercised and checked for duplicate or orphaned processes.
- [ ] The experiment records supported Windows versions, runtime versions, process observations, known lifecycle limitations, and reproducible evidence.
- [ ] The outcome states whether the selected Rust core and Tauri desktop shell are viable for v1 or identifies a release-blocking lifecycle issue.

## Comments

- 2026-09-06: Added an isolated fixture-driven Windows lifecycle experiment covering single-instance reuse, tray hide/show/exit, owned-resource cleanup, restart/abnormal termination, and reversible startup registration. Status remains `needs-info` pending validation in a real supported Windows VM/Tauri shell.
