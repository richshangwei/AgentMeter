# 07: Prove the selected-device USB loopback channel

**What to build:** A host-to-tablet connectivity experiment that serves an authenticated loopback endpoint through ADB reverse to one explicitly selected Android device, survives the expected reconnect flow, and never mistakes browser launch for a healthy connection.

**Blocked by:** None.

**Status:** ready-for-agent

- [ ] The host service binds only to loopback and exposes a minimal authenticated health response without listening on LAN interfaces.
- [ ] Setup selects an explicit device serial and installs reverse forwarding with no-rebind protection so another mapping is not silently replaced.
- [ ] Host and device ports are represented separately and a port conflict, occupied mapping, unauthorized device, offline device, emulator, or multiple-device state produces an actionable result.
- [ ] The tablet can reach the host endpoint through the configured mapping and the evidence identifies the selected serial, transport, and active port pair.
- [ ] Browser launch is recorded separately from endpoint reachability and does not mark the tablet online until an authenticated request or session signal succeeds.
- [ ] Unplug, replug, ADB restart, host restart, tablet sleep, and stale mapping scenarios are exercised with a documented recovery path.
- [ ] Teardown removes only the intended mapping and does not affect unrelated devices or reverse-forward entries.
- [ ] The outcome states whether selected-device USB transport is supported, constrained, or blocked for v1 and identifies any required operator step.
