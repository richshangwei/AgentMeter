# Real-process cancellation proof — 2026-09-08

Single-agent batch. Added an isolated subprocess integration harness for the cancellable collector. The fake app-server's cancellable_idle mode announces readiness through a temporary file held with Windows share_mode(0), then waits 30 seconds. The harness waits until the process is running before cancelling; environment changes are scoped to a separate test process rather than mutating the concurrent test runner's environment.

Verification: the two test entries passed (one is the child harness helper). The full invocation completed in approximately 0.16 seconds. The actual assertion requires cancellation to return within five seconds after readiness, with failure_code=cancelled and successful reopening of the previously exclusive file. Together with the collector's kill/wait path, this verifies direct-child cleanup rather than just an error return from a channel mock. Root all-targets Clippy with warnings denied passed.

Only test-owned temporary files were removed. No real Provider process or credentials were used. Actual desktop UI exit during collection, descendant cleanup, forced host termination and sign-out remain unverified and are not closed by this test. Formal P0 count stays 47/79 checked, 32 open. Changes remain uncommitted.
