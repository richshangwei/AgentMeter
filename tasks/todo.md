# 2026-11-21 發布 0.2.9 簽署更新

- [x] 確認版本、工作區、遠端同步及既有簽署金鑰。
- [x] 執行 verify-local 與 manifest 測試並建置簽署安裝包。
- [x] 驗證簽章、trusted comment、竄改拒絕。
- [x] 建立草稿、核對資產後發布並驗證公開更新資訊。

結果：verify-local 通過；安裝包內嵌版本 0.2.9，大小 73,845,008 bytes，SHA-256 FC1181EC058BC2F5AC4A8C3B10AB5194301D00F0562E28946F12E6BF8759256B。草稿三個資產大小與 SHA-256 和本機一致；公開 https://github.com/richshangwei/AgentMeter/releases/tag/v0.2.9 ，latest manifest 為 0.2.9，安裝檔 HEAD 200，tag 指向 b0cfe16。未在隔離環境實測更新安裝及重啟；NSIS 未具 Windows Authenticode 簽章。
風險：高。回復：將 latest 指回 0.2.8 或撤下 0.2.9 更新資訊；已安裝版本不自動降版。

# 2026-11-17 發布 0.2.8 簽署更新

- [x] 確認版本、工作區及既有簽署金鑰。
- [x] 驗證既有簽章金鑰識別碼與 0.2.7 相符並確認遠端狀態。
- [x] 執行完整本機驗證並建置簽署安裝包。
- [x] 驗證安裝包、簽章、防竄改及更新資訊。
- [x] 建立草稿、核對資產後發布並驗證公開更新資訊。

結果：完整 verify-local 通過；安裝包內嵌版本 0.2.8，大小 73,824,995 bytes，SHA-256 140C4B104600A186E619EF78241B0BE67A8458B7ED28D23323DDC70B713D0EFA。簽章、trusted comment、竄改拒絕與 manifest 核對通過；可執行檔包含正式 endpoint 和公鑰。GitHub 草稿三個資產名稱、大小、SHA-256 均與本機一致。公開 https://github.com/richshangwei/AgentMeter/releases/tag/v0.2.8 ，latest manifest 為 0.2.8，安裝檔 HEAD 200。未在隔離環境進行實際更新安裝及重啟驗收；此機未設定更新的 0.2.7 需手動安裝一次。NSIS 未具 Windows Authenticode 簽章，與 Tauri 更新簽章不同。
風險：高，公開更新會影響使用者；只使用既有簽章金鑰，先確認三個資產完整。回復：將 latest 指回 0.2.7 或撤下 0.2.8 的更新資訊；已安裝版本不自動降版。環境：Windows、既有 DPAPI 金鑰、Rust/Tauri、7-Zip 和 GitHub 發布權限。已安裝的未設定更新來源版本仍需手動安裝一次。

# 2026-11-16 修復已提交的合併衝突

- [x] 找出已提交的衝突標記與原始兩側內容。
- [x] 保留歷程、本機資源與可拖曳 HUD，合併工作紀錄。
- [x] 驗證無衝突標記、Cargo 設定與相關測試。
- [x] 記錄結果；修復提交待推送。

結果：git grep 無衝突標記；cargo metadata 通過；scripts/verify-local.ps1 全數通過（含桌面 37 項測試及 Clippy）。node scripts/verify-hud.cjs 因缺少 Playwright 未執行瀏覽器驗證；實體多螢幕與系統匣互動亦未驗證。

工作備註：HUD 目前支援拖曳並接收滑鼠事件，因此保留目前的 hover 行為及其瀏覽器驗證；歷史穿透教訓不代表現行設計。風險為中等；可回復本次修復提交。

# 2026-09-17 修正用量偵測時 CMD 閃現

## Goal & acceptance criteria
- [x] 重現並定位背景用量偵測期間出現可見 CMD 視窗的實際啟動路徑。
- [x] 所有桌面背景用量收集子行程在 Windows 均不建立可見 console，且不啟動第三方自動更新器／daemon。
- [x] 加入能在修正前失敗、修正後通過的回歸檢查，並保留實機正向控制驗證方式。
- [x] 不改 Provider 回傳資料、排程頻率或前景手動工具行為。

## Plan
- [x] Checkpoint A: 盤點收集器啟動路徑、建立最小紅燈測試並重現。
- [x] Checkpoint B: 依證據修正最小的 Windows 子行程啟動邊界。
- [x] Checkpoint C: 執行 targeted tests、完整本機驗證與實機 console 監測。
- [x] Checkpoint D: 記錄 root cause、驗證結果、風險與回復方式。

## Risk & rollback
- Risk: medium；影響背景 Provider CLI 子行程建立方式，錯誤可能造成某 Provider 無法收集，但不改資料格式或帳號設定。
- Rollback: 回復本任務的行程啟動參數與回歸測試；無資料遷移。
- Signals: 收集結果維持相同；可見 console 監測須先抓到正向控制，再在背景刷新期間維持零事件。

## Dependencies & environment
- Windows / Tauri / Rust；沿用既有 Provider CLI 與測試工具，不新增依賴。
- 現有工作樹包含其他未提交變更；只修改與本問題直接相關的檔案與本段稽核紀錄。

## Working notes
- 靜態 `CREATE_NO_WINDOW` 契約不足以證明沒有閃窗；實機監測必須先通過可見 console 正向控制。

## Results
- Root cause：Antigravity 官方要求 `AGY_CLI_DISABLE_AUTO_UPDATE=true`，原實作傳入 `1`；auto-updater 的 15 分鐘 TTL 到期後仍可能啟動背景 helper，造成間歇性 CMD 閃現。
- 開發與打包用 `quota-smoke.mjs` 均改用官方精確字串；契約測試同時拒絕舊值 `1`。
- 視窗監測新增 Antigravity updater fixture，並修正 Provider 執行失敗時證據被例外吃掉的問題。舊值可重現 `cmd → cmd → agy → node → powershell`，新值連跑三次零視窗。
- 驗證：正向控制成功捕捉可見 CMD；150 秒已安裝 GUI 排程觀測確實看見 2 個 collector 且零視窗；targeted 11/11、完整 `scripts/verify-local.ps1`、兩 crate fmt/test/Clippy 全部通過。
- 現有已安裝 0.2.6 尚未被本次工作區原始碼取代；需後續建置／發布後才會取得此修正。

# 2026-09-12 修正 Codex HUD 額度與透明互動

## Goal & acceptance criteria
- [x] Codex HUD 依額度語意選取「常規使用額度」；有多個常規週期時優先顯示每週用量，不能因陣列順序誤選 Spark 或備用模型額度。
- [x] HUD 視覺面積縮小約 50%，四列名稱與數值仍完整、無重疊、無裁切。
- [x] 透明度可設定至 100%；低於 100% 時，滑鼠移入只讓面板背景回復不透明，名稱與數字永遠保持 100% 不透明。
- [x] HUD 保持不搶焦點，但能接收 hover；桌面主清單與其他視窗不受 HUD 透明度影響。

