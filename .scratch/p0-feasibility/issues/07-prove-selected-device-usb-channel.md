# 07: Prove the selected-device USB loopback channel

**What to build:** A host-to-tablet connectivity experiment that serves an authenticated loopback endpoint through ADB reverse to one explicitly selected Android device, survives the expected reconnect flow, and never mistakes browser launch for a healthy connection.

**Blocked by:** None.

**Status:** needs-info

- [x] The host service binds only to loopback and exposes a minimal authenticated health response without listening on LAN interfaces.
- [ ] Setup selects an explicit device serial and installs reverse forwarding with no-rebind protection so another mapping is not silently replaced.
- [ ] Host and device ports are represented separately and a port conflict, occupied mapping, unauthorized device, offline device, emulator, or multiple-device state produces an actionable result.
- [ ] The tablet can reach the host endpoint through the configured mapping and the evidence identifies the selected serial, transport, and active port pair.
- [ ] Browser launch is recorded separately from endpoint reachability and does not mark the tablet online until an authenticated request or session signal succeeds.
- [ ] Unplug, replug, ADB restart, host restart, tablet sleep, and stale mapping scenarios are exercised with a documented recovery path.
- [ ] Teardown removes only the intended mapping and does not affect unrelated devices or reverse-forward entries.
- [x] The outcome states whether selected-device USB transport is supported, constrained, or blocked for v1 and identifies any required operator step.

## Comments

2026-09-06: Added the isolated `agentmeter-usb-p0` fixture-driven experiment, tests, and evidence. The modeled protocol is supported when an explicitly selected authorized physical serial has a loopback host bind, owned no-rebind reverse mapping, and authenticated health response. Real-device validation is `needs-info` because this environment has no authorized Android device; an operator must rerun with a selected serial and capture ADB/device evidence before v1 publication.

2026-09-06 audit: Corrected the evaluator to treat ADB reverse entries as per-device, reject an unknown mapping on the selected device, require no-rebind and unrelated-entry preservation, and cover explicit selection among physical/emulator/TCP/offline devices plus unauthorized state. The outcome is blocked for v1 publication and the exact operator steps are in `docs/evidence/usb-p0-2026-09-06.md`. Criteria 1–7 remain open: there is no listener implementation or ADB executable/device evidence in this environment.

2026-09-07 production-path follow-up: Added a real `TcpListener` service that binds `127.0.0.1` on an OS-assigned port. A real TCP integration test proves `/health` rejects a missing Tablet Session without returning Provider data. This closes only the listener/health criterion; ADB, browser, reconnect, and device-scoped criteria remain open. See `docs/evidence/tablet-http-p0-2026-09-07.md`.

2026-09-09 ADB command-layer follow-up: Added a production `std::process::Command` path that requires an absolute ADB executable, an explicit authorized physical-USB serial, separate non-zero device/host ports and bounded timeouts. It creates the exact mapping with `reverse --no-rebind`, verifies it through the selected device, and removes it only while the same serial/port tuple is still owned. Real-process-double tests cover command order, conflict, timeout and changed-mapping refusal. No checklist item is closed because this environment still has no ADB installation or physical Android device, and the command layer is not yet wired into the runnable tablet host. See `docs/handoffs/2026-09-09-adb-command-layer.md`.

2026-09-09 runnable USB-host follow-up: Wired the command layer into `agentmeter-tablet-host` behind the complete `--adb ABSOLUTE_PATH --serial SERIAL --device-port PORT` option group. Readiness records the selected serial, USB transport, distinct active ports and verified mapping, while `browser_launch` remains `not_requested` and `authenticated_health` remains `not_observed`. The separate `open` operator command launches the URL without changing health state, and normal exit verifies and removes only the owned mapping. Real child-process tests cover partial-option rejection and the complete setup/open/teardown command sequence. Formal criteria remain open pending a physical-device run and lifecycle evidence. See `docs/handoffs/2026-09-09-usb-host-controller.md`.

2026-09-09 authenticated-activity follow-up: Added a safe host-visible transport status containing only whether a session was established, authenticated request count, last authenticated route and timestamp. Pair/session success and valid private requests advance it; missing authorization does not. The host `status` command also reports browser-launch state, and a real-process-double test proves `open` leaves authenticated activity at zero. No secrets or online claim are emitted. Formal USB ticks remain open until the same signals are captured from a physical tablet. See `docs/handoffs/2026-09-09-authenticated-transport-status.md`.

2026-09-09 recovery-control follow-up: Added owned/missing/changed reverse-mapping inspection to the live ADB layer and exposed it through host `status`. The explicit `recover-usb` path recreates only a proven-missing mapping; an owned mapping is left alone, and an externally changed mapping is neither replaced nor removed. Real-process-double tests simulate ADB mapping loss and external replacement through the full host lifecycle. These are deterministic recovery-path tests, not physical unplug/replug, ADB restart, host restart or sleep evidence, so the formal recovery criterion remains open. See `docs/handoffs/2026-09-09-usb-recovery-control.md`.

2026-09-09 desktop USB integration follow-up: Wired the production ADB command layer into Tauri with explicit absolute ADB path, serial and device port controls. The desktop stores separate host/device ports, verifies owned mappings before browser launch, recovers only missing mappings, refuses changed mappings, and tears down only an owned mapping. Status/activity distinguish mapping and browser-launch state from a currently valid authenticated Tablet Session. Missing, changed, unavailable, disconnected, newly connected or recovered mapping generations revoke old sessions and SSE authorization before reuse. Fake-ADB/root tests and desktop contracts pass, but no real ADB executable or physical tablet was exercised, so criteria 2–7 remain open. See `docs/handoffs/2026-09-09-desktop-usb-integration.md`.
