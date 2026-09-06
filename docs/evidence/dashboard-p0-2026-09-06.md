# Dashboard Snapshot / Refresh P0 evidence — 2026-09-06

## Scope

The `agentmeter-dashboard-p0` fixture experiment exercises the v1 dashboard delivery contract without claiming that a real tablet or USB session was available. It models four provider cards, complete revisioned snapshots, SSE heartbeats, reconnect recovery, asynchronous refresh, and monitor-only permissions.

## Reproduction

```text
cargo test --offline --locked --test dashboard_streaming -- --nocapture
```

Expected result: 2 tests pass. The healthy fixture demonstrates revisions 7→8, heartbeat traffic, latest-snapshot recovery, and accepted/coalesced/throttled refresh. The stale fixture demonstrates rejection/constrained behavior for duplicate or descending revisions, incomplete reconnect recovery, and missing refresh controls.

## Gate

The fixture contract is viable as a P0 design seam. Real tablet rendering, paired USB reconnect after sleep/host restart, measured latency/resource use, and authenticated device evidence are still required before marking this capability supported for v1.
