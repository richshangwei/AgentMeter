# Stable tablet host origin handoff — 2026-09-08

Single-agent implementation. Added ServerConfig.listen_port and the host's explicit `--port 1..65535` option. The listener remains bound exclusively to IPv4 loopback. Omitting the flag retains ephemeral probe behavior. An occupied explicit port fails with an actionable message, never silently selects another origin.

Example: `cargo run --offline --locked --bin agentmeter-tablet-host -- --mock-providers --port 43127`. The port is an operator choice; this example does not reserve it or create an ADB reverse mapping. Use the same chosen port on restart to preserve the browser origin.

Verification: 3 host CLI tests and 15 tablet HTTP tests passed, including invalid/occupied port rejection, no readiness output on conflict, loopback restriction, and successful rebinding of the same origin after shutdown. These tests do not prove USB recovery or persisted browser authorization.

Remaining: durable browser cookie/CSRF recovery, actual browser pairing and stream tests, explicit ADB serial/mapping management, and live Provider integration. Stable origin alone does not complete restart recovery. Formal count remains 47/79 checked, 32 open. No user pairing store, device mapping or reserved port was left behind; changes remain uncommitted.
