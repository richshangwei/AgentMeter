# Ticket 08 evidence — Device Pair and Tablet Session

The fixture-driven experiment in `agentmeter-pairing-p0` models the v1 security boundary over the selected loopback channel. It keeps a single-use expiring pair-code policy separate from a durable Device Pair and a revocable Tablet Session. The healthy fixture exercises route authorization, replay, guessing/rate limiting, expiry, origin/CSRF rejection, forget/reset revocation, and secret-safe diagnostics; the negative fixture blocks an unapproved origin.

This is an offline protocol experiment. A real tablet reconnect, protected secret storage, and end-to-end USB/browser channel remain unverified and are required before promoting the capability to `supported`.
