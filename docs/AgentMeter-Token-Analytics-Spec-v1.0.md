# AgentMeter Token 分析中心

完整產品、UX、資料與技術規格書 v1.0  
日期：2026-09-12  
狀態：Core v1 可供設計、拆票、實作與驗收；Optional profiles 須另行啟用的開發基線  
參考網站：`http://localhost:3003/?agent=codex&tab=daily&date=2026-09-12&chart=kline`

本文件以 TokenUsageInsights 的實際網站、公開原始碼與 AgentMeter 現有架構為證據，定義「在 AgentMeter 內新增 Token 歷史分析能力」的目標規格。它補充而不取代 [`AgentMeter-Requirements-v1.1.md`](../AgentMeter-Requirements-v1.1.md)；領域詞彙仍以 [`CONTEXT.md`](../CONTEXT.md) 為準，既有不可逆決策仍以 [`docs/adr/`](./adr/) 為準。若三者衝突，必須先更新決策文件，不得由實作者自行選邊。

---

## 0. 文件讀法與證據分級

### 0.1 標記

| 標記 | 意義 | 可否直接宣稱已完成 |
|---|---|---|
| **[實測]** | 2026-09-12 以本機瀏覽器與 HTTP 請求觀察參考網站 | 只能代表當次本機版本與資料狀態 |
| **[上游]** | TokenUsageInsights 公開 README、DESIGN 或 Rust 原始碼可核對 | 代表上游目前的設計／實作，不代表 AgentMeter 已有 |
| **[現有]** | AgentMeter 儲存庫已有程式、測試或 ADR | 可依既有驗證範圍宣稱 |
| **[目標]** | 本文件要求後續新增或調整 | 完成驗收前不可宣稱已實作 |
| **[決策]** | 需要產品或架構負責人明確選擇 | 未決前不得用隱性預設上線 |

### 0.2 權威順序

同一行為若有差異，依序採信：可重現測試 → 公開 Interface／型別 → ADR／正式需求 → 實際實作 → 註解與敘述文件 → UI 觀察。UI 走查可證明使用者看到什麼，不能單獨證明內部資料正確。

### 0.3 隱私界線

走查過程曾看見本機 Session、路徑與對話內容，但本文件只保留欄位結構與互動行為，不記錄任何真實 prompt、response、Session ID、repository、使用者名稱或完整路徑。

### 0.4 上游版本邊界

公開原始碼的靜態分析基準是 `doggy8088/TokenUsageInsights` commit `c15cebc1b27d42ab6f78a4fb15a7583aecf657a1`（2026-09-11）。本機 `localhost:3003` 沒有暴露可驗證的 build SHA，因此「本機實測」與「該 commit 的原始碼事實」是兩條證據鏈；若兩者不同，以本機畫面描述使用者行為、以固定 commit 描述內部實作，不推定兩者完全相同。

---

## 1. 執行摘要

參考網站是一個 local-first 的 AI coding 使用量戰情室，核心能力不是單純的「額度卡片」，而是把九種 Agent 的本機紀錄正規化成每日、月度、年度、Session、模型、專案、價格與匯入批次等可追溯視圖。其辨識度最高的功能是：

1. 以 URL 保存 Agent、期間、日期與圖表模式，可直接分享／重開同一分析狀態。
2. 每日頁同時提供摘要、Token 組成、趨勢或 K 線、Session 明細與對話重建。
3. 月度／年度頁以日／月粒度彙總，並可下鑽專案與模型。
4. 本機背景同步、手動同步、匯入／匯出、批次回滾與設定指南形成完整營運工作流。
5. 以深色「Local Operations Console」呈現高密度資訊，並支援亮色、行動版與多語言。

AgentMeter 現有優勢是可信的 Provider／Quota 領域模型、即時 `LiveDashboard`、桌面殼層、系統匣、平板安全通道、SSE、配對與更新；主要缺口是歷史事件儲存、統一 ingestion、查詢、圖表與 Session privacy policy。目標不是把上游 UI 原封不動搬入，而是把其成熟的分析旅程落在 AgentMeter 已確認的資料品質、帳號隔離與 loopback 安全不變量上。

### 1.1 成功結果

設計目標是讓使用者在 10 秒內回答：

- 今天／本月／今年用了多少、由哪些 Token 類型組成？
- 哪一個 Provider Account、Source、模型、專案或 Session 造成變化？
- 數字是官方、從本機觀察、估算或手動匯入？涵蓋範圍是什麼？
- 資料最後何時成功更新？缺值、過期、解析失敗或尚未登入的原因為何？
- 若數字異常，能否追到來源事件或匯入批次，而不洩漏對話正文？

此 10 秒為 UX 指標，不是主觀宣稱：以至少 5 位未參與開發的目標使用者、預載相同去識別資料、從 Daily 首屏開始，分別回答「今日總量、最大組成、資料是否新鮮」三題；至少 80% 使用者每題在 10 秒內答對，並記錄錯誤類型。

### 1.2 非目標

- 不將不同 Provider、metric、unit 或不可證明相同的 Account 強行加成一個「總剩餘額度」。
- 不呼叫模型產生內容以探測用量，不代替 Provider 登入，不保存 Provider 憑證。
- 不預設保存完整 prompt／response／tool stdout；Session 內容重建必須另行 opt-in。
- 不把圖表推導值偽裝成官方帳單，不以 K 線視覺掩蓋未知或不完整資料。
- v1 不提供 LAN 公開服務、多人權限、雲端同步或動態第三方 Collector SDK。

---

## 2. 產品範圍與使用者

### 2.1 主要角色

| 角色 | 主要問題 | 必要能力 |
|---|---|---|
| 個人開發者 | 哪個 Agent 用量最高、是否快達限制 | 今日摘要、quota、模型／Session 下鑽、即時刷新 |
| 多工具使用者 | 不同 Agent 資料是否可比較 | Provider／Account／Source 篩選、scope 與品質標示 |
| 維護者 | Collector 或 schema 哪裡壞了 | 同步狀態、Failure Code、版本、診斷與可重現測試 |
| 隱私敏感使用者 | 本機紀錄是否被上傳或洩漏 | local-first、明確內容蒐集開關、可預覽匯出、資料清除 |

### 2.2 主要使用情境

| ID | 情境 | 可觀察結果 |
|---|---|---|
| UC-TA-01 | 開啟分析中心 | 立即看到最後可用快照與 freshness；背景查詢不阻塞 UI |
| UC-TA-02 | 切換 Provider／日期 | URL 更新；新請求完成前顯示該目標的 loading，不沿用前一 Provider 文案 |
| UC-TA-03 | 比較每日組成 | 摘要與圖表使用相同查詢範圍、metric 與時區 |
| UC-TA-04 | 追查尖峰 | 從 candle／趨勢點下鑽到該區間 Session，不洩漏正文 |
| UC-TA-05 | 查看月／年趨勢 | 日／月 summary 可追溯到 raw event 範圍與 summary timezone |
| UC-TA-06 | 匯入另一台機器資料 | 預覽、驗證、批次寫入、去重；可精確回滾本批新增列 |
| UC-TA-07 | Collector 失敗 | 保留帶時間的舊值並清楚顯示 stale／failure，不改寫成 0 |
| UC-TA-08 | 平板查看 | 使用既有配對與 session，在有界查詢內取得相同資料契約 |

---

## 3. 參考網站資訊架構與功能盤點

### 3.1 全域資訊架構

```text
Token 戰情室
├─ Agent 選擇（9 種）
├─ 期間選擇
│  ├─ 每日：摘要、K 線／趨勢、Sessions
│  ├─ 月度：摘要、每日趨勢、每日表、專案、模型
│  └─ 年度：摘要、每月趨勢、每月表、專案、模型
├─ 日期／月份／年份導覽
├─ 同步與自動刷新
└─ 工具
   ├─ 匯出、匯入、匯入紀錄／回滾
   ├─ 定價表
   ├─ 設定指南
   ├─ 語言、主題
   └─ GitHub 原始碼
```

### 3.2 Agent 支援矩陣

**[實測＋上游]** 參考網站呈現以下 Agent。這是參考網站的範圍，不是 AgentMeter v1 必須一次支援九種的承諾。

| Key | 顯示名稱 | 主要資料來源 | 精確度／限制 |
|---|---|---|---|
| `antigravity` | Antigravity CLI | statusline JSONL | 依 Collector 安裝與來源欄位 |
| `copilot` | GitHub Copilot | CLI statusline、本機 App SQLite、VS Code chat | App 以複合 cursor 增量同步；來源需分開 |
| `codex` | Codex | `sessions` 與 `archived_sessions` JSONL | 可重建 Session；只代表本機紀錄 |
| `claude` | Claude Code | projects JSONL／statusline | rate limit 與 session usage 是不同 scope |
| `cursor` | Cursor | projects 與選擇性 `state.vscdb` | Token 可能由文字估算；未知模型保留 Unknown |
| `grok` | Grok Build | session／updates JSONL | 優先 provider usage/cost，fallback 必須標 estimated |
| `pi` | Pi Coding Agent | 本機 JSONL | 可使用來源權威 cost |
| `omp` | OMP | 本機 JSONL | 可使用來源權威 cost |
| `muse` | Muse | `session.jsonl` | 依本機檔案是否存在 |

**[目標]** AgentMeter 第一階段仍只承諾 Claude、Codex、Copilot、Antigravity；其他 Adapter 應能依同一 `Collector` Interface 增加，但不得在沒有實帳證據時顯示為 stable。

來源 Adapter 必須再遵守：statusline 檔以 byte cursor 只讀完整換行，partial final line 不得前移 cursor；第三方 SQLite 永遠以 read-only 開啟；Copilot App 使用 `(created_at, id)` 複合高水位並按完整 turn 重聚合；Claude streaming chunk 依 request/message/uuid 去重；Codex active/archive 同 identity 去重；非整點時區 timestamp（例如 `+05:30`、`+09:30`）必須正確解析。

### 3.3 全域頁首／側欄

| ID | 功能 | 參考網站現況 | AgentMeter 目標規則 |
|---|---|---|---|
| FR-NAV-01 | Provider 切換 | 參考側欄為 Agent 按鈕 | AgentMeter 寫入 canonical `provider`；只列已設定 Source；顯示 maturity／freshness 徽章 |
| FR-NAV-02 | 期間切換 | 每日／月度／年度 | 維持可鍵盤操作的 tab pattern 與 `aria-selected` |
| FR-NAV-03 | 日期導覽 | 前一個、今天、下一個、重新整理 | 不允許超過今天；依顯示時區計算「今天」 |
| FR-NAV-04 | 快速統計 | Session、Token、cache、USD、duration、request | 未知顯示「尚未取得」；不可把 null 當 0 |
| FR-NAV-05 | 自動刷新 | 開關與 5／10／30 秒倒數 | 只刷新本機 cache／History view，絕不觸發 Provider collection；Collector cadence 仍由每 Source scheduler、最低 120 秒設定與 Provider rate limit 控制 |
| FR-NAV-06 | 側欄收合 | 桌面可收合，`Ctrl/Cmd+B` | 快捷鍵不得攔截輸入框；狀態持久化 |
| FR-NAV-07 | 工具列 | 匯出、匯入、歷史、語言、價格、指南、同步、主題 | 小螢幕收進有標籤的 overflow menu，不能只剩不明 icon |

