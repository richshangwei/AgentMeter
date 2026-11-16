# 02 四家 Provider 收集規則

> 目錄：共通硬規則 → Codex → Claude → Copilot → Antigravity → 排程與刷新 → 失敗時的呈現

**先讀這個前提：** 以下每一條都是在真實帳號上驗證過、或因事故而加上的。Provider 的 CLI 與輸出格式會變，所以每一條都附「為什麼」。實作新監控器時，請把這些當成「已知會出事的地方」的清單，而不是需要照抄的程式碼。

## 0. 共通硬規則

| 規則 | 為什麼 |
|---|---|
| 只做**唯讀**查詢：不發起模型請求、不消耗推論用量來探測、不呼叫 Billing 端點（除非明確設定） | 探測不能改變被量測的東西 |
| 使用 Provider 自己的登入流程；**不複製、不讀取、不保存憑證**。需要 token 時只在記憶體中交給官方 runtime（環境變數），不上 argv、不落檔、不進 log | 憑證外洩面最小化 |
| 子程序用**明確 argv 陣列**啟動，不拼接 shell 字串；Provider 識別碼只接受固定白名單 | 防注入；UI 永遠不能傳入任意命令 |
| Windows 子程序一律 `CREATE_NO_WINDOW`（Node 用 `windowsHide:true`） | 否則背景收集會閃出 CMD |
| 對每個官方 CLI 關閉**自動更新與常駐行為**，並使用文件記載的**精確字面值**（Antigravity：`AGY_CLI_DISABLE_AUTO_UPDATE=true`，不是 `1`；Claude：`DISABLE_AUTOUPDATER=1`；Copilot：`--no-auto-update`） | 第三方 CLI 會自己開出我們從沒 spawn 過的視窗（事故：2026-09-11、2026-09-17） |
| Windows 上用 **Job Object（KILL_ON_JOB_CLOSE）** 綁住收集器程序樹；逾時、取消、父程序結束都會連同子孫一起回收 | 否則 Node/ConPTY 孫程序會殘留 |
| 輸出有**硬上限**（stdout 256 KB～262144 bytes；超過即 `collector_output_too_large`）；stderr 不保存 | 防記憶體炸裂與機密落地 |
| 每次外部查詢有**逾時**：協定呼叫 15～20 秒；整個收集器 150 秒；Copilot Billing 單請求 1～60 秒 | 單一 Source 卡住不能拖垮整體 |
| 支援**取消**（協作式 `AtomicBool`）：回應等待每 50 ms 檢查；取消後不得再 spawn | 離開 App 時要能在有界時間內完整收尾 |
| Dashboard 只接受**白名單欄位**，絕不轉存整份 raw collector 回應 | 防機密與路徑洩漏（測試用 `PRIVATE` 哨兵字串驗證不外漏） |
| 發布的視圖內容未變 → **不遞增 revision** | 避免無意義的重繪與 SSE 噪音 |

## 1. Codex（Rust；官方 App Server）

**路徑：** `codex app-server --stdio`（JSON-RPC over stdio，換行分隔）。成熟度 `experimental`，live 時 `data_quality = official`。

**握手順序（id 配對，通知分流）：**
1. `initialize`（id 1，`clientInfo`、`capabilities.experimentalApi=false`）→ 失敗則停。
2. 通知 `initialized`。
3. `account/read`（id 2，`refreshToken:false`）→ 失敗則停。
4. `account/rateLimits/read`（id 3）。
5. `account/usage/read`（id 4，**選用**）：只記錄「是否支援」；逾時或不支援不使收集失敗。
6. 關閉 stdin → kill → wait。

**重試：** 只在 `process_exited` 時重試**一次**，其他錯誤不重試（避免無限重試）。

**正規化規則（`append_window`）：**
- 窗口來自 `rateLimitsByLimitId`（優先，非空時）或 `rateLimits`；每個 snapshot 取 `primary`、`secondary`，值為 null 則略過。
- `usedPercent` 與 `remainingPercent` 同時存在但 `remaining ≠ 100 − used` → **`schema_changed`**（兩者矛盾即失敗，不挑一個）。
- 只有 `used` → remaining = `max(0, 100 − used)`；只有 `remaining`（須在 0..100）→ used = `100 − remaining`；兩者皆無 → 保留未知。
- **Over Limit**：`used > 100` → `over_limit=true`；`display_remaining` 夾到 0，`raw_remaining_percent` 保留原值（可為負）。
- **保留元資料並一路傳到 UI**：`limitId`、`limitName`、`planType`、`windowDurationMins`、`resetsAt`。使用者看到的「5 小時」「每週」必須由 `windowDurationMins` 推導（300 → 5 小時、10080 → 每週），**不得顯示 `primary`/`secondary`** 這種傳輸欄位名，也不假定每個帳號一定同時有 5 小時與每週窗口。
- `credits`：`hasCredits`、`unlimited` 必須是 bool（缺少 → `schema_changed`）；`balance` 可為 null。
- `account.type` 缺失 → `schema_changed`。電子郵件遮罩為 `x***@domain`。
- 區分 ChatGPT 訂閱、API key 等認證模式；「沒有訂閱 quota」不代表 API 額度無限。

