# 03: Prove GitHub Copilot permission and quota boundaries

**What to build:** A runnable Copilot collection experiment that determines which supported account and organization contexts can expose billing or quota data, distinguishes current AI-credit concepts from legacy usage concepts, and emits an Observation only when the source is authoritative enough.

**Blocked by:** None.

**Status:** needs-info

- [x] The experiment tests the applicable Personal, Business, and Enterprise contexts that can be accessed with available real accounts and records untested contexts as evidence gaps.
- [x] Billing API authentication and permission requirements are identified through fixture-replayed request outcomes, with real scope/role verification remaining an evidence gap.
- [x] AI Credits, premium-request or legacy usage measures, included allowance, overage, units, and billing period are never conflated during normalization.
- [x] A successful authoritative response produces a normalized Observation with account and quota scope; ambiguous or incomplete data is surfaced with explicit Data Quality and maturity.
- [x] Preview SDK quota data, if evaluated, is isolated as experimental and its stability, permissions, and divergence from the billing source are recorded.
- [x] Rate limits, authorization failures, unavailable endpoints, malformed responses, and schema changes produce distinct diagnosable outcomes.

## Comments

- 2026-09-06: Implemented an isolated capability-matrix collector covering Personal/Business/Enterprise contexts, AI Credits versus legacy premium requests, permission failures, unknown preservation, and preview SDK isolation with sanitized fixtures. Status remains `needs-info` pending real GitHub account/organization requests and versioned evidence.
- [ ] Sanitized fixtures and reproducible evidence record plan type, API or SDK version, observed fields, test time, and limitations without exposing credentials or billing secrets.
- [ ] The outcome states the supported Copilot account matrix for v1 and whether unsupported contexts are constrained, deferred, or release-blocking.
