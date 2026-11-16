# 08 驗證策略、驗收條件與教訓

> 目錄：驗證分級 → 教訓與 tripwire → 驗收條件濃縮 → 完成定義 → 回報規則

## 1. 驗證分級

| 層級 | 內容 | 本專案指令 |
|---|---|---|
| Tier 1 | 針對性單元測試＋lint/型別（warnings-denied Clippy） | `cargo test --offline --locked`、`node --test`、`cargo clippy -- -D warnings` |
| Tier 2 | 整合測試＋本機重現腳本（真實子程序、真實 HTTP） | `./scripts/verify-local.ps1`（含獨立的 desktop crate 與瀏覽器邏輯測試） |
| Tier 3 | 真實帳號、實體平板、乾淨 VM、簽章更新安裝 | 另列的發布閘門，**不被 Tier 1/2 取代** |

三條通則：
- **先寫會失敗的測試**（能重現 bug 的最小測試），再修。
- **每個修正都配一個 tripwire**（grep 規則／測試／不變量斷言），防止同類錯誤回來。
- **無法驗證就明說**：寫出原因與之後要跑的確切指令，不得靜默略過。

## 2. 教訓 → 防再發規則

| 日期 | 失敗模式 | 防再發 |
|---|---|---|
| 09-17 | 布林環境變數用了 `1`，文件要求 `true`；背景 CMD 仍會閃 | 環境值從官方現行文件逐字複製；測試同時要求 `true` 並**拒絕** `1`；視覺觀察腳本在值不是 `true` 時必須能抓到閃現 |
| 09-14 | 以 desktop 為工作目錄跑相對路徑檢查而找不到檔案 | 批次執行相對路徑前先確認工作目錄 |
| 09-13 | HUD 週期夾限打斷跨螢幕拖曳；排隊的座標存到較新的螢幕；等待位置存檔導致鎖檔時 snapshot 被擋 | 序列化原生放置、拖曳期間延後幾何更新、去抖後再讀座標；設定錯誤與資料路徑分開處理 |
| 09-13 | 全域取代版本號動到 Cargo.lock 其他依賴；短 SHA 被發布 API 拒絕 | 只改應用程式 package 區段並看 lockfile diff；用完整 `git rev-parse HEAD` |
| 09-12 | 被動 HUD 為了 hover 取消原生穿透，蓋住系統匣 | 當時規則：「不可聚焦＋原生穿透」。**09-13 起已被可拖曳 HUD 取代**（現行：不可聚焦、接收滑鼠、僅拖曳、無 `main:hover`；見 03 §4）。仍有效的是：不可聚焦、不加 hover、互動必須是刻意模式 |
| 09-12 | HUD 用 `quota_windows[0]`，重排後顯示備用額度；容器透明連文字一起淡化 | 以 `limit_id` + 權威 duration 選；透明只套背景；用對抗性排序測試 |
| 09-12 | 共享稽核檔被夾帶別的任務內容提交 | 提交前比對 staged 檔尾與 name-status；只暫存本任務 hunk |
| 09-12 | Token 分析硬編 Codex | 先以 Provider 能力矩陣定義功能；入口涵蓋四家；拒絕把配額% 當 Token 數 |
| 09-12 | `cached_input_tokens: null` 變 0；已定價 > 已觀察仍標 100% | 保留 `None`；僅精確相等為 `complete`；矛盾聚合隱藏成本 |
| 09-11 | 發布檔名含空白，GitHub 正規化為句點 → 404 | ASCII 安全檔名；先發 prerelease 比對實際資產 URL |
| 09-11 | `primary`/`secondary` 與內部 ID 被當產品文案 | 端到端保留 `limitId/limitName/planType/windowDurationMins`；以 duration 推導標籤；測試拒絕這些詞出現在產品文案 |
| 09-11 | 簽章啟動：stderr 警告被 `Stop` 當失敗；改既有目錄 ACL 需無法取得的特權 | 隱藏 ProcessStartInfo＋判 exit code；記憶體產生金鑰、僅存 DPAPI 密文 |
| 09-11 | 只量外框，漏掉同格兄弟元素相交 | 驗證兄弟矩形不相交，涵蓋每個張數／視窗大小 |
| 09-11 | 偽元素繪製被裁切；數字比環內徑大；兩個窗口的 fixture 漏掉真實基數 | 驗證計算後的繪製矩形；以 `100%` 測數字；用真實基數；詳見 07 |
| 09-11 | 巢狀捲動被使用者否決 | 桌面監控必須 fit viewport；放不下就分頁 |
| 09-11 | 隱藏列改變 grid 隱式放置 | 明確 `grid-row`，並驗證 resize 序列 |
| 09-11 | 第三方 CLI 自己開 console | 對每個官方 CLI 關閉自更新／常駐；`strings` 檢視二進位；重現要跨更新 TTL |
| 09-11 | 觀察者沒有正向對照 | 先證明觀察者看得到可見 console，再解讀零結果 |
| 09-10 | 更新器在套件解壓前就停掉不可逆狀態 | 掛在確認的 pre-exit hook；所有失敗保持監控並可重試 |
| 09-10 | Tauri plugin 設定被反序列化成 `null`，裝好的 exe 無法啟動 | 封裝啟動設定契約＋真實 release 啟動探測 |
| 09-10 | 平板只渲染 `quota_windows`，隱藏有效的 `source_usage` | 每個「有資料」判斷式都要有對應渲染分支與 fixture |
| 09-10 | 立即寫入＋shutdown 的測試掩蓋實機 USB 失敗 | 測延遲、分段、不 shutdown 的請求與實際 forwarded Host/Origin |
| 09-10 | 一般「需要注意」狀態沒告訴新使用者怎麼辦 | 缺 CLI/未登入訊息要寫出官方工具名、登入動作、重新整理動作 |
| 09-10 | 輕量 mock 元素缺 `className` 預設 | 測試 harness 要模擬必要的 DOM 預設，馬上跑針對性測試 |
| 09-10 | 沿用的修正違反 clippy `collapsible_if` | 在描述「已驗證」前先跑 warnings-denied clippy |

