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

- 2026-09-06: Implemented the Rust P0 CLI, fixture replay, real subprocess protocol handshake, normalized Observation output, multi-bucket quota handling, and fail-closed diagnostics. The real local run completed `account/read` but `account/rateLimits/read` required Codex account authentication, despite `codex login status` reporting ChatGPT login. No real Observation was emitted. See `docs/evidence/codex-p0-2026-09-06.md`. Status changed to `needs-info` pending a usable authenticated app-server session; fixture success does not satisfy the real-account gate.
