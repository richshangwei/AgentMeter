# Claude test-isolation handoff — 2026-09-09

Single-agent diagnosis, implementation and verification. No multi-agent work was used.

## Failure and root cause

- A complete local verification run intermittently failed in `configuration_conflict_is_setup_required_and_never_overwritten` with Windows error 3 (path not found).
- The red-capable loop ran the full `claude_statusline` integration target 30 times and reproduced 2 failures in different tests. One failed inside the Claude process and one failed while removing its test directory.
- The test helper derived directory names only from process ID plus `SystemTime::now().as_nanos()`. A parallel 16-thread, 160,000-sample probe found 15,788 duplicate names on this Windows host. The reported nanosecond value therefore was not a uniqueness guarantee.
- Colliding tests could share a directory; the first test to finish then removed files still owned by another test.

## Fix

- Added a process-local `AtomicU64` sequence to every Claude integration-test temporary path. The timestamp remains useful for inspection while the atomic suffix guarantees uniqueness within the test process.
- Added `temporary_paths_remain_unique_across_parallel_tests`, which allocates 16,000 names across 16 threads and requires every path to be distinct.
- Removed the throwaway reproduction probe and confirmed no diagnostic instrumentation remains.

## Verification and boundary

- Pre-fix feedback loop: 2 failed runs out of 30.
- Post-fix feedback loop: 0 failed runs out of 30; 270 Claude test cases passed.
- Full local verifier passed: 107 root Rust tests, 7 desktop Rust tests, 11 JavaScript tests, PowerShell parsing, formatting, JavaScript syntax and warnings-denied Clippy for both crates.

Formal P0 status remains **47/79 checked, 32 open**. This fixes test isolation and prevents false red builds; it does not substitute for the one remaining real Claude installation/account acceptance observation.

Changes remain uncommitted in the existing dirty worktree.
