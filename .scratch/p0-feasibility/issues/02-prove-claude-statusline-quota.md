# 02: Prove Claude statusLine coexistence and quota collection

**What to build:** A reversible Claude collection experiment that coexists with the user's statusLine configuration, consumes event-driven status updates, and produces a trustworthy Observation when quota fields are available without disrupting the user's existing setup.

**Blocked by:** None.

**Status:** needs-info

- [x] Installation detects and preserves an existing statusLine configuration and records exactly what AgentMeter changes.
- [x] The experiment can be enabled, disabled, and removed while restoring the user's original configuration and executable behavior.
- [x] Status events containing quota information produce normalized Observations with source time, quota-window scope, values, units, and reset information when available.
- [x] Events without quota fields remain valid events but do not create invented quota values or falsely report a healthy quota reading.
- [x] Missing events, malformed payloads, user configuration conflicts, command failures, and schema changes are distinguishable in Availability, Collection State, Data Quality, and diagnostics.
- [x] Any optional log-derived fallback is tested separately, clearly labeled with lower maturity and quality, and never silently replaces the structured source.
- [ ] Evidence from a real Claude installation records version, configuration before and after, representative sanitized events, coexistence results, and cleanup verification.
- [x] The outcome states whether the Claude collector is supported, constrained, or blocked for v1 and identifies any user-visible setup requirement.

## Comments

- 2026-09-09 test-isolation follow-up: Diagnosed an intermittent Windows `claude_statusline` failure with a 30-run feedback loop (2 failures before the fix). The test helper's PID-plus-`SystemTime::as_nanos` directory names collided under parallel execution; a 160,000-sample probe observed 15,788 duplicates. Added an atomic sequence and a parallel uniqueness regression. The same 30-run loop then passed 270/270 cases, and the full 107-test root suite passed. This improves evidence reliability but does not close the remaining real-install acceptance tick. See `docs/handoffs/2026-09-09-claude-test-isolation.md`.

- 2026-09-07 event-path completion: Wrapper stdin now feeds documented `rate_limits` normalization into a sanitized sidecar while preserving prior display output. Eight targeted tests pass. Source time is unavailable in the documented contract and remains null with separate receipt time. See `docs/evidence/claude-event-path-2026-09-07.md` and `docs/handoffs/2026-09-07-claude.md`. Real-install criterion remains open.

- 2026-09-06: Implemented an isolated reversible statusLine lifecycle experiment, structured event normalization, fail-closed diagnostics, and explicitly degraded log fallback with sanitized fixtures. Status remains `needs-info` pending real Claude installation/version/account evidence and confirmation of the provider's event contract.
- 2026-09-06 audit: Replaced destructive statusLine substitution with a reversible wrapper seam that passes through prior command output and refuses to overwrite post-enable user edits. Fixture output now uses canonical axes, and log fallback is `estimated` / `experimental`. See `docs/evidence/claude-p0-2026-09-06.md`. The combined failure-class tick was reopened because real no-event and prior-command failures with every canonical axis remain unobserved; the fixture `usage` shape is only a normalization seam, not proof of Claude's official `rate_limits` contract.
- 2026-09-07: Completed the offline failure matrix. Missing event, malformed payload, configuration conflict, prior-command failure through the real wrapper subprocess, and schema change now return distinct Failure Codes in a complete canonical status envelope with diagnostics and null Observation. This closes the failure-behavior criterion; real installation/version/account evidence remains open.
- 2026-09-07 supervisor correction: Reopened criterion 3. The wrapper currently preserves and forwards the prior statusLine command but does not feed the same event into AgentMeter normalization, and the fixture normalizer consumes a synthetic `usage` shape rather than an observed Claude `rate_limits` contract. The isolated normalization seam remains useful regression coverage but is insufficient evidence for the installed event-to-Observation path.
