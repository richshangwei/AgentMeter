# Handoff — Copilot live Billing API boundary and Antigravity contract review

Date: 2026-09-09 (Asia/Taipei)  
Mode: single-agent  
Checklist: 50/79 checked, 29 open (unchanged)

## Outcome

This batch replaced the Copilot fixture-only executable boundary with a safe, runnable live GitHub CLI path while preserving the fixture suite. It did not close a formal acceptance tick because this host has no `gh` executable or authenticated Personal, Organization, or Enterprise account. Antigravity remains fixture-only because the reviewed Google documentation exposes interactive `/usage` but not a stable machine-readable quota contract.

## Implemented

- Added `agentmeter-copilot-p0 collect` with explicit absolute `--gh-bin`, `--context`, `--account`, and `--meter` inputs.
- Implemented all six documented GitHub Billing API combinations:
  - Personal, Organization/Business, Enterprise.
  - AI Credits and Premium Requests.
- Fixed the API contract at `2026-03-10` and supplies GitHub's recommended JSON media type.
- Added strict account-slug, year, month, executable-path, and 1–60 second timeout validation.
- Added bounded process execution with 256 KiB caps for stdout and stderr and forced termination on timeout.
- Added sanitized classification for authentication, permission, rate limit, unavailable endpoint, invalid request, provider failure, malformed JSON, schema drift, timeout, and excessive output.
- Normalizes documented gross, discounted, and net usage with product/SKU/model/unit provenance.
- Deliberately leaves allowance, remaining quota, and reset unknown; billing usage is never relabeled as remaining quota and does not emit a quota Observation.
- Added the test-only `fake-gh` process double. No network or credentials are used by tests.
- Recorded the official-contract analysis and the Antigravity fail-closed decision in `docs/evidence/copilot-antigravity-live-contract-2026-09-09.md`.

## Verification

Targeted checks:

```text
cargo test --offline --locked --test copilot_quota -- --nocapture
17 passed; 0 failed

cargo clippy --offline --locked --bin agentmeter-copilot-p0 --bin fake-gh --test copilot_quota -- -D warnings
passed

cargo fmt --all -- --check
passed

git diff --check -- <batch files>
passed (line-ending notices only)
```

Full repository verifier:

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/verify-local.ps1
119 root Rust tests passed
9 desktop Rust tests passed
11 browser logic tests passed
PowerShell syntax, Rust formatting, JavaScript syntax, root clippy and desktop clippy passed
```

Host discovery on 2026-09-09 found no `gh`, `agy`, or `antigravity` command. This batch therefore claims no live Provider Account response.

## Formal checklist decision

No acceptance checkbox was closed. The count remains **50/79 checked, 29 open**.

- Copilot #03 real-context criterion remains open until available Personal/Business/Enterprise accounts are exercised and missing contexts are explicitly recorded.
- Copilot #03 authoritative quota Observation criterion remains open because Billing REST is authoritative for usage but does not expose included allowance or remaining quota. The documented Copilot SDK quota path remains experimental and has no installed/authenticated runtime evidence here.
- Antigravity #04 real-install, structured source, fallback-version, and evidence criteria remain open.

## Next operator-ready step

Install GitHub CLI through the user's normal trusted software channel, authenticate it outside AgentMeter, then run the documented command with an explicit absolute executable path. Capture only sanitized account kind, role/permission outcome, API version, time period, field names, units, and values approved for evidence. Never place a token on the command line or in a handoff.

If Antigravity becomes available, first record its actual product-reported version and inspect officially supported commands. Do not enable the current text parser unless a real version/localization matrix and explicit release decision pass.
