# Loopback tablet HTTP production-path evidence — 2026-09-07

## Scope and result

`src/tablet.rs` is a real TCP/HTTP vertical slice, not a fixture evaluator. It binds an OS `TcpListener` only to `127.0.0.1`, accepts real socket requests, maintains concurrent server-side Device Pair / Tablet Session / Dashboard state, and uses the operating-system random source for all credentials.

Axum was not present in the approved offline crate cache, so the transport adapter uses `std::net`. The public HTTP/state seam remains replaceable by the ADR-preferred Axum adapter without changing the security contract. This is a P0 production-path probe, not the final HTTP parser.

## Reproduction

```powershell
cargo test --offline --locked --test tablet_http
```

Current result on 2026-09-09: 18 passed, 0 failed.

The tests use real loopback TCP connections and cover:

- loopback-only binding and an authenticated minimal `/health` response;
- eight-digit, single-use, 120-second, five-attempt pairing policy, replay, expiry, and rate limiting;
- high-entropy Device Pair and distinct expiring, rotating, revocable Tablet Session credentials;
- missing and malformed authorization on health, Dashboard Snapshot, history, SSE, refresh, pairing management, and future-monitor routes;
- Host/Origin rejection plus CSRF enforcement for refresh and Device Pair session exchange;
- authorization is re-evaluated while an SSE connection is active, and the stream closes after session revocation, pair revocation, or expiry;
- Forget Account, Clear Pairing, Reset AgentMeter, session rotation, and server shutdown actively close registered SSE sockets without waiting for the 15-second production heartbeat. Authorization and writes are serialized against revocation; data already delivered to the client cannot be retracted;
- Forget Account preserving the Device Pair while revoking sessions, with Clear Pairing and Reset AgentMeter revoking both;
- source-scoped usage with one deduplicated Provider Account quota bucket;
- complete revisioned SSE snapshots, non-advancing heartbeat, stale-event resume, prompt asynchronous `202`, coalescing/throttling, and later revision publication; and
- independent slow, failed, paused, unsupported, and schema-changed Provider outcomes.
- every JSON response, including pair/session credentials and failures, carries server-side `Cache-Control: no-store`, `Pragma: no-cache`, `Referrer-Policy: no-referrer`, and `X-Content-Type-Options: nosniff`;
- any request target containing a query is rejected before routing. Diagnostics retain only the query-free route, and socket tests inject pair/session/CSRF sentinels to prove responses and diagnostics do not echo them.

## Remaining gates

2026-09-08 update: the opt-in DPAPI durable store and server restart/revocation behavior are now verified in [the persistence handoff](../handoffs/2026-09-08-pair-persistence.md). The in-memory limitation applies only to the original 2026-09-07 probe and to current `pair_store_path: None` mode.

No ADB executable or physical tablet is available. The tests use a local TCP client, so they do not close criteria that explicitly require selected-device reverse mapping, browser launch, USB reconnect/sleep/host restart, or real-tablet rendering and measurements. Current-user DPAPI ciphertext and independent-process reload are proven on Windows; real-browser history and screenshot inspection are still required before the combined secret-hygiene criterion can close.

The transport-status regression additionally proves that an expired, reset, or transport-revoked Tablet Session immediately stops satisfying the authenticated-online signal. A transport generation change revokes the old Tablet Session and its SSE authorization while preserving the durable Device Pair so the tablet can explicitly exchange a new session after the mapping is trusted again.
