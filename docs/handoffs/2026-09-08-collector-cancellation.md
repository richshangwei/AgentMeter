# Collector cancellation and exit coordination — 2026-09-08

Single-agent implementation. Added a cancellable Codex collector entry point; existing callers keep the previous API. Response waits inspect cancellation at most every 50 ms while preserving the requested overall response deadline. Pre-cancelled requests do not spawn a process; cancellation is a distinct failure and prevents reconnect attempts. After collection returns, the direct child follows the existing kill/wait cleanup before a result is published.

Desktop refresh now holds a lifecycle permit in its blocking worker. Explicit tray/second-instance exit marks collection stopped, signals cancellation, rejects new work and waits off the UI thread for the worker permit to release before calling app.exit. Closing to tray retains monitoring behavior.

Verification: 14 Codex integration tests passed after cancellation changes, 2 cancellation unit tests passed, 7 desktop tests passed including shutdown/wait/reject-new-work coordination, and desktop all-targets Clippy with warnings denied passed.

Limits: actual UI-triggered exit during a live app-server collection has not been tested. The cancellation check bounds response waits, not OS process creation or blocking pipe writes; forced process termination, descendant processes and OS sign-out remain separate ownership gaps. No claim is made that the full lifecycle acceptance gate is complete. Formal count remains 47/79 checked, 32 open. Changes remain uncommitted.
