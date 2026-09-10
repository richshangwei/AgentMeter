# 03: Prove GitHub Copilot permission and quota boundaries

**What to build:** A runnable Copilot collection experiment that determines which supported account and organization contexts can expose billing or quota data, distinguishes current AI-credit concepts from legacy usage concepts, and emits an Observation only when the source is authoritative enough.

**Blocked by:** None.

**Status:** needs-info

- [ ] The experiment tests the applicable Personal, Business, and Enterprise contexts that can be accessed with available real accounts and records untested contexts as evidence gaps.
- [x] Billing API authentication and permission requirements are identified through fixture-replayed request outcomes, with real scope/role verification remaining an evidence gap.
- [x] AI Credits, premium-request or legacy usage measures, included allowance, overage, units, and billing period are never conflated during normalization.
- [ ] A successful authoritative response produces a normalized Observation with account and quota scope; ambiguous or incomplete data is surfaced with explicit Data Quality and maturity.
- [x] Preview SDK quota data, if evaluated, is isolated as experimental and its stability, permissions, and divergence from the billing source are recorded.
- [x] Rate limits, authorization failures, unavailable endpoints, malformed responses, and schema changes produce distinct diagnosable outcomes.
- [x] Sanitized fixtures and reproducible evidence record plan type, API or SDK version, observed fields, test time, and limitations without exposing credentials or billing secrets.
- [x] The outcome states the supported Copilot account matrix for v1 and whether unsupported contexts are constrained, deferred, or release-blocking.

## Comments

- 2026-09-09 live-evidence runner: Added an explicit opt-in PowerShell runner that exercises each supplied Personal, Business, and Enterprise account against both billing meters, records omitted contexts as evidence gaps, refuses overwrite, and excludes provider stderr and credential material. The runner's process-double tests pass; no real provider request was made because `gh` and authenticated accounts remain unavailable. See `docs/handoffs/2026-09-09-copilot-live-runner.md`.

- 2026-09-09 settings follow-up: Added v2 desktop source persistence for the absolute `gh` path, billing context, slug, and meter, with v1 Claude-only compatibility, independent save/forget semantics, no token or response persistence, fail-closed validation, and a rebuilt package. This completes local setup persistence but does not close either real-account acceptance tick. See `docs/handoffs/2026-09-09-desktop-copilot-settings.md`.

- 2026-09-09 desktop integration: Moved live GitHub collection behind a shared root module and connected the Tauri Copilot card to it. The card now accepts the explicit CLI path/context/account/meter, participates in cancellation, strips raw diagnostics and monetary fields, and renders official usage without inventing remaining quota. The rebuilt NSIS contains the command, API version and quota-guard markers. Real-account ticks remain open because `gh` and authenticated account contexts are unavailable. See `docs/handoffs/2026-09-09-desktop-copilot-integration.md`.

- 2026-09-09 live-boundary follow-up: Added an explicit absolute-path `gh api` collection seam for the documented `2026-03-10` Personal, Organization/Business, and Enterprise AI Credit and Premium Request usage endpoints. It validates all six paths with a process double, bounded timeout/output, sanitized failure classes, exact unit separation, and schema checks. Official REST responses are retained as billing usage while allowance/remaining/reset stay unknown; no quota Observation is fabricated. `gh` and real accounts remain unavailable, so both real-account ticks stay open. See `docs/evidence/copilot-antigravity-live-contract-2026-09-09.md` and `docs/handoffs/2026-09-09-copilot-live-contract.md`.

- 2026-09-07 implementation follow-up: Added explicit endpoint permission/role contracts and missing-access diagnostics, distinct AI Credits/legacy request allowance and overage fields, and shared-pool versus individual-limit separation. Missing permission evidence or conflicting meter units prevent an Observation; absent values remain unknown. Thirteen Copilot tests pass. See `docs/handoffs/2026-09-07-copilot.md`; real account and authoritative response criteria remain open.

- 2026-09-06: Implemented an isolated capability-matrix collector covering Personal/Business/Enterprise contexts, AI Credits versus legacy premium requests, permission failures, unknown preservation, and preview SDK isolation with sanitized fixtures. Status remains `needs-info` pending real GitHub account/organization requests and versioned evidence.
- 2026-09-06 audit: Reopened the real-context and authoritative-response ticks because no real GitHub Provider Account, organization role, or Billing API response was available. The normalizer now emits replay-marked `local_observed` Observations only for complete fixture shapes, keeps preview data experimental, and models permission-denied, authentication, rate-limit/backoff, endpoint-unavailable, malformed, and schema-change outcomes with orthogonal axes. `docs/evidence/copilot-p0-2026-09-06.md` records synthetic plan/version labels but deliberately does not close the real versioned-evidence tick.
- 2026-09-06 supervisor follow-up: Preview SDK fixture output now records `public_preview` stability, permission contract/status, and an explicit Billing API comparison (`matched`, `diverged`, or `not_comparable`). Regression tests include a deliberate divergence and an unavailable comparison; all values remain marked synthetic fixture evidence.
- 2026-09-07: Added a machine-readable sanitized-fixture evidence manifest with fixture plan types, timezone-qualified test time, API/SDK contract versions, observed-field lists, explicit limitations, and negative credential/billing-secret declarations. This closes only the fixture/evidence-metadata criterion; real context and authoritative-response criteria remain open.
