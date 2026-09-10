# Browser pairing recovery handoff — 2026-09-08

Single-agent implementation. Device Pair cookies now include a one-year Max-Age while retaining HttpOnly, SameSite=Strict and Path=/. Server-side revocation remains authoritative; cookie lifetime does not bypass it.

The browser persists only the 32-hex exchange CSRF value in origin-local storage. The actual Device Pair stays in its HttpOnly cookie and short-lived Tablet Session tokens remain memory-only. Startup exchanges the pair for a new session with a 10-second request bound, then fetches the latest full snapshot. Storage denial degrades to explicit re-pairing. Authentication/CSRF rejection clears the recovery value; network or service errors preserve it for retry. No credential is placed in URLs or console logs.

Recovery requires the same browser profile and origin, plus the host's explicit DPAPI pair store across host restarts. Use the same --port and --pair-store on restart. Default ephemeral host mode cannot restore a pair after host exit. Browser privacy policy may still evict cookie/storage state. The exchange CSRF is script-readable and must be excluded from future diagnostics; this does not claim browser-wide XSS immunity or complete secret-hygiene certification.

Verification: 7 JavaScript protocol/recovery tests passed, 15 tablet HTTP tests passed, and client syntax check passed. Tests assert persistence/restore/clear of the exchange value, denied/corrupt storage behavior, persistent cookie attributes and all existing server revocation/restart boundaries. No actual browser reload, restart, USB or physical-tablet test was performed in this batch.

Formal count remains 47/79 checked, 32 open. Next: actual browser end-to-end verification of pairing, reload, expiry, revoke and rendering; selected-device setup and live Provider integration remain unfinished. No user pairing data or USB mapping was created. Changes remain uncommitted.
