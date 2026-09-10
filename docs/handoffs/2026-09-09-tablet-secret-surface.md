# Tablet secret-surface hardening handoff — 2026-09-09

Single-agent implementation and verification. No multi-agent work was used and no real pairing credential was created.

## Completed in this batch

- All tablet JSON responses now include server-side `Cache-Control: no-store`, `Pragma: no-cache`, `Referrer-Policy: no-referrer`, and `X-Content-Type-Options: nosniff`. This includes pair/session credential responses and authentication failures; client-side fetch options are no longer the only cache control.
- The HTTP parser records whether a request target contained a query. Such requests are rejected with `400 query_not_allowed` before public or private route handling.
- Only the query-free path is retained in diagnostics. Query text is never reflected in the response or diagnostic record.
- Added a real loopback-socket regression that places the active pair code and Tablet Session in query targets, then verifies rejection and absence of the pair code, Device Pair, Tablet Session and both CSRF values from responses and serialized diagnostics.
- Existing pair responses now assert both `no-store` and `no-referrer`; unauthenticated JSON failures and session-exchange responses assert `no-store`.
- Added a process-local atomic suffix to the Windows DPAPI HTTP-test directory helper, preventing the timestamp collision class already reproduced in the Claude suite.
- Corrected the tablet HTTP evidence document: current-user DPAPI ciphertext/reload is already proven; only `pair_store_path: None` remains intentionally ephemeral.

## Verification and boundary

- Tablet real-socket integration target: 17 tests passed.
- Full local verifier passed: 110 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, both PowerShell syntax checks, formatting, JavaScript syntax and warnings-denied Clippy for both crates.
- Existing Windows tests continue to prove that persisted Device Pair and exchange-CSRF bytes are absent from the on-disk ciphertext and that sessions are memory-only.
- Existing host-process tests prove redirected/non-interactive output never discloses the pair code; the browser launch URL contains only loopback host and device port.

Formal status remains **47/79 checked, 32 open**. The product-controlled URL, cache, diagnostic, log and DPAPI surfaces are strengthened and covered, but criterion 08.7 also asks for real-browser history/screenshot evidence. With no physical tablet/browser evidence, that combined tick remains open rather than being partially claimed.

Changes remain uncommitted in the existing dirty worktree.
