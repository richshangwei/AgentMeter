# Lessons

## 2026-11-17 PowerShell single-result Git queries can collapse arrays
- Classification: incorrect environment assumption.
- Signal: a safe remote-ref preflight rejected matching refs because a single output line became a scalar string and indexing selected a character.
- Prevention: explicitly wrap command output in an array before count or index checks.
- Tripwire: use `@(git ls-remote ...)` and verify both the ref name and full commit ID before creating a release.

## 2026-11-16 已提交的衝突標記不能依賴 Git 狀態發現
- Classification: missing verification / merge integrity.
- Signal: merge commit contained conflict markers in tracked files while `git status` was clean; Cargo could not parse the manifest.
- Prevention: inspect the resulting tree and run targeted parsing and tests before committing or pushing a merge.
- Tripwire: `git grep -n -e "^<<<<<<< " -e "^=======$" -e "^>>>>>>> "` must return no matches, in addition to a successful `cargo metadata`.

## 2026-09-17 Boolean environment flags require the documented literal
- Classification: incorrect integration assumption / insufficient regression control.
- Signal: AgentMeter set `AGY_CLI_DISABLE_AUTO_UPDATE=1`, while Antigravity documents the accepted opt-out as `true`; the 15-minute updater path could therefore still launch a visible CMD even though source-regex tests passed.
- Prevention: copy third-party environment values exactly from current first-party documentation and test both the accepted literal and rejection of the prior value.
- Tripwire: `tests/background_window_contract.test.cjs` requires `true` and rejects `1`; `scripts/test-refresh-console.ps1 -AntigravityFixture` opens a visible updater CMD whenever the value is not exactly `true`, proving the visual observer can catch the regression.

## 2026-09-12 Non-focusable HUD windows still intercept clicks (superseded by draggable HUD)
- Current design: the HUD accepts pointer events for dragging; tray overlap requires separate native validation.
- Classification: misunderstood Windows interaction semantics / regression from a visual affordance.
- Signal: enabling HUD hover required removing `set_ignore_cursor_events(true)`, after which the always-on-top bottom-right WebView covered the Windows tray flyout and prevented clicking its items.
- Prevention: a passive HUD must remain both non-focusable and native click-through; do not add hover behavior unless there is a separate deliberate interaction mode that temporarily changes the native hit-test policy.
- Tripwire: `compact_hud_is_a_click_through_hidden_secondary_window` requires native cursor-event passthrough, rejects `main:hover`, and requires settings copy to disclose that clicks pass through.

## 2026-09-12 Compact HUD values require semantic quota selection
- Classification: incorrect data selection / incomplete interaction verification.
- Signal: the HUD used `quota_windows[0]`, so a reordered Codex response displayed Spark or reserve-model quota instead of the account's regular allowance; container-level transparency also risked fading content.
- Prevention: select Codex quota windows by stable `limit_id` and authoritative duration, apply transparency only to the panel background, and keep text opacity independent of the configured background alpha.
- Tripwire: `tests/hud.test.cjs` uses adversarial quota ordering, while `scripts/verify-hud.cjs` requires 240×132 containment and text opacity 1 across the supported background-alpha range.

## 2026-09-12 Shared audit files can change between release commits
- Classification: unsafe change scope / shared-workspace race.
- Signal: a release-evidence commit staged all of `tasks/todo.md` after another task appended an unrelated specification plan.
- Prevention: immediately before every commit, compare the staged file tail and name-status against the task baseline; for shared audit files, stage only the intended hunk or isolate a task-specific evidence file.
- Tripwire: `git show --stat --oneline HEAD` plus `git show --format= -- tasks/todo.md` must contain only the current release section before pushing.

## 2026-09-12 Release worktrees need regenerated ignored runtimes
- Classification: environment-dependent verification setup.
- Signal: a clean release worktree passed source tests but the desktop build could not find `resources/quota-helper`, then the background-window contract lacked the ignored `scripts/quota-smoke-support/node_modules` dependency.
- Prevention: before verification or packaging in a new worktree, stage the pinned CLI fixture and run `scripts/prepare-desktop-quota.ps1`; never assume ignored runtime directories follow tracked source.
- Tripwire: the full verifier must reach both Desktop tests and the ConPTY background-window contract from the exact release worktree before signing.

