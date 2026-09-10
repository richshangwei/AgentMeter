# Claude file monitoring handoff — 2026-09-08

Single-agent implementation and verification; no delegated review.

## Delivered

The desktop Claude card can now start and stop monitoring an explicitly selected local event-report file. It reads again five seconds after each completed attempt, retries failures, and permits only one read in flight. Changing the path or selecting one-shot import stops the previous monitoring generation. Late results from stopped or superseded reads do not update the card. No Claude settings or credentials are changed.

The UI preserves the report's timestamp, calls out that a successful file read does not mean new Provider activity, and labels failed reads as old values for reference only. Codex refresh no longer overwrites this warning with an older backend Claude snapshot. Paths and monitoring preferences are not persisted; restart requires explicit selection again. Closing to tray does not intentionally stop monitoring, but WebView background timer throttling may delay checks.

## Evidence

- `node --test desktop-p0/tests/source-watch.test.cjs`: 4 passed (repeat/stop, retry/one-shot, source switch/non-overlap, in-flight cancellation).
- `node --check desktop-p0/ui/dashboard.js`: passed.
- Desktop Rust tests: 3 passed.
- Offline locked desktop build: passed after stopping locked preview processes.
- Whitespace diff check passed before this handoff update.

This batch has no interactive UI verification or authenticated Claude event evidence. It does not close physical-tablet or live-provider acceptance criteria. Formal P0 count remains 47/79 checked, 32 open.

## Lifecycle finding and next work

Normal `--request-exit` launched another resident preview rather than closing the existing process across execution environments. Observed both PIDs 24168 and 38768 pointing to this project's debug executable. Both were explicitly terminated after path verification to release the build lock; their memory-only preview values were discarded. The single-instance acceptance gate must remain open. The rebuilt executable was not relaunched during this batch.

Next: resolve the Windows single-instance/exit boundary, verify monitor controls in the running UI, persist explicit source configuration, and continue live Provider adapters and tablet integration. All work remains uncommitted and the overall development goal remains incomplete.
