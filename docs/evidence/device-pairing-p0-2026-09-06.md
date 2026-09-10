# Ticket 08 evidence — Device Pair and Tablet Session

The fixture-driven experiment in `agentmeter-pairing-p0` models the v1 security boundary over the selected loopback channel. It now enforces the approved eight-digit, 120-second, five-attempt policy and advances modeled elapsed time beyond expiry. It checks missing and malformed authorization for each named route, pair-code replay, guessing/rate limiting, origin and CSRF rejection, revoked-session replay, Clear Pairing, Reset AgentMeter, and secret-safe diagnostics. Forget Account now revokes the active Tablet Session while preserving the independent Device Pair, matching the data-lifecycle requirements.

Reproduction:

```text
cargo test --offline --locked --test pairing_security -- --nocapture
```

Result on 2026-09-06: 3 passed, 0 failed. A negative fixture proves that a six-digit policy, an attempt at exactly (not beyond) the expiry boundary, and one untested invalid-auth route cannot claim support. The report deliberately returns `persisted_secret: not_observed`; no DPAPI result is fabricated.

This section records the original offline state-machine contract experiment. It did not prove HTTP middleware. The later production-path socket evidence in `tablet-http-p0-2026-09-07.md` supersedes that limitation for criteria 1–6. Protected DPAPI storage, browser-history/screenshot inspection, a real tablet, and end-to-end USB evidence remain unverified, so Ticket 08 remains `needs-info`.

The operator must pair from a real selected tablet, capture reconnect/expiry/revoke/cleanup outcomes, and run missing/invalid authorization against health, snapshot, refresh, events, pairing management, history, and a future monitor route. Also record bad Host/Origin, missing/bad CSRF, code replay, five failed attempts, expiry after 120 seconds, old mapping-generation session replay, and a DPAPI-at-rest inspection. Evidence must redact pair codes, cookies, sessions, verifiers, URLs containing secrets, and personal device identifiers.