## 2026-09-11 Verify release-host filename normalization before freezing an updater allowlist
- Failure: the updater and manifest pinned Tauri's default NSIS filename containing a space, but GitHub normalized that space to a period on upload; the immutable 0.2.0 client therefore requested a 404 URL.
- Detection: publish as a non-Latest prerelease first and compare the exact manifest URL against the actual asset URL using unauthenticated HTTP requests.
- Prevention: release assets use an ASCII safe filename with letters, digits, hyphen, underscore and period only. Unit tests reject `%20`/spaces; every release gate verifies exact uploaded names and HTTP 200 before setting Latest.

## 2026-09-11 Quota transport fields are not product terminology
- Classification: misunderstood domain data / missing metadata propagation.
- Signal: Codex `primary` and `secondary` were rendered as 「主視窗／次視窗」 and an internal `base_model_inference` ID became 「基本模型」, although the API supplied actual window durations and a displayable limit name.
- Prevention: preserve `limitId`, `limitName`, `planType`, and `windowDurationMins` end to end; derive user-facing periods from duration and let the returned rows decide whether five-hour, weekly, or both limits exist.
- Tripwire: Plus- and Pro-shaped quota parser fixtures plus desktop/tablet label tests must reject `primary` / `secondary` in product-facing copy.

## 2026-09-11 Signing bootstrap under Windows execution policies
- Signal: redirected native stderr with PowerShell Stop treated a warning as failure; rewriting existing directory ACLs required unavailable security privileges.
- Prevention: capture sensitive CLI output via hidden ProcessStartInfo and judge exit codes. Generate private keys in memory and persist only DPAPI ciphertext, never rely on temporary plaintext file ACLs. Do not log captured output or claim portable backup from CurrentUser DPAPI.

## 2026-09-11 Check sibling relationships in quota cards
- Failure: percentage sizing checks missed the period label sharing the same grid cell and using width-dependent percentage margins.
- Fix: allocate separate grid columns for gauge/value and period/progress/reset information.
- Prevention: verify visible sibling rectangles do not intersect across every count/viewport, including compact layouts. Do not claim a component is verified from its outer bounds alone.

## 2026-09-10 Missing verification
- Signal: rendered compact UI height exceeded 640px although DOM unit tests passed.
- Prevention: measure real browser geometry at target viewport, with success and stale/error data before declaring layout complete.
- Tripwire: run node .scratch/verify-compact-ui.cjs.

## 2026-09-10 Test harness assumption
- Signal: added class queries encountered undefined className in lightweight mock elements.
- Prevention: model required DOM defaults in the test harness and run the targeted test immediately.

## 2026-09-10 Inherited build validation
- Signal: full verifier caught collapsible_if in the previously added path-normalization fix.
- Prevention: run warnings-denied Clippy before describing inherited changes as verified.

## 2026-09-10 New-PC guidance
- Signal: generic needs-attention state did not tell users what to install or how to recover.
- Prevention: provider-specific missing-CLI/auth messages must name the official tool, login action, and refresh action.

## 2026-09-10 Tablet HTTP regression
- Signal: immediate writes with shutdown hid Windows accepted-socket nonblocking inheritance; physical USB requests failed before parsing.
- Prevention: test delayed, fragmented requests without write shutdown and the actual forwarded Host/Origin. Never infer hardware or browser incompatibility from a generic parse error. Wait for packaging completion before handing off a build.

## 2026-09-10 Updater lifecycle ordering
- Signal: the first updater path stopped irreversible collector state before package extraction could fail, leaving an apparently running but unusable app.
- Prevention: attach shutdown to the updater's confirmed pre-exit hook; all download, verification and extraction errors must leave monitoring alive and the update retryable.
- Tripwire: `tests/updater_contract.test.cjs` rejects pre-install lifecycle shutdown and requires update retention on both failure paths.

