# Copilot and Antigravity live-contract research — 2026-09-09

## Decision

GitHub has a documented, versioned REST contract suitable for an experimental live collection seam. Google documents Antigravity's interactive `/usage` command, but the primary sources reviewed here do not document a non-interactive JSON/statusLine quota contract. AgentMeter therefore implements only the GitHub live seam and keeps Antigravity fixture-only and release-prohibited.

## GitHub Copilot billing contract

The selected API version is `2026-03-10`. GitHub documents two distinct measures at each supported billing level:

| Context | AI Credits endpoint | Premium Requests endpoint | Minimum documented permission |
|---|---|---|---|
| Personal | `/users/{username}/settings/billing/ai_credit/usage` | `/users/{username}/settings/billing/premium_request/usage` | User `Plan: read` |
| Organization / Business | `/organizations/{org}/settings/billing/ai_credit/usage` | `/organizations/{org}/settings/billing/premium_request/usage` | Organization `Administration: read`; organization administrator |
| Enterprise | `/enterprises/{enterprise}/settings/billing/ai_credit/usage` | `/enterprises/{enterprise}/settings/billing/premium_request/usage` | Enterprise billing `read`; enterprise administrator, billing manager, or qualifying custom role |

Sources: [GitHub user and organization billing usage](https://docs.github.com/en/rest/billing/usage?apiVersion=2026-03-10) and [GitHub Enterprise Cloud billing usage](https://docs.github.com/en/enterprise-cloud@latest/rest/billing/usage?apiVersion=2026-03-10).

The response reports `timePeriod`, the billed account, and `usageItems` containing product, SKU, model, unit type, gross quantity, discounts, net quantity, and monetary amounts. It does **not** document an included allowance, remaining quota, or quota reset. Consequently:

- AgentMeter may label a validated live response as official billing **usage**.
- AgentMeter must not derive remaining quota by subtracting usage from a hard-coded plan allowance.
- AI Credits and Premium Requests remain separate meter types and units.
- A successful Billing REST response does not, by itself, produce a remaining-quota Observation.

GitHub separately documents Copilot SDK `account.getQuota`, including used requests, remaining percentage, and reset date, but also distinguishes session metrics from authoritative billing. The SDK path remains experimental and is not treated as a stable replacement for Billing REST evidence: [Copilot SDK usage and billing metrics](https://docs.github.com/en/copilot/how-tos/copilot-sdk/features/usage-and-billing).

## Implemented live boundary

`agentmeter-copilot-p0 collect` now invokes an explicitly supplied absolute `gh` executable and never accepts a token argument. It supplies the official media type and API-version header, filters the API request to the Copilot product, validates the account slug and period, bounds execution to 1–60 seconds, caps each output stream at 256 KiB, parses only the documented response shape, and sanitizes process failures.

The same implementation is exposed through the Tauri desktop card. The shared module returns a sanitized report; the desktop adapter further strips raw diagnostics, monetary fields and unrelated response data before rendering. Full application exit shares the cancellation signal with the GitHub CLI subprocess.

Desktop source settings use schema v2 and persist only the absolute CLI path, context, slug, and meter. Existing schema-v1 Claude-only settings remain readable and migrate only on a later explicit save. Tokens and collected response data are outside this settings seam.

The process-double matrix covers all six context/meter endpoint combinations plus authentication, permission, rate-limit, missing-endpoint, bad-request, provider-unavailable, malformed/schema-changed, timeout, over-size output, relative executable path, and unsafe slug cases. This proves the local process and parser boundary; it is not a real GitHub account result.

The opt-in `scripts/test-copilot-live.ps1` runner now provides a reproducible operator boundary for real evidence. It requires an explicit `-AllowProviderRequests`, absolute existing collector and `gh` paths, an explicit output path that must not already exist, and at least one account slug. It invokes both meters per supplied context, records missing contexts as `untested_contexts`, deletes temporary provider stderr, stores collector/CLI hashes and sanitized JSON only, and emits `success`, `completed_with_gaps`, or `failure`. Its three Windows integration tests pass; this remains a readiness seam, not proof of a real provider response.

## Antigravity boundary

Google's official codelab identifies `agy` as the Antigravity CLI and documents `/usage` as an interactive command for checking model usage: [Spec-driven development with Antigravity CLI](https://codelabs.developers.google.com/sdd-agy-cli). The reviewed official getting-started material describes Antigravity CLI as an interactive TUI but does not expose a documented machine-readable quota command: [Getting started with Antigravity IDE](https://codelabs.developers.google.com/getting-started-agy-ide).

The statement above is a bounded research finding, not proof that no private or future interface exists. Until Google publishes a stable non-interactive contract or a real installed version supplies a verified structured event, AgentMeter must not screen-scrape `/usage`, infer a `statusLine` schema, or enable the fixture text parser in release builds.

## Host evidence and remaining gate

On 2026-09-09, `gh`, `agy`, and `antigravity` were not discoverable on this Windows host. No account, organization, enterprise, product version, authentication, quota, or billing response is claimed. Closing the real-context gates still requires user-authorized login on an available account followed by sanitized execution evidence.

## Reproduction

```powershell
cargo test --offline --locked --test copilot_quota -- --nocapture
cargo test --offline --locked --manifest-path desktop-p0/Cargo.toml -- --nocapture

# Example only after gh is installed and already authenticated:
cargo run --offline --locked --bin agentmeter-copilot-p0 -- collect `
  --gh-bin C:\Program Files\GitHub CLI\gh.exe `
  --context personal --account YOUR_GITHUB_LOGIN `
  --meter ai-credits --year 2026 --month 9
```

Do not put a token on the command line or in an evidence file.
