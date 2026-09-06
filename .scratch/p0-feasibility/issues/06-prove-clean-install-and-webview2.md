# 06: Prove clean Windows installation and WebView2 handling

**What to build:** An installable package of the minimal desktop slice that succeeds on a clean Windows VM, handles the supported WebView2 deployment path, and leaves the machine in a predictable state after install, repair, restart, upgrade, and uninstall.

**Blocked by:** 05: Prove the Windows desktop lifecycle.

**Status:** needs-info

- [ ] A clean supported Windows VM with no project toolchain can install and launch the packaged desktop slice.
- [ ] Packaging uses the approved WebView2 download-bootstrapper strategy and documents what happens when the runtime is present, missing, outdated, or cannot be downloaded.
- [ ] Installer and application failures present actionable diagnostics rather than silently exiting or leaving an unusable background process.
- [ ] Restart, repair, same-version reinstall, supported upgrade, and uninstall are exercised with expected application and startup-registration behavior.
- [ ] Uninstall removes application-owned binaries and registration while preserving or removing local user data according to the approved product behavior.
- [ ] Installed size, download size, first-launch time, idle resource use, and runtime prerequisites are measured on the clean VM.
- [ ] Evidence includes the VM image details, package identity, runtime versions, test sequence, screenshots or logs, and reproducible results.
- [ ] The outcome states whether Windows distribution is supported, constrained, or blocked for v1 and records any installer or WebView2 release gate.

## Comments

- 2026-09-06: Added the fixture-driven `agentmeter-windows-install-p0` probe and integration tests. It models the approved WebView2 download-bootstrapper path, present/missing runtime and download failure diagnostics, lifecycle operation results, and nullable clean-VM measurements. Real clean Windows VM/package evidence is still required before distribution can be marked supported.
