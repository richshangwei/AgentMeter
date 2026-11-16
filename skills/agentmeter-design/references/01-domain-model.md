# 01 領域模型與不變量

> 目錄：詞彙 → 五條正交狀態軸 → Failure Code → Observation 結構 → 核心不變量

這份文件定義「說話的方式」。所有其他規則都建立在這些詞上；詞用錯，後面的規則就會悄悄失效（例如把 `official` 當成 `stable`，就會在 UI 上替實驗性來源背書）。

## 1. 詞彙（括號內為禁用同義詞）

| 詞 | 定義 | 禁用 |
|---|---|---|
| **Provider** | 被監控的外部 AI 服務。AgentMeter 固定四家：`codex`、`claude`、`copilot`、`antigravity` | Agent、service、vendor |
| **Provider Account** | Provider 端可被外部辨識的帳號。一個帳號可被多個 Source 觀察 | Login、profile、user |
| **Account Link** | 使用者確認、可撤銷的「這些 Source 屬於同一帳號」聲明。**絕不由路徑、顯示名稱或「目前只有一個帳號」推論** | Account merge、automatic match |
| **Source** | 帳號在某個本機環境的可觀察存在（原生 Windows、某個 WSL distribution） | Provider、Collector、installation |
| **Quota Window** | Provider 定義的限額期間，有自己的單位、用量、剩餘量、重置時間 | Balance、total quota、limit |
| **Observation** | 帶時間戳、範圍、品質、新鮮度的 Provider 量測；可以是未知，**絕不以 0 或滿額補值** | Reading、result、current value |
| **Data Quality** | 數值的權威來源：`official`、`local_observed`、`estimated`、`manual` | Reliability、maturity、confidence |
| **Collector Maturity** | 整合的支援程度：`stable`、`experimental`、`unsupported`。與 Data Quality 獨立 | Data quality、provider status |
| **Availability** | Source 在當前環境能否被收集，與收集動作、資料新舊無關 | Status、health、freshness |
| **Collection State** | Source 當前的收集活動：`idle`、`collecting`、`ready`、`backing_off`、`error` | Availability、freshness |
| **Freshness** | Observation 對該 Source 的更新方式而言是否夠新 | Health、reliability、maturity |
| **Usage Series** | 只由「相同帳號＋scope＋metric＋unit」的可比較 usage 組成的歷史 | Daily total、combined usage |
| **Quota Trend** | 單一 Quota Window 的時間序列；快照描述狀態，**絕不相加** | Usage total |
| **Daily Summary** | 日分桶結果，保存分桶時使用的時區，不因顯示時區改變而被重新詮釋 | Daily total、global usage |
| **Dashboard Snapshot** | 某一時刻所有 Provider 的完整、帶 revision 的視圖。新的 `stream_id` 代表 App 重啟 | Provider snapshot、SSE patch |
| **Over Limit** | 回報用量超過上限的 Quota Window。主值顯示 0，原始超額保留 | Negative remaining |
| **Monitor-only Tablet Client** | 只能看、能請求受節流刷新；不能設定、登入、清資料、執行命令、控制 Provider、退出 App | Read-only tablet、remote control |
| **Device Pair** | 授權給單一平板瀏覽器 profile 的長效授權 | Session、ADB connection |
| **Tablet Session** | 由 Device Pair 衍生的短效、輪替授權 | Pairing code、login |
| **History** | 已存的 usage、quota、health、alert 紀錄。清除 History 保留設定、帳號、Link、Pair | All data、settings、cache |
| **Forget Account** | 明確移除單一帳號、其 Link 與其全部 History | Logout、disable source |
| **Reset AgentMeter** | 需獨立確認，回到初始狀態（History、身分、設定、配對全清） | Clear history、uninstall |
| **Setup Required** | 需要使用者明確同意的設定後才能收集（為了不破壞既有流程） | Unsupported、error |
| **Paused** | 使用者選擇停止收集／重試／通知，但保留既有 Observation 並標示 paused，直到明確 Resume | Offline、stopped、stale |

## 2. 五條正交狀態軸

**為什麼正交：** 單一 status enum 會逼你替每個組合命名，也會丟資訊。「被限流且資料過期」「可用但退避中」都是真實狀態；用正交欄位可同時表示（ADR 0009）。

| 軸 | 值 |
|---|---|
| `availability` | `available` · `not_installed` · `setup_required` · `needs_login` · `unsupported` · `permission_denied` |
| `collection_state` | `idle` · `collecting` · `ready` · `backing_off` · `error` |
| `freshness` | `unknown` · `fresh` · `stale` |
| `failure_code` | `null` 或 §3 的代碼（解釋失敗，不取代 state） |
| `collector_maturity` | `stable` · `experimental` · `unsupported` |

另外獨立於上述的：`data_quality`（權威來源）、USB 連線狀態（`usb_attached` / `adb_authorized` / `reverse_healthy` / `browser_session_online`）。**通道在線 ≠ 資料新鮮**，兩者必須同時顯示、不得互相覆蓋。