### 3.4 每日頁

#### 3.4.1 摘要卡

| ID | 卡片 | 定義 |
|---|---|---|
| FR-DAY-01 | Total Tokens | 同一 query scope 內可靠 usage delta 的總和；顯示資料品質 |
| FR-DAY-02 | Non-cache Input | 不含 cache read 的 input；若來源 input 已含 cache，必須標示而非重扣 |
| FR-DAY-03 | Cache Read | 來源可明確辨識時才分列；附占總量比例 |
| FR-DAY-04 | Output | output token 與比例 |
| FR-DAY-05 | Cost | `official_cost` 與 `estimated_cost` 分欄／分徽章；不可混合 |
| FR-DAY-06 | 其他 Token | cache write、reasoning 等於明細可見；是否進 Total 由來源語義明示 |

#### 3.4.2 圖表模式

**趨勢圖（穩定）**

- 預設顯示最近 15 個 Session 或依時間 bucket 的 Token series；模式名稱必須說明是「Session」或「時間」。
- series 可切換 input、output、cache read、reasoning、total；同圖不得混用不同 unit。
- hover／focus tooltip 顯示時間、值、資料品質、Session 數與 scope。
- 點擊資料點可將 Session 表篩到對應區間；URL 寫入 `from`／`to` 或等價 filter。

**K 線（實驗）**

| ID | 規則 |
|---|---|
| FR-CHART-01 | `chart=kline` 明確標示 experimental，不當作金融價格或 quota 圖。 |
| FR-CHART-02 | 參考網站以 UTC 00:00–24:00；AgentMeter 由指定 IANA timezone 建立 local intervals，再轉成 `[from_utc,to_utc)` 查詢。DST 日合法具有 23／25 小時。 |
| FR-CHART-03 | 對選定 metric 的 cumulative series：O = bucket 第一事件前的可靠 baseline；依 `source_sequence` 套用非負 delta 後，H/L = running total 最大／最小，C = bucket 結束 total。baseline 未知則整根 unknown；不得猜 0。 |
| FR-CHART-04 | candle 的 volume／附屬值為該 bucket 新增 Token delta；不可由 quota snapshot 推算。 |
| FR-CHART-05 | 可選 5m、15m、30m、1h、2h、4h；query 回傳完整日（5m 通常 288 根，DST 依實際 interval），viewport 最多 24 根，支援拖曳／slider。 |
| FR-CHART-06 | 過去無事件 bucket 不產生 candle，形成 gap；不 carry-forward 假 candle；未來 bucket 不繪製；未知不得畫成 0。 |
| FR-CHART-07 | 顯示 interval、有效 candles、累計、official cost、MA5；MA5 只對連續有效 close 計算，遇 gap 重新起算。 |
| FR-CHART-08 | 軸標籤以使用者顯示時區呈現，tooltip 同時能查看精確 UTC。 |
| FR-CHART-09 | 鍵盤可逐點移動；另有等價資料表與「減少動態效果」模式。 |

> K 線是 optional `kline-prototype` profile，不屬 Core v1 DoD。先決條件：累積 counter reset／rotation 已正確轉 delta、同一 series 的 Account／scope／metric／unit／policy 相同，且 aggregation 測試通過；否則只提供趨勢圖。

#### 3.4.3 Sessions 表

| ID | 功能 | 規格 |
|---|---|---|
| FR-SES-01 | 列表 | 顯示 Session、model、turns、input、output、reasoning、cache、total、cost、duration、時間、CWD alias |
| FR-SES-02 | 排序 | 所有數值與時間欄可排序；同值以 `session_key` 穩定排序 |
| FR-SES-03 | 搜尋 | 可搜尋已建立的 privacy-safe prompt index；未 opt-in 時不提供內容搜尋 |
| FR-SES-04 | 專案篩選 | 以去識別 alias 顯示；完整路徑只在本機、明確展開後顯示 |
| FR-SES-05 | 分頁 | cursor pagination，預設 50、最大 200；不得一次載入整年明細 |
| FR-SES-06 | Subagent | 以 parent relationship 巢狀呈現 nickname／role；缺 parent 不丟棄資料 |
| FR-SES-07 | 下鑽 | 點列開啟 Session drawer；返回後保留 scroll、排序與 filter |
| FR-SES-08 | 空值 | null 顯示「—／尚未取得」，0 只代表已確認為零 |

#### 3.4.4 Session drawer

**[實測]** 參考網站可重建 SystemStatus、UserPrompt、AssistantReply 與 tool invocation，並顯示 metadata、Token、成本、tool exit code、stdout／stderr、Markdown 預覽與複製。

**[目標]** AgentMeter 分兩級：

1. `metrics_only`（預設）：只保存 Session key、時間、model、turn、Token、duration、匿名專案 alias、parent；不保存正文、附件、tool args、stdout、stderr。
2. `content_opt_in`（明確開啟）：內容只在本機保存，UI 常駐敏感提示，可按 Source 關閉與清除；export 預設仍排除內容。

| ID | 要求 |
|---|---|
| FR-SES-09 | Drawer 是 modal dialog：有名稱、focus trap、Escape 關閉、關閉後 focus 返回原列。 |
| FR-SES-10 | metadata 分組：Source／Account alias、model、branch alias、起訖時間、品質、parser version。 |
| FR-SES-11 | timeline 依 `observed_at`＋source order 穩定排序；同 turn 的事件不任意重排。 |
| FR-SES-12 | Markdown 經 allowlist sanitizer；連結禁止 `javascript:`，外連明確標示。 |
| FR-SES-13 | tool args、stdout、stderr 預設收合並做 secret／path redaction；內容大小有上限。 |
| FR-SES-14 | 複製動作需區分「已遮罩文字」與「原始文字」；原始內容需再次確認。 |

### 3.5 月度頁

| ID | 功能 | 規格 |
|---|---|---|
| FR-MON-01 | 月摘要 | sessions、total、input、output、cache、reasoning、official／estimated cost |
| FR-MON-02 | 每日趨勢 | 一天一個 bucket；缺日為 gap 或「無事件」，不可默認資料遺失等於 0 |
| FR-MON-03 | 每日表 | 日期、各 metric、Session 數、成本、資料完整性；點擊導向每日頁 |
| FR-MON-04 | 專案表 | privacy-safe alias、sessions、Token、成本；未知專案獨立保留 |
| FR-MON-05 | 模型表 | model＋mode／reasoning effort、sessions、Token 組成、成本，可下鑽 Session |
| FR-MON-06 | Provider breakdown | 多 Provider 匯入資料分列，不可在不同 unit 下顯示無意義總和 |

### 3.6 年度頁

| ID | 功能 | 規格 |
|---|---|---|
| FR-YEAR-01 | 年摘要 | 與月摘要相同，但 query period 為 calendar year＋timezone |
| FR-YEAR-02 | 每月趨勢 | 每月一 bucket；保留每個 summary 的 `bucket_timezone` |
| FR-YEAR-03 | 每月表 | 點擊導向月度頁；顯示是否仍可從 raw event 重建 |
| FR-YEAR-04 | 專案／模型 | cursor pagination；避免整年明細直接載入前端 |

### 3.7 匯入、匯出與回滾

| ID | 功能 | 規格 |
|---|---|---|
| FR-IO-01 | 單日匯出 | 匯出目前 Agent／日期／filter；先顯示 manifest、筆數、敏感欄位摘要 |
| FR-IO-02 | 全量匯出 | 包含 native 與 imported usage；不含設定、憑證、配對資料、內容正文與匯入批次紀錄 |
| FR-IO-03 | 匯入預覽 | 驗證格式版本、Assistant、日期、筆數、大小、checksum、警告；未通過不得啟用確認 |
| FR-IO-04 | 大小限制 | 解壓前與解壓後皆限 200 MB；防 zip bomb、path traversal 與過深 JSON |
| FR-IO-05 | 去重 | `(stable_source_namespace, external_fact_id)` 與核心重算的 FactIdentityV1 hash 雙重檢查；不得跨 Source／Account 誤去重 |
| FR-IO-06 | 批次原子性 | 單一 Agent 批次原子；多 Agent 檔依 Agent 分批，較晚失敗不回滾較早成功批次 |
| FR-IO-07 | 匯入紀錄 | 最近 50 筆／Agent；顯示成功、跳過、失敗數、時間、檔名雜湊與 schema version |
| FR-IO-08 | 精確回滾 | 只刪除該 batch 真正新增的 row；原生資料、舊匯入與設定不受影響 |
| FR-IO-09 | 相容性 | reader 支援目前與前一個 major format；未知 major fail closed |

資料交換分兩種，不得把 summary export 再當成可 ingestion 的 event：

- `portable_summary`：供報表交換，只含已聚合 bucket、維度、quality／completeness，不參與 raw event 去重。
- `canonical_events`：供重新 ingestion，必須保留完整 series identity、source namespace、順序、counter 與 provenance。

建議 canonical export envelope：

```json
{
  "format": "agentmeter-usage-export",
  "format_version": 1,
  "provider": "codex",
  "range": { "from": "2026-09-12", "to": "2026-09-12", "timezone": "Asia/Taipei" },
  "exported_at": "2026-09-12T00:00:00Z",
  "export_kind": "canonical_events",
  "privacy_mode": "metrics_only",
  "records": [
    {
      "external_fact_id": "opaque-stable-id",
      "provider": "codex",
      "account_namespace": "source-scoped-provisional",
      "source_namespace": "opaque-source",
      "source_event_key": "opaque-event",
      "fact_ordinal": 0,
      "session_ref": "opaque-session",
      "model": "example-model",
      "scope": "local_session",
      "metric": "input_tokens",
      "counter_kind": "delta",
      "raw_value": "123",
      "delta_value": "123",
      "unit": "token",
      "source_sequence": "example-file-id:42",
      "observed_at": "2026-09-12T00:00:00Z",
      "collected_at": "2026-09-12T00:00:01Z",
      "parser_version": "example-parser-v1",
      "accounting_policy_version": "example-policy-v1",
      "claimed_data_quality": "local_observed",
      "provenance": "local_transcript"
    }
  ]
}
```

以上數值全為 schema 範例，不是真實使用量。

未驗證簽章的匯入資料一律 `ingestion_mode=imported`、effective quality 至少降為 `manual`；來源宣稱只放 `claimed_data_quality`，不得直接取得 `official`。若 native 與 imported 內容 canonical hash 相同，即使 external ID 不同也只能擁有一份 fact；import batch 不得取得既有 native fact 的 ownership。

