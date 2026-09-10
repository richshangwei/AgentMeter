# 01: Prove Codex app-server can produce trusted Observations

**What to build:** A runnable Codex collection experiment that connects to a real authenticated account, reads the supported account, rate-limit, and usage surfaces, and presents the result as an AgentMeter Observation with enough provenance and diagnostic detail to judge whether the collector is viable.

**Blocked by:** None.

**Status:** needs-info

- [ ] A successful run performs the app-server handshake and reads the available account, rate-limit, and usage data without scraping rendered UI.
- [x] The run emits a normalized Observation using the canonical Provider, Provider Account, Source, Quota Window, Data Quality, Availability, Collection State, and Freshness terms.
- [x] Window limits, remaining or used values, reset times, units, scope, and source timestamps are preserved when supplied and remain explicitly unknown when absent.
- [x] Authentication failures, unsupported methods, malformed responses, timeouts, and schema changes produce distinct diagnosable outcomes without fabricating usage data.
- [x] Sanitized fixtures cover a successful response and each material failure class so normalization can be rerun without a live account.
- [ ] Evidence from at least one real authenticated account records the app-server version, observed methods and fields, test time, limitations, and reproducible steps.
- [x] The outcome states whether the Codex collector is supported, constrained, or blocked for v1 and identifies any remaining release risk.

## Comments

- 2026-09-06: Implemented the Rust P0 CLI, fixture replay, real subprocess protocol handshake, normalized Observation output, multi-bucket quota handling, one process-exit reconnect, usage probing, and fail-closed diagnostics. The real local run completed `account/read` but both quota and usage reads required Codex account authentication, despite `codex login status` reporting ChatGPT login. No real Observation was emitted. See `docs/evidence/codex-p0-2026-09-06.md`. Status changed to `needs-info` pending a usable authenticated app-server session; fixture success does not satisfy the real-account gate.
- 2026-09-06 audit: Rechecked every criterion. The two real-account ticks remain open: the local handshake was real but returned no quota/usage, so neither a successful real Observation nor real-account evidence can be claimed. Checked items are covered by fixture/subprocess regression tests and are not treated as provider proof.
- 2026-09-06 supervisor follow-up: Optional `account/usage/read` failures after a successful quota response now preserve the quota Observation while recording distinct capability and diagnostic codes for authentication, permission, timeout, schema, unsupported-method, rate-limit, process-exit, and generic request failures. Sanitized regression fixtures cover authentication, permission, timeout, and schema cases; none fabricates Source usage.
- 2026-09-07 retry: Re-ran the live collector against Codex CLI `0.151.0-alpha.7.1`. `account/read` succeeded, but both quota and usage methods still returned `authentication_required`; no Observation was emitted. The two real-account ticks remain open.
- 2026-09-09 authenticated-readiness retry: Re-ran the real subprocess collector against Codex Desktop app-server `0.153.4`. The handshake and `account/read` succeeded, while `account/rateLimits/read` and `account/usage/read` returned `authentication_required`; `codex login status` independently reported `Not logged in`. No credentials or settings were changed and no Observation was emitted, so the two real-account ticks remain open pending an explicit user login. See `docs/handoffs/2026-09-09-live-provider-readiness.md`.
