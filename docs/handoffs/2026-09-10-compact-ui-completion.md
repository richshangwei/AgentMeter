# 2026-09-10 Claude compact desktop UI completion

See docs/evidence/desktop-compact-ui-2026-09-10.md for implementation, tests and installer inspection results.

- Compact UI saved; four cards at1080px and default window640px.
- Actual pre-cleanup desktop WebView acceptance passed4/4 with3/2/1/4 quota windows. Evidence: docs/evidence/desktop-webview-quota-2026-09-10.json.
- Temporary DEBUG-quota-desktop instrumentation removed only after that acceptance; permanent startup diagnostics and path normalization test retained.
- Final verify-local.ps1 passed after cleanup. Edge fixture tests cover realistic window counts/labels, stale/failure/Claude trust states and responsive tablet controls.
- Do not confuse fixture screenshots with real account screenshots. The new installer has not been installed or clean-VM tested in this task.
- No source changes committed; preserve the pre-existing dirty worktree.