`import_batches` 使用 `validating|importing|completed|failed|rolling_back|rolled_back`。所有 fact 寫入、batch ownership 與受影響 summary 重算在同一 transaction；程序在 importing/rolling_back 中終止時由 SQLite rollback，重啟後依 durable batch marker 標 failed 或重試，不留下半批資料。Rollback 以 stable fact provenance／batch link 刪除本批擁有的 facts，再重算 sessions、summaries 與 optional estimates；duplicate/native facts 沒有本批 ownership。

### 3.8 設定、定價、語言與主題

| ID | 要求 |
|---|---|
| FR-SET-01 | 每個 Source 顯示資料目錄、存在性、collector／hook 狀態、parser version 與修復步驟；顯示路徑前預設縮寫使用者目錄。 |
| FR-SET-02 | 安裝／修改 statusline 前先同意、預覽 diff、備份；不得覆蓋既有設定。 |
| FR-SET-03 | 定價表支援 model、deployment、unit、input、cache input、output、batch；顯示 catalog version、currency、effective date。 |
| FR-SET-04 | 估價僅為估算並保留 `pricing_catalog_version`；本地／OSS 零價不得推論為「官方免費」。 |
| FR-SET-05 | zh-TW 為正式基線；所有文案用 resource key，日期與數字依 locale 格式化。 |
| FR-SET-06 | 深色／亮色／跟隨系統；主題狀態本機持久化，初次 render 不閃爍錯誤主題。 |
| FR-SET-07 | 手動同步只排程 Collector；重複要求按 Source 合併，UI 顯示 queued/running/result。 |

### 3.9 載入、空資料與錯誤

| State | 必要內容 | 禁止行為 |
|---|---|---|
| `loading` | skeleton、目標 Agent／期間、可取消 generation | 顯示前一 Agent 的空資料或錯誤文案 |
| `ready` | generated_at、freshness、資料品質、範圍 | 把跨 scope 數字混在一起 |
| `empty` | 明確 Agent／日期、可能原因、設定指南與刷新 | 顯示全零假裝成功 |
| `stale` | 舊資料、last success、failure、重試時間 | 清空仍有用的舊快照 |
| `error` | failure code、可執行修復、診斷入口 | 顯示原始 stack trace／秘密 |
| `paused` | 舊資料與 paused 標示、明確恢復按鈕 | refresh 暗中解除暫停 |
| `unsupported` | 原因、支援版本、替代方案、maturity | 空白卡片宣稱已支援 |

**[實測缺陷 OBS-01]** 參考網站切換 Agent 後，後端回覆完成前可能短暫沿用前一 Agent 的空資料文案與路徑。AgentMeter 必須用 request generation／AbortController；只有 `response.generation === currentGeneration` 才能 commit UI state。

### 3.10 參考網站技術架構

**[上游]** 固定 commit 的實作是 Rust 2021 單一執行檔，使用 Axum 0.7、Tokio、Serde、Chrono、tower-http 與 bundled rusqlite；前端沒有 React/Vue/Svelte 或 bundler，而是 `index.html`＋CSS＋原生 JavaScript。npm package 只是 Node 18.18+ 的跨平台下載／啟動 wrapper，不是前端 build chain。

```text
CLI statusline JSONL / Session transcript JSONL
VS Code chat storage / Copilot App SQLite / Cursor SQLite
                         │
                  source scanners/parsers
                         │
       UsageEntry / TokenStats / CostStats normalization
                         │
                   SQLite usage_entries
       ┌─────────────────┼───────────────────┐
       │                 │                   │
  sync_state      import_batches    cursor attribution
       └─────────────────┼───────────────────┘
                         │
       daily / monthly / yearly / session handlers
                         │ JSON API
                         ▼
                 vanilla JavaScript SPA
       cards / charts / tables / drawer / modals
```

關鍵上游模組：

| 路徑 | 責任 |
|---|---|
| `src/main.rs` | 入口、router、CORS、static files、背景同步 |
| `src/db.rs` | SQLite schema、同步協調、多數 parser 與 query |
| `src/handlers/daily.rs` | 日報、搜尋、Session path validation、timeline |
| `src/handlers/monthly.rs`／`yearly.rs` | 期間彙總與下鑽 |
| `src/handlers/misc.rs` | 價格、同步、rate limit、import/export/rollback |
| `src/timeline.rs` | 各來源 transcript 到統一 TimelineItem |
| `src/pricing.rs` | `pricing.csv`、模型匹配、估價公式 |
| `src/vscode.rs`／`grok.rs`／`pi.rs`／`omp.rs`／`muse.rs` | 來源專屬 Adapter |
| `static/app.js` | 全域狀態、API、渲染、篩選、排序與圖表 |
| `static/chart-utils.js`／`session-utils.js` | K 線運算與 Session 比較 |
| `static/i18n.js`／`styles.css` | 五語資源與完整 theme/RWD |

背景首次同步在 blocking task 執行，之後固定每 5 秒掃描；前端 5／10／30 秒 auto refresh 是重新抓 view data，不等於 Collector 同步週期。Session 內容採按需解析：日報先回摘要與列表，點列後才用 assistant＋session＋source identity 安全定位 transcript 並建 timeline。

上游優點是單一 binary、來源涵蓋廣、增量同步與下鑽完整；主要架構債是 `db.rs` 責任過大、front-end state 集中於單一 `app.js`、API 缺 schema/auth，以及本地優先頁面仍依賴外部 CDN。AgentMeter 目標架構因此保留可見功能，將 ingestion、storage、query、redaction 與 chart 切成深 Module。

---

## 4. URL、狀態與導覽契約

### 4.1 Query parameters

| 參數 | 格式／值 | 預設 | 無效值 |
|---|---|---|---|
| `provider` | Provider key | 上次選擇或第一個 enabled | 無效則回安全預設並顯示提示 |
| `account` | opaque local ref | active account | 不存在則不跨 account fallback |
| `source` | opaque local ref | all linked sources | 未確認 link 時不可 all-merge |
| `tab` | `daily\|monthly\|yearly` | `daily` | `daily` |
| `date` | `YYYY-MM-DD`／`YYYY-MM`／`YYYY` | 顯示時區的今天 | 顯示錯誤並回今天；不可送模糊日期到 DB |
| `chart` | `trend\|kline` | `trend` | `trend` |
| `interval` | `5m\|15m\|30m\|1h\|2h\|4h` | `1h` | 依 chart 合法集合 |
| `project` | opaque alias | 無 | 忽略並提示 |
| `model` | percent-encoded model id | 無 | 忽略 |
| `sort` | allowlist column | `timestamp` | `timestamp` |
| `order` | `asc\|desc` | `desc` | `desc` |
| `cursor` | opaque signed/local cursor | 無 | 400，不猜測 |

參考網站使用 `dir` 傳完整／家目錄相對／唯一尾碼路徑；AgentMeter 為降低 URL 洩漏，改用 privacy-safe `project` alias。只有使用者明確選擇「複製含本機路徑的連結」時才允許完整 path，並顯示敏感資訊提示。

`agent` 只作參考網站舊連結的相容 alias，讀取後以 `replaceState` 正規化成 AgentMeter 的 canonical `provider`。AgentMeter 目標文件、DTO 與程式使用 Provider／Provider Account／Source；Agent／Assistant 只描述上游 UI 或 import compatibility format。

### 4.2 狀態優先序

1. 合法 URL 參數。
2. 該裝置已保存的偏好（主題、收合、自動刷新、interval）。
3. 應用預設。

資料 scope 不得只放 localStorage；Account／Source／日期必須反映於 URL 或 query object，避免畫面與後端範圍不一致。Back／Forward 必須還原 UI，不重複新增 history entry。

---

## 5. 資料領域與計算規則

### 5.1 核心不變量

1. Unknown ≠ 0；缺值一律 `null`。
2. `DataQuality`、`CollectorMaturity`、`Availability`、`CollectionState`、`Freshness` 正交，不壓成單一 status。
3. Usage 只加總相同 Provider Account、scope、metric、unit 的可靠 delta。
4. Quota 是快照，只能做 trend、period-end、period-min 與 reset event，絕不相加。
5. Account 合併需要穩定 identity 或可撤銷的 Account Link；路徑、顯示名稱與單帳號假設都不是證據。
6. 時間以 UTC 儲存；每個 Daily Summary 保存 `bucket_timezone`。
7. 原始值、正規化值、source time、collected time 分開。
8. Parser 無法理解新版 schema 時 fail closed，保留舊快照並標 `schema_changed`。
9. prompt 衍生的 `session_name` 仍是敏感內容，即使只有前 100 字也不得當作無害 metadata。
10. 第三方資料庫與日誌只讀；AgentMeter 不在來源目錄建立 cursor、lock 或 migration 檔。

### 5.2 Canonical types

```rust
struct ObservationBatch {
    source: SourceRef,
    provider_account: AccountRef,
    identity_state: AccountIdentityState,
    parser_version: String,
    collected_at: DateTime<Utc>,
    usage: Vec<UsageEvent>,
    quota: Vec<QuotaSnapshot>,
    health: HealthObservation,
}

struct UsageEvent {
    fact_key: String,
    source_event_key: String,
    fact_ordinal: u32,
    source_sequence: SourceSequence,
    session_key: Option<String>,
    model: Option<String>,
    metric: UsageMetric,
    raw_value: Option<Decimal>,
    delta_value: Option<Decimal>,
    unit: UsageUnit,
    scope: UsageScope,
    counter_kind: CounterKind,
    accounting_policy_version: String,
    observed_at: DateTime<Utc>,
    data_quality: DataQuality,
}
```

`UsageMetric` 至少包含 `input_tokens`、`output_tokens`、`cache_read_tokens`、`cache_write_tokens`、`reasoning_tokens`、`request_count`、`duration_ms`、`official_cost`。費用 unit 必須有 ISO 4217 currency；Token unit 不帶 currency。`estimated_cost` 是衍生資料，不是 Collector Observation。

未知 Provider identity 也必須建立非空、只屬於該 Source 的 provisional AccountRef，不使用 SQL `NULL` 表示未知。`AccountIdentityState` 為 `provisional|stable|linked`：

- stable identity hash 的 keyed-hash 金鑰在首次啟動產生，以 DPAPI 保護；原文 identity 不落盤。
- 金鑰遺失時既有 Account 保持 opaque orphan，不用新 hash 自動合併；需使用者重新確認 Link。
- 金鑰輪替建立 versioned hash 並在 transaction 中保留舊→新 mapping，不能重寫來源事件身分。
- Account Link 建立／撤銷不搬移 raw event；query 依帶 revision 的 link graph 解析，summary 保存 link revision，變動後排程重建。
- Forget provisional Account 只刪該 Source-scoped identity 與其 history，不波及同 Provider 其他 provisional Account。

#### FactIdentityV1

`fact_key` 只能由 History 核心重算；Collector／import 提供的 hash 只供比對，不受信任。Identity 的精確欄位順序是：`provider_key`、`account_namespace`、`source_namespace`、`source_event_key`、`fact_ordinal`、`session_key`、`model`、`scope`、`metric`、`unit`、`counter_kind`、`accounting_policy_version`。Account／Source namespace 都非空；optional string 使用顯式 presence byte，不能用空字串冒充 null。

