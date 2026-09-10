# Full regression and remaining-scope audit — 2026-09-08

Single-agent batch. Used diagnosing-bugs after the full root suite found a failure.

## Regression evidence and correction

`cargo test --offline --locked` failed because the immediate-exit fake Codex process was classified as timeout. The exact isolated test reproduced the same failure. A direct differential probe with only the deadline changed returned timeout at 50 ms, and process_exited at 500 and 2000 ms. This supports a process-start/scheduling deadline race rather than proof of incorrect production failure mapping.

The classification test now allows 2000 ms for exit/malformed fixtures, retaining 50 ms for the deliberately unresponsive fixture. A subsequent full parallel run also timed out three 1000 ms handshake tests; these protocol tests now use the desktop's 5000 ms allowance. No assertion or production collector timeout changed. These are functional tests, not startup-performance benchmarks.

The subsequent full root suite passed: 6 library entries plus 74 integration tests, 80 total. The six library entries include the independent-process DPAPI helper. This is one complete passing run, not a long-duration stress certification.

## Remaining formal gates

| Area | Open ticks | Required next evidence/work |
| --- | ---: | --- |
| Codex | 2 | Authenticated app-server quota/usage run |
| Claude | 1 | Real installed statusLine coexistence and cleanup evidence |
| Copilot | 2 | Real applicable account contexts and authoritative Observation |
| Antigravity | 4 | Authenticated structured source and version-bounded fallback investigation |
| Windows lifecycle | 4 | Actual focus/tray/full child cleanup and lifecycle scenarios |
| Installer | 7 | Build package, clean-VM install/runtime/reinstall/uninstall/measurements |
| USB | 6 | Explicit physical-device selection, safe mapping, recovery and teardown |
| Pairing | 2 | Complete secret hygiene and real tablet security evidence |
| Dashboard | 4 | Real tablet client, reconnect/rendering/monitor-only and measurements |

Total remains **47/79 checked, 32 open**. PATH lookup found no adb, claude or gh commands in this execution environment; this does not prove those products are absent elsewhere. No devices, accounts or VM were provisioned by this batch.

## Important implementation boundary

The tablet library has real HTTP/SSE routes, but inspected pairing/dashboard binaries remain fixture evaluators, not a user-runnable tablet host and browser client. Building that host/client is still meaningful local development even before a physical tablet becomes available. Do not treat all 32 items as external-only blockers. Desktop UI integration tests and source/collector lifecycle also remain local work.

Next priority: a runnable monitor-only tablet client/host around the existing security seam, then local browser recovery tests, while keeping real-device and real-provider gates open. No full-goal completion is claimed; changes remain uncommitted.
