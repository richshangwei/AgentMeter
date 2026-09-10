# Handoff — Persistent desktop Copilot source settings

Date: 2026-09-09 (Asia/Taipei)  
Mode: single-agent  
Checklist: 50/79 checked, 29 open (unchanged)

## Outcome

The desktop Copilot source can now be saved, restored after restart, and explicitly forgotten. Only non-sensitive source identity is persisted: absolute `gh` path, billing context, account/organization slug, and meter kind. GitHub tokens, raw responses, usage values, diagnostics, and monetary data are never written to the settings file.

## Settings module

- Upgraded `sources.json` to schema version 2 with independent Claude and Copilot fields.
- Existing schema-v1 Claude settings are accepted and normalized in memory. The file is migrated to v2 only on the next explicit save, avoiding an unsolicited startup write.
- Saving Claude preserves Copilot; saving or forgetting Copilot preserves Claude.
- Writes retain the existing bounded, temporary-file, synchronized replacement behavior.
- Malformed JSON, unsupported versions, oversized documents, missing required fields, relative paths, invalid contexts, unsafe slugs, and invalid meter names fail closed without overwriting the previous file.
- The shared Copilot module validates the persisted absolute path and account shape. Executable existence is checked at collection time, so a removed/not-yet-installed CLI restores visibly and fails actionably instead of corrupting settings.

## Desktop behavior

- Added `儲存來源` and `忘記來源` actions to the Copilot card.
- Startup restores all four Copilot fields without automatically making a Provider request.
- Editing any field marks it unsaved.
- Forgetting clears only the persisted source fields. It does not alter `gh` authentication, delete external files, or erase the already-rendered historical result.
- UI and Tauri contract tests require both commands and continue to prohibit token input and invented remaining quota.

## Verification

`scripts/verify-local.ps1` passed:

```text
120 root Rust tests passed
12 desktop Rust tests passed
11 browser logic tests passed
PowerShell syntax, both Rust formatters, JavaScript syntax and warnings-denied Clippy passed
```

Desktop settings tests cover source independence, v1 read compatibility, explicit v2 migration, forget semantics, negative validation, corruption preservation, size limits, and absence of token-named fields.

## Rebuilt package

- Path: `desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- Installer size: 2,095,574 bytes
- Installer SHA-256: `F6B86C80A646375A0869012B0C09ACE7CD8F8281E8573DBDE9D099E8644F1AB3`
- Installer Authenticode: `NotSigned`
- Embedded executable size: 9,582,080 bytes
- Embedded executable SHA-256: `6B9A5FE5582D47037CE78957117E111E970875D1EE179604448AE261CEDAC2AB`
- Embedded executable Authenticode: `NotSigned`
- Product/file version: `0.1.0`
- WebView2 download component: present
- Startup diagnostic markers: present
- Copilot refresh/save/clear, API-version and no-invented-quota markers: present

The rebuilt application was opened visibly and remained responsive as PID 18956.

## Formal acceptance boundary

This closes the local desktop-settings development gap, not a real-provider acceptance criterion. `gh` and authenticated GitHub billing contexts remain unavailable on the host; no official response or remaining-quota Observation is claimed. Formal status remains **50/79 checked, 29 open**.