編碼為 `agentmeter.fact.v1\0` domain separator，加上上述順序的欄位；enum 用小寫 ASCII，string 先做 Unicode NFC、UTF-8，再以 big-endian u32 長度前綴，ordinal 用 big-endian u32。`fact_key = SHA-256(encoded_identity)`。另算 `source_payload_hash`，只包含來源事件不可變內容：identity bytes、來源 raw value、來源 observed instant、source sequence 與必要的原始 provenance 欄位。`collected_at` 是 first/last-seen ingestion metadata；`delta_value` 是可重建衍生值；effective quality 與 parser version 是 versioned interpretation，四者都不參與 collision hash。相同 fact key＋source payload hash 才是 duplicate；fact key 相同但 immutable source payload 不同才 quarantine。

Parser／quality 升級採 deterministic reparse：保留原 source payload，寫入新的 `interpretation_revision`、parser/accounting policy/effective quality，於同一 History transaction 重算受影響 counter segment 與 summaries；結果相同則只更新 interpretation metadata。升級不能變更 FactIdentityV1；若新 parser 拆分／合併 facts，必須升 identity version並以 migration 明確取代舊 facts。

`external_fact_id` 只在其 `stable_source_namespace` 內唯一，不能取代 FactIdentityV1。Export/import 必須帶原 namespace；若無法證明 namespace 穩定，建立新的 imported Source，不與 native Source 去重或合併。

### 5.3 累積 counter

| 情況 | delta 規則 |
|---|---|
| 首次看見 counter | 若來源可證明起點為 0，可記當前值；否則 `delta=null` 並等待下一筆 |
| 同 session 正常增加 | `current - previous` |
| 值相同 | delta 0，但 ingestion 可去重不另寫 summary |
| 值下降 | 若 session／reset／rotation 證據成立，開新 segment；否則標 anomaly，不產生負 delta |
| 檔案重讀 | `source_event_key` 命中即跳過 |
| parser 版本改變 | 不能自動跨版本接續 counter，除非 migration 規則有測試 |

Counter series key 固定為 `(source, account, session_or_segment, model, scope, metric, unit, accounting_policy_version)`。`CounterKind::Delta` 直接採非負 event delta；`CounterKind::Cumulative` 才使用前值。所謂「前一筆」依來源權威順序，不依 ingestion arrival：優先 source-provided sequence，其次 `(file_identity, byte_offset)`，最後才是 `(observed_at, stable_tie_breaker)`。

- 同 event key、同 canonical payload：duplicate。
- 同 event key、不同 payload：quarantine 為 `identity_collision`，兩筆皆不進 summary。
- 晚到／out-of-order event 插入正確位置後，重算該 series 受影響 segment 與 summary；不能只與最後到達值相減。
- 負值、overflow、NaN／Infinity 或超過設定上限：拒絕 fact 並計 rejected；不截斷。
- input 是否包含 cache、reasoning 是否已包含 output，寫入 `accounting_policy_version`；只有相同 policy 的 facts 可直接合計。
- `source_sequence` 必須可持久化且 deterministic；同 timestamp 必須有 stable tie-breaker。

### 5.4 成本決策

現有 AgentMeter v1.1 明定只顯示 Provider 直接回傳的官方費用；參考網站支援 `pricing.csv` 本機估算。目標架構允許保存兩者，但 UI 與聚合永遠分開：

- `official_cost`：來源直接回傳，`data_quality=official|local_observed` 依來源性質。
- `estimated_cost`：由 query／summary 的可靠 Token 經獨立 Derived Estimate Module 計算；保存 usage lineage、model mapping、價格版本、effective interval、currency、公式與 rounding 版本。
- 不得產生 `total_cost = official + estimated`。
- 若同一事件已有 official cost，estimated 只作比較資料，預設不顯示於總卡。
- **[Core v1 決策 D-01]** 不計算、不持久化、不顯示 estimated cost。Optional `estimated-cost` profile 經 ADR 與獨立驗收後才可啟用；關閉不只是 UI 隱藏。

### 5.5 SQLite 邏輯模型

沿用既有 v1.1 資料表並擴充，不建立第二套平行歷史庫。

| Table | 關鍵欄位／索引 | 規則 |
|---|---|---|
| `provider_accounts` | provider、非空 opaque identity hash、identity state/key version | identity hash 使用 DPAPI 保護金鑰的 keyed hash；未知也建立 Source-scoped provisional row |
| `account_links` | account_id、source_id、confirmed／revoked | 只允許穩定 identity 或使用者確認 |
| `provider_sources` | provider、environment、path alias、capabilities、parser | 完整 path 不提供給 tablet |
| `usage_events` | fact key、source/account/event key/ordinal/sequence/session/model/metric/raw/delta/unit/scope/times/quality/policy | `fact_key` 為 canonical serialization 的 SHA-256，所有 nullable 維度先正規化；唯一且碰撞 fail closed |
| `quota_snapshots` | 非空 source/account、provider bucket identity、window generation、used/limit/raw/display remaining/unit/percent/reset、observed/collected time、quality、source revision、material hash | unique `(source_id, provider_account_id, provider_bucket_key, window_generation, source_revision)`；逐 Source 原樣保存，不可 sum |
| `daily_summaries` | account/scope/metric/unit/date/timezone/value、structured source range、rebuildable_until、algorithm/computed/link revision、quality、completeness reasons | 可由仍保留 raw event 的範圍重建；不同 timezone 不靜默合併 |
| `sessions` | session key、source、account、model、start/end、turns、project alias | metrics-only baseline |
| `session_events` | session、event key、type、time、redacted payload | 只在 opt-in；payload 大小限制 |
| `collector_health_current` | source、五軸狀態、failure、success/retry/schema | 每 Source 最新狀態，供 Dashboard 快照 |
| `collector_health_events` | source、狀態轉換、時間、failure、counts | append-only bounded history，供診斷與趨勢 |
| `source_cursors` | source、file identity、offset、last complete event、parser version、upstream cursor | AgentMeter 自己的 DB；處理 truncation/rename/rotation/archive/補寫 |
| `import_batches` | batch、assistant、hash、schema、counts、status | rollback 的權威清單 |
| `import_batch_rows` | batch、table、row id | 只記本批新增 row |
| `pricing_catalogs` | version、currency、effective、checksum | 估價可重現 |
| `derived_estimates` | usage/summary lineage、catalog/model/formula/rounding、currency/value | 只在 optional profile；可刪除重算，不是 source observation |
| `alert_events` | account、bucket、threshold、window、notified | 防重複通知 |

SQLite 必須啟用 FK 並用 CHECK 保護非負 Token、合法時間與 enum；所有 schema migration 在單一 transaction 中執行，禁止忽略 `ALTER TABLE` 失敗。中央 DB 的 journal、synchronous、busy timeout、檔案 ACL 與備份策略必須在 Adapter 內顯式設定並受測試；不得依賴平台偶然預設。

Quota 衝突規則：所有 Source snapshot 原樣保存；同一 Provider Account＋bucket identity 的 effective view 先採使用者指定 preferred Source，否則依 DataQuality、Freshness、source observed time、stable SourceRef 排序。若同優先等級來源在同一窗口回報不一致，狀態為 `conflicting`，畫面顯示選中值與可展開比較，不靜默平均。`reset_at` 改變只有在舊窗口已過期或來源提供新 generation 證據時才建立新窗口；否則視為同窗口校正。normalized material hash 改變即寫 snapshot；未變時最多每 5 分鐘寫一次 heartbeat。

索引至少涵蓋：

- `usage_events(account_id, scope, metric, unit, observed_at)`
- `usage_events(source_id, source_event_key, fact_ordinal)`
- `sessions(source_id, started_at)`
- `daily_summaries(account_id, bucket_timezone, bucket_date)`
- `quota_snapshots(account_id, provider_bucket_key, observed_at)`
- `import_batches(provider, imported_at)`

### 5.6 Retention

- quota raw snapshots：30 日；數值改變或每 5 分鐘 heartbeat 才寫。
- raw usage events：預設 90 日，可選 30／90／180／365；清理後 summary 保留。
- Daily Summary：180 日以上，使用者可調整。
- content opt-in Session events：預設 30 日且獨立開關。
- collector diagnostics：7 日且有總大小上限。
- 刪除採 transaction＋scope guard；執行前回傳預估列數，完成後回傳實際列數。
- raw facts 只可在同一 maintenance transaction 確認對應 summary 已完成、checksum 正確且 `rebuildable_until` 更新後刪除；失敗時 raw 與 summary 一起 rollback。
- 查詢遇到不同 `bucket_timezone` 的歷史 summary 時，預設分組並標 `mixed_timezone=true`；只有 raw 尚存且使用者明確要求重建後才能合併到同一 timezone。

### 5.6.1 Cursor 與 rotation

- file identity 使用可跨 rename 辨識的檔案 ID；不可用 path 單獨當 identity。
- 檔案縮短視為 truncation；保留舊 generation，從新 generation offset 0 讀取並用 fact key 去重。
- active→archive rename 不造成新 Source 或重複；同 identity 只改 location。
- partial final line 不更新 last complete offset；下次補齊後解析一次。
- symlink、junction、reparse point canonicalize 後仍必須位於 allowlisted Source root。
- 外部 SQLite 用 read-only snapshot／transaction與有界 busy timeout；絕不 migration 或寫 lock/cursor 到 Provider 來源。

### 5.7 日期與時區決議

參考實作同時存在 UTC 前端日期、依 timestamp 文字前綴寫 DB、以及由 OS 本地日期命名的 statusline 檔，三者的日界線並不完全一致。AgentMeter 不沿用這個模糊性：

1. `observed_at` 必須解析為 RFC 3339 instant；另保存可用的原始 offset／原始字串供診斷。
2. `bucket_date` 一律以使用者設定的 IANA `bucket_timezone` 從 instant 計算，不以檔名或 timestamp 前 10 字決定。
3. 顯示時區可與 bucket timezone 不同，但 UI 必須同時標示「彙總時區」；切換顯示時區不暗中改 summary。
4. DST、`±HH:MM`、半小時／45 分鐘 offset、跨午夜與不存在／重複的 local time 都需 fixture。
5. statusline 檔名日期只作 discovery hint，不是權威 event date。

### 5.8 SQLite 備份與還原

- WAL 模式使用 SQLite online backup interface 取得一致 snapshot；若採 checkpoint，必須先證明 active writers 已協調並以測試覆蓋，不直接複製 `.db` 猜測一致性。
- 備份寫入同磁碟暫存檔，flush/fsync 後 atomic rename；manifest 包含 DB identity、from/to schema version、時間、大小、SHA-256。
- restore 前驗證 checksum、SQLite integrity、schema compatibility；先把現行 DB 另存 recovery copy，再原子切換。
- migration、backup、restore 的強制終止點都要測試；任何一步不確定即 fail closed。
- 即使 history feature flag 關閉，binary 也只在使用者已建立 History DB 且 migration 已通過時升 schema；不可為未啟用功能無條件建立新 DB。

