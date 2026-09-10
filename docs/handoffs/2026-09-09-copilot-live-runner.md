# Handoff — Copilot live evidence runner

Date: 2026-09-09 (Asia/Taipei)  
Mode: single-agent  
Checklist: 50/79 checked, 29 open

## Outcome

Added a controlled PowerShell runner for collecting sanitized, reproducible Copilot billing evidence from the shared collector. The runner is deliberately opt-in: it refuses to contact GitHub unless `-AllowProviderRequests` is supplied, accepts only absolute existing collector and `gh` paths, refuses to overwrite an evidence file, and records no provider stderr, token material, or raw credential data.

The runner tests each supplied Personal, Business, and Enterprise account against both `ai-credits` and `premium-requests`. Contexts without an explicitly supplied account slug are recorded as evidence gaps rather than silently skipped. Billing usage remains usage-only; included allowance, remaining quota, and reset are not inferred.

## Files and behavior

- `scripts/test-copilot-live.ps1` runs the six supported context/meter combinations that have account slugs, captures the collector's documented JSON contract, deletes temporary stderr, and writes `agentmeter.copilot-live-evidence/v1` using an atomic create-only write.
- `tests/copilot_live_runner_script.rs` covers authorization refusal, six separate requests with sanitized evidence, and partial account matrices with explicit untested contexts.
- `src/bin/fake-gh.rs` adds a `matrix` process-double mode that verifies the Personal, Organization/Business, and Enterprise endpoint families and both meter endpoints.
- `scripts/verify-local.ps1` now includes the runner script in PowerShell syntax checks.

## Verification completed

- PowerShell parser check for `scripts/test-copilot-live.ps1`: passed.
- `cargo test --offline --locked --test copilot_live_runner_script -- --nocapture`: **3 passed**.
- `cargo test --offline --locked --test copilot_quota -- --nocapture`: **17 passed**.
- Current checklist recount: **50 checked / 29 open / 79 total**.

The full `scripts/verify-local.ps1` suite was rerun after this runner batch and passed: PowerShell syntax, root/desktop formatting, root/desktop tests, browser logic tests, both JavaScript syntax checks, and root/desktop Clippy. The NSIS package was already rebuilt and inspected before these script/test-only changes; no package rebuild was required for this batch.

## Runtime and evidence boundary

The rebuilt desktop application remains open and responsive as PID `18956` at:

`D:\WorkSpace\AgentMeter\desktop-p0\target\x86_64-pc-windows-msvc\release\agentmeter-desktop-p0.exe`

The host still has no discoverable `gh`, `agy`, or `antigravity` executable and no authenticated GitHub account evidence. No real provider request was made in this batch. The runner is ready for an operator who has already authenticated `gh` and explicitly supplies account slugs.

## Next operator step

After installing/authenticating GitHub CLI, run an example such as:

```powershell
& .\scripts\test-copilot-live.ps1 `
  -CollectorPath 'D:\WorkSpace\AgentMeter\desktop-p0\target\x86_64-pc-windows-msvc\release\agentmeter-copilot-p0.exe' `
  -GitHubCliPath 'C:\Program Files\GitHub CLI\gh.exe' `
  -OutputPath 'D:\WorkSpace\AgentMeter\docs\evidence\copilot-live-2026-09.json' `
  -PersonalAccount 'YOUR_GITHUB_LOGIN' `
  -Year 2026 -Month 9 -AllowProviderRequests
```

Do not put a token on the command line or in the evidence file. Review the resulting `outcome`, `results`, and `untested_contexts`; a successful response still does not establish remaining quota.

## Remaining gates

The real-account context and authoritative remaining-quota acceptance ticks remain open. The local verifier is current; the next operator step is to use the runner only with user-authorized real account contexts and attach sanitized evidence.
