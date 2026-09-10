# Runnable tablet host handoff — 2026-09-08

Single-agent implementation. Added agentmeter-tablet-host as an operator-run host for the existing real HTTP/SSE service; the older fixture evaluators remain separate.

## Run

```powershell
cargo run --offline --locked --bin agentmeter-tablet-host -- --mock-providers
```

The program requires explicit mock-provider opt-in and announces the loopback origin and mock-data boundary in readiness JSON. In the same interactive terminal, `pair` displays a fresh two-minute code, `clear-pairing` revokes pairing, and `quit` stops the listener. EOF also stops it. Pair codes are suppressed if stdin or stdout is redirected; do not capture or screenshot the interactive pairing display. `--pair-store ABSOLUTE_PATH` opts into the existing Windows DPAPI persistence. Default mode is ephemeral and does not claim restart durability.

This is a transport probe, not the production selected-device setup flow: it does not create ADB mappings, select a tablet, or collect real Provider values. The browser client is still missing, so the announced origin is currently an API endpoint rather than a rendered landing page.

## Verification

Two real-process integration tests passed: explicit mock opt-in, readiness schema, unauthorized dashboard rejection over TCP, redirected pair-code suppression, quit listener teardown, and EOF exit. The tests do not prove authenticated browser pairing, SSE rendering, USB recovery or full descendant cleanup. Existing server tests cover the authenticated API separately.

No user pairing store or device mapping was created. Tests shut down their spawned hosts. Formal count remains 47/79 checked, 32 open. Next: implement the monitor-only browser landing/client and browser-compatible authenticated requests, then test reconnect/session exchange without weakening Origin/CSRF checks. Changes remain uncommitted.