## Plan
- [x] Checkpoint A: 建立 Codex 反向排序額度與 HUD hover/文字不透明的失敗測試。
- [x] Checkpoint B: 實作語意選取、緊湊尺寸、背景專用透明度與滑鼠命中。
- [x] Checkpoint C: 執行單元、Rust 契約、瀏覽器幾何與完整本機驗證。
- [x] Checkpoint D: 視覺檢查、記錄結果與新增防止回歸的 lesson。

## Risk & rollback
- Risk: medium；影響 HUD 顯示資料選擇、視窗大小與滑鼠命中，不改 Provider 原始資料或主儀表板額度。
- Rollback: 回復本任務的 HUD model/JS/CSS/Tauri 設定、Rust 視窗設定與對應測試；不需資料遷移。
- Signals: 反向排序 fixture 必須顯示常規每週額度、文字 computed opacity 必須為 1、hover 背景 alpha 必須為 1、240×132 四列不得溢出。

## Results
- Codex HUD 不再使用第一筆額度；它會先找 `codex`／`codex/default` 常規額度，有每週視窗時優先每週，否則顯示帳號實際提供的唯一常規視窗。
- 四列 HUD 由 360×204 級距縮至 240×132，面積約減少 57%；實際 Edge 畫面確認四個名稱與數值無重疊、無裁切。
- 透明度範圍擴為 35–100%。透明度只套用在 HUD 背景色，文字固定 opacity 1；滑鼠移入時背景以 160ms 回復 100%。
- HUD 視窗仍為 non-focusable，但不再忽略游標事件，因此 hover 能正常觸發。主儀表板未載入 `hud.css`，其清單透明度不受影響。
- 驗證：HUD 單元測試 7/7、Rust HUD 契約 1/1、Edge HUD 幾何/hover 驗證、完整 `scripts/verify-local.ps1` 與 Clippy 全部通過。

## Dependencies & environment
- 使用既有 vanilla JS、Tauri/Rust 與 Playwright 驗證，不新增依賴。
- 真實帳號資料只作語意依據；固定 fixture 是可重現的回歸證據。

## Working notes
- `quota_windows` 的順序不是產品契約；Codex 必須依 `limit_id` 與 `window_duration_mins` 選擇。
- CSS 不得把 `opacity` 套在 HUD 父層；透明度只能進入背景色 alpha。

# 2026-09-17 發布 0.2.7 CMD 閃現修正版

## Goal & acceptance criteria
- [x] 從公開 0.2.6 基線只移植 Antigravity updater 停用值與視窗回歸測試，不混入主工作區其他功能。
- [ ] 應用程式、Cargo lock、安裝檔、manifest、commit 與 tag 版本一致為 0.2.7。
- [x] 完整測試、實機 console 紅／綠控制、簽署建置、NSIS 檢驗與更新簽章全部通過。
- [ ] GitHub Release 僅包含安全命名安裝檔、`.sig`、`latest.json`，公開 Latest 與匿名下載驗證通過。

## Plan
- [x] Checkpoint A: 審核 0.2.6 基線、發布差異、秘密與排除項目。
- [x] Checkpoint B: 移植最小修正、更新 0.2.7 版本並執行完整驗證。
- [x] Checkpoint C: 建立簽署草稿，驗證安裝包、manifest、簽章與竄改拒絕。
- [ ] Checkpoint D: 提交／推送、建立 v0.2.7 Release，驗證 Latest 與公開資產。

## Risk & rollback
- Risk: high；公開可執行檔、不可變 tag／資產與自動更新 Latest 指向。
- Rollback: 發布前停止並刪除草稿；發布後將 v0.2.7 改為非 Latest／draft，停止新下載，再以更高修正版恢復；不覆寫既有資產或自動降版。
- Security: 私鑰只以既有 CurrentUser DPAPI 在記憶體解密；不輸出、不提交、不上傳金鑰。Release 只允許三個核准資產。
- Exclusions: 主工作區未提交的 History／Token 分析／HUD 與文件、Logo、截圖、evidence、`.scratch`、target 不納入本次 hotfix diff 或 Release 資產；基線既有追蹤檔不在本次清理範圍。

## Dependencies & environment
- Windows、既有 Rust/Tauri/Node/NSIS/7-Zip/GitHub CLI、本機既有 DPAPI updater key。
- 發布基線固定為 `origin/master` 的 0.2.6 發布紀錄提交；隔離工作樹為 `.release-worktrees/0.2.7`。

## Working notes
- 官方 Antigravity CLI 文件要求 `AGY_CLI_DISABLE_AUTO_UPDATE=true`；舊值 `1` 可能未停用 15 分鐘 TTL updater。
- 完整 `scripts/verify-local.ps1`、11 個 targeted Node tests、Antigravity 三輪零視窗 fixture 與故意可見 CMD 正向控制皆通過。
- 簽署草稿：`desktop-p0/target/update-drafts/0.2.7-70ee52da142c45f89ba41c9daf71d0e6`；installer 73,628,013 bytes；SHA-256 `093178E0973B2F8C84228F4F0C983A3329856FAEA7FFE828992E6FB5E90D8FAA`。
- NSIS 內嵌 product/file version 均為 0.2.7；Tauri updater signature、trusted comment 與單位元竄改拒絕通過。Windows Authenticode 仍為 NotSigned。

# 2026-09-12 發布 0.2.5 HUD 修正版

## Goal & acceptance criteria
- [x] 僅發布已驗證的 Codex HUD 額度語意、緊湊尺寸與透明 hover 修正。
- [x] 版本、安裝檔、簽章、manifest 與 Git tag 一致為 0.2.5。
- [x] GitHub Release 僅包含安全命名的安裝檔、`.sig` 與 `latest.json`，並可由 0.2.4 自動更新發現。

## Plan
- [x] 由 `origin/master` 建立隔離發布工作樹，只帶入 HUD 修正與測試。
- [x] 執行完整本機驗證與簽署建置。
- [x] 審核 diff，以詳細繁中 commit 提交並推送。
- [x] 建立 v0.2.5 Release，驗證下載、SHA256、簽章與 Latest manifest。

## Risk & rollback
- Risk: high；這會發布可執行安裝檔及不可變更新資產。
- Rollback: 發布前停止；發布後將 v0.2.5 改為非 Latest，並以更高修正版恢復，不覆寫既有資產。
- Exclusions: 不包含原工作區的共享規格文件、任務紀錄或任何 `.scratch`／`Claude outputs` 圖片。

## Working notes
- 隔離工作樹需先以固定 SHA-512 的 Antigravity CLI 執行 `scripts/prepare-desktop-quota.ps1`，重建被 Git 忽略的測試依賴與打包 runtime。
- 完整 `scripts/verify-local.ps1`、Edge HUD 幾何/hover 驗證及 Tauri 簽章/可信註解/單位元竄改拒絕皆通過。
- 本機草稿：`desktop-p0/target/update-drafts/0.2.5-cb9e3e0b6d954a559c333da915025924`；安裝檔 73,594,573 bytes；SHA256 `5301BD1CF2BDF3E6F155B484A43B06FE96399A74ED3C393EE704FDB77CA27735`。Windows Authenticode 仍為 `NotSigned`。

