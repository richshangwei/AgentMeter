# 2026-09-12 發布 0.2.5 HUD 修正版

## Goal & acceptance criteria
- [x] 僅發布已驗證的 Codex HUD 額度語意、緊湊尺寸與透明 hover 修正。
- [ ] 版本、安裝檔、簽章、manifest 與 Git tag 一致為 0.2.5。
- [ ] GitHub Release 僅包含安全命名的安裝檔、`.sig` 與 `latest.json`，並可由 0.2.4 自動更新發現。

## Plan
- [x] 由 `origin/master` 建立隔離發布工作樹，只帶入 HUD 修正與測試。
- [x] 執行完整本機驗證與簽署建置。
- [ ] 審核 diff，以詳細繁中 commit 提交並推送。
- [ ] 建立 v0.2.5 Release，驗證下載、SHA256、簽章與 Latest manifest。

## Risk & rollback
- Risk: high；這會發布可執行安裝檔及不可變更新資產。
- Rollback: 發布前停止；發布後將 v0.2.5 改為非 Latest，並以更高修正版恢復，不覆寫既有資產。
- Exclusions: 不包含原工作區的共享規格文件、任務紀錄或任何 `.scratch`／`Claude outputs` 圖片。

## Working notes
- 隔離工作樹需先以固定 SHA-512 的 Antigravity CLI 執行 `scripts/prepare-desktop-quota.ps1`，重建被 Git 忽略的測試依賴與打包 runtime。
- 完整 `scripts/verify-local.ps1`、Edge HUD 幾何/hover 驗證及 Tauri 簽章/可信註解/單位元竄改拒絕皆通過。
- 本機草稿：`desktop-p0/target/update-drafts/0.2.5-cb9e3e0b6d954a559c333da915025924`；安裝檔 73,594,573 bytes；SHA256 `5301BD1CF2BDF3E6F155B484A43B06FE96399A74ED3C393EE704FDB77CA27735`。Windows Authenticode 仍為 `NotSigned`。

# 2026-09-12 發布 0.2.4 自動更新

## Goal & acceptance criteria
- [x] 將同步頻率設定與右下角 HUD 納入 0.2.4，排除預覽圖片與所有私鑰材料。
- [x] 完整測試、簽署建置、NSIS 檢驗、manifest URL 與簽章驗證全部通過。
- [ ] 以詳細繁中 commit 提交並推送 master，建立不可變的 v0.2.4 tag。
- [ ] GitHub Release 只包含安全檔名安裝檔、簽章與 latest.json，匿名下載皆為 HTTP 200。
- [ ] Latest manifest 回報 0.2.4，且已發布 0.2.3 可發現並驗證此更新。

## Plan
- [x] Checkpoint A: 審核工作樹、既有發布規則、遠端與金鑰邊界。
- [x] Checkpoint B: bump 0.2.4，執行完整與瀏覽器驗證。
- [x] Checkpoint B: 使用既有 CurrentUser DPAPI 私鑰完成簽署建置與草稿驗證。
- [ ] Checkpoint C: 精確 stage 原始碼，排除圖片/金鑰，提交、推送與建立 tag。
- [ ] Checkpoint C: 發布三個資產，驗證 Latest、匿名下載、SHA256 與簽章。
- [ ] Checkpoint D: 記錄發布證據、風險、回復與未完成的真實安裝驗收。

## Risk & rollback
- Risk: high；公開可執行檔與不可變更新簽章。發布前可停止；發布後不能替換 v0.2.4 資產，只能撤下 Latest 或發布更高修正版。
- Rollback: 發布前刪除草稿；發布後把 v0.2.4 改為非 Latest／草稿以停止新下載，已安裝者需用更高版本修復。
- Security: 不讀出、不記錄、不提交 `updater.key.dpapi` 或任何私鑰內容；Release 僅允許 installer、`.sig`、`latest.json`。

## Dependencies & environment
- Windows、既有 Tauri/Rust/Node/NSIS 工具鏈、GitHub CLI 與目前 Windows 使用者可解密的 DPAPI 金鑰。
- 網路寫入僅限 `richshangwei/AgentMeter` 的 master、v0.2.4 tag 與對應 GitHub Release。

## Working notes
- Draft: `desktop-p0/target/update-drafts/0.2.4-583fa835c5b045a1ae58c188ff73c6da`。
- Installer: 73,603,937 bytes；SHA-256 `3AF973A5F308AF873DB4DBAD9AB6791B3CF0613A701A4188AC8B3DAA8FA6E841`；內嵌 product/file version 0.2.4。
- Tauri installer signature、trusted comment 及單位元竄改拒絕通過；Windows Authenticode 為 NotSigned。
- `scripts/verify-local.ps1`、18 組 Edge 主畫面矩陣、HUD/設定幾何與透明度 35/48/72/95 全部通過。

# 2026-09-12 可設定額度同步頻率

## Goal & acceptance criteria
- [x] 後台設定可選擇額度自動更新頻率，重啟後仍保留。
- [x] 背景排程即時採用新頻率，關閉視窗至通知區後仍生效。
- [x] 頁首「同步時間」顯示實際資料同步時間，不隨 2 秒畫面輪詢虛假更新。
- [x] 手動更新、自動更新開關、平板監看與 Provider 收集行為維持相容。

## Plan
- [x] Checkpoint A: 定位固定排程、設定儲存、桌面設定 UI 與現有測試。
- [x] Checkpoint B: 先加入頻率驗證、持久化、排程與 UI 的回歸測試。
- [x] Checkpoint B: 實作最小後端設定與桌面設定介面。
- [x] Checkpoint C: 執行 targeted tests、Rust format/Clippy/build 與桌面 UI 驗證。
- [x] Checkpoint D: 記錄結果、風險與回復方式。

## Risk & rollback
- Risk: medium；影響背景額度收集排程與本機偏好設定，不改 Provider、帳號、配對或資料格式。
- Rollback: 還原本任務的 auto_quota、main、desktop UI 與測試變更；刪除獨立的 `quota-sync.json` 即恢復預設值，不影響來源設定。
- Signals: 頻率 allowlist、連續儲存/重載、排程讀取原子值、同步時間不因 snapshot polling 改變。