---

## 6. 目標技術架構

### 6.1 架構圖

```text
Provider local API / statusline / files / SQLite
                    │
             Collector Adapters
                    │ ObservationBatch
                    ▼
       Canonical Observation Normalizer
                    │ typed, validated
                    ▼
       History::ingest internal implementation
       validate ─ dedup ─ counter ─ account isolation
                    │ one transaction
                    ▼
                History Interface
      ingest / query / plan_clear / apply_clear
                    │
       internal SQLite Repository Adapter
       raw events / summaries / health / imports
                    │
       bounded query / aggregation / provenance
          ┌─────────┴──────────┐
          ▼                    ▼
    Tauri Adapter      Authenticated HTTP Adapter
          ▼                    ▼
    Desktop UI          Tablet UI / SSE refresh
```

### 6.2 Module、Interface、Seam、Adapter

| Module | Interface | 隱藏的 Implementation 複雜度 | Depth／Leverage |
|---|---|---|---|
| Provider Collector Adapter | `probe/collect` | protocol、檔案 cursor、版本與解析 | 高；隔離每個 Provider 不穩定來源 |
| Collector Coordinator | `refresh/pause/resume/shutdown` | schedule、subprocess、timeout、backoff、coalescing、owner lifecycle | 高；所有 Provider 共用生命週期語義 |
| History ingestion（內部） | 只由 `History::ingest` 進入 | 驗證、去重、counter reset、Account 隔離、transaction | transaction 的唯一 owner；不形成第二個公開 Seam |
| History | `ingest/query/plan_clear/apply_clear` | ingestion、aggregation、timezone、pagination、provenance、清除計畫 | 最高；外部只有一個深 Interface |
| SQLite Repository Adapter | History 的內部 port | SQLite、migration、索引、retention | 高；Tauri/HTTP 不知道 SQL；測試另有 in-memory Adapter |
| LiveDashboard | `snapshot/publish/failure` | revision、去重、stale-on-failure、privacy allowlist | 維持現有即時 read model，不作歷史 authority |
| TabletServer | HTTP／SSE | pair/session、Origin、CSRF、bounded response | 現有深 Module；新增 query Adapter 即可 |
| Chart Adapter | `render(ChartModel)` | Canvas/SVG、tooltip、zoom、a11y fallback | 中；只消費查詢結果，不計算業務數字 |

關鍵 Seam：

- `Collector` 隔離不同 Provider 的不穩定來源；Coordinator 擁有排程與 shutdown，Collector 本身不擁有全域 lifecycle。
- `History` 隔離 ingestion/query/clear domain 與 SQLite；SQLite repository 只在 Module 內可見。
- `ChartAdapter` 隔離資料語義與視覺 library。
- `Clock`、`DisplayTimezone`、`PricingCatalog`、`Redactor` 皆需可替換以做 deterministic tests。

建議 Interface：

```rust
trait Collector {
    fn probe(&self, source: &SourceRef) -> Result<Capabilities, CollectorFailure>;
    fn collect(
        &self,
        request: CollectRequest,
        cancelled: &AtomicBool,
    ) -> Result<ObservationBatch, CollectorFailure>;
}

trait CollectorCoordinator {
    fn refresh(&self, sources: &[SourceRef], reason: RefreshReason) -> RefreshOutcome;
    fn pause(&self) -> PauseOutcome;
    fn resume(&self) -> RefreshOutcome;
    fn shutdown(&self) -> ShutdownOutcome;
}

trait History {
    fn ingest(&self, batch: &ObservationBatch) -> Result<IngestOutcome, StorageFailure>;
    fn query(&self, query: &HistoryQuery) -> Result<HistoryResult, StorageFailure>;
    fn plan_clear(&self, scope: ClearHistoryScope) -> Result<ClearPlan, StorageFailure>;
    fn apply_clear(&self, plan: ClearPlanToken) -> Result<ClearOutcome, StorageFailure>;
}
```

呼叫者只需知道 request、結果、錯誤與取消，不需知道 Provider protocol、SQL、重算順序或 chart 計算。這提升 Locality，避免 SQL、日期與去重規則散落在 Collector、Tauri、HTTP 與 JS。

`History::ingest` 是唯一 ingestion authority；Normalizer 只建立尚未持久化的 typed batch，不去重、不開 transaction。History 的內部 ingestion implementation 驗證並重算 FactIdentityV1、取得 repository transaction、寫 facts/cursors/health、重算 summary，最後一次 commit。SQLite Repository 不自行啟動巢狀 transaction。

### 6.3 建議檔案配置

```text
src/history/
  mod.rs
  model.rs
  normalize.rs
  ingestion.rs
  repository.rs
  sqlite.rs
  query.rs
  pricing.rs
  redaction.rs
  migrations/
desktop-p0/src/history.rs
desktop-p0/ui/history-model.ts
desktop-p0/ui/history-chart.ts
desktop-p0/ui/history-view.ts
tablet-ui/history.ts
tests/history_*.rs
tests/history_*.test.cjs
```

**[現有]** `src/live_dashboard.rs` 保持最新值；`src/tablet.rs` 的 `/api/v1/history` 從固定空結果改接 `History`；`desktop-p0/src/tablet_bridge.rs` 作為 Adapter，不可自行實作另一套 aggregation。H0 必須建立 TypeScript build/test/bundle pipeline，再新增 History 前端；若要正式接受無建置 JavaScript，需先修改 v1.1 需求與 ADR，不能默默延續偏差。

### 6.4 程序與排程

1. App 啟動：驗證設定 → DB backup／migration → 建立 History → 啟動 coordinator → 發布帶原 observed time 且先標 stale/unknown 的舊 snapshot → 非同步 collect；只有新 collect 成功才可標 fresh。
2. migration 失敗：fail closed；Collector 與 DB write 都不啟動，只提供診斷與還原。
3. 每 Source 有自己的 schedule、timeout、backoff 與 cancellation；單一慢 Source 不阻塞其他 Source。
4. 同一 Source 的重複 refresh 合併；正在執行時只記一個 follow-up generation。
5. collection 結束順序：停止新排程 → cancel → 等待有界 grace period → commit／rollback → shutdown subprocess → 關 DB。
6. 每次同步回傳 per-source `ok|partial|error|skipped`、scanned／parsed／inserted／duplicate／rejected counts 與最後錯誤；所有 Source 失敗不得回假成功。

---

## 7. API 契約

### 7.1 參考網站現況 API

**[實測＋上游]** 下表用於還原參考行為；不是 AgentMeter 對外相容承諾。

| Method／route | 回應／行為 |
|---|---|
| `GET /api/:assistant/dates` | `{dates}` |
| `GET /api/:assistant/setup-info` | platform、workspace/home 與各 Agent 路徑／安裝資訊 |
| `GET /api/:assistant/usage/:date` | date、summary、sessions、raw_entries |
| `GET /api/:assistant/usage/:date/session-search` | server-side Session search |
| `GET /api/:assistant/usage/:date/export` | 目前日期匯出 |
| `POST /api/:assistant/usage/:date/import` | 驗證並匯入，body 上限 200 MB |
| `GET /api/:assistant/imports` | 匯入批次 |
| `DELETE /api/:assistant/imports/:batch_id` | 回滾批次新增資料 |
| `GET /api/:assistant/session/:session_id` | metadata 與 timeline |
| `GET /api/:assistant/months` | `{months}` |
| `GET /api/:assistant/monthly/:year_month` | summary、daily、projects、models、agent breakdown |
| `GET /api/:assistant/model-sessions` | 模型下鑽 |
| `GET /api/:assistant/years` | `{years}` |
| `GET /api/:assistant/yearly/:year` | summary、monthly、projects、models、agent breakdown |
| `GET /api/:assistant/pricing` | pricing rows |
| `GET /api/:assistant/sync` | 手動同步 |
| `GET /api/:assistant/rate-limit` | 有資料才成功；無資料可 404 |

參考錯誤：不支援 Agent 為 400；找不到日期／Session 為 404。參考 `/sync` 雖使用 GET 卻會修改 SQLite；AgentMeter 不沿用此契約，所有排程、匯入、回滾與刪除皆使用 Tauri command 或 POST/DELETE 加 CSRF。目標 API 應使用穩定 machine-readable error code，而非讓前端解析中文句子。

參考 `usage` 的 canonical record 包含 timestamp、Session identity/name、transcript path、CWD、版本、turn、model/model_id、累積與 delta Token、context、cost、source identity、parent/nickname/role/reasoning effort；其中 `home_dir`、`raw_entries`、path 與 prompt-derived Session name 都是敏感面。AgentMeter 的一般 API 不直接回 raw record，只回經 `History` 投影且符合 privacy mode 的 DTO。

### 7.2 AgentMeter Tauri commands

| Command | Request | Response |
|---|---|---|
| `history_query` | `HistoryQueryDto` | `HistoryResultDto` |
| `history_session` | session ref、content mode | metadata、metric timeline 或授權後內容 |
| `refresh_sources` | source refs、reason、request id | queued/running/paused 與 generation；既有 `refresh_quota` 委派此 Coordinator，逐步移除 legacy refresh，不新增第三條路徑 |
| `history_export_preview` | query、format、privacy | manifest、size estimate、warnings |
| `history_export` | preview token、destination | checksum、row counts |
| `history_import_preview` | file handle | manifest、compatibility、counts、warnings |
| `history_import` | preview token | batch outcomes |
| `history_rollback_import` | batch id、confirmation | exact deleted counts |
| `history_plan_clear` | scope | 有效期限、query fingerprint、affected tables／counts、不可變 plan token |
| `history_apply_clear` | plan token、confirmation | deleted counts、recomputed summaries、preserved categories |

所有 DTO 帶 `schema_version`。Tauri error 映射到第 7.4 節 code；長查詢支援 cancellation。preview／clear plan token 綁定 scope、DB revision、manifest hash，5 分鐘失效，核心 `History` 會再次驗證，不依賴 UI 防護。Reset 涉及 SQLite、settings JSON 與 DPAPI PairStore，採可恢復 saga：先 plan／備份 → 撤銷 runtime sessions → 刪 History → 刪 settings/accounts → 最後清 PairStore；每一步寫 recovery marker，重啟後可繼續或還原，不宣稱跨 store 單一 transaction。

### 7.3 Tablet HTTP API

沿用現有 `/api/v1` security contract。

```http
GET /api/v1/history?provider=codex&from=2026-09-01&to=2026-09-12&granularity=day&limit=100
Authorization: Bearer <high-entropy-tablet-session>
```

建議成功回應：