## 2026-09-10 Data-shape rendering parity
- Signal: the tablet status recognized `source_usage`, but the card body only rendered `quota_windows`, hiding valid Copilot data.
- Prevention: every “has data” predicate must have a matching render branch and a fixture/test for each accepted observation shape.

## 2026-09-10 Packaged plugin configuration
- Signal: unit tests and compilation passed, but the installed executable failed before window creation because an optional-looking Tauri plugin config was deserialized from `null`.
- Prevention: every newly registered Tauri plugin must have a packaged-startup config contract and a real release executable launch probe before installer handoff.
- Tripwire: `tests/updater_boot_config.test.cjs` plus the release `--hidden`/`--request-exit` smoke test.

## 2026-09-11 Security copy and visual containment
- Signal: the first full verifier caught removal of the pairing-code screenshot warning, while initial geometry checks only proved card borders—not their contents—fit the viewport.
- Prevention: preserve security copy as a tested product contract; viewport tests must assert primary data and actions remain inside every visible card and dialog.
- Tripwire: `cargo test --test desktop_bundle_config` and `node .scratch/verify-adaptive-ui.cjs`.

## 2026-09-11 Dashboard ownership and count-driven layout
- Classification: misunderstood product flow and incomplete viewport verification.
- Signal: the reserved add tile reduced monitoring space, and short settings/three-card states overflowed even though the card grid itself fit.
- Prevention: keep monitor membership/order controls in settings; derive dashboard topology from the visible card count, not viewport heuristics; catalog-only providers must never become data cards without collector support.
- Tripwire: test the exact 1/2/3/4 matrix, zero add tiles, partial-page geometry, and content containment at short landscape sizes with `node .scratch/verify-adaptive-ui.cjs`.

## 2026-09-11 Pseudo-element paint containment
- Classification: missing visual verification at a high-DPI-equivalent viewport.
- Signal: a primary quota ring looked cut flat at the top on a 2491 × 1312 physical-pixel display, while the old test only proved the `.window` stayed inside the card.
- Prevention: for decorative data graphics rendered by pseudo-elements, verify the computed paint rectangle against the nearest clipping ancestor; include physical-size and DPI-scaled CSS viewport pairs.
- Tripwire: `node .scratch/verify-adaptive-ui.cjs` must include 2491 × 1312 and 1993 × 1050 and reject any `.window` or wide four-card ring/glow outside `.quota`.

## 2026-09-11 Gauge typography must follow the inner diameter
- Classification: responsive visual proportion regression.
- Signal: after the ring clipping fix, a 64px percentage remained centered but extended across the ring stroke because its breakpoint was independent of the ring's 69% inner opening.
- Prevention: size center labels from the gauge's usable inner diameter, measure only visible text, and include the widest valid value (`100%`) rather than testing only current fixture values.
- Tripwire: the adaptive browser verifier requires visible primary text width to stay within 90% of the wide four-card ring's inner diameter at physical and high-DPI-equivalent viewports.

## 2026-09-11 Real data volume and readable overflow
- Classification: missing regression coverage; prior geometry tests used only two quota windows and assumed every card must fit without scrolling.
- Signal: user screenshots with eight Codex windows, four Antigravity windows and 24.7% Copilot showed centered overflowing content cropped above and below the quota region.
- Prevention: test realistic data cardinality, fractional values, long reset text, empty/error states, and keyboard scrolling. Fit the grid when possible; preserve a minimum readable card height and explicit scroll access instead of hiding quota rows or actions. Unchanged polling must preserve DOM nodes and reading position.
- Tripwire: `scripts/verify-desktop-responsive.cjs` (Playwright via NODE_PATH), 11 viewport pairs and 1/2/3/4 cards; supersedes the old desktop no-scroll assertion in `.scratch/verify-adaptive-ui.cjs`.

## 2026-09-11 Console silence requires a positive control
- Classification: verification gap.
- Signal: a process monitor on an isolated desktop can report zero visible windows even for deliberately visible console creation.
- Prevention: verify the observer can see the interactive desktop and detects a positive-control console before interpreting zero observations. Do not claim a console-flash fix from process flags or static tests alone.