## Results
- 發布 commit `92ad463a437d8c4c6885947dda4d2a2df4779561` 已推送至 `master`，同一 commit 已建立並推送不可變 tag `v0.2.5`。
- GitHub Release `v0.2.5` 已公開、非 prerelease 並設為 Latest；資產僅有安全命名安裝檔、相鄰簽章與 `latest.json`。
- 匿名重新下載三個資產成功；遠端安裝檔為 73,594,573 bytes，SHA256 與本機草稿完全相同，公開 `latest.json` 回報 0.2.5 且安裝網址精確指向 v0.2.5 資產。
- 0.2.4 已編入相同 Latest endpoint 與更新公鑰，可發現、下載並驗證 0.2.5；安裝仍需使用者明確確認。未在本機覆寫安裝中的 0.2.4，也未聲稱完成真實升級安裝驗收。

# 2026-09-12 發布 0.2.4 自動更新

## Goal & acceptance criteria
- [x] 將同步頻率設定與右下角 HUD 納入 0.2.4，排除預覽圖片與所有私鑰材料。
- [x] 完整測試、簽署建置、NSIS 檢驗、manifest URL 與簽章驗證全部通過。
- [x] 以詳細繁中 commit 提交並推送 master，建立不可變的 v0.2.4 tag。
- [x] GitHub Release 只包含安全檔名安裝檔、簽章與 latest.json，匿名下載皆為 HTTP 200。
- [x] Latest manifest 回報 0.2.4，且已發布 0.2.3 可發現並驗證此更新。

## Plan
- [x] Checkpoint A: 審核工作樹、既有發布規則、遠端與金鑰邊界。
- [x] Checkpoint B: bump 0.2.4，執行完整與瀏覽器驗證。
- [x] Checkpoint B: 使用既有 CurrentUser DPAPI 私鑰完成簽署建置與草稿驗證。
- [x] Checkpoint C: 精確 stage 原始碼，排除圖片/金鑰，提交、推送與建立 tag。
- [x] Checkpoint C: 發布三個資產，驗證 Latest、匿名下載、SHA256 與簽章。
- [x] Checkpoint D: 記錄發布證據、風險、回復與未完成的真實安裝驗收。

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

## Results
- 發布提交 `24433f4` 已推送至 `master`，annotated tag `v0.2.4` 指向同一提交。
- GitHub Release 為公開穩定版且設為 Latest；資產僅有 `AgentMeter-P0_0.2.4_x64-setup.exe`、相鄰 `.sig` 與 `latest.json`。
- 未登入的公開 Latest endpoint 回報 0.2.4，安裝檔 URL 精確指向 `v0.2.4` 安全檔名並成功下載。
- 遠端安裝檔大小 73,603,937 bytes、SHA-256 `3AF973A5F308AF873DB4DBAD9AB6791B3CF0613A701A4188AC8B3DAA8FA6E841`，與本機簽署草稿一致；公鑰簽章、trusted comment 與竄改拒絕再次通過。
- 未執行使用者已安裝 0.2.3 的實際 GUI 升級與重啟；自動檢查會在啟動時或每 6 小時執行，也可從設定立即按「檢查更新」。

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
# 2026-09-12 AgentMeter 網站設計架構與功能規格書

## Goal & acceptance criteria
- [x] 以實際網站走查與原始碼為雙重證據，完整盤點所有可見頁面、狀態、互動與錯誤/空資料流程。
- [x] 還原前端、桌面殼層、Rust 核心、Provider collectors、持久化、配對、USB/HTTP/SSE、更新與安裝的模組架構及資料流。
- [x] 定義可供後續開發直接引用的功能需求、資料契約、狀態機、非功能需求、安全限制、測試矩陣與驗收標準。
- [x] 對現況與規劃中功能做明確標示；未知或未能實機驗證者不得寫成已完成。
- [x] 產出完整 Markdown 規格書並驗證文件內的路徑、符號、路由、指令與跨章一致性。

## Plan
- [x] Checkpoint A: 走查網站所有分頁、日期/圖表/Provider 切換、響應式與錯誤狀態，保存觀察證據。
- [x] Checkpoint A: 由測試、介面、ADR/需求文件與實作建立現況清單及權威來源排序。
- [x] Checkpoint B: 建立產品資訊架構、使用者旅程、功能矩陣、資料模型/契約與狀態機。
- [x] Checkpoint B: 建立深模組、介面、seam、adapter、執行緒/程序、儲存與安全架構規格。
- [x] Checkpoint C: 完成 UI/UX、RWD、無障礙、效能、可靠性、隱私、安全、可觀測性與更新部署規格。
- [x] Checkpoint C: 完成測試矩陣、驗收案例、開發優先序、風險/回滾與未決事項。
- [x] Checkpoint D: 用原始碼搜尋、現有測試與文件檢查交叉驗證規格書，補上 Results。

## Risk & rollback
- Risk: low；本工作只新增/更新文件與任務紀錄，不改變執行中網站、帳號、配對資料或程式行為。
- Rollback: 移除本次新增規格文件與本節任務紀錄即可。
- Signals: 網站實際畫面、DOM/瀏覽器狀態、Rust/JS 符號、路由與測試名稱能互相對應。

## Dependencies & environment
- 本機網站 `http://localhost:3003`、現有 Rust/Tauri/vanilla JS 原始碼、測試、ADR、README 與 v1.1 需求書。
- 實際登入 Provider、實體平板、ADB 裝置、簽章發行與乾淨 VM 若不可用，僅列為外部驗收邊界。

## Working notes
- 以「現況已實作 / 現況受限 / 後續規格」三種標籤避免把設計目標誤寫為現況。
- 介面需包含呼叫者必須知道的資料、順序、不變量、錯誤模式、組態與效能特性。

## Results
- 產出 `docs/AgentMeter-Token-Analytics-Spec-v1.0.md`：17 章、1,235 行，分離參考網站實測、固定上游 commit、AgentMeter 現況與 Core/optional 目標規格。
- 實際走查 Daily/Monthly/Yearly、trend/K 線、Session drawer、搜尋/篩選、定價、設定、匯入紀錄、空資料、深淺主題與 390×844 響應式；完成 API 及 DOM 契約核對，未保存私人 Session／路徑／用量值。
- 以 codebase-design 收斂為單一 `History` 深 Module、Provider Collector Adapter、Collector Coordinator、內部 ingestion 與 SQLite Repository Adapter，並保留 `LiveDashboard` 為即時 read model。
- 安全審閱納入 loopback＋Bearer session、vendored runtime＋CSP、metrics-only、FactIdentityV1、Source-scoped provisional Account、counter ordering、WAL-safe backup、匯入 provenance、IANA timezone 與 bounded API。
- Core v1 固定為四 Provider、trend、metrics-only、official cost；K 線、estimated cost、Session content 與其他 Provider 為獨立 optional profiles。
- 文件驗證通過：12 個本機連結全數存在、30 個 code fences 成對、Markdown table 無欄位錯位、116 個 normative row ID 無重複、私人值與已知阻斷字串掃描皆為 0、`git diff --check` 無文件錯誤。
- 本次只改文件，未執行應用程式測試／build；本機網站最後已還原為原始 Codex／Daily／2026-09-12／K 線／深色桌面狀態。
# 2026-09-12 HUD 不攔截系統視窗與對話用量可行性

