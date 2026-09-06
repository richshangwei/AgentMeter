# 04: Prove Antigravity quota sources and fallback

**What to build:** A runnable Antigravity collection experiment that prefers structured statusLine quota data, validates its window and account scope, and evaluates a version-bounded headless text fallback without presenting parsed text as equally trustworthy.

**Blocked by:** None.

**Status:** needs-info

- [ ] Structured statusLine data is captured from a real authenticated installation and mapped to a normalized Observation with provenance.
- [ ] The experiment establishes whether reported quota windows, limits, usage or remaining values, resets, units, and account scope can be identified unambiguously.
- [ ] Missing quota fields remain unknown and do not cause a successful status event to be misrepresented as a fresh quota reading.
- [ ] The headless usage-text fallback is evaluated separately, tied to tested product versions, and assigned an explicitly lower Collector Maturity or Data Quality than the structured source.
- [ ] Text changes, localization, unexpected output, authentication failures, command failures, and structured schema drift produce distinct diagnostics, including `schema_changed` where applicable.
- [ ] Sanitized structured and text fixtures reproduce normalization and failure handling without requiring a live account.
- [ ] Evidence records tested versions, source precedence, representative fields or output, test time, limitations, and reproducible steps.
- [ ] The outcome states whether Antigravity is supported, constrained, or blocked for v1 and whether the fallback is permitted in a release build.

## Comments

- 2026-09-06: Implemented an isolated structured `statusLine` and version-bounded headless text experiment with sanitized fixtures, source precedence, unknown-field preservation, and schema/text diagnostics. Status remains `needs-info` pending a real authenticated Antigravity installation/version run and release decision for the experimental fallback.
