# Handoff — source discovery and simplified connection

Date: 2026-09-09 (Asia/Taipei)

## Delivered

- Copilot's primary action now accepts empty paths and account fields. It locates `gh.exe` through absolute PATH entries and common Windows installation locations, then resolves the current authenticated personal identity via `GET /user`. Requests explicitly target github.com, run with bounded output/time and cancellation, and do not display a console window.
- Claude's primary action accepts an empty report path and looks for the wrapper's existing `settings.agentmeter-statusline.observation.json` under CLAUDE_CONFIG_DIR, or USERPROFILE/.claude when no override is set. Existing report validation still applies. Discovery does not enable the wrapper.
- Manual fields are collapsed under advanced settings. Empty Copilot values persist automatic discovery; explicit existing sources still take precedence. Organization/enterprise selection is not inferred from a personal identity.
- Antigravity is labeled as not supporting automatic connection rather than asking users to configure an unavailable desktop integration.

Identity API reference: https://docs.github.com/en/rest/users/users#get-the-authenticated-user

## Verification

All root tests, 14 desktop tests, 11 browser tests, PowerShell and JavaScript syntax checks, and root/desktop formatting passed. The verifier then found a Clippy test-module ordering issue; after moving the module, both root and desktop Clippy passed with warnings denied, and formatting was rechecked. No authenticated provider request was performed.

Release build passed. New executable:

`D:\WorkSpace\AgentMeter\desktop-p0\target\release\agentmeter-desktop-p0.exe`

The previously running executable and the NSIS installer were not replaced. Exit the previous instance through its tray menu before launching the new executable; the single-instance guard otherwise activates the old instance.

## Remaining work

- First-use installation/login guidance still includes a manual `gh auth login` step. Add a desktop-guided connection experience.
- Claude still requires its event wrapper to have been enabled. Add a reversible enable/disable setup flow that preserves an existing statusLine, with configuration preview and explicit enable action.
- Organization/enterprise discovery and account selection are not implemented; advanced settings remain necessary for company billing.
- Antigravity has no desktop live connection yet.
- Copilot billing usage does not establish remaining quota. Real-account evidence and the earlier acceptance gates remain open.
- Rebuild and inspect NSIS after the next desktop batch; do not distribute the old installer as including these changes.
