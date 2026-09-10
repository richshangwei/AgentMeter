# 09: Prove Dashboard Snapshot streaming and asynchronous refresh

**What to build:** A real tablet dashboard slice that loads a complete Dashboard Snapshot for four provider mocks, receives ordered SSE revisions over the paired USB session, and requests asynchronous refresh without turning a slow collector into a blocked HTTP request.

**Blocked by:** 08: Prove the Device Pair and Tablet Session security boundary.

**Status:** needs-info

## Comments

- 2026-09-09 desktop integration follow-up: The Tauri UI can now explicitly start/stop the existing authenticated TabletServer, generate a two-minute pair code, and clear pairing; full app exit owns server teardown. The control surface labels all tablet Provider data as mock and does not equate loopback startup with ADB mapping or tablet online state. This closes the standalone-to-desktop code integration gap, but all four criteria still require real-tablet rendering/recovery/measurement evidence. See `docs/handoffs/2026-09-09-desktop-tablet-integration.md`.

- 2026-09-09 browser follow-up: A real in-app browser completed mock-only pairing, rendered all four canonical Provider states, accepted async refresh, received SSE revision 2, recovered the pair across reload, and showed stale/disconnected state after host shutdown. One immediate post-pair connection was transiently disconnected before reload recovery. This was not a physical tablet, USB, sleep, host-restart or real Provider test, so compound acceptance ticks remain open. See `docs/handoffs/2026-09-09-browser-e2e.md`.

- 2026-09-06: Added fixture-driven `agentmeter-dashboard-p0` experiment and tests. It proves the complete four-provider snapshot contract, source-scoped usage and account deduplication, revisioned SSE ordering with heartbeat/stale handling, reconnect-to-latest recovery, prompt accepted/coalesced/throttled refresh requests, independent collector outcomes, and the monitor-only tablet boundary. Real paired-tablet rendering, USB reconnect/sleep/host-restart behavior, latency/resource measurements, and authenticated end-to-end evidence remain required.

- 2026-09-06 audit: Tightened the evaluator to require exact canonical signals, stream identity/time, complete SSE event identity, non-advancing heartbeat, all reconnect reasons, prompt 202 refresh, every isolation outcome, and only monitor routes. Corrected Provider Account deduplication so two Sources can retain separate usage while one account-scoped quota bucket is emitted. Measurements remain `not_observed`. All acceptance criteria remain open because this is not a real HTTP/SSE implementation or tablet rendering; the exact operator sequence is in `docs/evidence/dashboard-p0-2026-09-06.md`.

- [ ] An authenticated tablet loads one complete Dashboard Snapshot containing all four provider states, global revision metadata, and the canonical setup, paused, availability, collection, freshness, quality, and maturity signals.
- [x] The snapshot preserves source-scoped usage and quota-window facts while applying the approved Provider Account deduplication rules.
- [x] SSE delivers revisioned complete snapshots, heartbeat traffic, and enough event identity for the tablet to reject stale or duplicate revisions.
- [ ] Disconnect, tablet sleep, host restart, expired session, and missed events recover by reconnecting and fetching the latest complete snapshot rather than applying uncertain partial state.
- [x] A refresh request returns an accepted asynchronous result promptly, coalesces or throttles repeated requests, and exposes progress or completion through subsequent snapshot revisions.
- [x] Slow, failed, paused, unsupported, and schema-changed mock collectors remain independently visible and do not prevent other providers from updating.
- [ ] The real tablet rendering demonstrates the monitor-only boundary: it can inspect status and request refresh but cannot modify provider credentials or host configuration.
- [ ] Evidence records latency, reconnect behavior, revision ordering, heartbeat behavior, throttle behavior, and resource use; the outcome states whether this delivery model is viable for v1.

2026-09-07 production-path follow-up: Added real loopback HTTP Dashboard Snapshot and SSE endpoints plus asynchronous refresh backed by concurrent state, rather than reading result booleans from a fixture. TCP tests prove separate Source usage with one account-scoped quota bucket, stream/revision event IDs, complete snapshots, non-advancing heartbeat, `Last-Event-ID` resume, prompt `202`, coalescing/throttling, later revision publication, and isolated slow/failed/paused/unsupported/schema-changed mocks. Real tablet rendering, sleep/host restart/session-expiry recovery, and measured device/resource evidence remain open. See `docs/evidence/tablet-http-p0-2026-09-07.md`.

2026-09-09 live-online correction: Desktop activity now polls the current server authorization state rather than a sticky success history. Natural Session expiry, Clear/Reset and every untrustworthy USB mapping transition clear online and close existing SSE authorization; a replacement mapping requires an explicit fresh Session exchange. This implements the fail-closed host side of reconnect recovery, but criterion 4 remains open until a real tablet proves sleep, host restart, missed-event and re-fetch behavior end to end. See `docs/handoffs/2026-09-09-desktop-usb-integration.md`.
