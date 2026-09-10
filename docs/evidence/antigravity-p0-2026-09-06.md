# Google Antigravity quota-source P0 evidence — 2026-09-06

## Result

**Constrained; the collector is not release-enabled.** The offline experiment accepts the structured statusLine `quota` map (and legacy array fixtures), derives bucket identity from map keys, preserves unknown values, and does not mark an event with no numeric quota as a ready quota Observation. Structured data has precedence over the headless text fallback.

The text fallback is `estimated` / `experimental` and is allowed only for the exact fixture-tested version `1.8.2`; this is a parser test boundary, not evidence that real Antigravity 1.8.2 has that output. It is **not permitted in a release build** until a real versioned run and explicit release review approve it.

`antigravity` was not discoverable on `PATH`, so no real installation, Provider Account, quota scope, localization, or authenticated output is claimed.

The 2026-09-09 [official-contract review](copilot-antigravity-live-contract-2026-09-09.md) found documentation for the interactive `agy` `/usage` command but no documented non-interactive JSON/statusLine quota interface. That bounded finding reinforces the existing fail-closed decision: do not invent a live command or release-enable screen scraping.

## Reproduction

```powershell
cargo test --offline --locked --test antigravity_quota -- --nocapture
cargo run --offline --locked --quiet --bin agentmeter-antigravity-p0 -- antigravity collect --fixture tests/fixtures/antigravity/structured.json
```

Sanitized fixtures cover structured map data, missing values, headless text, an unrecognized text/version, authentication failure, and schema drift. Tests also distinguish missing-file command failure and reject an untested minor version.

The report carries a machine-readable evidence boundary: `tested_version: 1.8.2`, `version_evidence: fixture_declared`, timezone-qualified `tested_at`, structured-first source precedence, `replay: true`, explicit limitations, and `fallback_release_policy: prohibited_pending_real_version_validation`. This is versioned parser evidence only.

| Fixture condition | Diagnostic |
|---|---|
| Unsupported locale, including `zh-TW` | `localization_unsupported` |
| Known version with changed format revision | `text_changed` |
| Validated version/locale/revision with unparseable output | `unexpected_output` |
| Authentication required | `authentication_failed` |
| Missing fixture/provider command | `command_failed` |
| Structured schema revision changed | `schema_changed` |

The structured fixture also contains unusable headless text; the test confirms structured statusLine data wins and the fallback is not selected.

## Required real-install gate

Record a product-reported version, account/scope, test time, structured field names, bucket meanings, units, reset forms, and sanitized output from an authenticated installation. Repeat with missing windows/values and a changed schema. Separately capture the headless command in the reported version and locale, rerun the diagnostic matrix, confirm structured precedence, and make an explicit release decision for the fallback. Only this real run can replace `version_evidence: fixture_declared` and close the version-dependent criteria.
