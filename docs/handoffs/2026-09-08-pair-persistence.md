# Device Pair persistence handoff — 2026-09-08

## Implemented

`ServerConfig.pair_store_path` opts the HTTP experiment into the Windows current-user DPAPI store. Its parent directory must already exist. `None` remains the ephemeral test mode; the standalone desktop shell is not wired to this service yet.

The service loads protected Device Pairs before accepting requests. A successful pair response requires saving the pair first. Sessions remain memory-only and are invalid after restart. Normal shutdown stops authorization and SSE, releases the store lock and preserves durable pairs. Forget Account revokes sessions while retaining the pair. Clear Pairing and Reset AgentMeter save an empty protected store and report success only after the write succeeds.

On a persistent revocation write failure, current authorization is disabled and the caller receives an error. The operator must resolve the storage failure and retry the operation before treating revocation as durable: an unchanged old ciphertext may still contain the old pair. The service deliberately does not claim successful revocation when disk writes fail.

## Verification so far

Thirteen real TCP tablet-service tests passed, including three Windows persistence regressions: restart preserves pairs but not sessions; Clear/Reset survive restart; ciphertext excludes cleartext secrets; duplicate store ownership, corrupted data and failed pairing writes are rejected; failed revocation disables current authorization until explicit recovery.

Final verification: `cargo test --offline --locked` passed all 74 integration tests and 6 library test entries (including the child-process entry used by the independent-process reload test). Clippy with warnings denied, formatting and diff checks passed. The Clippy nested-if finding was corrected before its final passing run. See also [DPAPI module handoff](2026-09-08-dpapi-store.md).

## Chief-supervisor sign-off

The independent read-only chief supervisor accepted #08.2 specifically for server-side durable Device Pair storage and distinct memory-only Tablet Sessions. Browser persistence/reconnect is not claimed. Criteria #08.7 and #08.8 remain open. Current total: **47/79 checked, 32 open**; issue 08 is now 6/8. Other ticket counts are unchanged from the [previous batch](2026-09-07-p0-batch.md). Production release remains no-go.

## Remaining boundaries

This closes only the server storage portion of the P0 experiment. The browser client, persistent browser-cookie behavior, ADB mapping generations, real tablet reconnect/history/screenshot inspection and packaged desktop integration remain outstanding. No real user's pairing records or LocalAppData configuration were changed: tests used unique owned temporary directories and removed their test data after handles closed.

Store format is versioned DPAPI ciphertext with exclusive ownership and atomic replacement; it is a P0 file store rather than the final database schema. Non-Windows durable operation must return Unsupported instead of writing plaintext. Pair/CSRF credentials are stored only inside ciphertext; session credentials are not persisted. Prior P0 worktree changes remain uncommitted and must be preserved.
