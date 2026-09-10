# SSE revocation handoff — 2026-09-07

## Completed change

`src/tablet.rs` registers active SSE sockets and shuts them down on Forget Account, Clear Pairing, Reset AgentMeter, session rotation, and server drop. Payload writes and revocation share the state lock so a completed revocation cannot be followed by a new application payload write. Socket writes have a 250 ms timeout. Expiry is checked while waiting between heartbeats (10 ms polling); authorization is checked again before each payload.

## Verification

`cargo test --offline --locked --test tablet_http`: 10 passed, 0 failed. The new real TCP test drains the initial event, uses the production 15-second heartbeat, and requires EOF with no additional bytes within a one-second socket timeout for all five revocation/shutdown actions. Existing session-expiry, negative authorization, refresh and snapshot tests pass.

## Limits and remaining work

Already transmitted or OS-buffered bytes cannot be retracted. The short bounded write may delay a concurrent revocation. Device Pairs are still in memory; durable DPAPI storage remains open. Server shutdown closes SSE, but a full lifecycle for all detached HTTP/refresh workers is not established. Reset/Forget Account here are authorization probes, not database deletion implementations. Physical ADB/tablet validation is outstanding.

## Review and continuation

Accepted by the read-only chief supervisor, who independently reran the ten TCP tests. No additional acceptance tick was closed by this change. Final checklist disposition is recorded in the batch handoff. Changes remain uncommitted alongside previous P0 work.