## Goal & acceptance criteria
- [x] 右下角 HUD 保持置頂與不搶焦點，但所有滑鼠操作可穿透到下方的 Windows 系統匣、通知中心與一般視窗。
- [x] 移除與滑鼠穿透衝突的 hover 行為及說明，透明度、文字可讀性、四列緊湊尺寸維持不變。
- [x] 以既有 Token 分析規格及實際資料來源，確認「送出的對話／Token 浪費分析」可行範圍、隱私邊界與最小落地順序。
- [x] 通過 HUD 原生視窗契約、HUD 單元/瀏覽器驗證與完整本機驗證。

## Plan
- [x] Checkpoint A: 依使用者截圖重建症狀，定位 HUD 原生命中行為與現有回歸測試。
- [x] Checkpoint B: 先讓滑鼠穿透契約測試失敗，再做最小原生視窗與文案/CSS 修正。
- [x] Checkpoint C: 執行 targeted tests、HUD 瀏覽器幾何驗證、完整本機驗證與 diff 檢查。
- [x] Checkpoint D: 整理對話用量功能的現況差距、privacy-first MVP、風險與後續實作切片。

## Risk & rollback
- Risk: low（HUD 視窗互動）／high（未來對話正文蒐集）。本次程式碼只恢復 HUD 點擊穿透，不新增對話內容蒐集或持久化。
- Rollback: 回復本節的 HUD 原生視窗、CSS、設定文案與測試變更；不涉及資料遷移。
- Signals: 原生視窗契約必須要求 `set_ignore_cursor_events(true)`；HUD 不含可操作控制項；瀏覽器幾何與文字透明度維持通過。

## Dependencies & environment
- Windows/Tauri/WebView2；既有 HUD 與 Playwright/Edge 驗證工具，無新增依賴。
- 對話分析依各 Provider 本機 session/transcript 可用性；內容只能 desktop-only、明確 opt-in，且不得進 tablet、一般 diagnostics 或預設 export。

## Working notes
- 截圖顯示 HUD 疊在 Windows 系統匣展開面板上；`set_focusable(false)` 不等於滑鼠穿透。
- 前一版為支援 hover 移除了 `set_ignore_cursor_events(true)`，直接違反原始「可穿透點擊」驗收條件。

## Results
- 恢復 Tauri HUD 的 `set_ignore_cursor_events(true)`；維持 `alwaysOnTop`、non-focusable 與 240×132 四列版面，底下的系統/應用視窗可接收點擊。
- 移除不可能與原生滑鼠穿透並存的 hover 不透明效果，設定文案改為明確說明「滑鼠操作會直接穿透到下方視窗」。
- 回歸測試先觀察到精確失敗，再於修正後通過；HUD model 7/7、原生視窗契約 1/1、Edge HUD/設定幾何驗證及完整 `scripts/verify-local.ps1` 全數通過，`git diff --check` 無錯誤。
- 對話功能可行但尚未實作：現況沒有 History/Session/content 儲存，`/api/v1/history` 仍回空陣列。建議先做 Codex（再 Claude）的 metrics-only Session/turn Token 明細；「我送出的內容」採 desktop-only、逐來源 opt-in、按需讀取、預設遮罩，Copilot/Antigravity 在來源格式實證前顯示不支援。
- 不以單一比率宣稱「浪費」；只呈現可驗證的檢視訊號，例如高 input、低 cache reuse、重複 prompt 指紋與 retry。

# 2026-09-12 Codex Session／Turn Token 分析第一階段

## Goal & acceptance criteria
- [x] 戰情室提供 Codex Token 分析入口，顯示最近 Session 與 turn 的 input、cached input、cache write、output、reasoning、total Token、model、時間與 duration。
- [x] 只讀 Codex 已存在的 `sessions`／`archived_sessions` JSONL；不修改來源、不建立來源目錄內 cursor/lock，不呼叫模型或額外消耗 Token。
- [x] metrics-only：DTO、DOM、log、錯誤與測試輸出不得包含 prompt、response、tool payload、Session 原始 ID、完整來源路徑或 repository 名稱。
- [x] active/archive 同 Session 去重；partial final line 不解析；unknown 與 0 分開；schema drift fail closed 並回報安全的 rejected/limited 狀態。
- [x] 掃描有檔案數、單檔 bytes、總 turns 與回傳 Session 上限；UI 能呈現載入、空資料、部分資料與失敗狀態。
- [x] targeted Rust/Node tests、桌面瀏覽器互動/幾何、完整本機驗證與 diff 檢查通過。

## Confirmed seams
- `CodexHistory::scan(codex_home)`：Provider JSONL Adapter 與 metrics-only History read model 的公開測試 seam；呼叫者不需知道檔案配置、事件版本、去重或 bounds。
- Tauri `history_sessions`／desktop History dialog：桌面 Adapter 與使用者可觀察行為 seam；只接受後端 DTO，不直接解析來源檔。

## Plan
- [x] Checkpoint A: 確認第一階段範圍與兩個測試 seam，讀現有規格、ADR、Tauri/UI 模式及本機去識別 schema shape。
- [x] Checkpoint A: 建立合成 Codex rollout fixture 與第一個失敗的 scan 行為測試。
- [x] Checkpoint B: 實作 `CodexHistory` 深 Module 的 bounded、read-only、metrics-only 掃描與 DTO。
- [x] Checkpoint B: 建立 Tauri command 與戰情室 Token 分析 dialog，涵蓋 loading/empty/partial/error。
- [x] Checkpoint C: 增加 active/archive、partial line、schema drift、privacy negative 與 UI 回歸測試。
- [x] Checkpoint C: 執行 targeted、完整本機、瀏覽器互動/幾何與 diff 驗證。
- [x] Checkpoint D: correctness/security/privacy 複核，記錄結果、限制與第二階段 seam。

## Risk & rollback
- Risk: medium。讀取本機敏感 transcript 檔，但只以 typed deserialization 擷取 Token metadata；正文欄位被忽略且不離開 Module。無 DB migration、無來源寫入。
- Rollback: 移除 Codex History Module、desktop command/入口/dialog/UI model 與對應 fixture/tests；不需清資料，因第一階段不持久化。
- Signals: serialized DTO 與 DOM sentinel 掃描為 0；來源檔案 hash/mtime 不變；bounds/partial/duplicate tests；完整 path 不出現在錯誤。

## Dependencies & environment
- 既有 Rust `serde`/`serde_json`、Tauri 與 vanilla JS；不新增依賴。
- Codex rollout 是本機觀察來源而非穩定公開 API；parser version 固定、未知 shape 拒絕並顯示部分資料提示。

