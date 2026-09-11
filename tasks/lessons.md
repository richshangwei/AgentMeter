# Lessons

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