## Dependencies & environment
- Windows / Tauri 2 / Rust / vanilla JS；不新增依賴。
- 預設依 `AgentMeter-Requirements-v1.1.md` 統一為 120 秒，允許的頻率以固定選項限制，避免過度呼叫官方工具。

## Working notes
- `setInterval(pollQuota, 2000)` 只負責 UI snapshot，不是 Provider 收集頻率。
- 真正收集排程位於 `desktop-p0/src/main.rs`；原本固定 60 秒，現由後端原子狀態讀取已驗證的設定值。

## Results
- 設定視窗新增 2、5、10、15、30、60 分鐘選項；後端嚴格 allowlist，預設 120 秒，並將開關與頻率保存到獨立 `quota-sync.json`。
- 排程以最近一次收集完成時間計算下一次執行；變更頻率或重新啟用後會在 250ms 檢查週期內採用新設定，collector mutex 繼續防止重疊。
- 頁首同步時間改取 Provider 的 `checked_at`／`collected_at`，2 秒 snapshot polling 不再製造假同步時間。
- Verification: `scripts/verify-local.ps1` 全數通過；9 個 quota UI 測試、27 個 desktop Rust 測試、root/desktop fmt、Clippy 與 locked offline build 通過；`git diff --check` 通過。
- Visual caveat: 獨立 Playwright responsive runner 在目前 shell 缺少 `playwright` 套件，瀏覽器亦依安全政策禁止 `file://` 本機頁面；已完成 HTML/CSS contract、語法與完整專案 verifier，未宣稱額外的實際瀏覽器截圖驗證。

# 2026-09-12 右下角精簡數據模式

## Goal & acceptance criteria
- [x] 設定可開啟或關閉獨立右下角 HUD，重啟後保留。
- [x] HUD 只顯示已選監控的 Provider 名稱與主要數據，無按鈕、圖示或多餘狀態。
- [x] HUD 固定於可用桌面右下角、置頂、不取得焦點且可穿透點擊。
- [x] 設定可調整面板背景透明度，文字保持清晰可讀。
- [x] 資料未取得時顯示 `—`，不把未知假裝成 0。

## Plan
- [x] Checkpoint A: 確認 Tauri 視窗、snapshot 資料形狀、顯示選擇儲存與設定 UI。
- [x] Checkpoint B: 先新增 HUD 投影、透明度邊界與視窗合約測試。
- [x] Checkpoint B: 實作獨立 HUD 視窗、設定、持久化、定位與即時資料。
- [x] Checkpoint C: 執行 targeted tests、Rust format/Clippy/build、主畫面 RWD 與 HUD 實際幾何/可讀性驗證。
- [x] Checkpoint D: 記錄結果、風險與回復方式。

## Risk & rollback
- Risk: medium；新增第二個置頂視窗與本機 UI 偏好，不變更 Provider 收集、帳號、配對或資料格式。
- Rollback: 移除 HUD 視窗設定、後端視窗命令與 HUD 靜態檔；舊版會安全忽略 localStorage 偏好。
- Signals: 視窗必須 transparent/decorations=false/alwaysOnTop/skipTaskbar/visible=false；高度隨 0–4 列可控且不越出可用畫面。

## Dependencies & environment
- Windows / Tauri 2 / vanilla HTML/CSS/JS；不新增套件。
- HUD 與主視窗使用相同本機 origin 偏好與 dashboard snapshot。

## Results
- 新增獨立透明 HUD 視窗；預設關閉，開啟後固定於目前可用桌面右下角，保持置頂、略過工作列、不搶焦點且滑鼠可穿透。
- 設定新增開關與 35–95% 面板透明度；只改玻璃背景 alpha，14px 名稱與 21px tabular 數值保持不透明。
- HUD 依主畫面監控順序顯示最多四個 Provider 的主要額度；有 quota 顯示剩餘百分比，僅有 source usage 時顯示用量／上限，未知顯示 `—`。
- Verification: HUD model 5/5、desktop layout 7/7、desktop bundle 10/10；Edge 實際 HUD 360×204 與設定 640×520 驗證無重疊、溢位或控制項污染，透明度 35/48/72/95 均正確；18 個主畫面 RWD viewport 全數通過。
- `scripts/verify-local.ps1`、locked offline Rust build/Clippy、JS syntax 與 `git diff --check` 通過。未重新打包安裝程式；真實桌面透明／置頂行為需下一次打包後在 Windows 安裝版驗收。

# 2026-09-10 Continue Claude desktop handoff

## 2026-09-11 Publish 0.2.1 automatic update
- [x] Audit the complete pending product diff and exclude screenshots, generated previews and signing keys from Git; keep the reusable ciphertext-only signing helper without embedded key material.
- [x] Set desktop version 0.2.1, run full verification and build with the existing trusted updater key.
- [x] Verify installer identity, signature, tamper rejection and `latest.json` URLs.
- [ ] Commit in detailed Traditional Chinese, push `master`, create and push signed release tag `v0.2.1`.
- [ ] Publish GitHub Release with only installer, updater signature and `latest.json`; verify latest endpoint and asset downloads.
- [ ] Confirm an installed 0.2.0 can discover 0.2.1 without triggering an automatic install.
- Acceptance: release assets are public and immutable at exact expected URLs; 0.2.0 reports 0.2.1 available and verifies it before install; no secret or screenshot is committed/uploaded.
- Risk: high, public executable release and permanent update trust. Rollback before install by removing Latest exposure; after install publish a higher fixed version, never replace an existing tag asset.
- Build evidence: draft `desktop-p0/target/update-drafts/0.2.1-9e235b558b5f4e828e11b6122ba13747`; installer 73,590,196 bytes; SHA256 `F03CADE3C22603B118800F6195C36D3187FADD5FC1E2FD2FF24D3FDEBDC149F3`; embedded version 0.2.1. Tauri signature and trusted comment verified; in-memory single-byte tamper rejected. Windows Authenticode remains NotSigned.
- Verification: full verify-local.ps1 passed serially; updater/manifest/lifecycle 17 tests passed; settings updater UI passed 1080x640, 640x520 and 375x844; desktop dashboard passed all 18 responsive matrices.
- Release discovery: GitHub normalized spaces in uploaded asset names to dots; the 0.2.0-required `%20` URL returned HTTP 404 while the dotted URL returned 200. v0.2.1 was exposed only as a non-Latest prerelease for the HTTP probe and immediately returned to draft. Existing 0.2.0 cannot consume a differently named asset.