## Working notes
- 現行 rollout 同時可有 `token_usage_record` 與 `event_msg.token_count`；同一檔優先使用含 turn identity 的前者，避免雙重計算。
- `turn_token_usage` 是該 turn 的累積值；同 turn 取最後一筆。`cached_input_tokens` 是 input 的子集合，總量使用來源 `total_tokens`，不可再把 cache/reasoning 重複加總。
- Session/turn 原始 ID 只在 Module 內去重，回傳 UI 使用掃描結果內的 opaque ordinal ref。
- 舊版 `token_count` 以第一筆合法 `last_token_usage` 加上後續 cumulative delta 計算，重複 snapshot 為 0，不重播；modern thread/turn snapshot 優先。
- `.jsonl.zst` 第一階段不新增解壓依賴，會明確標示 incomplete；`.tmp` 永不讀取。
- History command 只允許 `main` webview，並以 single-flight 阻止重複重型掃描；tablet API 未暴露此 DTO。

## Results
- 新增 `CodexHistory::scan` metrics-only 深 Module：掃描 active/archive、同 identity 去重、legacy/modern usage 正規化、partial/schema drift 處理、精確 decimal-string DTO 與檔案/bytes/turn bounds。
- 戰情室新增「Token 分析」對話框，依 Session 總 Token 排序並可展開 turn；顯示 input/cache read/cache write/output/reasoning/total、model、時間與 duration，unknown 保留為 `—`。
- 隱私負向測試涵蓋 prompt、response、tool/path/原始 ID sentinel、惡意 timestamp/model；DOM 只用 `textContent`，來源檔 byte-for-byte 不變，命令不接受前端路徑。
- TDD targeted：Codex History Rust 11/11、desktop command 3/3、History/既有 quota UI 11/11 通過；correctness 與 security/privacy 雙重複核的高/中風險發現已處理。
- 完整 `scripts/verify-local.ps1` 通過全部 Rust/Node tests、format 與 Clippy；Playwright/Chromium 18 個 dashboard viewport 加 History 1080×640、640×520 通過，`git diff --check` 無錯誤。
- 第一階段沒有 SQLite 持久化、內容正文、平板 History、成本估算或其他 Provider Session；也未建立安裝包。壓縮 archive 會顯示資料未完整，而不假裝 complete。

# 2026-09-12 四 Provider 對話分析與內容檢視第二階段

## Goal & acceptance criteria
- [x] 戰情室的「對話分析」以 `codex`、`claude`、`copilot`、`antigravity` 為共用選擇範圍，不再把功能寫死為 Codex。
- [x] 每個 Provider 都回傳一致的 capability/status；找不到來源或 schema 未受支援時明確顯示，不以 quota/剩餘百分比推算 Session Token。
- [x] Codex、Claude、Copilot 對已知本機 JSONL shape 提供 bounded/read-only Session 與使用者 prompt 解析；Antigravity 只解析經證實的 shape，否則 fail closed。
- [x] 第二階段正文只能在桌面主視窗、使用者逐次勾選同意後按需讀取；關閉 dialog 或切換 Provider 立即清除 DOM 與同意狀態。
- [x] 正文不持久化、不進 tablet/diagnostics/export，不回傳 assistant/tool/system 內容、原始 Session ID、專案名稱或完整來源路徑；只用 `textContent` 呈現並做長度/數量限制及基本 redaction。
- [x] targeted Rust/Node tests、完整本機驗證、桌面響應式互動與 diff 檢查通過。

## Confirmed seams
- `ProviderHistory::scan(provider, root)`：四 Provider 共用 metrics-only read model；adapter 封裝檔案布局、schema、bounds 與 capability 狀態。
- `ProviderHistory::read_session_content(provider, root, session_ref, expected_started_at, expected_ended_at)`：只回傳該 Session 的 redacted user prompts；公開 ordinal ref 加雙時間戳唯一性校驗，0 或多筆匹配都 fail closed。
- Tauri `history_sessions(provider)`／`history_session_content(provider, session_ref, expected_started_at, expected_ended_at, consent)`：只允許 main webview，來源 root 一律由 backend 決定。

## Plan
- [x] Checkpoint A: 建立四 Provider capability matrix、修正 scope lesson、確認共用 seam 與官方/本機 schema 證據。
- [x] Checkpoint A: 先增加四 Provider 選擇、truthful status 與未同意不得讀正文的失敗測試。
- [x] Checkpoint B: 實作共用 History dispatcher、backend root resolution 與 Provider adapter 的最小安全 slice。
- [x] Checkpoint B: 實作按次 opt-in prompt view、redaction、bounds 與關閉/切換清除行為。
- [x] Checkpoint C: 增加 malformed/partial/schema drift、stale ref、privacy sentinel、main-window-only 與 UI 回歸測試。
- [x] Checkpoint C: 執行 targeted、完整本機、瀏覽器互動/幾何與 diff 驗證。
- [x] Checkpoint D: correctness/security/privacy 複核並記錄 Results、已驗證能力與環境限制。

## Risk & rollback
- Risk: high。此階段按需讀取本機 prompt，可能包含 secret、程式碼、個資與路徑；採 explicit per-open consent、最小欄位、基本 redaction、bounded response、desktop-only 與 zero persistence 降低風險。
- Rollback: 移除共用 Provider dispatcher、content command/view 與 adapters，保留既有 Codex metrics-only module；沒有 DB migration 或內容資料需要清除。
- Signals: 未同意時 content command 次數為 0；關閉/切換後 DOM sentinel 為 0；序列化結果不含 assistant/tool/system/path/原始 ID；unsupported/unavailable 不產生 Token 數值。

## Dependencies & environment
- 既有 Rust `serde`/`serde_json`、Tauri、vanilla JS 與合成 fixtures；不新增 runtime dependency。
- 本機目前未發現 Claude/Copilot/Antigravity 歷史目錄，所以 adapter 以合成官方 shape 驗證；實機來源缺席必須顯示 `source_unavailable`，不得宣稱已做真人資料驗證。

## Working notes
- Provider root：Codex 使用 `CODEX_HOME`/`%USERPROFILE%\.codex`；Claude 使用 `CLAUDE_CONFIG_DIR`/`%USERPROFILE%\.claude`；Copilot 使用 `%USERPROFILE%\.copilot\session-state`；Antigravity 使用 `%USERPROFILE%\.gemini\antigravity`。
- Copilot billing quota 和 session Token 是不同事實；沒有 persisted usage event 時 Token 欄位保持 unknown。
- Prompt redaction 是風險降低措施，不是秘密偵測保證，UI 必須保留警告。
- `consent:true` 是防止意外讀取的 backend guard，逐次 checkbox 是明確 UX 同意；若 main WebView 本身遭任意程式執行，renderer 可呼叫 Tauri command，因此真正安全邊界仍是 CSP、main-window allowlist、backend root/Session 校驗與不持久化，而不是 checkbox 本身。

