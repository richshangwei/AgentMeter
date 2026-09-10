# 08: Prove the Device Pair and Tablet Session security boundary

**What to build:** A complete pairing and session experiment over the selected USB loopback channel that establishes a durable Device Pair, issues a separate revocable Tablet Session, protects every route, and demonstrates that copied, replayed, cross-origin, or revoked credentials fail closed.

**Blocked by:** 07: Prove the selected-device USB loopback channel.

**Status:** needs-info

## Comments

- 2026-09-08 supervisor acceptance: Closed criterion 2 for server-side persistence with opt-in `ServerConfig.pair_store_path`. Windows current-user DPAPI protects durable pairs; separate sessions remain memory-only. TCP restart, independent-process reload, persistent Clear/Reset and failure/locking tests passed. Browser persistence/reconnect is not claimed; client cookie/CSRF retention and real-tablet recovery remain open. See `docs/handoffs/2026-09-08-pair-persistence.md`.

- 2026-09-06: Added fixture-driven `agentmeter-pairing-p0` experiment and tests. The model demonstrates single-use/expiring pair attempts, durable Device Pair versus revocable Tablet Session, authorization on every route, replay/guess/rate-limit/origin/CSRF failures, forget/reset revocation, and secret-safe diagnostics. Real tablet reconnect, protected Windows secret persistence, and end-to-end USB/browser evidence are still required.

- 2026-09-06 audit: Fixed the model to require 8 digits/120 seconds/5 attempts, advance modeled time past expiry, check missing and invalid auth route-by-route, preserve the Device Pair on Forget Account while revoking the Tablet Session, and model Clear Pairing separately from Reset AgentMeter. DPAPI stays explicitly `not_observed`. All acceptance criteria remain open because the artifact is a state-machine evaluator rather than HTTP middleware, protected Windows persistence, or real-tablet evidence. Reproduction and the required human attack/reconnect sequence are in `docs/evidence/device-pairing-p0-2026-09-06.md`.

- [x] An unpaired tablet can exchange a short-lived, single-use pair code for a Device Pair only within the approved attempt and expiry limits.
- [x] A paired device obtains a Tablet Session that is distinct from the durable pair record and contains enough server-side state for expiry and revocation.
- [x] Every health, snapshot, refresh, event-stream, pairing-management, and future monitor route rejects missing or invalid authorization.
- [x] Pair-code replay, session replay after revocation, guessed codes, excessive attempts, malformed credentials, and requests from an unapproved origin fail closed and create safe diagnostics.
- [x] State-changing requests include the approved CSRF and origin protections even though transport is restricted to local USB loopback.
- [x] Forget Account and Reset AgentMeter behavior revoke the relevant pairs and sessions according to the requirements without conflating the two concepts.
- [ ] Secrets are not exposed in URLs, logs, screenshots, browser history, fixtures, or diagnostics, and persisted secret material uses the approved Windows protection boundary.
- [ ] Evidence from a real tablet records pair, reconnect, expiry, revoke, attack-case, and cleanup results; the outcome states whether the security model is viable for v1.

2026-09-07 production-path follow-up: Replaced most of the fixture-only boundary with a real loopback TCP/HTTP service in `src/tablet.rs`. Integration tests exercise OS-random eight-digit codes, 120-second/five-attempt policy (using shortened test clocks), distinct high-entropy Device Pair and expiring/revocable Tablet Session credentials, every private route, replay/guess/bad-origin failures, Origin plus CSRF on refresh and Device Pair session exchange, and separate Forget Account/Clear Pairing/Reset behavior. Criterion 2 remains open because the Device Pair record is currently in-memory rather than durable across application restart. Safe diagnostics contain route/failure codes only. DPAPI/browser-history inspection and real-tablet evidence remain open. See `docs/evidence/tablet-http-p0-2026-09-07.md`.

2026-09-09 secret-surface follow-up: All JSON responses now send server-side no-store/no-cache, no-referrer and nosniff headers. Query-bearing targets are rejected before routing and only their query-free path reaches diagnostics. A real socket test injects active pair/session material into query targets and proves no response/diagnostic echo; pair, session and error responses assert cache protections. Current-user DPAPI ciphertext/reload and non-interactive pair-code suppression remain covered. Real tablet browser-history/screenshot inspection is still unavailable, so criterion 7 remains open. See `docs/handoffs/2026-09-09-tablet-secret-surface.md`.

2026-09-09 desktop integration follow-up: Integrated the authenticated TabletServer into the Tauri lifecycle with explicit start/stop, DPAPI-backed pair storage, short-lived code display and Clear Pairing. Ordinary desktop status never includes the code; full exit stops the server before process exit. A desktop test reaches the real landing page over TCP and verifies lifecycle ownership. The package is rebuilt, but no physical tablet/browser evidence exists, so criteria 7–8 remain open. See `docs/handoffs/2026-09-09-desktop-tablet-integration.md`.

2026-09-09 transport-generation follow-up: Corrected the desktop online signal from a sticky “ever established” flag to the presence of a currently authorized, unexpired Tablet Session. USB mapping creation, loss, external replacement, inspection failure, recovery and disconnect revoke all transport-bound sessions and close unauthorized SSE streams; durable Device Pairs intentionally survive so a newly trusted mapping can exchange a fresh Session. Real TCP tests prove the old Session receives 401, the online signal clears, and the Device Pair obtains a distinct replacement Session. Criteria 7–8 remain open because browser-history/screenshot and physical-tablet lifecycle evidence are still absent. See `docs/handoffs/2026-09-09-desktop-usb-integration.md`.