## 2. Claude（兩條路徑）

### 2a. statusLine 事件（Rust；規格中的優先來源）

事件驅動，**不是**帳號輪詢 API：
- 只有 Pro/Max 且 session 首次回應後才可能出現 `rate_limits`，且窗口可各自缺失。
- 只認 `five_hour`、`seven_day`（規格要求依回傳內容建模、不硬編；現行程式實作為此二窗口）。
- 驗證：`used_percentage` 必須是 0..100 的數字；`resets_at` 必須為 unsigned int（unix 秒）或 null；任何欄位型別錯誤 → `schema_changed`（整個事件失敗，不留部分結果）。
- 非 JSON → `malformed_payload`；根不是物件 → `schema_changed`；合法但沒有 `rate_limits` → `missing_quota`（不是錯誤，是「還沒有資料」）。
- 觀察時間 = 接收時間，`source_timestamp = null`。
- 「定時重跑顯示腳本」不等於取得新的官方額度。

### 2b. 設定接收器（statusLine 整合）

修改使用者 `settings.json` 屬高風險，必須：
1. **預覽差異 → 取得明確同意 → 備份 → 套用**；使用 wrapper 保留既有 statusLine 輸出。
2. 設定檔 ≤ 1 MiB、必須是 JSON 物件；解析失敗就**不改任何東西**。
3. 以 `create_new` 寫備份（不覆寫既有檔），暫存檔 + 原子 rename 替換；套用前在備份後**再檢查一次**檔案未被改動。
4. 備份 manifest（schema `agentmeter.claude-setup/v1`）記錄原狀態、安裝的指令、原 shell（`bash`/`powershell` 且路徑為絕對）；驗證不完整就拒絕還原。
5. 卸載只移除 AgentMeter 自己加入且**未被使用者改動**的部分。
6. 接收器指令用 PowerShell `-EncodedCommand`（UTF-16LE base64），路徑文字不經外層 shell 解讀。
7. 接收器只挑必要統計欄位，原子寫入本機交接檔；不保存提示詞或回應正文。
8. 無法可靠串接 → 維持原設定，顯示 `setup_required`。

### 2c. `/usage` 終端解析（Node；目前預設即時路徑，`experimental`）

為什麼存在：statusLine 需要使用者先改設定且等待事件；`/usage` 可在不改設定下取得 5 小時與每週視窗。
- 以 `node-pty`（ConPTY）＋`@xterm/headless` 啟動 `claude --safe-mode --permission-mode dontAsk --tools ''`（**不啟用任何模型工具**），看到 `Safe mode:` 後送 `/usage`。
- 解析標題 `Current session`（→ `five_hour`）與 `Current week (all models)`（→ `seven_day`）：下一行須符合 `NN% used`，再下一行 `Resets …`。任一窗口缺或 `used > 100` → **整體回空**（all-or-nothing，不回半套）。
- 等兩個窗口都出現後再 **settle 2.5 秒**，因為非同步取得的帳號資料會取代初始 UI 值，再重新解析；仍不滿兩窗口 → `terminal_schema_changed`。
- 25 秒總逾時 → `terminal_quota_timeout`。
- **工作區信任**：只有出現「Yes, I trust this folder」且使用者已在設定按下「啟用 Claude 讀取」才送出確認；否則回 `workspace_trust_required`（→ `setup_required`）。信任範圍**僅限 AgentMeter 專屬 workspace**。
- 重置時間是**顯示文字**（如 `3am`），不是絕對時間戳；限制必須在 UI 揭露（版本敏感的英文終端 parser）。
- 先檢查 `claude auth status`（`loggedIn`），未登入 → `authentication_required`。**絕不以 auth 成功或 session token 用量取代帳號額度。**

## 3. Copilot

### 3a. 即時額度（目前預設；`experimental`）