## 2026-09-11 GitHub-safe bootstrap 0.2.2 and update 0.2.3
- [x] Capture the public release failure: `%20` asset URL is 404 while GitHub-normalized dotted asset is 200; keep v0.2.1 as a draft.
- [x] Require the GitHub-safe `AgentMeter-P0_<version>_x64-setup.exe` name in manifest and backend, rejecting the legacy space form.
- [x] Build, verify, commit, tag and publish 0.2.2 as the one-time manual bootstrap; anonymous asset URL and Latest manifest both return HTTP 200.
- [x] Bump, build, verify, commit, tag and publish 0.2.3 as Latest.
- [x] Verify unauthenticated latest manifest/asset HTTP status, downloaded SHA256/signature and 0.2.2-to-0.2.3 discovery contract.
- [ ] Installed end-to-end confirmation requires the user to manually install 0.2.2 once, then use Settings → Check update to confirm 0.2.3 download and installation; no unattended install was authorized.
- Acceptance: GitHub does not rewrite either 0.2.2/0.2.3 release asset name; 0.2.3 latest.json contains the exact public 200 URL and valid signature; user receives a clear one-time 0.2.2 install handoff.
- Risk: existing 0.2.0 is permanently unable to auto-update because its immutable allowlist requires a GitHub asset name that cannot exist. Recovery is a single manual 0.2.2 install; all later versions use the safe name.
- 0.2.2 build evidence: draft `desktop-p0/target/update-drafts/0.2.2-ef13221be66544ad8e62934b585c018d`; installer 73,592,379 bytes; SHA256 `0D5376ADBBC56814D2A6658C46DF9C0E78628638EEFE57B3D57BB1BB3FE43607`; embedded version 0.2.2. Signature/trusted comment pass and modified-byte verification fails as required. Full serial verifier passes.
- 0.2.3 build evidence: draft `desktop-p0/target/update-drafts/0.2.3-ccd68629e1d64525a75c756ae5f3f20c`; installer 73,601,470 bytes; SHA256 `D1D9B1535CAAD006F379E03E97A99C264802E0C455C60BCC7113CEFE1734BFF6`; embedded version 0.2.3. Signature/trusted comment pass and modified-byte verification fails as required. Full serial verifier passes.
- Published evidence: repository is PUBLIC; v0.2.2 and v0.2.3 exact safe-name assets both return anonymous HTTP 200; `releases/latest/download/latest.json` reports 0.2.3 and its exact v0.2.3 asset URL. The complete remote installer was downloaded again, matched SHA256 above, passed public-key signature/trusted-comment verification and rejected an in-memory one-byte mutation. v0.2.1 remains a non-public draft as evidence of the failed legacy filename probe.

## 2026-09-11 Local signing bootstrap
- [x] Generate private key only in memory and store only Windows CurrentUser DPAPI ciphertext outside Git; never log secret material.
- [x] Build signed 0.2.0 local draft using existing release tooling and verify artifact/signature metadata.
- [x] Record artifact hashes and trust/backup limitations; no release publication, installation, or running-app replacement.
- Risk: signing trust root; refuse to overwrite existing keys. No plaintext private-key file is created. DPAPI recovery requires this Windows account; portable encrypted backup remains a release gate. ACL modification was denied by the execution environment; switched to in-memory generation and ciphertext-only storage rather than relying on filesystem ACL confidentiality.
- Results: draft `desktop-p0/target/update-drafts/0.2.0-89e12ca03ffb4fd6aecce3c867cd0e81`, installer 73,567,979 bytes, SHA256 `8FD0A8220855C9137F9FB429037648ADA3EF5F113AF0AFDA8B5F7C6E1561CA46`. NSIS embedded version 0.2.0. Tauri updater signature verified with public key; trusted comment verified; single-byte tampering rejected in memory. Authenticode remains NotSigned.
- Verification: full verify-local.ps1 with RUST_TEST_THREADS=1 passed (tests, formatting, Clippy); signature verifier passed; no installer or application launched. Updated bundle test to compare actual desktop Cargo package version rather than separately versioned core library.

## 2026-09-11 Signed application automatic updates
- [x] Inspect existing updater, release configuration, lifecycle boundaries and tests.
- [x] Implement serialized check/download/verified-ready/install states with progress and retry safety.
- [x] Implement persistent automatic-check and optional automatic-download preferences, recurring checks and clear Traditional Chinese settings feedback.
- [x] Add local signed-release/manifest tooling and publishing instructions; never publish or upload a private key implicitly.
- [x] Add state, concurrency, frontend and manifest regression tests; run full local checks and browser/settings verification.
- [x] Record outcomes and remaining signing/release enablement requirements.
- Acceptance: checking and downloading never stop quota collection; installer launches only after explicit confirmation and verified download; unsigned/unconfigured builds cannot install arbitrary packages; retries cannot race installs; startup and recurring checks avoid duplicate work.
- Risk: high (trusted executable updates). Existing updater signature verification remains mandatory. Rollback: revert focused updater/UI/tooling changes; do not alter signing trust or release artifacts. No account data/schema changes.
- Environment: Tauri 2 / updater 2.10.1, Windows x64 NSIS, offline Cargo lockfiles, GitHub origin richshangwei/AgentMeter. Signing-key creation approval is pending; release publishing is not authorized.
- Results: updater controller has 9 deterministic tests (including late progress and busy timer races), manifest has 6 tests, lifecycle contracts have 2 tests, Rust updater has 5 tests. Edge updater interaction checks pass at 1080x640, 640x520 and 375x844; existing dashboard passes all 18 viewport matrices.
- Verification caveat: initial parallel full run failed two existing tablet_http tests (pairing 401 and revision not yet advanced after a fixed 50ms sleep). Isolated serial tablet_http run passes all 22; full scripts/verify-local.ps1 with RUST_TEST_THREADS=1 passes, including formatting, Rust/Node tests and both all-target Clippy checks. Do not claim the default concurrent suite is reliable; follow up on timing-sensitive tests separately.
- Activation gate: no signing key generated, no release uploaded, and no installer executed. Existing unconfigured previews require a manual enabled build once. Real signed update/invalid-signature/installer lifecycle acceptance in isolated Windows remains required before release. SDK cannot recover after Windows installer handoff; only pre-shutdown preparation failures are retryable.

