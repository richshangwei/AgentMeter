# Tablet browser client handoff — 2026-09-08

Single-agent implementation. Added a public, secret-free landing page and first-party script served by TabletServer after Host validation. The page explicitly labels all Provider values as mock data and offers pairing plus monitor-only per-Provider refresh controls.

## Implemented

The client posts the pairing code in a body, clears the password input, keeps session/CSRF material in memory, fetches complete snapshots, rejects duplicate/older revisions within a stream, and consumes complete SSE snapshots using fetch streaming. A failed stream reconnects after three seconds and fetches a full snapshot first. Session expiry attempts exchange using the Device Pair cookie and the in-memory exchange CSRF value. Failed pairing recovery exposes the pairing controls again. No credentials go into URLs or console logs.

Dashboard and events endpoints now accept POST as well as their existing GET method, allowing browser requests to use normal Origin handling without weakening the existing exact Origin and Bearer checks. Refresh retains its existing CSRF validation. Static responses have no-store, no-referrer, nosniff and a restrictive CSP. No account/host configuration commands are exposed by the page.

## Verification

- Existing tablet HTTP tests: 13 passed after route changes.
- Host CLI tests: 2 passed.
- New browser-route TCP test passed: static assets contain no current pair code; protected POST routes reject unauthenticated access; authenticated dashboard returns four Providers; wrong Origin remains rejected.
- All-targets root Clippy with warnings denied and JavaScript syntax check passed.

No real browser or physical tablet rendering test was completed in this batch. Client-side reconnect logic is implemented but not yet end-to-end verified; do not close the corresponding issue checkboxes on this evidence.

## Run / remaining

Run `cargo run --offline --locked --bin agentmeter-tablet-host -- --mock-providers` in an interactive terminal, open the announced origin, and obtain a code with the terminal's `pair` command. Do not record the sensitive pairing display.

Page reload currently requires fresh pairing because exchange CSRF is memory-only. Host restart also changes its dynamically selected origin. Durable browser recovery, stable selected USB ports, idle/heartbeat watchdogs, browser event parsing tests, real browser/physical-device validation and live Provider adapters remain unfinished. This is explicitly a mock-data transport probe, not production selected-device setup. Formal count remains 47/79 checked, 32 open. No overall completion is claimed.