## Results
- 戰情室入口改為「對話分析」，新增 Codex／Claude Code／GitHub Copilot／Antigravity 共用 selector、同一份 `agentmeter.provider-history.v1` DTO 與 ready/empty/source unavailable/schema unverified 文案。
- 新增 `ProviderHistory` 深 Module：Codex 沿用第一階段精確 Token；Claude 讀 `projects/**/*.jsonl` 的 external user/assistant usage；Copilot 讀 `session-state/*/events.jsonl` 的 `user.message`，沒有可信 persisted usage 時 Token 保持 unknown；Antigravity 因 transcript event schema 未驗證而 fail closed。
- 第二階段新增 desktop-only `history_session_content`：backend allowlist/root resolution、main-window check、明確 consent、Session ordinal＋start/end timestamp 重掃校驗、200 turns／4,000 chars／50 files／128 MiB bounds；不接受前端 path/raw ID。
- Prompt 只取已辨識的 user-authored text，排除 assistant/tool/system；遮罩絕對路徑、secret assignments、Bearer、CLI/JSON secret、常見 token prefix、URL credentials 與 private-key blocks。任何 Provider 的重複 timestamp 都會回 content unavailable，不猜選來源。
- Correctness/security diff review 發現並修正 Claude cache token 漏算（總 input = uncached + cache read + cache creation）以及 Claude/Copilot 同時間戳重綁風險；同時補上 skipped/rejected/partial 計數，Token 加總 overflow 保留 unknown、不做飽和截斷。
- UI 未同意時正文命令為 0 次；關閉 Prompt/History、切換 Provider、重新整理與晚到 response 均清除或拒絕 render。DOM 只用 `textContent`，無 browser storage、Markdown/HTML renderer、tablet route 或 persistence。
- 驗證：Provider core 11/11、Codex regression 11/11、desktop history command 3/3、History/既有 quota UI 14/14、desktop bundle 11/11 通過；完整 `scripts/verify-local.ps1`（全部 Rust/Node tests、fmt、Clippy）通過；Playwright 18 個 dashboard viewport 與 Prompt/History 1080×640、640×520 通過；`git diff --check` 通過。
- 環境邊界：本機沒有 Claude/Copilot/Antigravity 歷史目錄，因此三者以合成官方 shape/固定 capability 測試驗證，未宣稱真人資料實測；未建立 installer，也未把 Prompt 寫入 DB。

# 2026-09-12 Token 效率圖表與 API 等值成本

## Goal & acceptance criteria
- [x] 戰情室先顯示四個可理解的 KPI：總 Token、API 等值美元、估價覆蓋率、快取重用率；原始六欄 Token 明細降為第二層。
- [x] 以互斥 Token 流向圖呈現未快取輸入、快取輸入、快取寫入與輸出，圖表同時具備文字數值與螢幕閱讀器說明。
- [x] 顯示可驗證的檢查訊號與 Top sessions，不產生武斷的「浪費分數」；高輸入低輸出、低快取等只能標為風險訊號並附數字與行動建議。
- [x] 美元成本依 Provider、實際模型及 input/cache read/cache write/output 分開計算；未知模型、截斷資料、無 Token Provider 顯示 partial/unavailable，不以 quota 或訂閱價格冒充 Token 成本。
- [x] Prompt 逐次同意後，顯示同一 Session 內「完全相同且具足夠長度」的重複 Prompt 次數；不將它誤稱為重複 context，且不保存 prompt/fingerprint。
- [x] targeted Rust/Node tests、完整本機驗證、桌面響應式與 diff 檢查通過，重新啟動桌面程式供實測。

## Confirmed seams
- `TokenAnalysis::analyze(&ProviderHistoryReport)`：唯一負責 Provider Token 語意正規化、成本、覆蓋率與風險規則；前端不得重算費率或 Provider 差異。
- `HistorySessionsView.analysis`：在桌面 command envelope 提供互斥 flow、cost state、signals 與 session analysis 的版本化 read model；未知資料以 `null`／明確 state 表示。
- `ProviderSessionContent.duplicate_prompt_count`：只在使用者逐次同意的按需讀取期間，以正規化後完全相同 Prompt 計數，不回傳或保存 fingerprint。

## Plan
- [x] Checkpoint A: 以官方第一方價格頁固定可支援模型、費率、日期與「API 等值、不等於帳單」限制。
- [x] Checkpoint A: 先新增成本、覆蓋率、Token flow、風險訊號與重複 Prompt 的失敗測試。
- [x] Checkpoint B: 實作 `TokenAnalysis` 深 Module 與內容重複計數，保持 bounded/read-only/zero persistence。
- [x] Checkpoint B: 實作 KPI、Token flow、signals、Top sessions、方法與成本註記的無依賴 UI。
- [x] Checkpoint C: 驗證 complete/partial/unavailable、未知模型、overflow、截斷、Provider 語意與隱私負向案例。
- [x] Checkpoint C: 執行 targeted、完整本機、響應式/可及性與 diff 驗證。
- [x] Checkpoint D: correctness/security/privacy 複核、記錄 Results 並啟動桌面程式。

## Risk & rollback
- Risk: medium。價格會變動且 API 等值不一定等於訂閱實際帳單；啟發式訊號也可能把合理的長 context 標成值得檢查。
- Rollback: 移除 `TokenAnalysis`、分析 DTO/UI 與重複 Prompt 計數欄位即可；無資料庫 migration、無來源寫入或 prompt 持久化。
- Signals: 費率來源/日期/覆蓋率可見；未知模型不計價；UI 不出現 raw prompt（除既有逐次同意視窗）、raw ID/path；完整測試與響應式矩陣通過。

## Dependencies & environment
- 只使用既有 Rust、serde、vanilla JS/CSS 與官方定價資料；不新增 runtime dependency。
- UI/UX 技能的本機 `search.py` 路徑指標失效，依其文件中的 dashboard、chart、a11y 原則手動落實並以現有瀏覽器驗證補強。

## Working notes
- 成本一律標為「目前公開 API 單價等值估算」，不是 ChatGPT/Claude/Copilot 訂閱信用卡帳單，也不回推歷史日期當時價格。
- Codex cached input 是 input 的子集合；Claude cache read/cache creation 是獨立輸入類別。互斥 flow 必須由 backend 正規化。
- 重複 Prompt 是 context 浪費的窄證據；低快取與高輸入低輸出只是檢查線索，不足以直接判定浪費。

## Results
- 新增純函式 `TokenAnalysis`：依 Codex/Claude 不同 cache 語意正規化互斥 flow，reasoning 不重複計價；以固定小數 `u128` 計算 current API equivalent，未知模型、長 context modifier、缺欄位與不一致 totals 均 fail closed。
- OpenAI exact model rate 與 Claude exact model/alias rate 來自官方頁；Claude cache creation TTL 未知時回傳 5 分鐘至 1 小時成本範圍。Copilot 缺 persisted Token/model、Antigravity schema 未驗證，因此明確顯示無法估價。
- 戰情室新增 4 KPI、Token flow 堆疊圖＋文字 legend/ARIA、保守效率訊號、Top 5 sessions、費率日期/帳單限制；完整原始 Token 收進第二層 details。
- 逐次同意的 Prompt 視窗新增 exact nontrivial duplicate 計數；redacted、truncated、過短內容不參與，不保存文字指紋。
- 最終複核發現並修正兩個 fail-closed 邊界：cache `null` 不得視為 0；priced turns 大於 observed totals 時不得顯示金額或 coverage。新增對應負向測試。
- 驗證：TokenAnalysis 6/6、ProviderHistory 12/12、History UI 7/7；`scripts/verify-local.ps1` 全部 Rust/Node tests、fmt、Clippy 通過；Edge/Playwright 18 組 dashboard viewport 與 History 1080×640、640×520 通過；`git diff --check` 通過。
- 桌面 debug 版已重新啟動供實測；無 DB migration、來源寫入、installer rebuild 或訂閱帳單存取。

