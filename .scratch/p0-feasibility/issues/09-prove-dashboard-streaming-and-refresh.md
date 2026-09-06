# 09: Prove Dashboard Snapshot streaming and asynchronous refresh

**What to build:** A real tablet dashboard slice that loads a complete Dashboard Snapshot for four provider mocks, receives ordered SSE revisions over the paired USB session, and requests asynchronous refresh without turning a slow collector into a blocked HTTP request.

**Blocked by:** 08: Prove the Device Pair and Tablet Session security boundary.

**Status:** ready-for-agent

- [ ] An authenticated tablet loads one complete Dashboard Snapshot containing all four provider states, global revision metadata, and the canonical setup, paused, availability, collection, freshness, quality, and maturity signals.
- [ ] The snapshot preserves source-scoped usage and quota-window facts while applying the approved Provider Account deduplication rules.
- [ ] SSE delivers revisioned complete snapshots, heartbeat traffic, and enough event identity for the tablet to reject stale or duplicate revisions.
- [ ] Disconnect, tablet sleep, host restart, expired session, and missed events recover by reconnecting and fetching the latest complete snapshot rather than applying uncertain partial state.
- [ ] A refresh request returns an accepted asynchronous result promptly, coalesces or throttles repeated requests, and exposes progress or completion through subsequent snapshot revisions.
- [ ] Slow, failed, paused, unsupported, and schema-changed mock collectors remain independently visible and do not prevent other providers from updating.
- [ ] The real tablet rendering demonstrates the monitor-only boundary: it can inspect status and request refresh but cannot modify provider credentials or host configuration.
- [ ] Evidence records latency, reconnect behavior, revision ordering, heartbeat behavior, throttle behavior, and resource use; the outcome states whether this delivery model is viable for v1.
