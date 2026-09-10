# Source settings persistence handoff — 2026-09-08

Single-agent implementation; no independent reviewer.

## Delivered

Added explicit Save Path and Forget Path controls for the Claude report Source. The selected path is stored as versioned JSON in Tauri's per-user app config directory, sources.json. It contains a path only, not credentials or imported observations. Startup restores the input field but does not read the report or enable monitoring. Forgetting the saved path stops monitoring and clears the input, without deleting the original report or claiming that old displayed values are current.

Writes use a unique create-new temporary file, sync it, and rename over the destination. A process-local mutex serializes commands. Read and serialized-write limits are both 16 KiB. Invalid paths are rejected; malformed, oversized or unknown-version settings report errors rather than silently resetting. Paths rely on the Windows user boundary, consistent with ADR 0008; they must be excluded from future diagnostic exports.

## Verification

- Desktop Rust tests: 6 passed, including read-after-write, replacement, forget, invalid-path preservation and malformed/unknown-version/oversized rejection.
- Source monitor JavaScript tests: 4 passed.
- Dashboard JavaScript syntax check passed.
- Desktop all-targets Clippy with warnings denied passed before the final serialized-size guard; subsequent Rust tests passed.
- Final offline locked desktop build passed.

Tests used isolated temporary directories and cleaned up only their own files. No actual user source configuration was saved during this batch. GUI restart/IPC verification remains outstanding; disk round-trip tests are not claimed as full interactive evidence.

## Remaining

Verify save/restore/forget controls in the desktop UI, including a real application restart. Continue authenticated Provider integration, desktop-owned collector shutdown, tablet client rendering and outstanding external acceptance gates. Formal P0 count remains 47/79 complete, 32 open; this batch closes no device/provider acceptance gate. Worktree changes remain uncommitted.