```json
{
  "schema_version": 1,
  "query": {
    "provider": "codex",
    "account_ref": "opaque-account",
    "source_refs": ["opaque-source"],
    "from_inclusive": "2026-09-01T00:00:00+08:00",
    "to_exclusive": "2026-09-13T00:00:00+08:00",
    "timezone": "Asia/Taipei",
    "granularity": "day",
    "metrics": ["input_tokens", "output_tokens"],
    "unit": "token",
    "model": null,
    "sort": "period",
    "order": "asc"
  },
  "generated_at": "2026-09-12T00:00:00Z",
  "dataset_revision": 42,
  "bounded": true,
  "coverage_from": null,
  "coverage_to": null,
  "is_complete_for_requested_range": false,
  "truncated": false,
  "items": [],
  "next_cursor": null,
  "quality": {
    "complete": false,
    "reasons": ["example_no_events"]
  },
  "sources": [{"source_ref":"opaque-source","freshness":"stale","warning":"example_no_events"}]
}
```

限制：

- 每次最大 31 日 raw detail 或 366 日 summary；超出回 400 `range_too_large`。
- 預設 100、最大 500 summary items；Session 最大 200。
- response body 最大 2 MB；超出必須 pagination，不截成看似完整的資料。
- cursor 綁定 canonical query fingerprint、sort/tie-breaker 與 dataset revision；資料集變動使 cursor 失效並回 `cursor_stale`。
- tablet 不回完整 path、identity hash、prompt／response、tool payload 或 DB row id。
- tablet 永遠不提供 content route，即使 desktop 已啟用 content optional profile。
- 所有 history route 需 Tablet Session；Origin／Host 只是額外防護，不代替認證。
- 單一 Source `schema_changed` 不使整個 History query 失敗；回既有資料、`complete=false` 與 source-level warning。

### 7.4 錯誤 envelope

```json
{
  "error": {
    "code": "schema_changed",
    "message_key": "history.error.schema_changed",
    "retryable": false,
    "source_ref": "opaque-ref",
    "diagnostic_ref": "opaque-local-ref"
  }
}
```

允許 code：`invalid_request`、`unsupported_provider`、`not_found`、`range_too_large`、`cursor_stale`、`value_out_of_range`、`content_search_disabled`、`identity_collision`、`unauthorized`、`csrf_failed`、`paused`、`busy`、`source_unavailable`、`not_logged_in`、`timed_out`、`rate_limited`、`schema_changed`、`migration_failed`、`import_too_large`、`import_incompatible`、`storage_failed`。Client 對未識別的新 code 顯示安全的 generic error 與 correlation id，不以成功處理；錯誤不得回傳秘密、原始 payload 或完整本機路徑。

### 7.5 Summary／Session DTO 欄位

| DTO | 必要欄位 |
|---|---|
| `UsageSummaryDto` | sessions、total/input/output/cache_read/cache_write/reasoning tokens、duration_ms、request_count、official_cost、estimated_cost、currency、quality、complete |
| `SessionSummaryDto` | opaque session ref、source ref、model、turns、各 Token、duration、cost sources、start/end、parent ref、nickname/role、reasoning effort、project alias、quality |
| `DailyBreakdownDto` | date、各 Token、Session count、cost sources、quality、rebuildable |
| `PeriodBreakdownDto` | period、timezone、各 Token、Session count、cost sources、quality |
| `ProjectSummaryDto` | project alias、Session count、各 Token、cost sources；desktop opt-in 才可帶 display path |
| `ModelSummaryDto` | model、mode／reasoning effort、Session count、各 Token、cost sources、quality |
| `ProviderBreakdownDto` | provider、source/account refs、各 Token、Session count、cost sources、quality；不同 unit 不產生 combined total |
| `SyncOutcomeDto` | generation、overall status、逐 Source status、scanned/parsed/inserted/duplicate/rejected、duration、failure code |

Token 在 Rust／SQLite 使用經上限檢查的非負 i64；JSON 一律使用 canonical decimal string，避免超過 JavaScript `2^53-1`。Chart Adapter 將可見範圍正規化成安全數值，但 tooltip／表格保留精確字串。費用使用固定 scale 的 nano/minor unit 或 canonical decimal string，並定義 half-even rounding；任何 overflow 回 `value_out_of_range`，不得飽和截斷。`complete=false` 必須附 reasons。模型、nickname、role 與 alias 有長度上限並經 Unicode／控制字元正規化。

### 7.6 輸入驗證

- Assistant／provider、metric、unit、sort、order、granularity 只接受 allowlist。
- 日期用 calendar parser 驗證，`2026-02-31` 必須拒絕；不可只檢字串長度或日介於 1–31。
- Session／Source ref 僅接受 opaque token；若需定位檔案，canonical path 必須位於已登錄來源 root，並覆蓋 symlink、Windows case 與 UNC 測試。
- search query trim 後 1–200 字；metrics-only 模式直接回 `content_search_disabled`。
- cursor 綁定 query fingerprint 與 expiry，不能拿到另一 Account／Source 使用。
- import 先完成整檔結構與資源限制驗證，再開始 transaction；unknown major version fail closed。

---

## 8. 狀態機

### 8.1 Collection

```text
disabled ──enable──> probing ──ok──> ready ──schedule──> queued
   ▲                    │fail         ▲                    │
   │                    ▼             │                    ▼
   └────disable──── unavailable       └────success──── collecting
                                           │ fail
                                           ▼
                                    backing_off / stale
```

- `paused` 是跨 Source 的排程狀態，不覆蓋 Availability／Freshness。
- collecting 失敗保留舊 observation，更新 health 與 next retry。
- schema_changed 不自動無限 retry；需版本或使用者操作解除。

### 8.2 View request

```text
idle → loading(generation N) → ready | empty | stale | error
              │ target changes
              └→ abort N → loading(generation N+1)
```

只有當 response query scope 與當前 scope 完全相同且 generation 最新，才可 commit。切換時先清除前一 scope 的空資料／錯誤文案，但可在標明「上一筆」的 cache panel 保留舊值。

### 8.3 Import

```text
selected → validating → preview_ready → importing → completed
               │             │              │
               └ rejected    └ cancelled    └ partial_by_agent / failed
completed → rollback_preview → rolling_back → rolled_back
```

preview token 綁定 file checksum、schema、size、目標與 expiry；檔案在 preview 後改變即失效。

### 8.4 Session drawer

`closed → loading → metrics_ready → content_authorizing → content_ready → closed`。content 未授權時 API 不回正文；不是只在前端隱藏。

---

## 9. 視覺與互動規格

### 9.1 設計方向

**[上游]** TokenUsageInsights 的 north star 是 restrained、dense、traceable 的「Local Operations Console」。AgentMeter 應保留自身品牌，而採用下列原則：

- 資訊密度高但可掃讀；邊界、標題、數字 alignment 比裝飾重要。
- local-first 與 freshness 是一級資訊，不藏在 tooltip。
- cyan／blue 用於互動與主要資料，purple 區分特殊 series；success／warning／danger 只表達狀態。
- 避免過度 glow、glass、gradient、dashboard grid；動畫只用於 state transition。

### 9.2 Token 建議

| 類型 | Dark | Light |
|---|---|---|
| app background | `#07090D` | `#F6F8FB` |
| sidebar／surface | `#0B0E14`／`#0D1118` | `#FFFFFF` |
| primary text | `#F4F7FB` | `#172033` |
| secondary／muted | `#C8D2DF`／`#94A3B8` | `#475569` |
| border | `rgba(219,228,238,.11)` | `rgba(23,32,51,.12)` |
| accent | `#00D2E0`／`#2D8CFF` | 相同但需 4.5:1 對比驗證 |
| success/warn/danger | `#31D0AA`／`#F6BE4F`／`#FF5C8A` | 需搭配 icon／文字，不只靠色彩 |

Live site 載入 IBM Plex Sans／Mono，而公開 DESIGN 文件描述的字體基線不同。**[決策 D-02]** AgentMeter 應沿用現有品牌字體與 system fallback，不把第三方 CDN 字體設為執行必要條件。數字使用 tabular numerals。

Spacing：4、8、12、16、20、24、32 px。Radius：4、6、8、10、12、14 與 pill。Touch target 至少 44×44 px。

### 9.3 響應式

| 寬度 | Layout |
|---|---|
| `> 1200` | 280–292 px 固定側欄；內容 max-width；5 張摘要卡；完整 Sessions 表 |
| `993–1200` | 可收合側欄；摘要卡 3+2；次要欄可隱藏到 row detail |
| `641–992` | 側欄成 drawer；header 可兩行；卡片 2 欄；表格優先 sticky first column 或 card list |
| `<= 640` | 單欄；工具進 menu；摘要卡 stack；圖表全寬；Sessions card list；無水平捲動 |

**[實測]** 390×844 時參考網站能將側欄收成 drawer、header 換行、empty actions 疊放。AgentMeter 還需驗證既有最低 640×520 desktop、高 DPI 與 Tab S6 Lite 直向＋200% font scaling。

### 9.4 無障礙

- WCAG 2.2 AA；一般文字 4.5:1，大字 3:1，非文字狀態 3:1。
- 使用語意 landmark、heading order、真正 button/table/dialog；不得以可點 div 代替。
- tabs 支援方向鍵、Home、End；表格排序以 `aria-sort`。
- dialog focus trap、Escape、return focus；背景 `inert`。
- 圖表提供 summary、data table、keyboard focus 與非色彩 series pattern。
- live sync 以低干擾 `aria-live=polite`；倒數每秒變化不逐秒朗讀。
- `prefers-reduced-motion` 停用圖表 transition、drawer slide 與 pulse。
- 文字放大 200% 不遮住按鈕或造成雙向捲動。

---

## 10. 安全、隱私與威脅模型

### 10.1 關鍵結論

**[上游]** 參考服務的預設 bind host 是 `0.0.0.0`，而歷史／Session routes 沒有等同 AgentMeter Tablet Session 的認證。CORS allowlist 只能限制瀏覽器讀取，不能阻止同網路 client 直接發 HTTP，因此不可沿用此部署預設。

**[目標]** AgentMeter：

1. 桌面 WebView 透過 Tauri command；不需要公開 HTTP。
2. Tablet server 只有使用者啟用平板功能時 listen `127.0.0.1`，再透過 owned ADB reverse。
3. 所有私人 HTTP route 需要高熵 Tablet Session；refresh／destructive route 另需 CSRF。
4. 任何 LAN mode 都是未來獨立產品能力，需 TLS、認證、權限、rate limit 與新威脅模型。

### 10.2 威脅與控制

| Threat | 風險 | 必要控制 |
|---|---|---|
| LAN client 讀取歷史 | prompt、路徑、用量外洩 | loopback-only、session auth、LAN negative test |
| 惡意網頁 CSRF | 觸發 refresh／刪除／匯入 | SameSite/HttpOnly session、Origin/Host、CSRF token |
| 匯入惡意檔 | OOM、zip traversal、JSON bomb | 200 MB 雙重限制、stream parse、depth/count limit、無 path extract |
| Markdown／tool output XSS | WebView code execution | DOMPurify 等 allowlist sanitizer、CSP、禁止 dangerous URL |
| Session 內容洩漏 | secrets、code、personal paths | metrics-only default、opt-in、redactor、retention、export exclusion |
| SQLite migration 損壞 | 歷史不可恢復 | migration 前 backup、checksum、最多三份、fail closed、restore test |
| Source schema drift | 錯誤數字 | versioned parser、fixture、schema_changed、舊值 stale |
| Account 誤合併 | 跨帳號資料污染 | stable keyed identity／Account Link、可撤銷、scope test |
| 日誌注入 | 診斷偽造／秘密輸出 | structured log、escape、redact、no raw payload |
| 公式／價格篡改 | 成本誤導 | signed/bundled catalog 或 checksum、版本與 effective date |
| 遠端 runtime asset | CDN compromise 可用同源權限讀 Session API | Chart／Markdown／sanitizer／font 全部 vendored、lockfile、CSP；離線驗收 |
| Markdown 遠端資源 | 開啟 Session 時向第三方洩漏 IP／referer／追蹤碼 | 移除 remote image/resource，自動載入一律禁止；只顯示需確認的外連 |

