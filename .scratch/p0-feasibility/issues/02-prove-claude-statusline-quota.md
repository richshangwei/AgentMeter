# 02: Prove Claude statusLine coexistence and quota collection

**What to build:** A reversible Claude collection experiment that coexists with the user's statusLine configuration, consumes event-driven status updates, and produces a trustworthy Observation when quota fields are available without disrupting the user's existing setup.

**Blocked by:** None.

**Status:** ready-for-agent

- [ ] Installation detects and preserves an existing statusLine configuration and records exactly what AgentMeter changes.
- [ ] The experiment can be enabled, disabled, and removed while restoring the user's original configuration and executable behavior.
- [ ] Status events containing quota information produce normalized Observations with source time, quota-window scope, values, units, and reset information when available.
- [ ] Events without quota fields remain valid events but do not create invented quota values or falsely report a healthy quota reading.
- [ ] Missing events, malformed payloads, user configuration conflicts, command failures, and schema changes are distinguishable in Availability, Collection State, Data Quality, and diagnostics.
- [ ] Any optional log-derived fallback is tested separately, clearly labeled with lower maturity and quality, and never silently replaces the structured source.
- [ ] Evidence from a real Claude installation records version, configuration before and after, representative sanitized events, coexistence results, and cleanup verification.
- [ ] The outcome states whether the Claude collector is supported, constrained, or blocked for v1 and identifies any user-visible setup requirement.
