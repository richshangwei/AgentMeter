# 08: Prove the Device Pair and Tablet Session security boundary

**What to build:** A complete pairing and session experiment over the selected USB loopback channel that establishes a durable Device Pair, issues a separate revocable Tablet Session, protects every route, and demonstrates that copied, replayed, cross-origin, or revoked credentials fail closed.

**Blocked by:** 07: Prove the selected-device USB loopback channel.

**Status:** ready-for-agent

- [ ] An unpaired tablet can exchange a short-lived, single-use pair code for a Device Pair only within the approved attempt and expiry limits.
- [ ] A paired device obtains a Tablet Session that is distinct from the durable pair record and contains enough server-side state for expiry and revocation.
- [ ] Every health, snapshot, refresh, event-stream, pairing-management, and future monitor route rejects missing or invalid authorization.
- [ ] Pair-code replay, session replay after revocation, guessed codes, excessive attempts, malformed credentials, and requests from an unapproved origin fail closed and create safe diagnostics.
- [ ] State-changing requests include the approved CSRF and origin protections even though transport is restricted to local USB loopback.
- [ ] Forget Account and Reset AgentMeter behavior revoke the relevant pairs and sessions according to the requirements without conflating the two concepts.
- [ ] Secrets are not exposed in URLs, logs, screenshots, browser history, fixtures, or diagnostics, and persisted secret material uses the approved Windows protection boundary.
- [ ] Evidence from a real tablet records pair, reconnect, expiry, revoke, attack-case, and cleanup results; the outcome states whether the security model is viable for v1.
