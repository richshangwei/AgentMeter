# Dashboard Snapshot / Refresh P0 evidence — 2026-09-06

## Scope

The `agentmeter-dashboard-p0` fixture experiment exercises the v1 dashboard delivery contract without claiming that a real tablet or USB session was available. It models four provider cards, complete revisioned snapshots, SSE heartbeats, reconnect recovery, asynchronous refresh, and monitor-only permissions.

## Reproduction

```text
cargo test --offline --locked --test dashboard_streaming -- --nocapture
```

Result on 2026-09-06: 2 passed, 0 failed. The healthy fixture demonstrates a nonempty stream identity and generation time, exact canonical signal names, revisions 7→8, a heartbeat that does not advance revision, complete event identity, latest-snapshot recovery for disconnect/sleep/host restart/expired session/missed events, and accepted (202 within 300ms)/coalesced/throttled refresh. The stale fixture demonstrates constrained behavior for descending revisions, incomplete reconnect recovery, and missing refresh controls.

The source-scoping check now proves the intended seam: Codex has two Source usage records for one Provider Account but one account-scoped quota bucket. Duplicate quota bucket keys fail instead of requiring every account to appear only once, which previously could not test the ADR's multi-Source deduplication rule. Provider status is represented by separate setup, paused, Availability, Collection State, Freshness, Data Quality, and Collector Maturity signals.

## Gate

The fixture contract is viable as a P0 design seam. The later production-path socket evidence in `tablet-http-p0-2026-09-07.md` adds a real HTTP/SSE server for source scoping, revisions/heartbeat, asynchronous refresh, and failure isolation. It is still not a tablet UI. Real tablet rendering, paired USB reconnect after sleep/host restart, session-expiry recovery, measured latency/resource use, and authenticated end-to-end device evidence remain required, so Ticket 09 remains `needs-info`.

The operator must capture the Tab model/year, Android/browser versions, portrait rendering and 200% font behavior, authenticated initial snapshot, 20 update latencies, SSE event IDs/heartbeats/stale rejection, foreground recovery, host restart with new stream identity, session expiry, refresh 202 latency/coalescing/throttling/paused behavior, independent slow/failure/schema-change results, five-minute idle resources, and proof that credential/configuration/clear/command/exit routes are absent.