## 2026-09-11 "Reachable by scrolling" is not "readable"
- Classification: misunderstood requirement.
- Signal: the user rejected nested scroll regions after the real-data fix; at 1498 × 908 only one of eight Codex windows was visible per card.
- Prevention: desktop monitoring views must fit the viewport. Measure the region, compute tile count/size and derive typography from it; when content cannot be readable, reduce cards per page (pager) instead of adding scrollbars.
- Tripwire: `scripts/verify-desktop-responsive.cjs` rejects any scrollable or overflowing `.app-shell`, `.cards`, card or quota region.

## 2026-09-11 Hidden grid rows shift implicit placement
- Classification: incorrect assumption about CSS behaviour.
- Signal: after a viewport change the card grid collapsed to 106px because `display:none` on `.dashboard-head` moved `.cards` into the second `auto` row.
- Prevention: give app-shell children explicit `grid-row` whenever a row can be hidden; verify after resizing from a different size, not only on fresh load.

## 2026-09-11 Third-party CLIs can open consoles we never spawn
- Classification: verification gap / integration boundary.
- Signal: process flags and windowsHide were correct everywhere, yet a flash was tied to AgY/Terminal and was intermittent (agy checks for updates at most every 15 minutes).
- Prevention: for every bundled/official CLI, disable self-update and background daemons via their documented env/flags; inspect binaries (`strings`) for updater/daemon code paths. Reproduction attempts must span the tool's update TTL.
- Tripwire: `tests/background_window_contract.test.cjs` requires `AGY_CLI_DISABLE_AUTO_UPDATE`, `DISABLE_AUTOUPDATER` and `--no-auto-update`.

## 2026-09-12 Provider-wide analytics scope
- Classification: misunderstanding requirements / provider-specific scope.
- Signal: the first token-analysis slice hard-coded Codex even though the war room's stable domain boundary is the four-provider `PROVIDERS` set.
- Prevention: define analytics features against the Provider capability matrix first; provider adapters may report truthful unavailable/unsupported states, but the product entry point and contract must include every supported Provider.
- Tripwire: backend and UI tests require `codex`, `claude`, `copilot`, and `antigravity`, and reject any fallback that presents quota percentages as session/turn Token counts.

## 2026-09-12 Unknown counters and contradictory aggregates must fail closed
- Classification: incorrect assumption about provider data / missing negative verification.
- Signal: final diff review showed that `cached_input_tokens: null` became zero and that priced turn totals above observed totals could still be labelled 100% complete.
- Prevention: preserve `None` through provider-aware normalization; only exact observed/priced equality may be `complete`, and contradictory aggregates must hide cost and coverage instead of clamping or saturating into a plausible value.
- Tripwire: `tests/token_analysis.rs` covers missing cache counters and `priced_tokens > observed_tokens`; both must return unavailable without low-cache signals or dollar values.

## 2026-09-13 HUD native movement and polling ownership
- Classification: concurrency / missing verification.
- Signal: review found periodic work-area clamping could interrupt cross-monitor dragging, and queued move coordinates could be saved against a newer current monitor.
- Prevention: serialize native placement operations, defer geometry updates during native dragging, and read the current position after debouncing; ignore startup movement before saved placement is restored.
- Tripwire: HUD geometry tests plus native drag across monitor boundaries while the two-second polling loop is active.
- Additional review signal: awaiting position persistence before snapshot meant a locked position file suppressed live data. Placement/configuration errors now have separate handling and a browser regression proves snapshot still runs.

## 2026-09-13 Release metadata and API identifiers
- Signal: a global application version replacement modified unrelated Cargo.lock entries; isolated verification rejected unavailable dependency versions. GitHub release creation also rejected a short commit SHA.
- Prevention: edit only the application package stanza and inspect the lockfile diff; use the full `git rev-parse HEAD` value for release target_commitish. Preserve all dependency versions/checksums and never include signing material or build artifacts in commits.