# 2026-09-13 流量浮動視窗多螢幕與拖曳

## Goal & acceptance criteria
- 設定可列出已連接螢幕並切換浮動視窗；滑鼠可直接拖曳到任意位置。
- 記住使用者位置，更新數據、透明度、監控數量不重設位置；重啟恢復位置。
- 支援負座標及不同 DPI，螢幕拔除後回到可见工作區；提供重設位置。

## Plan
- [x] A: 找出 HUD 前後端與既有測試，確認目前 CSS screen 座標與滑鼠穿透限制。
- [x] B: 後端統一管理實體螢幕座標與保存；前端加入螢幕選項、重設及拖曳。
- [x] C: 幾何邊界與 UI 行為測試、Rust/Node 驗證及瀏覽器版面檢查。
- [x] D: 差異複核並記錄驗證及實機限制。

## Risk & rollback
- Medium：混合 DPI、螢幕拔除與移動事件；所有定位只作用 HUD。
- 回復本次 HUD 變更即可回原功能；新增的 hud-position.json 可忽略，無 DB migration。

## Dependencies & environment
- 既有 Tauri 2、Rust、vanilla JS/CSS；不新增 dependency。
- 工作區已有多項未提交修改，保留既有成果，只改本功能必要範圍。

## Working notes
- 舊版 dashboard 和 HUD 都會將視窗推回各自 screen 右下角；後端應成為唯一定位來源。
- 支援直接拖曳需要關閉原本滑鼠穿透，設定文案同步調整。

- HUD verification checkpoint: Node 18/18 and Edge HUD/settings browser checks passed; 18 dashboard viewport combinations and history views passed. First full verifier stopped at formatting while the backend agent was finishing its last edit; rerun only after that handoff is complete.

## Results
- 新增 `desktop-p0/src/hud.rs` 統一管理實體座標、工作區、螢幕選擇、跨 DPI 大小及 `hud-position.json`；main 只註冊命令與轉送移動事件。
- 設定加入螢幕選單、重新整理與重設位置；面板可原生拖曳、保存位置，透明度與數據更新不固定回右下角。
- 位置操作互斥；啟動事件不覆蓋保存位置；300 ms debounce 後讀取當前位置；拖曳期間依 Windows 主要按鍵狀態暫停重新定位（包含左右鍵交換設定）。
- 幾何更新採先定位後設定目標 DPI 尺寸，避免原生 DPI 移動再次縮放；螢幕拔除／位置超出工作區時回到可見範圍。
- 保存錯誤不阻斷 snapshot；位置設定與重試狀態可見。安全／正確性複核無剩餘 P1/P2 問題。
- 驗證：`scripts/verify-local.ps1` 全部 Rust/Node tests、fmt、Clippy 通過（`.scratch/hud-verify-local.log`）。最後調整 DPI 呼叫順序後再次通過 desktop fmt、HUD 6/6 tests、all-targets Clippy；前端相關 18/18、Edge HUD/settings（含 7 螢幕及保存失敗）與 18 組 dashboard viewport 通過，`git diff --check` 通過。
- 建置：`cargo rustc --manifest-path desktop-p0/Cargo.toml --offline --locked --bin agentmeter-desktop-p0 -- -o .scratch\agentmeter-hud-preview.exe` 成功。`-o` 產生兩條預期輸出路徑 warning；程式碼 warnings-denied Clippy 已通過。
- 實機限制：舊開發版 PID 62412 不回應正常 `--request-exit`，沒有強制關閉；只清理由本次建立的退出 helper PID 78668。新版尚未取代執行中的舊版，也未建立 installer。使用者結束舊版後可啟動 `.scratch/agentmeter-hud-preview.exe`。
- 尚未以實體多螢幕验证混合 DPI 拖曳、拔除與重啟位置；這些情境目前由生產共用幾何／選擇函式測試覆蓋。手動流程：設定選第二螢幕 → 拖曳跨螢幕超過 2 秒 → 調透明度／監控數量確認不跳回 → 重啟確認位置 → 拔除該螢幕確認面板仍可見。

# 2026-09-13 發布 0.2.6 多螢幕浮窗
- [x] 確認公開最新版 0.2.5，隔離分支僅移植 HUD 功能。
- [ ] 完整測試與 UI 驗證。
- [ ] 使用既有 DPAPI 金鑰建置簽章包，驗證簽章與 manifest。
- [ ] 提交並推送版本碼；上傳 draft 全部資產後發布 latest。
- [ ] 公開下載校驗與記錄結果。
- 風險：混合 DPI 尚無實體多螢幕驗收。回復：latest 回指 v0.2.5，停用新 manifest；已升級使用者採修正版，不自動降版。

## 2026-09-13 0.2.6 發布結果
- [x] 在 `.release-worktrees/0.2.6` 從遠端 0.2.5 基線隔離 HUD 變更，保留 0.2.5 hover，未混入對話分析。
- [x] 完整 Rust/Node、fmt、Clippy、Edge HUD、NSIS 版本/內容及簽章驗證通過。
- [x] 推送程式碼 `6e08cf2da5299fa03b215f7e82e5e809948926d6`，上傳全部資產並設 `v0.2.6` 為 latest。
- [x] 公開 latest 回傳 0.2.6；下載的 73,634,971-byte 安裝包 SHA-256 為 `025C0F0587706C94238636A0974980828271464C235DA5E4A8134BEFD402173E`，更新簽章／trusted comment／竄改拒絕再次通過。
- 發布：https://github.com/richshangwei/AgentMeter/releases/tag/v0.2.6 。沿用既有金鑰，無私鑰外傳、無強制安裝或重啟。實體混合 DPI 驗收仍待確認。
- 主工作區保持原狀，未覆蓋未提交的其他功能；release 分支上的最新記錄已推送遠端 master。

# 2026-09-14 目前工作區打包安裝檔
- 驗收：依目前工作區 0.2.4 原始碼產生 Windows x64 NSIS，檢查內嵌版本、必要資源與 SHA-256；提供檔案位置。
- 範圍：包含目前未提交功能，保留版本號；不發布、不安裝。
- [x] 確認版本與既有建置／檢查流程。
- [x] 建置目前工作區 NSIS。
- [x] 執行既有驗證與安裝包檢查。
- [x] 記錄結果並交付。
- 環境：既有 Rust/Tauri 2、NSIS、quota-helper；低風險，本次只產生本機建置輸出與工作記錄。