### 10.3 Web security headers

即使只在 loopback，Web response 應設定：

- `Content-Security-Policy`：預設 `default-src 'self'`；不依賴公開 CDN，必要 asset bundled。
- `X-Content-Type-Options: nosniff`
- `Referrer-Policy: no-referrer`
- `frame-ancestors 'none'`（CSP；必要時另加 `X-Frame-Options: DENY`）
- 私人資料 `Cache-Control: no-store`
- 嚴格 allowlist methods／content types；不使用 `Access-Control-Allow-Origin: *`。

參考網站會在 runtime 載入 Google Fonts 與 jsDelivr 的 Chart.js／marked／DOMPurify，且沒有 CSP。AgentMeter 不沿用此供應鏈邊界；所有執行期資源與字體隨安裝包提供並納入 SBOM。Sanitizer 不可用時必須 fail closed 為純文字 escape。

完整 Source path 只存在 desktop 設定 store，History DB 只存 opaque SourceRef 與 keyed alias；tablet 永遠不可取得完整 path 或 content。project/CWD alias 以 DPAPI 保護金鑰的 HMAC＋使用者可編輯 display label 建立，碰撞時附穩定短 suffix，重新命名不改 identity。若 optional content profile 未另行採用 DPAPI envelope encryption，必須清楚標示只依賴 Windows ACL／磁碟保護。FTS／prompt index 屬 content 資料，與 content table 同 retention、同 clear transaction，並測試 index page 無殘留。

### 10.4 資料操作邊界

| 操作 | 刪除 | 保留 | 確認 |
|---|---|---|---|
| Clear History | usage、quota、health history、alerts、session metrics/content | settings、accounts、links、pairs | preview＋一次確認 |
| Forget Account | 指定 account、links、其 history | 其他 account、pairs、全域設定 | 輸入顯示名稱確認 |
| Clear Pairing | Tablet Sessions、Device Pairs | history、accounts、settings | 一次確認 |
| Reset AgentMeter | 全部應用資料 | 無（卸載 binary 另計） | 二次確認＋明確列出路徑 |

---

## 11. 非功能需求

除真機項目外，基準機至少為 Windows 11 x64、4 logical cores、16 GB RAM、NVMe SSD、Balanced 電源模式與當期支援的 WebView2 stable；報告必須記錄實際 CPU、RAM、磁碟、OS、WebView2 與 build SHA。Cold query 以新程序且清除應用 cache 執行，warm query 在相同資料集連續執行；fixture 為四個 Core Provider、至少 8 個 Source、1M events，含 10% out-of-order、5% duplicate、1% anomaly 與跨時區資料。

| ID | 需求 | 量測 |
|---|---|---|
| NFR-TA-01 | cached daily view 300 ms 內可互動 | 20 次 p95，SSD 驗收機 |
| NFR-TA-02 | 31 日 summary query p95 < 200 ms | 100k usage events、cold/warm 分列 |
| NFR-TA-03 | 一年 monthly summary p95 < 300 ms | 1M events fixture；不得前端聚合 raw rows |
| NFR-TA-04 | Session page p95 < 250 ms | 50 rows，indexed cursor query |
| NFR-TA-05 | Collector 不阻塞 UI 或其他 Source | 一個 Source 60s timeout 時其他 Source 正常更新 |
| NFR-TA-06 | 匯入記憶體有界 | 200 MB streaming import 相對穩態 working set 增量 <128 MB，無一次載入整檔 |
| NFR-TA-07 | ingestion 冪等 | 同檔重掃／App restart／重匯入，summary 不變 |
| NFR-TA-08 | crash consistency | transaction 中強制終止後 DB integrity 與 batch 狀態一致 |
| NFR-TA-09 | 長期穩定 | 暖機 1 小時後 8 小時 working-set 線性 slope <1 MB/小時、結束值不超過基線 10%，0 crash／DB corruption |
| NFR-TA-10 | 離線可用 | 已收集歷史、設定與圖表不依賴 CDN／外網 |
| NFR-TA-11 | 可觀測 | 每次 collect／ingest／query 有 correlation id、duration、counts、result code；無秘密 |
| NFR-TA-12 | 相容 | Windows 11 x64、Tauri 2/WebView2；正式支援下限依 P0 VM 鎖定 |
| NFR-TA-13 | RWD | 640×520 desktop、100–200% DPI、Tab S6 Lite portrait、390×844 無水平捲動 |
| NFR-TA-14 | 可回復 | migration／import／clear 均有備份或精確 transaction rollback |

8 小時 soak 固定 workload：每 Source 依最低 120 秒 cadence collect、本機 view 每 10 秒刷新、每 30 秒交替查 daily/monthly/yearly、每小時 USB reconnect、每 2 小時注入一個 Source timeout/schema error；允許預期的 source-level error，但 UI/API 不得 false-success、資料不得重複或跨帳號。

---

## 12. 驗收測試矩陣

### 12.1 資料正確性

| AC | 情境 | 通過條件 |
|---|---|---|
| AC-TA-01 | 同一來源事件重讀 3 次 | `usage_events` 與 summary 只增加一次 |
| AC-TA-01A | 同一 source event 含多 model／scope／相同 metric | canonical fact key 均唯一且完整保存；NULL 維度不繞過 uniqueness |
| AC-TA-01B | 同 event key 收到不同 canonical payload | 兩者 quarantine、summary 不變、health 記 `identity_collision` |
| AC-TA-02 | counter 100→140→10 且無 reset 證據 | 只記 +40；下降標 anomaly，不記 -130 或 +10 |
| AC-TA-03 | counter reset 有新 session 證據 | 開新 segment，依起點規則計算，不與舊 counter 相減 |
| AC-TA-03A | 晚到、同 timestamp、檔案補寫事件 | 依 source sequence 插入並只重算受影響 segment；結果與一次按權威順序 ingestion 相同 |
| AC-TA-04 | input 含 cache 的 Provider | 依來源語義建模，不重複扣除／加總 cache |
| AC-TA-05 | quota 連續 snapshots | trend 顯示快照，summary 不對 percent 求和 |
| AC-TA-05A | 同 Account/bucket 兩 Source 回報不同 | 兩筆都保留，effective view deterministic 且顯示 conflicting，不平均 |
| AC-TA-06 | null、0、missing 三種 fixture | UI 分別顯示未知、0、缺欄原因，不互換 |
| AC-TA-07 | Asia/Taipei 23:59／00:01 事件 | 分到正確日期；切換 timezone 可重建範圍正確 |
| AC-TA-08 | 兩 Account 同 model 同時使用 | filters 與 summary 不交叉；未 link 不產生 combined view |
| AC-TA-09 | official 與 estimated cost 同時存在 | 分列且總卡預設只採 official |
| AC-TA-10 | raw retention 後查舊 summary | 顯示 summary 原 timezone 與不可完整重建狀態 |
| AC-TA-10A | `+05:30`、`+09:30`、DST 轉換與跨日事件 | RFC3339 instant 與 IANA bucket date 正確，不以文字前綴／檔名分日 |
| AC-TA-10B | statusline partial final line | 第一次不解析且 cursor 不前移；補上換行後恰好匯入一次 |
| AC-TA-10C | Token 在 `2^53-1` 上下與 i64 邊界、精細費用 | JSON decimal string 往返無精度損失；超出 i64 安全拒絕；rounding 符合版本規則 |

### 12.2 UI／互動

| AC | 情境 | 通過條件 |
|---|---|---|
| AC-TA-11 | 快速連切 Codex→Claude→Antigravity | 只顯示最後目標 loading／結果，無前一 Provider 文案或資料閃現 |
| AC-TA-12 | URL 重開／Back／Forward | Agent、tab、date、chart、filters 可重現且不產生循環 history |
| AC-TA-13 | 未來日期 | next disabled；手改 URL 顯示 validation，不查錯誤範圍 |
| AC-TA-14 | empty／stale／error／paused | 文案、動作與保留資料符合第 3.9 節 |
| AC-TA-15 | K 線無事件與未來 bucket | 過去 gap、未來不畫、無假 0；等價資料表一致 |
| AC-TA-16 | Session drawer 鍵盤操作 | focus trap、Escape、return focus、screen reader 名稱通過 |
| AC-TA-17 | 390×844 與 200% 字體 | 無水平捲動、操作可達、重要數字不截斷 |
| AC-TA-18 | prefers-reduced-motion | 無非必要 transition/pulse，資訊不損失 |

### 12.3 安全／隱私

| AC | 情境 | 通過條件 |
|---|---|---|
| AC-TA-19 | 同 LAN 未配對 client | 無法連線或取得任何私人資料；health 不洩漏 account／usage |
| AC-TA-20 | 錯誤 session／Origin／CSRF | 私人讀取與 mutation 皆拒絕；log 無 token |
| AC-TA-21 | 惡意 Markdown／URL／tool output | 無 script、event handler、dangerous URL 執行 |
| AC-TA-22 | metrics-only 預設 | DB、API、diagnostics、export 不含 prompt、response、code、完整 path |
| AC-TA-23 | content opt-in 關閉與清除 | 新內容停止寫入；既有 content 依 preview 精確刪除 |
| AC-TA-24 | 200 MB、深層 JSON、zip bomb、path traversal | 安全拒絕，無 OOM、無 workspace 外寫入、無 partial batch |
| AC-TA-25 | migration backup 失敗 | Collector／DB write 不啟動，可診斷與還原 |
| AC-TA-25B | WAL 有未 checkpoint 資料時 backup/restore | online backup checksum/integrity 通過且資料完整；任一強制終止點可安全恢復 |
| AC-TA-25A | sanitizer／CDN 全部不可用 | bundled app 仍可離線運作；惡意內容只以安全純文字顯示 |

### 12.4 匯入／清除／API

| AC | 情境 | 通過條件 |
|---|---|---|
| AC-TA-26 | 同 export 重試匯入 | duplicate skipped，summary 不變 |
| AC-TA-26A | native/imported 同 fact、不同 external id，匯入宣稱 official | canonical hash 去重；native ownership 不變；import effective quality 不升為 official |
| AC-TA-27 | 多 Agent 匯入第二批失敗 | 第一批成功保留，第二批無 partial rows，結果逐 Agent 清楚 |
| AC-TA-28 | 回滾匯入 | 只刪 batch 擁有的 facts 並重算衍生資料；非本批邏輯 rows 與值完全相同（不要求 SQLite 檔案 byte-for-byte） |
| AC-TA-29 | tablet 超大 query | `range_too_large` 或 pagination；不截斷成 `complete=true` |
| AC-TA-30 | 四種清除操作 | 各自只影響第 10.4 節定義範圍，counts 可核對 |