> 實作現況的偏離（遇到時照實處理，不要「修正」成規格）：初始空白 Provider 視圖使用 `availability: "unknown"`、`data_quality: "unknown"`、`setup: "required"`；Copilot 曾出現 `freshness: "provider_reported_period"`。詳見 `09-decisions-and-drift.md`。

### Freshness 判定

- 一般輪詢 Source：超過 `max(3 × 週期, 5 分鐘)` 未成功 → `stale`。
- 事件驅動 Source（例如 Claude statusLine）：以實際活動與 health 判斷，顯示「最後活動 Observation」，**不套用虛構的輪詢週期**。
- 收集失敗但有舊值：保留舊值並設 `stale`；從未成功過則為 `unknown`（不是 `stale`）。
- 電腦喚醒後：先把舊 Observation 標 stale，再重新收集；**不能把重置倒數歸零當成官方額度已恢復**（額度到期但查詢失敗 → 顯示「待確認」）。

## 3. Failure Code 全集（來自程式實際使用）

| 層 | 代碼 |
|---|---|
| Codex 協定層 | `authentication_failed`、`method_unsupported`、`timeout`、`cancelled`、`malformed_response`、`schema_changed`、`process_exited`、`permission_denied`、`rate_limited`、`request_failed`、`collector_error` |
| 桌面收集器層（顯示給使用者的 reason 白名單） | `authentication_required`、`cli_not_found`、`workspace_trust_required`、`collector_runtime_missing`、`terminal_probe_unavailable`、`terminal_quota_timeout`、`terminal_schema_changed`、`quota_schema_unrecognized`、`cli_quota_request_failed`、`permission_denied`、`timeout`、`quota_missing`、`quota_invalid`、`collector_response_invalid`、`cancelled`；其他一律折成 `collector_failed` |
| Claude statusLine | `malformed_payload`、`schema_changed`、`missing_quota`（合法事件但沒有 quota 欄位） |
| 平板刷新 | `refresh_failed` |

**白名單折疊的理由：** 失敗原因會顯示在 UI 上，未知字串可能夾帶路徑、token 或帳號。只放行已知代碼，其餘一律 `collector_failed`。

reason → availability 對映（桌面收集器）：`authentication_required → needs_login`、`cli_not_found → not_installed`、`workspace_trust_required → setup_required`、其餘 → `available`。

Codex 錯誤分類順序：`-32601` → `method_unsupported`/`unsupported`；訊息含 `auth`/`login` → 需登入；含 `permission`/`forbidden` → `permission_denied`；`timeout`；`schema`/`malformed`；`process_exited`；`rate limit`/`429` → `rate_limited`；其餘 `request_failed`。不支援的方法**回報一次、不重試**。

## 4. Observation 結構（最小必備）

```
provider, provider_account(masked), source{kind,version,mode,replay},
quota_windows[], credits[], source_usage,
data_quality, collector_maturity, availability, collection_state, freshness,
source_timestamp (可為 null), collected_at
```

- 原始值、正規化值、**來源時間**、**本機蒐集時間**分開保存。Claude statusLine 沒有量測時間戳，必須寫 `source_timestamp: null` 並另存 `observed_at`（接收時間）——接收時間不得偽裝成 Provider 來源時間。
- Fixture 重放必須明確標 `mode: fixture_replay`、`replay: true`、`data_quality: local_observed`、`freshness: unknown`，**fixture 只是回歸證據，不是 Provider 證明**。
- 帳號識別顯示前必須遮罩（例：`j***@example.com`）；穩定 Provider ID 只存 keyed hash。

## 5. 核心不變量（任何實作都不得違反）

1. **Unknown ≠ 0**：缺值一律 `null`，UI 顯示「尚未取得」或「—」，不補 0、不補 100%。
2. Data Quality、Collector Maturity、Availability、Collection State、Freshness 彼此正交。
3. Usage 只加總相同帳號＋scope＋metric＋unit 的可靠差量；Quota 是快照，只能看趨勢、期末值、期間最低值與重置事件，**絕不相加**。
4. Token、Credits、請求數、限流百分比分開呈現，不建立沒有共同基礎的「四家總剩餘量」。
5. 帳號合併需要穩定 identity 或可撤銷的 Account Link。
6. 時間以 UTC 儲存；每個 Daily Summary 保存 `bucket_timezone`（預設 Asia/Taipei）。
7. Parser 看不懂新版 schema → fail closed：保留舊快照、標 `schema_changed`、**不得把舊數字顯示為成功**。錯誤的數字比沒有資料更危險。
8. 矛盾的聚合（例如已定價 Token 大於已觀察 Token）→ 隱藏成本與覆蓋率，不夾住、不飽和成看似合理的值。
9. 第三方資料庫與日誌**唯讀**；不在來源目錄建立 cursor、lock 或 migration 檔。
10. 從 prompt 衍生的 `session_name` 仍是敏感內容，即使只有前 100 字。
11. 單一 Source 失敗不得阻塞其他 Source。
12. 估算（`estimated`）與官方（`official`）永不相加；不產生 `total_cost = official + estimated`。