官方 Copilot CLI：`--headless --stdio --no-auto-update --log-level none`，**Content-Length 框架化** JSON-RPC。流程：`ping` → `auth.getStatus`（`isAuthenticated` 必須為 true）→ `account.getQuota`。
- 窗口採用條件：`entitlementRequests > 0` 且 `remainingPercentage` 是 0..100 的有限數字。
- **重置時間不可信**：`resetDate` 只存為 `reported_reset_at`、`reset_time_verified:false`；進入 Dashboard 時 `resets_at` 一律為 `null`。
- Copilot 的「Premium interactions」**不是 AI credits**，不得混算。
- 認證 fallback：Copilot CLI 自身未登入 → 讀取 `gh auth token`（記憶體內，10 秒逾時），以 `--auth-token-env AGENTMETER_QUOTA_TOKEN` 交給官方 runtime；不看憑證檔、不印 token、不上 argv。標記 `auth_source`（`copilot_cli` / `active_github_cli_account`）。

### 3b. Billing API 收集器（Rust；舊制／回歸用）

回傳的是**用量**，不是剩餘額度：GitHub Billing REST 不含配額或重置資料，**只有消費值時不得自行推論剩餘額度**。
- 呼叫：`gh api --hostname github.com --method GET -H "Accept: application/vnd.github+json" -H "X-GitHub-Api-Version: 2026-03-10" <endpoint>`。
- 帳務範圍：`personal`（`users`，權限 `Plan:read`）、`business`/`organization`（`organizations`，`Administration:read`）、`enterprise`（`enterprises`，`Enterprise billing:read`）。無權限 → `permission_denied`，不推論可讀取組織 pool。
- 計量：`ai-credits`（單位 `credits`/`ai-credits`）與 `premium-requests`（單位 `requests`）**分開存放**，單位不符即拒絕。1 AI Credit = USD 0.01。
- 輸入驗證在邊界完成：`gh` 路徑須絕對；account slug 為 1–100 字元、僅 ASCII 字母數字與內部連字號；year 2000–9999；month 1–12；timeout 1–60 秒。

## 4. Antigravity（`experimental`）

- 指令：`agy --print /usage --print-timeout 20s`（環境 `AGY_CLI_DISABLE_AUTO_UPDATE=true`，進程逾時 25 秒）。
- 輸出是 **4 欄 TSV**：`bucket<TAB>window<TAB>percent<TAB>reset`。嚴格驗證：
  - bucket ∈ {`Gemini Models`, `Claude and GPT models`}
  - window ∈ {`Weekly Limit Remaining`, `Five Hour Limit Remaining`}
  - percent 符合 `^\d+(\.\d+)?%$` 且 0..100；reset 可被 `Date.parse`。
  - 任何一列不符 → **整份回空**（→ `quota_schema_unrecognized`）。
- 各模型窗口分列；無法確認是否共享額度時，不做模型間合計。
- 結構化來源（statusLine JSON `quota` map，含 `remaining_fraction`、`reset_time`、`reset_in_seconds`）是規格中的優先來源；文字解析只能是**鎖定版本**的 fallback。
- 找不到 `agy` CLI（桌面 IDE ≠ CLI）→ `official_cli_not_found_desktop_is_not_cli`。不繞過驗證、不掃描其他程序記憶體、不偽造請求。

## 5. 排程與刷新

| 項目 | 規則 |
|---|---|
| 自動同步 | 設定檔 `version:1`，`enabled`（預設 **true**）、`interval_seconds` ∈ {120, 300, 600, 900, 1800, 3600}（預設 120）；其他值一律拒絕 |
| 到期判斷 | `polling_due(last_completed, interval)`：從未完成過 → 立即；之後用**最新**間隔 |
| 設定檔讀寫 | 讀取上限 4096 B（來源設定 16384 B）；超過、損毀、版本不符都明確報錯；寫入走「同目錄暫存檔 → fsync → rename」，失敗清暫存 |
| 序列化 | 同時只有一個收集（佇列鎖＋生命週期 permit）；重複請求合併 |
| 手動刷新（平板） | 每個 Source 節流 10 秒；`202 Accepted` 立即回傳，結果為 `accepted`/`coalesced`/`throttled`（Paused 另回 `paused`）；實際資料由後續 Snapshot 送達 |
| 退避 | 遵守 `Retry-After`；帶抖動；上限 15 分鐘 |
| 離開 | `stop_and_wait`：設取消旗標 → 等運作中收集釋放 permit → 之後所有新收集一律拒絕 |

## 6. 失敗時的呈現

- **保留上一次的值，標 `stale`**，並附 `failure_code`。不得以 0 或滿額取代。
- 新電腦的指引必須具體：缺 CLI／未登入的訊息要說出**要裝哪個官方工具、在哪登入、如何重新整理**（教訓 2026-09-10）。
- 每條「有資料」判斷式都要有對應的渲染分支與 fixture（教訓：平板曾因只渲染 `quota_windows` 而隱藏有效的 Copilot `source_usage`）。