## 2026-09-11 Repackage current fit-to-viewport build on Windows
- [x] Verify current source/staged Antigravity updater-disable flag matches.
- [x] Run Windows full local verifier (exit 0), targeted UI/layout tests (13 passed), and Edge responsive verifier (18 viewport matrices passed).
- [x] Build locked offline x64 NSIS and inspect final installer/version/hash: version 0.1.0, unsigned, 73,576,709 bytes; SHA256 `12077DBE3CF8307736A242A5E51681249A6746FF9A5025B4B409FA2D8D1D88B1`; inspection exit 0.
- Scope: packaging only; no installation or configuration changes. The long-interval interactive console probe is not rerun for this packaging request.

## 2026-09-11 Fit-to-viewport desktop cards and silent Antigravity refresh
- [x] User report: resizing produced cramped/cropped content with scrollbars; detection still flashed a console window.
- [x] Replace scrollable `.cards`/`.quota` regions with a measured fit: `quotaTileLayout` (layout.js) places every quota window as a tile and derives ring/text sizes from the tile rectangle; variants ring → stack → bar → line.
- [x] `desktopPageCapacity` keeps the 1×1 / 2×1 / 1×3 / 2×2 topology and only drops to 2 or 1 cards per page when a card (or the densest card's tiles, min 90×56) would be unreadable; the pager reaches the rest.
- [x] Wide short cards lay out heading | quota | actions horizontally; informational notes move to the update-time tooltip (failures always visible, 2–3 line clamp + tooltip + guide).
- [x] Root cause for the console flash: bundled `agy.exe` (Go, console subsystem) contains `jetski/cli/updater.RunBackgroundUpdate` / `prepareBgCommand` and honours `AGY_CLI_DISABLE_AUTO_UPDATE`. Its background updater starts outside our hidden console, so its helpers open a visible console/Windows Terminal. The collector now sets `AGY_CLI_DISABLE_AUTO_UPDATE=1` (both quota-smoke.mjs copies); Claude (`DISABLE_AUTOUPDATER`) and Copilot (`--no-auto-update`) were already covered.
- [x] Verification (Linux Chromium, synthetic data): `scripts/verify-desktop-responsive.cjs` passes 18 viewports × 1/2/3/4 cards with 8/2/1/4 windows plus edge states — no scrollable/overflowing container, every tile/child inside its box, no sibling overlap, percentages inside ring openings, no value truncation, ≥10px text, pager reaches every monitor. Node tests: desktop_layout, auto-quota-ui, background_window_contract (new agy test), quota_smoke pass.
- [ ] On Windows: run `scripts/verify-local.ps1`, `AGENTMETER_BROWSER_CHANNEL=msedge node scripts/verify-desktop-responsive.cjs`, rebuild NSIS, then `scripts/test-refresh-console.ps1 -LiveProvider antigravity` a few times >15 minutes apart (agy only checks for updates every 15 min). Not run from this session: the device shell was unavailable.
- Risk: medium (desktop presentation + one collector env var). Rollback: revert desktop-p0/ui/{index.html,style.css,style-overrides.css,layout.js,dashboard.js}, scripts/verify-desktop-responsive.cjs and the agy `env` line.

## 2026-09-11 Real-data responsive layout and silent refresh
- [x] Reproduce clipping with 8 quota windows, fractional percentages, long labels, and scaled viewports.
- [x] Fix layout containment without changing provider data semantics.
- [x] Verify all quota rows/actions are reachable, rings are complete, and refresh preserves scroll.
- [x] Run local checks and record evidence/limitations: full local verifier passed; 10 targeted UI/layout tests passed; 11 responsive viewport matrices passed.
- [ ] Identify the user's CMD flash: positive-control monitoring works, but four individual live collectors, installed helper path and passive GUI observation have not reproduced the flash. No speculative production process changes made.
- Console evidence: deliberate visible-grandchild control fails as intended; default credential-free fixture passes; four individual live providers pass; installed `--quota-collect` repeated three times passes. Geometry/ancestry-aware 100-second GUI observation confirmed three collector processes with no attributable console event. An earlier AgY/Terminal event had already exited before ancestry capture and cannot establish causation. Remaining question: which single provider refresh (or only all-provider refresh) reproduces the user's flash?
- [x] Package/inspect the RWD correction; explicitly retain the unresolved console limitation. NSIS 0.1.0 unsigned, 73,575,683 bytes; SHA256 `C478B006E03C5093D70EFBF04F345C993E4407A8D21C76A41ED11F92CC3263A5`.
- Acceptance: preserve count-driven monitor topology; never silently hide quota rows/actions; reserve readable chart space and provide deliberate scrolling when full-screen content cannot fit.
- Risk: medium, desktop presentation and Windows child processes. Roll back focused CSS/render and process-launch changes only; preserve existing working-tree changes. No credentials or provider data changes.
- Environment: Windows, local Edge/Playwright runtime, Rust offline lockfiles; synthetic provider fixtures for UI and process probes.
- Final verification: full `scripts/verify-local.ps1` exit 0, `scripts/verify-desktop-responsive.cjs` all 11 viewport matrices passed, NSIS inspection exit 0, focused `git diff --check` exit 0. No installation or production process-launch change performed.

## 2026-09-11 Repackage latest UI
- [x] Run full local verifier.
- [x] Build locked offline x64 NSIS installer from current UI.
- [x] Inspect installer and report artifact/hash.
- Risk: build artifacts only; no installation or user configuration changes.
- Results: full local verifier and NSIS inspection passed; version 0.1.0, 73,576,036 bytes, unsigned preview. Clean-VM installation not tested.
- Artifact: `desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- SHA256: `54C814B13C4C58957E5B5934F8612599A31781A06BAA4362C70B5F611F2A5CC5`

## 2026-09-11 Separate period labels from gauge values
- [x] Inspect the complete gauge/period/progress/reset relationship.
- [x] Replace width-dependent overlay positioning with separate grid cells: percentage left, period/progress/reset right.
- [x] Add label/value intersection checks to all desktop count/viewport runs.
- [x] Verify browser captures and targeted desktop tests (9 passed).
- Risk: low, desktop CSS layout only; rollback the focused period placement rules. No data changes.

## 2026-09-11 Primary percentage fits the gauge center

### Goal & acceptance criteria
- [x] Reproduce the supplied 2440 × 1288 physical-pixel state and its 125%-scale CSS equivalent with a deterministic text-to-ring geometry assertion.
- [x] Keep the primary percentage fully inside the circular gauge's clear center in the reported normal-density four-card desktop layout.
- [x] Preserve ring size, four-card composition, labels, progress bars, and compact fallbacks.
- [x] Pass red/green browser verification, targeted/full checks, updated screenshot comparison, and design QA.

### Plan
- [x] Checkpoint A: measure the real percentage and computed ring geometry; confirm the new assertion fails before editing CSS.
- [x] Checkpoint B: adjust only responsive percentage typography and verify all count/viewport states.
- [x] Checkpoint C: capture and compare the corrected state at the reported aspect ratio.
- [x] Checkpoint D: run full verification and record the result/lesson.

### Risk & rollback
- Risk: low. Affected component: desktop primary quota numeral typography only.
- Rollback: revert the focused font-size rules and geometry assertion; no data or settings change.
- Signal: primary percentage width must remain within the ring's transparent center with a small readability margin.

### Results
- Red-capable browser repro measured the visible `82%` text at 123.61px against a 168px ring: 73.6% of the outer diameter and wider than the 69% transparent center.
- Root cause: the wide breakpoint raised the numeral to 64px independently of the ring's usable inner diameter; positioning remained centered and was not the fault.
- Responsive caps now use 28px for the 108px medium-height ring, up to 44px for the 168px tall ring, and smaller compact values. Ring dimensions and the four-card topology are unchanged.
- The browser regression uses a `Range` over the visible text node, excludes screen-reader-only copy, and requires `text width <= 90% of inner diameter`; a dedicated 100% stress snapshot passes.
- Adaptive verification passed 9 desktop and 7 tablet/phone sizes, including 2440 × 1288 and 1952 × 1030. Targeted tests passed 16/16; full local verifier, locked offline build, browser interaction/console check, and `git diff --check` passed.

## 2026-09-11 Ultra-wide quota ring clipping

### Goal & acceptance criteria
- [x] Reproduce the supplied 2491 × 1312 screenshot state with a deterministic browser assertion that catches a quota ring clipped by its quota viewport.
- [x] Keep every primary quota ring fully visible at ultra-wide/high-DPI-equivalent desktop sizes without shrinking normal desktop or phone layouts unnecessarily.
- [x] Preserve the exact 1/2/3/4 card composition, actions, secondary quota windows, and unknown states.
- [x] Pass the targeted red/green repro, adaptive browser matrix, full local verification, and updated design QA.

### Plan
- [x] Checkpoint A: add the exact screenshot aspect/size and quota-within-viewport assertion; confirm it fails before the fix.
- [x] Checkpoint B: isolate the flex sizing/overflow cause and apply the smallest CSS correction.
- [x] Checkpoint C: capture the corrected ultra-wide screen and compare it with the supplied screenshot.
- [x] Checkpoint C: run targeted UI tests, adaptive verification, full verifier, build, and diff check.
- [x] Checkpoint D: record the result and prevention lesson.

### Risk & rollback
- Risk: low. Affected component: desktop quota-card internal sizing only.
- Rollback: revert the focused CSS and browser-verifier assertions; no provider data or persisted settings change.
- Signal: every visible `.window` must remain fully inside its `.quota` clipping viewport at the reported and supported sizes.

### Working notes
- Supplied screenshot is 2491 × 1312 physical pixels and may represent a high-DPI CSS viewport; verify both the exact size and a 1.25×-scaled equivalent.

### Results
- Red-capable repro: `node .scratch/verify-adaptive-ui.cjs` failed at the 1993 × 1050 high-DPI-equivalent viewport because the Codex primary window started above its `.quota` clipping boundary; a computed pseudo-element paint check also captured the ring/glow extent.
- Root cause: the flex card compressed `.quota` below the total intrinsic height of the primary and weekly windows while the wide breakpoint enlarged the primary ring/window; hidden overflow cut the top edge.
- Fix: use two height-aware comfortable sizes, reserve the ring's glow inside the primary window, enter compact mode for dense short-height four-card layouts, and collapse per-card actions only at the existing extreme 400–480px desktop fallback where global refresh remains available.
- Post-fix adaptive verification passed 7 desktop and 7 tablet/phone sizes, including 2491 × 1312 and 1993 × 1050, all 1/2/3/4 count states, and 12-monitor pagination. No card, quota window, ring paint area, dialog, or document overflow remained.
- Targeted UI tests passed 16/16. `scripts/verify-local.ps1`, locked offline desktop build, browser-console check, and `git diff --check` passed.

## 2026-09-10 Reference-led adaptive desktop and mobile UI

### Goal & acceptance criteria
- [x] Desktop follows the supplied dark futuristic dashboard reference while keeping real AgentMeter behavior.
- [x] Tablet/phone follows the supplied light, large-number card reference.
- [x] Desktop and tablet both support adding/removing monitor cards and retain one add slot.
- [x] No horizontal or vertical page scrollbar at supported desktop, tablet, phone, portrait, or landscape sizes; additional cards shrink/reflow to remain inside the viewport.
- [x] Keyboard focus, 44px touch targets, reduced motion, error/setup states, pairing and update behavior remain usable.

### Plan
- [x] Measure reference composition and existing interaction/test contracts.
- [x] Implement shared adaptive grid sizing and desktop monitor selection.
- [x] Restyle desktop and tablet/mobile surfaces without replacing real data with mock content.
- [x] Add overflow/interaction regression coverage for 1, 4, 8 and future Provider counts.
- [x] Capture desktop/tablet/phone screenshots, compare against references, fix P0-P2 drift, and write `design-qa.md`.
- [x] Run full verifier, rebuild/inspect installer, and update handoff.

### Risk & rollback
- Risk: medium; UI shell, monitor visibility preferences, and viewport layout only.
- Rollback: revert UI/assets/tests from this task. No account configuration, credentials, pairing data or database schema is changed.
- Invariant: valid data remains visible, unknown is never fabricated, and settings only hide/show cards rather than stopping Provider collection.

### Results
- Implemented a dark operations-room desktop dashboard and a light, large-metric tablet/phone dashboard using the supplied references.
- Added persistent monitor selection, dynamic future Provider cards, adaptive grid density, main/settings pagination, fullscreen, and one add tile per page on both surfaces.
- Browser verification passed 11 viewport shapes with 12 monitors, no document scrollbars, contained primary content/actions/dialog controls, and 44 px tablet touch targets.
- Rejected unsafe/colliding future Provider IDs and preserved settings focus across background refreshes.
- Full verifier, final release lifecycle, and NSIS inspection passed. Installer: 73,154,793 bytes; SHA-256 `2B90CCCB7256118EFB92C95AC35B939C1F839BE16C75302DF6A54095EBEDDB7F`.

## 2026-09-10 Packaged updater startup failure

### Goal & acceptance criteria
- [x] Reproduce the installed `desktop_startup_failed` updater-config error with a deterministic contract test.
- [x] Add the smallest safe updater base configuration without enabling unsigned or unconfigured downloads.
- [x] Prove the rebuilt release executable starts without writing a new startup failure diagnostic.
- [x] Run targeted/full verification, rebuild NSIS, inspect the artifact, and update the handoff.

### Risk & rollback
- Risk: medium; desktop process currently cannot start after installation.
- Rollback: remove the updater plugin and its commands, or revert the explicit base config. No user data migration is involved.
- Preserve fail-closed signature and exact-GitHub-endpoint validation.

### Results
- Root cause was `plugins.updater = null` during packaged plugin initialization, not WebView2.
- Regression test failed before the config fix and passed after it.
- Repaired release process remained alive without changing the startup failure log, then exited normally via `--request-exit`.
- Full verifier passed. Replacement installer: 71,656,790 bytes; SHA-256 `3A073752C00508EFAA9317B0616AAB3EF3736B751C74E77A86630DF53AB46C55`.

## 2026-09-10 Dynamic tablet monitors, silent collection, and updates

### Goal & acceptance criteria
- [x] Desktop background refresh never opens a visible console window.
- [x] Tablet dashboard exposes an accessible fullscreen control and prioritizes quota data over explanatory copy.
- [x] Monitor tiles can be added or removed at runtime, persist across reloads, and never drop below one visible slot.
- [x] Settings allow monitor count/source selection and a manual update check.
- [x] App startup performs a non-blocking GitHub release check; failure never blocks monitoring.
- [x] Unavailable providers show provider-specific, step-by-step install/sign-in/retry guidance.
- [x] Existing pairing, monitor-only tablet permissions, stale-data semantics, and four-provider defaults remain intact.

### Plan
- [x] Checkpoint A: build tight repro/tests for console process flags and current tablet/settings behavior.
- [x] Checkpoint A: inspect existing provider, persistence, packaging, and GitHub release seams.
- [x] Checkpoint B: implement the smallest silent-process and dynamic monitor/settings slice.
- [x] Checkpoint B: implement fullscreen and a content-first responsive tablet visual system.
- [x] Checkpoint C: add regression coverage for min-one, persistence, update states, and provider guidance.
- [x] Checkpoint C: run targeted tests, Clippy/build, browser geometry/a11y checks, then full local verifier.
- [x] Checkpoint D: document release/update trust boundaries, rollback, and any clean-VM or physical-tablet limits.

### Risk & rollback
- Risk: medium. Affected components: Windows child-process creation, tablet UI state, desktop settings, and release-network behavior.
- Rollback: revert this section's focused files; no schema/data deletion. Persisted monitor preferences must tolerate absence and unknown future provider IDs.
- Rollout: startup update checks are advisory and fail closed; installation remains an explicit user action.
- Signals: no visible console regression test, update state/result, tablet render geometry, provider setup CTA availability.

### Dependencies & environment
- Windows/Tauri/Rust, vanilla browser UI, offline locked Cargo for normal verification.
- GitHub release checks require network only at runtime; tests use fixtures/mocks.
- Physical tablet and signed production releases remain separate acceptance boundaries unless available locally.

### Working notes
- Domain invariant: the tablet remains monitor-only; settings that mutate desktop/provider configuration must stay on desktop unless an existing contract explicitly permits otherwise.
- Preserve unknown observations as unknown; never fabricate zero/full quota.

### Results
- Completed dynamic/fullscreen tablet UI, setup guides, silent ConPTY collection and signed GitHub updater seam.
- Post-review fixes preserve source-usage-only values, show guidance for initial unavailable cards, keep monitoring alive on updater pre-launch failure, scope the Device Pair cookie, and rebuild bundled Node dependencies from the lockfile.
- `scripts/verify-local.ps1` and three-viewport browser geometry checks passed.
- Rebuilt and inspected unsigned preview installer: 71,654,359 bytes, SHA-256 `76CC976B89899A676CE936D0C10F80640EDCF17E8D1A2C85112418657A5C1BB9`.
- Signed GitHub release, clean-VM install and physical-tablet acceptance remain external gates.

## 2026-09-10 Tablet request_parse_failed
- [x] Reproduce delayed browser request failure on Windows before fixing.
- [x] Restore blocking mode on per-connection workers; allow verified USB origin only.
- [x] Add fragmented request and USB origin/pairing/revocation regression tests.
- [x] Run root/desktop tests and Clippy; build and inspect new NSIS installer.

Installer: desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter-20260910-tablet-http-fixed-setup.exe (70,601,411 bytes), SHA256 9C6B70155613087291305EE0398336D8D3EAC19902A5F591BF10F8C94C477AC3. NSIS inspection passed, embedded version 0.1.0. Physical tablet acceptance remains pending because ADB lists no device. Details: docs/evidence/tablet-http-fix-2026-09-10.md.

Risk: medium, HTTP and origin boundary. Loopback binding, pairing and CSRF remain required. Rollback: revert this task's socket/origin changes and rebuild; no stored pairs are deleted.
Environment: Windows, offline locked Cargo, existing Tauri/NSIS toolchain.
Evidence: delayed_fragmented_browser_request_is_not_rejected failed with connection abort before set_nonblocking(false), passed after. Full tests found an inherited missing transport_id validation despite the existing USB test and comment; restored the intended check.

## Acceptance criteria
Compact command bar and four quota cards fit the 1080 x 640 default window with tablet settings collapsed. Accurate remaining-percent bars and failure chips. Preserve tablet controls. Require actual WebView evidence before claiming live 4/4 or removing temporary diagnostics.

## Checklist
- [x] Inspect repository and handoff; compact UI was not saved locally.
- [x] Implement compact UI and regression coverage.
- [x] Verify actual desktop provider rendering and decide diagnostic cleanup: real WebView 4/4; diagnostics removed.
- [x] Run scripts/verify-local.ps1 and browser layout checks.
- [x] Rebuild release/installer, inspect bundled runtime and version, and document results.

## Risk & Rollback
Low-risk UI changes; preserve unrelated working-tree changes. Restore only this task's five UI/config/test files from task baseline if needed. Do not change account trust or login settings.

## Dependencies & Environment
Windows PowerShell, Rust/Cargo, Node, Tauri/NSIS and existing bundled quota helper. Real provider checks depend on existing local logins and network access.

## Working Notes
Screenshot is a handoff reference, not proof of saved changes or successful verification. Existing handoff requires real WebView 4/4, not a CLI-only result. No tasks/lessons.md existed at session start.


## Results
Compact UI and diagnostic cleanup completed. Full local verifier and Edge layout checks passed. Actual pre-cleanup WebView4/4 evidence captured. NSIS installer built and statically inspected; not installed or clean-VM tested. See docs/evidence/desktop-compact-ui-2026-09-10.md.

## 2026-09-11 AgentMeter brand and glass dashboard redesign

### Goal & acceptance criteria
- [x] Resolve the supplied AgentMeter logo and dashboard reference as the visual source of truth.
- [x] Integrate a transparent AgentMeter wordmark and icon into desktop, tablet, window, installer, and tray surfaces.
- [x] Rebuild every existing desktop and tablet UI surface with the supplied deep-navy glass, cyan glow, and provider-accent visual language.
- [x] Preserve quota semantics, monitor paging/add slot, provider setup guidance, settings/update flow, tablet pairing/USB controls, and security copy.
- [x] Keep the default 1080 x 640 desktop and supported tablet/phone viewports usable without document overflow.
- [x] Pass targeted UI tests, full local verification, browser interaction/geometry checks, and visual design QA.

### Plan
- [x] Checkpoint A: inspect the current desktop/tablet UI, asset pipeline, runtime icon paths, tests, and existing dirty-worktree changes.
- [x] Checkpoint B: add production brand assets and implement the desktop redesign as the smallest behavior-preserving slice.
- [x] Checkpoint B: extend the same visual system to tablet pairing, dashboard, settings, and guide surfaces.
- [x] Checkpoint C: update/add regression contracts for brand assets and run targeted tests plus full verification.
- [x] Checkpoint C: capture matching desktop/tablet views, compare against the supplied reference, fix P0/P1/P2 drift, and record `design-qa.md`.
- [x] Checkpoint D: record results, operational impact, rollback, and any packaging or device-only verification gap.

### Risk & rollback
- Risk: medium. Affected components: all user-visible desktop/tablet UI, static tablet asset routing, build/window icon, installer icon, and tray icon.
- Rollback: revert only this section's brand/UI/static-asset changes; no provider data, persisted pairing, or settings schema is changed.
- Signals: desktop/tablet geometry assertions, UI contract tests, CSP/static asset tests, Tauri build, and visual comparison captures.

### Dependencies & environment
- Existing vanilla HTML/CSS/JS desktop UI and embedded tablet UI; no new runtime dependency.
- Windows/Tauri offline Cargo toolchain. Browser QA uses the existing local preview helper and browser surface.
- Product Design saved-context preflight could not run because no installed Python interpreter is available; the two supplied images remain the explicit source of truth for this task.

### Working notes
- Preserve the current dirty worktree and existing untracked adaptive UI work; do not replace unrelated product or backend changes.
- Brand source is the supplied charcoal/teal AgentMeter mark. Dashboard source is the supplied 1452 x 1086 desktop visual.
- Existing provider data may expose quota windows or source usage; unknown must stay visually distinct from zero.

### Results
- Rebuilt the desktop dashboard, settings, update, tablet connection, and setup-guide surfaces plus tablet pairing, dashboard, settings, and guide surfaces in one deep-navy glass visual system.
- Integrated transparent AgentMeter branding, four provider icons, a cloud/slash empty-state illustration, Tauri window/installer icons, and a native tray icon without adding runtime dependencies.
- Preserved real quota/error/stale semantics, monitor paging/add tile, future-provider fallback, tablet monitor-only behavior, pair-code boundaries, and existing DOM/API contracts.
- Targeted Rust tests passed (9 desktop bundle/config + 22 tablet HTTP). Adaptive UI verification passed at 5 desktop and 7 tablet/phone viewports with zero document scroll.
- In-app browser interaction checks covered desktop settings/tablet dialogs and tablet dashboard/settings; both consoles were clean. Same-size design QA passed with no actionable P0/P1/P2 findings; see `design-qa.md`.
- `scripts/verify-local.ps1`, `git diff --check`, and `cargo build --manifest-path desktop-p0/Cargo.toml --offline --locked` all passed. Physical tablet, authenticated providers, clean-VM installation, and signed production packaging remain external acceptance boundaries.

## 2026-09-11 Full-screen monitor composition and settings-owned catalog

### Goal & acceptance criteria
- [x] Remove the add-monitor tile from desktop and tablet dashboards; settings is the only add/remove/reorder entry point.
- [x] Render each page by visible count: 1 = 1×1, 2 = 2×1, 3 = 1×3, 4 = 2×2; paginate additional monitors four at a time.
- [x] Keep every visible card and control inside the viewport at the supported desktop, tablet, phone portrait, and phone landscape sizes.
- [x] Show Cursor and Kiro as catalog-only agents in settings with honest unsupported/not-checked status; never create quota cards or refresh calls until collectors exist.
- [x] Preserve provider data semantics, future-provider safety, selection persistence, and tablet monitor-only behavior.
- [x] Pass targeted tests, full local verification, browser interaction checks, and updated visual design QA.

### Plan
- [x] Checkpoint A: reproduce the cramped preview and inspect layout, selection, provider, and test contracts.
- [x] Checkpoint B: lock the new count-driven topology and settings reorder behavior with failing tests.
- [x] Checkpoint B: implement desktop dashboard composition and catalog/settings UX.
- [x] Checkpoint B: implement matching tablet composition and settings UX without desktop-only mutations.
- [x] Checkpoint C: update adaptive geometry verification and run targeted/full checks.
- [x] Checkpoint C: capture the revised preview and rerun same-source design QA.
- [x] Checkpoint D: document results, risks, rollback, and external verification gaps.

### Risk & rollback
- Risk: medium. Layout, pagination, selection persistence, and settings UI change on both desktop and tablet.
- Rollback: revert only this task's layout/view/client/dashboard/HTML/CSS/test edits. No provider data, pairing credentials, or backend collector is changed.
- Signals: exact 1/2/3/4 topology tests, maximum-four pagination, zero add-tile assertion, content containment, console errors, and full local verifier.

### Dependencies & environment
- No new runtime dependency. Existing HTML/CSS/JS and Rust HTTP embedding remain unchanged.
- Product Design saved-context and UI/UX search scripts could not run because the configured Python 3.11 executable is unavailable; the supplied dashboard remains the source of truth and the skill's responsive/accessibility checklist is applied directly.

### Working notes
- Cursor/Kiro are catalog-only. Without a collector or reliable desktop discovery result, their installation state is `未檢查`, never `未安裝`.
- Existing users must not receive newly supported providers automatically; they add them explicitly in settings.

### Results
- Removed dashboard add tiles and made settings the sole add/remove/reorder surface on desktop and tablet.
- Implemented exact visible-count compositions (1 × 1, 2 × 1, 1 × 3, 2 × 2) and fixed four-item pages; a partial final page derives its geometry from its own visible count.
- Added honest catalog-only Cursor and Kiro rows. They cannot create cards, quota refreshes, or fabricated installation status until a backend collector publishes support.
- Targeted UI contracts passed 16/16. Adaptive browser verification passed 5 desktop and 7 tablet/phone viewports with all four count states, 12-monitor pagination, zero add tiles, zero document scroll, and contained card/dialog content.
- `scripts/verify-local.ps1`, locked offline desktop build, and `git diff --check` passed. Physical tablet, authenticated providers, clean-VM installation, and signed production packaging remain external acceptance boundaries.

## 2026-09-11 Codex quota period labels

### Goal & acceptance criteria
- [x] Preserve Codex limit name, plan type, and actual `windowDurationMins` through the desktop collection pipeline.
- [x] Label 300-minute windows as `5 小時用量限制` and 10080-minute windows as `每週使用上限`; never expose `primary` / `secondary` as user terminology.
- [x] Show understandable group names for regular, Spark, and reserve-model limits on desktop and tablet.
- [x] Let the API shape determine whether an account shows five-hour, weekly, or both windows; do not infer windows from Plus/Pro names.

### Plan
- [x] Checkpoint A: reproduce the misleading labels and trace metadata loss from App Server to both UIs.
- [x] Checkpoint B: add failing collection/projection/UI regression tests.
- [x] Checkpoint B: preserve metadata and implement duration-based labels in the smallest shared seams.
- [x] Checkpoint C: run targeted Node/Rust tests, responsive browser verification, full local verification, and diff checks.
- [x] Checkpoint D: record results and any external verification gap.

### Risk & rollback
- Risk: low. Affected components: Codex quota metadata projection and display copy only.
- Rollback: revert this task's helper, projection, UI formatter, and test changes; no stored observations or credentials are mutated.
- Signals: exact Plus/Pro-shaped fixtures, no `primary`/`secondary` in rendered Codex labels, desktop/tablet parity, responsive containment.

### Dependencies & environment
- Existing Codex App Server response and vanilla JS/Rust pipeline; no new dependency.
- Live account evidence is read-only. Offline fixtures remain the deterministic regression source.

### Working notes
- `primary` / `secondary` are transport field positions, not product-facing period names.
- The authoritative period is `windowDurationMins`: current Pro-like live data reports regular weekly only, while Spark reports both five-hour and weekly windows.

### Results
- Codex collector output and Rust projection now retain limit ID/name, plan type, window identity, and duration without changing credentials or stored schema.
- Desktop and tablet render `常規使用額度 · 每週使用上限`, `GPT-5.3-Codex-Spark · 5 小時用量限制`, `GPT-5.3-Codex-Spark · 每週使用上限`, and `備用模型額度 · 每週使用上限` from authoritative metadata.
- Plus-shaped 300/10080 and Pro-shaped weekly-only fixtures pass; absent rows remain absent, so the UI never fabricates a plan window.
- Verification: 29 focused Node tests passed, 4 desktop auto-quota Rust tests passed, 18 Windows Edge viewport matrices passed, and `scripts/verify-local.ps1` completed with all tests, formatting, and Clippy checks passing.
- External boundary: no installer was rebuilt and no credential or account state was changed. Current official documentation describes plan-dependent five-hour/weekly limits but does not guarantee one fixed shape for every Plus or Pro account.
