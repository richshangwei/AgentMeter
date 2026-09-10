# Handoff — guided local setup and reversible Claude integration

Date: 2026-09-09 (Asia/Taipei)

This is one implementation batch toward the user's request to continue the unfinished product features. It does not close that overall objective or any real-account P0 acceptance tick.

## Delivered

- The desktop has a local setup summary with automatic tool discovery and next-step guidance. Inspection makes no provider requests. ADB discovery fills an empty field but preserves an explicit source.
- Claude has a configuration preview dialog and explicit enable/restore actions. Preview is read-only; apply creates a byte-for-byte backup of an existing settings file, preserves other settings and existing statusLine properties/output, and rejects changes detected since preview. Missing settings and a null versus absent statusLine are handled distinctly.
- Setup reads only user-level settings resolved from CLAUDE_CONFIG_DIR or USERPROFILE/.claude. Project/managed settings may override this source; the UI does not claim effective configuration or authenticated success from installation detection.
- The normalizer is shared between the old P0 CLI and the built-in desktop receiver. The desktop handles --claude-status before Tauri or the single-instance plugin starts. The generated shell command encodes UTF-16LE PowerShell script text so spaces, apostrophes and non-ASCII paths survive the outer shell.
- The receiver preserves the existing display command using the detected native Git Bash or Windows PowerShell, passes event input in memory, writes only an allowlisted report atomically, and bounds the display subprocess wait. Original display stdout is intentionally preserved, not republished in the dashboard.
- Enable saves the generated report source and starts the existing watcher; disable stops that watcher and restores the statusLine. Backups, manifest and old reports are retained. Invalid events now produce a read failure rather than a misleading successful empty report.

## Verification

- Full scripts/verify-local.ps1: passed, including root tests, 15 desktop tests, 15 browser tests, syntax/formatting and root/desktop Clippy.
- tests/claude_setup.rs: 5 tests cover preview/backup/restore, conflict rejection, missing/null/invalid configuration, corrupted manifest and actual PowerShell receiver execution with sanitized events.
- The receiver execution test also passed against desktop-p0/target/release/agentmeter-desktop-p0.exe via AGENTMETER_TEST_DESKTOP_RECEIVER. This proves the GUI-subsystem executable's redirected input/output path, not a real Claude session.
- No real Claude user configuration was changed; fixture settings were confined to temporary test directories. No authenticated provider requests were made.

## Remaining product work and limitations

- The overall scope remains the unfinished items in AgentMeter-Requirements-v1.1.md: live desktop/tablet data sharing, unified scheduling/pause/retry, SQLite/history, notifications, account isolation, device selection/recovery UX, diagnostics/data operations, installer/update/signing, and real-account/VM/tablet acceptance.
- Setup inspection is discovery, not a full version/authentication scan. The installation/login guidance is still textual; organization/enterprise account picking is still advanced configuration.
- Claude real installation, Git Bash coexistence and effective project/managed-settings behavior need target-environment verification. A custom Git Bash installation should use CLAUDE_CODE_GIT_BASH_PATH; an invalid override refuses shell selection.
- Receiver configuration references the desktop executable's absolute path. Do not move/remove that executable while integration is enabled. Installer uninstall/relocation integration must restore owned Claude settings before deleting it; this is not yet implemented. Until then use the desktop's disable action first.
- The new desktop setup flow is separate from the legacy P0 CLI manifest. Existing legacy integrations are refused rather than wrapped again; restore them with the original tool before enabling the new flow.
- Backup and settings replacement are separate operations with pre/post-backup checks, not a cross-process transaction. A backup/manifest may remain after a failed write; the UI reports failure and requires a fresh preview. Backups can contain the user's original configuration and are local-only, not diagnostic exports.
- NSIS has not been regenerated for this batch. Existing installers do not contain this flow. The running old desktop instance was not replaced.

## References

- https://code.claude.com/docs/en/statusline — Windows shell selection, event stdin/stdout and statusLine settings.
- https://code.claude.com/docs/en/settings — user settings location and configuration precedence.