## 2026-09-14 Packaging command working directory
- Signal: read-only resource checks used repository-relative paths with desktop-p0 as workdir and failed to find files.
- Prevention: run repository-relative checks from the repository root; keep desktop build commands separate from resource checks.
- Tripwire: verify workdir before batching relative file paths.

## 2026-11-16 Compiled binary is not the running installation
- Classification: missing verification / incorrect deployment assumption.
- Signal: the release build succeeded but the visible single-instance process still ran the older installed executable, so the new control was absent.
- Prevention: compare the running executable path and start time against the build output, and verify the newly launched UI before claiming the feature is visible; explain that compiling does not update the installed copy.
- Tripwire: `Get-Process agentmeter-desktop-p0 | Select-Object Path,StartTime` and inspect the application's visible resource control after a deliberate restart.

## 2026-11-16 Monitoring data should remain visible together
- Classification: misunderstanding requirements / missing user workflow verification.
- Signal: CPU, RAM and disk were put in a separate modal, preventing simultaneous viewing with provider cards; the user requested an on-screen setting.
- Prevention: for dashboard monitoring requests, confirm whether readings must be visible together and default to an in-dashboard display with an explicit visibility preference when appropriate.
- Tripwire: resources UI test checks that the panel is visible by default, shows all three readings, and stops polling when disabled.

## 2026-09-13 Release metadata scope
- Signal: release verifier detected unrelated lockfile packages bumped by a global version replacement.
- Prevention: change only the application package stanza; inspect the lockfile diff before running verification or packaging. Preserve every dependency version/checksum.
- Tripwire: `git diff -- desktop-p0/Cargo.lock` must contain only app version and explicitly required direct dependency entries.

## 2026-11-21 Stale instance blocks launch; PowerShell EnumWindows callbacks swallow output
- Classification: environment-dependent / missing verification.
- Signal: Task Manager showed `AgentMeter P0` running (7 threads, no msedgewebview2 child) with no window or tray icon; a later launch wrote `startup-error.log` with "IPC window is unavailable". Killing the stale PID and relaunching restored main window, HUD, tray and IPC window (36 threads, WebView2 child).
- Pitfall: a PowerShell scriptblock passed as an `EnumWindows` delegate does not emit its output to the pipeline, so it falsely reports "no windows". Enumerate inside an Add-Type C# helper that returns a list.
- Tripwire: for "running but invisible" reports, check `startup-error.log`, thread count, and `Win32_Process` children with ParentProcessId = app PID before touching code.

## 2026-11-21 Local desktop build prerequisites on this machine
- Signal: `cargo` missing, then `link.exe` missing (VS installs lacked the C++ workload), then build.rs failed on missing `desktop-p0/resources/quota-helper`, then one transient "存取被拒 (os error 5)" on a fresh build script.
- Prevention: rustup lives in scoop (`RUSTUP_HOME`/`CARGO_HOME` under `~/scoop/persist/rustup`; set them explicitly in shells started before install). Stage quota-helper via `scripts/prepare-desktop-quota.ps1` or copy the installed runtime; a non-bundled release exe also needs `target/release/quota-helper`. Retry once on os error 5 (antivirus scanning new executables) before diagnosing.

## 2026-11-23 Config-created webviews can call commands before setup manages state
- Classification: incorrect assumption about framework behaviour / concurrency-timing.
- Signal: installed 0.2.8 exited silently on every launch (WER 0xc0000409, no startup-error.log). Launching with `-RedirectStandardError` revealed `state() called before manage() for AutoQuota`: Tauri builds config windows before the setup hook, and creating the second WebView2 window pumps messages, so the main page's synchronous `auto_quota_settings` ran against unmanaged state; the panic in a WebView2 callback cannot unwind and aborts.
- Prevention: windows use `"create": false` and are built with `WebviewWindowBuilder::from_config` at the end of setup, after every `app.manage`. For silent desktop exits, read the Application event log and capture stderr of the release exe before guessing.
- Tripwire: `tests/desktop_bundle_config.rs::webviews_are_created_only_after_setup_registers_command_state`.