## 3. 驗收條件（濃縮；完整 30 條見需求書 §13）

- **資料語意**：AC-03 缺窗口顯示未知不顯示 100%；AC-04 Token 與 quota 範圍不同不誤加總；AC-05 日誌重讀與重啟只計一次；AC-06 到期但查詢失敗顯示待確認；AC-07 429／逾時／schema 變更正確分類、退避、保留帶時間的舊快照；AC-21 正交狀態可同時表示（rate-limited＋stale 等）。
- **帳號**：AC-18 切換帳號時額度與 History 隔離；無穩定 identity 不跨 Source 合併；Link 可建立與撤銷。
- **生命週期**：AC-08 關閉視窗後 process、Collector、tray、server 都存活，各種啟動情境都只喚醒同一實例，tray Exit 才完整 shutdown；AC-16 migration 前備份、失敗時 Collector 與 DB 寫入不啟動、卸載預設保留資料、既有 statusLine 不被破壞；AC-17 睡眠／喚醒／斷網顯示停更並恢復，不製造錯誤用量。
- **通道**：AC-09 未授權 USB 給指引不假成功；AC-10 多台 Android（含 emulator／TCP）只操作選定的 USB transport；AC-11 `--no-rebind` 拒絕未知 mapping race，只移除 owned mapping；AC-13 通道在線但來源過期須同時顯示；AC-22 同 stream revision 單調、heartbeat 不遞增、漏事件以完整快照收斂；AC-23 refresh 202、合併重複、Paused 不暗中恢復。
- **安全**：AC-15 / 24 / 25（見 05 §9）；AC-27 statusLine 共存；AC-30 診斷匯出先預覽 manifest、已遮罩。
- **發布**：AC-28 WebView2 缺 runtime 時 bootstrapper 成功；AC-29 簽章、SHA-256、SBOM、授權清單。
- **效能（工程目標，需量測）**：UI 回饋 300 ms；啟動 5 秒內出現 tray；資料 2 秒內到平板（20 次 ≥ 95%）；閒置 5 分鐘 CPU < 2%；App＋WebView 工作集 < 250 MB（分列各程序）；8 小時無崩潰且記憶體不持續增長；拔線 45 秒內顯示中斷；USB 可用後 15 秒內復原 mapping。

## 4. 完成定義與回報

一項工作要「完成」須同時有：行為符合驗收條件、測試／lint／型別／建置通過（或有文件化的未執行理由）、高風險變更有回滾策略、遵循既有慣例、以及一段短的驗證敘事——**改了什麼＋我們怎麼知道它有效**。

回報時：
- 先說結果與影響，再列證據（指令 → 預期 → 實際）；不堆整段 log。
- 區分「已驗證」「未驗證（原因＋指令）」「已知限制」。
- 寫新的 dated handoff：改了什麼、怎麼驗證、剩下什麼閘門（本專案放在 `docs/handoffs/`，證據放 `docs/evidence/` 且須去識別）。
- 每次被使用者糾正或發現自己犯錯：把「失敗模式、偵測訊號、預防規則、tripwire」寫進 `tasks/lessons.md`。