### 12.5 建議自動化命令

現有基線：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify-local.ps1
cargo test --offline --locked --test cli_codex_collection --test dashboard_streaming --test tablet_http
cargo test --manifest-path desktop-p0/Cargo.toml --offline --locked
node --test tests/desktop_layout.test.cjs tests/tablet_protocol.test.cjs tests/tablet_view.test.cjs desktop-p0/tests/auto-quota-ui.test.cjs
```

History 完成後新增：

```powershell
cargo test --offline --locked --test history_migrations
cargo test --offline --locked --test history_ingestion
cargo test --offline --locked --test history_aggregation
cargo test --offline --locked --test history_api
node --test tests/history_model.test.cjs tests/history_chart.test.cjs
node scripts/verify-history-responsive.cjs
```

模擬資料只證明 UI／演算法；每個 stable Collector 仍需真實帳號同時間來源對照，且 fixture 去識別化。

---

## 13. 開發階段、風險與回滾

### 13.1 Thin vertical slices

| 階段 | 內容 | Exit criteria |
|---|---|---|
| H0 決策／基線 | 採用第 14.2 節 Core profile、建立 TypeScript pipeline、凍結 DTO／event identity | ADR／CONTEXT 更新；baseline tests 綠燈 |
| H1 可信資料 | typed ObservationBatch、單一 ingestion、SQLite migration、dedup、counter、timezone | AC-TA-01～10 與其所有 suffix、19、22、25／25A／25B；尚不顯示 K 線 |
| H2 Daily MVP | daily query、摘要、trend、Session metrics、URL state、empty/error | AC-TA-11～14、16、29 |
| H3 月／年與平板 | summaries、projects/models、authenticated tablet history | AC-TA-07、10、17、19、20、29 |
| H4 Core I/O | canonical/summary export、import/history/rollback | AC-TA-24、26～28 |
| O1 K 線 prototype | optional `kline-prototype` profile | AC-TA-15；獨立 prototype DoD，不阻塞 Core v1 |
| O2 估價／內容 | optional `estimated-cost`、`session-content` profiles | 各自 ADR、安全與獨立 DoD，不阻塞 Core v1 |
| H5 產品化 | retention、clear、8h/perf/a11y | 全部 Core AC/NFR 與實帳證據 |

### 13.2 風險等級

整體風險：**高**，原因是本機內容隱私、資料 migration、跨帳號污染與累積 counter 錯算會造成不可見且長期的資料錯誤。

| Risk | Mitigation | Rollback |
|---|---|---|
| typed path 與舊 JSON path 並存造成雙寫 | H1 設單一 ingestion authority；feature flag 禁止 production 雙寫 | 關閉 history flag，保留 live dashboard |
| SQLite migration 失敗 | preflight backup、transaction、version check、fail closed | 還原最新成功 backup 與舊 binary |
| summary 算錯 | raw events 在 retention 內可重建；保存 algorithm version | 清 summary 後由 raw rebuild |
| performance 不足 | bounded query、indexes、precomputed summary、profiling | 關閉明細／縮小範圍，不降資料正確性 |
| content 隱私 | metrics-only default、separate table/key、redaction | 全域關閉並清 content table，不刪 metrics |
| 上游 parser 改版 | schema version＋fixtures＋maturity downgrade | 停用該 Source，保留 stale snapshot |

### 13.3 部署與監測

- History 先以 disabled-by-default feature flag 進 preview。
- migration 只在啟動前執行；顯示 backup 路徑 alias 與還原按鈕。
- 監測：collect duration/result、events seen/inserted/duplicate/anomaly、query duration/rows、DB size、migration version、import outcomes；禁止高基數 prompt/path label。
- H1/H2 期間保留可一鍵回到純 `LiveDashboard` 的路徑。
- 正式啟用前做 8 小時 soak、強制終止 transaction、睡眠／喚醒、來源 rotation、USB 重插與 LAN negative tests。

---

## 14. 現況差距與待決事項

### 14.1 AgentMeter 現況差距

| 能力 | 現況 | 需要變更 |
|---|---|---|
| 即時 quota | 已有 `LiveDashboard`、revision、stale-on-failure | 保持 read model；補齊不應遺失的 quota metadata |
| Collector | Rust、Node helper 與 legacy dashboard 多路徑 | 收斂成 typed `Collector`＋單一 ingestion |
| 歷史儲存 | settings JSON／pair store；無 SQLite history | 建立 schema、migration、backup、retention |
| History API | `/api/v1/history` 固定空陣列 | 注入 `History`，沿用 Bearer 認證與 bounds |
| 桌面 History UI | 無頁面／router／chart | 建 view model、URL state、trend first |
| 平板 History UI | 無 | 共用 DTO；不暴露敏感欄位 |
| Session | 無內容策略與儲存 | metrics-only first；content opt-in later |
| Cost | v1 禁止自行估價 | Core v1 只顯示 official；估價為 optional profile |

### 14.2 Core v1 與 optional profiles

以下是本規格已採用的 Core 決策；變更必須另立 ADR，不是交給實作者臨場選擇。

| Decision | Core v1 | Optional profile／影響 |
|---|---|---|
| D-01 Estimated cost | 不計算、不保存、不顯示 | `estimated-cost`：需 catalog/model/currency/formula/法律文案驗收 |
| D-02 Typography | AgentMeter bundled/system font | 換字體需品牌、離線 asset 與授權驗收 |
| D-03 Session content | metrics-only；不建 content/FTS | `session-content`：opt-in、desktop-only、加密/ACL、redaction/retention/clear 驗收 |
| D-04 K 線 | 不屬 Core；trend 為預設 | `kline-prototype`：依第 3.4.2 節，不能標 stable |
| D-05 Providers | Claude、Codex、Copilot、Antigravity | 其他 Provider 各自需要 P0 實帳、fixture、維護矩陣 |

---

## 15. Traceability

| 本規格 | AgentMeter v1.1 對應 | 實作證據目標 |
|---|---|---|
| 第 5 節資料不變量 | §4、§10.1 | history ingestion／aggregation tests |
| 第 6 節架構 | §9、ADR 0005/0006/0011 | typed modules、Tauri/HTTP adapters |
| 第 7.3 節平板 API | §7.3、§10.2、ADR 0003/0004/0010 | authenticated tablet history tests |
| 第 10 節安全 | §11 | LAN/CSRF/XSS/privacy negative tests |
| 第 11 節 NFR | §12 | benchmark、responsive、8h evidence |
| 第 12 節驗收 | §13 | CI commands＋真機／實帳 evidence |
| 第 13 節 rollout | §14、§15 | feature flag、backup/restore、monitoring |

---

## 16. 參考與證據索引

### 16.1 上游 primary sources

- [TokenUsageInsights README](https://github.com/doggy8088/TokenUsageInsights)：功能、支援來源、匯入匯出、執行與驗證方式。
- [TokenUsageInsights DESIGN](https://github.com/doggy8088/TokenUsageInsights/blob/main/DESIGN.md)：Local Operations Console、色彩、間距、響應式與無障礙方向。
- [TokenUsageInsights `main.rs`](https://github.com/doggy8088/TokenUsageInsights/blob/main/src/main.rs)：Axum routes、背景同步、body limit、CORS 與 bind host。
- [TokenUsageInsights `db.rs`](https://github.com/doggy8088/TokenUsageInsights/blob/main/src/db.rs)：SQLite persistence 與 query implementation。
- [固定分析基準 commit](https://github.com/doggy8088/TokenUsageInsights/tree/c15cebc1b27d42ab6f78a4fb15a7583aecf657a1)：本文件靜態原始碼證據的版本錨點。

### 16.2 AgentMeter authoritative local sources

- [`AgentMeter-Requirements-v1.1.md`](../AgentMeter-Requirements-v1.1.md)：產品、資料、安全、NFR 與驗收基線。
- [`CONTEXT.md`](../CONTEXT.md)：Provider、Account、Source、Observation、Quality、Freshness 等領域語言。
- [`src/model.rs`](../src/model.rs)：現有 typed domain。
- [`src/live_dashboard.rs`](../src/live_dashboard.rs)：即時 snapshot、revision、stale-on-failure read model。
- [`src/tablet.rs`](../src/tablet.rs)：loopback HTTP、pair/session、Origin/CSRF、SSE 與 history route seam。
- [`desktop-p0/src/tablet_bridge.rs`](../desktop-p0/src/tablet_bridge.rs)：桌面與 tablet server Adapter。
- [`docs/adr/0004-use-authenticated-loopback-http-over-adb-reverse.md`](./adr/0004-use-authenticated-loopback-http-over-adb-reverse.md)：平板安全邊界。
- [`docs/adr/0007-preserve-the-timezone-of-each-daily-summary.md`](./adr/0007-preserve-the-timezone-of-each-daily-summary.md)：summary timezone。
- [`docs/adr/0011-fail-closed-on-database-migration.md`](./adr/0011-fail-closed-on-database-migration.md)：migration fail-closed。

---

## 17. Definition of Done

Core v1 Token 分析中心只有在以下全部成立時才可標示完成：

- 行為符合 Core FR、資料不變量與第 14.2 節決策；optional profiles 預設未安裝／關閉。
- typed ingestion 是唯一歷史寫入權威，無 legacy 雙寫。
- migration、backup、restore、dedup、counter、timezone、Account 隔離與 retention 測試通過。
- desktop 與 tablet 使用同一 History query 語義；private routes 認證、bounded 且通過負向測試。
- metrics-only 預設下，DB、API、log、diagnostics、export 都沒有對話正文、secret 或完整個人路徑。
- Daily／Monthly／Yearly、trend、Session、empty/stale/error/paused、URL state 與 RWD 驗收通過。
- targeted tests、完整 local verification、效能、8 小時 soak、實帳來源對照與 Tab 真機證據均已保存。
- 發布說明包含 schema version、migration、feature flag、已知限制、rollback 指令與監測訊號。

Optional profile 各有獨立 DoD：

- `kline-prototype`：第 3.4.2 節 OHLC、ordering、DST、gap、viewport、下鑽、a11y table 與 AC-TA-15 全數通過，UI 仍標 experimental。
- `estimated-cost`：lineage、catalog/version/effective interval、model mapping、currency、rounding、重算與官方費用分離通過；不得改寫 Core facts。
- `session-content`：desktop-only explicit consent、at-rest policy、redaction、XSS/remote-resource、FTS retention/clear/export exclusion 與負向測試全數通過。
- 新 Provider：完成 v1.1 定義的實帳、去識別 fixture、scope/quality、schema drift、failure 與同時間官方對照證據。
