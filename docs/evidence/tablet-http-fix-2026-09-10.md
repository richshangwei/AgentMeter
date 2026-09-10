# Tablet HTTP request_parse_failed

Windows accepts connections from a nonblocking listener into sockets that retain that mode. The per-connection worker previously treated a read before bytes arrived as a malformed HTTP request. USB forwarding exposes this timing; the previous tests wrote the complete request immediately and closed the write side.

The worker now explicitly switches to blocking mode and applies the existing two-second timeout. A regression connects, delays, sends two request fragments, and reads without shutting down the write side. It failed with connection abort before the fix and returns HTTP 200 after it.

ADB preserves the tablet's Host and Origin. The desktop now allows the exact loopback origin associated with its verified USB mapping, clears it on transport loss, and restores it after ownership verification. Pairing and session exchange validate the transport generation under the session-state lock. Cross-origin requests remain forbidden.

Verification:
- scripts/verify-local.ps1 passed (root/desktop tests, JavaScript tests, formatting, Clippy).
- After final transport recovery review changes, tablet_http: 21 passed; desktop tests: 19 passed; desktop all-targets Clippy passed.
- Existing USB selection unit test caught a missing transport_id check in inherited work; the check now matches the documented Windows fallback.
- Physical tablet verification remains pending: ADB listed no connected device during final checks.

Rollback: remove only this task's blocking socket and verified-origin changes and rebuild. No stored device pairs were deleted. Installer verification is recorded in tasks/todo.md.
