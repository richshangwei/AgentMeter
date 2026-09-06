# 10: Publish the P0 capability matrix and gate P1

**What to build:** A decision-ready P0 capability report that integrates the nine experiments, assigns each capability a supported, constrained, or blocked result, and translates the evidence into explicit v1 scope, setup requirements, risks, and go/no-go criteria for P1.

**Blocked by:** 01: Prove Codex app-server can produce trusted Observations; 02: Prove Claude statusLine coexistence and quota collection; 03: Prove GitHub Copilot permission and quota boundaries; 04: Prove Antigravity quota sources and fallback; 05: Prove the Windows desktop lifecycle; 06: Prove clean Windows installation and WebView2 handling; 07: Prove the selected-device USB loopback channel; 08: Prove the Device Pair and Tablet Session security boundary; 09: Prove Dashboard Snapshot streaming and asynchronous refresh.

**Status:** needs-info

- [x] The matrix covers all four provider collectors, Windows lifecycle, clean installation and WebView2, selected-device USB transport, pair and session security, Dashboard Snapshot streaming, and asynchronous refresh.
- [x] Every row links reproducible evidence and records tested versions, environments, account or device prerequisites, limitations, Data Quality, and Collector Maturity where applicable.
- [x] Each capability is classified as supported, constrained, or blocked using stated criteria rather than an undocumented judgment.
- [x] Constrained results define the exact v1 user-visible limitation, setup requirement, fallback, telemetry or diagnostic need, and follow-up work.
- [x] Blocked results identify the unmet requirement, attempted alternatives, release impact, and the product or architecture decision required before proceeding.
- [x] Cross-experiment assumptions are reconciled, including account identity, source precedence, lifecycle ownership, local-only transport, authorization, revisions, and failure-state terminology.
- [x] Any requirements or ADR changes discovered by the experiments are proposed explicitly and are not silently folded into the implementation plan.
- [x] The report concludes with an auditable P1 go/no-go decision and a prioritized list of remaining work without treating mocked evidence as proof of real-world viability.

## Comments

- 2026-09-06: Published `docs/evidence/p0-capability-gate-2026-09-06.md`. All nine capabilities are currently constrained: fixture experiments establish protocol seams and fail-closed behavior, while real provider accounts, clean Windows/Tauri VM, physical Android device, protected storage, and authenticated tablet evidence remain outstanding. P1 implementation may proceed behind explicit experimental gates; production release is no-go until the listed real-world acceptance gates pass.