## Results
- `cargo tauri build --ci --target x86_64-pc-windows-msvc --bundles nsis -- --offline --locked` 成功（desktop-p0 工作目錄）。
- 安裝檔：`desktop-p0/target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_0.2.4_x64-setup.exe`；73,822,715 bytes。
- SHA-256：`6C32F470D20C460AC1C51937FDD0C8D1DA849E6AB5B1FF800EC91ADCAD3CBE58`，旁附 `.sha256`。
- 完整 verify-local 通過：`.scratch/package-current-verify.log`；inspect-nsis 通過：`.scratch/package-current-inspection.json`；node/agy/quota helper 資源存在，helper 腳本與來源 hash 一致。
- 本機未簽章安裝包，保留工作區 0.2.4；若已安裝 0.2.6，防降版設定可能阻止安裝。未發布、未執行安裝；乾淨 VM、實體裝置與真實帳號驗收不在本次驗證範圍。

# 2026-11-06 邏輯規則重新盤點與設計 skill 化
- 目標：重新盤點全專案邏輯規則，將設計整理成可分享的完整 skill（`skills/agentmeter-design/`）。
- 驗收：規則以程式與測試為準並標出與規格的偏離；附可執行的快照檢查器，含負向測試；SKILL.md 可路由到 9 份參考文件。
- 風險：低（僅新增文件與無依賴腳本，未改產品程式）。回復：刪除 `skills/agentmeter-design/` 與本段落。
- [x] 盤點來源：CONTEXT、需求書 v1.1、Token 分析規格、ADR 0001–0011、lessons、design-qa、`src/`、`desktop-p0/`、`scripts/`、`tablet-ui/`。
- [x] 撰寫 SKILL.md、`references/01–09`、`scripts/check-snapshot.mjs` 及測試、`assets/dashboard-snapshot.example.json`。
- [x] 驗證：`node --test skills/agentmeter-design/scripts/check-snapshot.test.mjs` → 13/13 通過；範例快照 `check-snapshot.mjs` 退出碼 0。
- [x] 對照程式抽查並修正文件：HUD 現為可拖曳／接收滑鼠（舊教訓「原生穿透」已過時）。

## Results / 發現
- 規格與實作偏離（已寫入 `references/09`）：無 SQLite／migration；無任何 Collector 標 `stable`；Claude／Antigravity／Copilot 預設走 `/usage`／`getQuota` 而非 statusLine／Billing；`/api/v1/history` 為佔位；HTTP 為 `std::net` 而非 Axum。
- 後續（未處理，避免擴大範圍）：`tasks/lessons.md` 2026-09-12「HUD 必須原生穿透」已被 09-13 可拖曳設計取代，建議補註「已被取代」；是否重新引入系統匣被遮擋問題尚無實機證據。
- 未執行：skill 觸發率評測（description optimization）與子代理 A/B 評測；真實帳號、乾淨 VM、實體平板驗證不在範圍。

# 2026-11-16 本機硬體使用量

## 驗收與計畫
- [x] 顯示本機 CPU 使用率、實體記憶體及 Windows 系統磁碟已用／總量，讀取失敗時明確顯示未知。
- [x] 僅開啟本機資源視窗時定期更新，維持既有額度版面；不新增依賴。
- [x] 執行 Rust 測試與前端檢查，記錄結果。

## 工作備註
- 範圍為 Windows 桌面版；磁碟使用量為系統磁碟，而非全部磁碟合計。

## 結果
- Windows 原生 API 取得 CPU、實體記憶體及系統磁碟使用量；本機資源視窗開啟時每 2 秒更新。
- 使用工作區 Cargo 快取執行 CPU 測試通過；既有桌面前端測試 18 項通過，JavaScript 語法檢查通過。未進行實際桌面視窗互動驗證。

# 2026-11-16 主畫面顯示本機資源

- [x] 將 CPU、記憶體與系統磁碟資料放在主畫面，預設顯示並提供可保存的設定開關。
- [x] 資訊列顯示時定期更新；隱藏時停止更新，保留錯誤狀態。
- [x] 驗證設定、更新流程與既有桌面測試，編譯 Release。

工作備註：現有已安裝版本與工作區建置版是不同執行檔；編譯不會自動取代安裝版。

結果：資源列預設顯示，設定關閉會保存偏好並停止輪詢；19 項前端測試與 release 編譯通過。未進行實際視窗幾何驗證，因本環境無可用 Playwright。


## 2026-09-13 Release 0.2.6 — multi-monitor HUD
- [x] Base on published 0.2.5 plus release evidence; isolate HUD-only scope from unfinished analytics.
- [x] Port backend physical positioning, persistence, native drag and settings; preserve existing hover opacity.
- [x] Node 18 tests and Edge HUD checks including seven monitor choices, drag/reset and persistence failure.
- [x] Full local checks; signed installer, signature/tamper verification and NSIS inspection.
- [x] Commit/push release source; upload all three draft assets before publishing latest.
- [x] Verify public latest manifest and downloaded artifact digest.
- Risk: medium, native multi-monitor mixed-DPI behavior still requires physical acceptance. Rollback: set latest back to v0.2.5 and withdraw new manifest; no automatic downgrade.
- Environment: cached pinned dependencies, existing DPAPI signing key, fixed GitHub updater endpoint; no key rotation or credential changes.
- Release source: `6e08cf2da5299fa03b215f7e82e5e809948926d6`; public tag `v0.2.6`, published 2026-09-13T15:42:20Z.
- Installer: `AgentMeter-P0_0.2.6_x64-setup.exe`, 73,634,971 bytes; SHA-256 `025C0F0587706C94238636A0974980828271464C235DA5E4A8134BEFD402173E`.
- Existing public key matched 0.2.5; local signature, trusted comment and tamper rejection passed. All three GitHub uploaded asset digests and sizes matched before publishing.
- Full `scripts/verify-local.ps1` and Edge HUD checks passed. NSIS embedded executable product/file version is 0.2.6; no Windows Authenticode signature, unchanged from previous release. Physical mixed-DPI monitor interaction remains unverified; no local installer was run.
- Public `releases/latest/download/latest.json` returned 0.2.6; the publicly downloaded installer matched the local SHA-256 and passed signature/trusted-comment/tamper-rejection verification.

## 2026-11-21 主題配色、放大本機資源、列出所有磁碟
- [x] 設定新增「外觀主題」：深色科技（預設）/ 明亮清爽 / AI 極光；localStorage `agentmeter.theme.v1`，`ui/theme.js` 於首次繪製前套用。
- [x] 本機資源改為大字百分比 + 容量明細 + 使用率條（≥80% 警告、≥90% 危險色）。
- [x] 後端 `local_resources` 新增 `disks`（所有 DRIVE_FIXED 磁碟）；保留 `disk`/`disk_path` 相容舊前端，前端也相容舊後端。
- [x] Verify: 以 scoop rustup (1.100.0) + VS 18 Build Tools C++ 工作負載：desktop `cargo fmt --check` 通過、`cargo test` 39/39、Clippy `-D warnings` 通過、`cargo build --release` 成功；啟動 target/release 版實機截圖確認 7 顆磁碟卡片與明亮主題。
- Results: `node --test` 桌面 UI 測試 58/60 通過（2 項 background_window_contract 為既有失敗，缺未產生的 quota-helper 檔案，stash 基準相同）；Edge headless 以 mock 截圖確認三種主題與設定對話框。HUD 浮動視窗未套用主題。
