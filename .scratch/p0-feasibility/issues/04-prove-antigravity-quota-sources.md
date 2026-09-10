# 04: Prove Antigravity quota sources and fallback

**What to build:** A runnable Antigravity collection experiment that prefers structured statusLine quota data, validates its window and account scope, and evaluates a version-bounded headless text fallback without presenting parsed text as equally trustworthy.

**Blocked by:** None.

**Status:** needs-info

- [ ] Structured statusLine data is captured from a real authenticated installation and mapped to a normalized Observation with provenance.
- [ ] The experiment establishes whether reported quota windows, limits, usage or remaining values, resets, units, and account scope can be identified unambiguously.
- [x] Missing quota fields remain unknown and do not cause a successful status event to be misrepresented as a fresh quota reading.
- [ ] The headless usage-text fallback is evaluated separately, tied to tested product versions, and assigned an explicitly lower Collector Maturity or Data Quality than the structured source.
- [x] Text changes, localization, unexpected output, authentication failures, command failures, and structured schema drift produce distinct diagnostics, including `schema_changed` where applicable.
- [x] Sanitized structured and text fixtures reproduce normalization and failure handling without requiring a live account.
- [ ] Evidence records tested versions, source precedence, representative fields or output, test time, limitations, and reproducible steps.
- [x] The outcome states whether Antigravity is supported, constrained, or blocked for v1 and whether the fallback is permitted in a release build.

## Comments

- 2026-09-09 official-contract review: Google primary documentation exposes the interactive `agy` `/usage` command, but the reviewed sources do not document a stable non-interactive JSON/statusLine quota contract. No speculative live integration was added; fixture parsing and the text fallback remain release-prohibited. `gh`, `agy`, and `antigravity` are absent on this host, so all real-install/version criteria remain open. See `docs/evidence/copilot-antigravity-live-contract-2026-09-09.md`.

- 2026-09-06: Implemented an isolated structured `statusLine` and version-bounded headless text experiment with sanitized fixtures, source precedence, unknown-field preservation, and schema/text diagnostics. Status remains `needs-info` pending a real authenticated Antigravity installation/version run and release decision for the experimental fallback.
- 2026-09-06 audit: Added the specified structured quota-map path, bucket identity preservation, no-value `idle` behavior, exact-version fallback rejection, and authentication/command diagnostics. See `docs/evidence/antigravity-p0-2026-09-06.md`. Real structured/account-scope, real tested-version fallback, localization-specific diagnostics, and versioned evidence ticks remain open. The current outcome is constrained and the text fallback is prohibited from release until those gates and a release review pass.
- 2026-09-07: Added separate `localization_unsupported`, `text_changed`, and `unexpected_output` paths plus source-precedence and machine-readable fixture-version evidence. The diagnostic criterion is now closed. The fallback/version-evidence criteria remain open because `1.8.2` is fixture-declared rather than observed from a real installed product; release use remains prohibited.
