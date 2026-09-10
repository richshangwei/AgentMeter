# 2026-09-10 Continue Claude desktop handoff

## 2026-09-10 Dynamic tablet monitors, silent collection, and updates

### Goal & acceptance criteria
- [ ] Desktop background refresh never opens a visible console window.
- [ ] Tablet dashboard exposes an accessible fullscreen control and prioritizes quota data over explanatory copy.
- [ ] Monitor tiles can be added or removed at runtime, persist across reloads, and never drop below one visible slot.
- [ ] Settings allow monitor count/source selection and a manual update check.
- [ ] App startup performs a non-blocking GitHub release check; failure never blocks monitoring.
- [ ] Unavailable providers show provider-specific, step-by-step install/sign-in/retry guidance.
- [ ] Existing pairing, monitor-only tablet permissions, stale-data semantics, and four-provider defaults remain intact.

### Plan
- [ ] Checkpoint A: build tight repro/tests for console process flags and current tablet/settings behavior.
- [ ] Checkpoint A: inspect existing provider, persistence, packaging, and GitHub release seams.
- [ ] Checkpoint B: implement the smallest silent-process and dynamic monitor/settings slice.
- [ ] Checkpoint B: implement fullscreen and a content-first responsive tablet visual system.
- [ ] Checkpoint C: add regression coverage for min-one, persistence, update states, and provider guidance.
- [ ] Checkpoint C: run targeted tests, Clippy/build, browser geometry/a11y checks, then full local verifier.
- [ ] Checkpoint D: document release/update trust boundaries, rollback, and any clean-VM or physical-tablet limits.

### Risk & rollback
- Risk: medium. Affected components: Windows child-process creation, tablet UI state, desktop settings, and release-network behavior.
- Rollback: revert this section's focused files; no schema/data deletion. Persisted monitor preferences must tolerate absence and unknown future provider IDs.
- Rollout: startup update checks are advisory and fail closed; installation remains an explicit user action.
- Signals: no visible console regression test, update state/result, tablet render geometry, provider setup CTA availability.

### Dependencies & environment
- Windows/Tauri/Rust, vanilla browser UI, offline locked Cargo for normal verification.
- GitHub release checks require network only at runtime; tests use fixtures/mocks.
- Physical tablet and signed production releases remain separate acceptance boundaries unless available locally.

### Working notes
- Domain invariant: the tablet remains monitor-only; settings that mutate desktop/provider configuration must stay on desktop unless an existing contract explicitly permits otherwise.
- Preserve unknown observations as unknown; never fabricate zero/full quota.

## 2026-09-10 Tablet request_parse_failed
- [x] Reproduce delayed browser request failure on Windows before fixing.
- [x] Restore blocking mode on per-connection workers; allow verified USB origin only.
- [x] Add fragmented request and USB origin/pairing/revocation regression tests.
- [x] Run root/desktop tests and Clippy; build and inspect new NSIS installer.

Installer: desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter-20260910-tablet-http-fixed-setup.exe (70,601,411 bytes), SHA256 9C6B70155613087291305EE0398336D8D3EAC19902A5F591BF10F8C94C477AC3. NSIS inspection passed, embedded version 0.1.0. Physical tablet acceptance remains pending because ADB lists no device. Details: docs/evidence/tablet-http-fix-2026-09-10.md.

Risk: medium, HTTP and origin boundary. Loopback binding, pairing and CSRF remain required. Rollback: revert this task's socket/origin changes and rebuild; no stored pairs are deleted.
Environment: Windows, offline locked Cargo, existing Tauri/NSIS toolchain.
Evidence: delayed_fragmented_browser_request_is_not_rejected failed with connection abort before set_nonblocking(false), passed after. Full tests found an inherited missing transport_id validation despite the existing USB test and comment; restored the intended check.

## Acceptance criteria
Compact command bar and four quota cards fit the 1080 x 640 default window with tablet settings collapsed. Accurate remaining-percent bars and failure chips. Preserve tablet controls. Require actual WebView evidence before claiming live 4/4 or removing temporary diagnostics.

## Checklist
- [x] Inspect repository and handoff; compact UI was not saved locally.
- [x] Implement compact UI and regression coverage.
- [x] Verify actual desktop provider rendering and decide diagnostic cleanup: real WebView 4/4; diagnostics removed.
- [x] Run scripts/verify-local.ps1 and browser layout checks.
- [x] Rebuild release/installer, inspect bundled runtime and version, and document results.

## Risk & Rollback
Low-risk UI changes; preserve unrelated working-tree changes. Restore only this task's five UI/config/test files from task baseline if needed. Do not change account trust or login settings.

## Dependencies & Environment
Windows PowerShell, Rust/Cargo, Node, Tauri/NSIS and existing bundled quota helper. Real provider checks depend on existing local logins and network access.

## Working Notes
Screenshot is a handoff reference, not proof of saved changes or successful verification. Existing handoff requires real WebView 4/4, not a CLI-only result. No tasks/lessons.md existed at session start.


## Results
Compact UI and diagnostic cleanup completed. Full local verifier and Edge layout checks passed. Actual pre-cleanup WebView4/4 evidence captured. NSIS installer built and statically inspected; not installed or clean-VM tested. See docs/evidence/desktop-compact-ui-2026-09-10.md.

