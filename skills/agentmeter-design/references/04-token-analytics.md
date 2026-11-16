# 04 Token 歷史分析與成本估算

> 目錄：範圍與隱私 → 讀取界限 → Token 流量正規化 → 成本估算 → 效率訊號 → 累積 counter → 保存與時區

**定位：** 配額監控回答「還剩多少」；Token 分析回答「用在哪裡」。兩者資料來源不同、不得互相替代。分析面向全部四家 Provider；各家 adapter 可以誠實回報「不支援」，但入口與契約必須涵蓋四家（教訓 2026-09-12：第一版曾寫死 Codex）。

## 1. 隱私與讀取界限

- 摘要報告**永不含** prompt 文字、來源路徑、Provider 內部 ID。Session 以公開序號＋時間戳引用（`session_ref`）。
- Prompt 內容只經**獨立、有界的 content 介面**取得，且重新掃描並比對序號與時間戳，避免過期的 UI 選取洩漏另一個 session。
- Prompt 有界：每則最多 4000 字元；過濾控制字元；遮罩 PEM 私鑰區塊、疑似機密行（`[REDACTED_SENSITIVE_LINE]`）、`Bearer` 後的 token、絕對路徑（`[REDACTED_PATH]`）、機密鍵名的 `key=value`。重複 prompt 只計未遮罩且未截斷者。
- 第三方資料庫／日誌**唯讀**；路徑 canonicalize（symlink / junction / reparse point）後仍須位於允許的 Source 根目錄內。

### 掃描上限（防止被超大歷史拖垮；超過即標示 `limited`）

| 項目 | 上限 |
|---|---|
| 檔案數 | 50 |
| 探索的目錄項目 | 10,000 |
| 目錄深度 | Codex 5、跨 Provider 6 |
| 單檔 | 32 MiB |
| 總量 | 128 MiB |
| 每 session 回合 | 1,000（回傳 200） |
| 並行掃描 | 同時僅一次（`history_busy`） |

資料不完整（`limited`、跳過檔案、拒收紀錄、部分紀錄）時，必須發出 `incomplete_data` 訊號——「圖表與判斷可能偏低」。

## 2. Token 流量正規化（`normalize_flow`）

不同 Provider 對「input 是否含快取」的語意不同，先轉成統一流量：

| Provider | 規則 |
|---|---|
| Codex | `input` **已含**快取：`uncached = input − cached − cache_write`；`classified = input + output` |
| Claude | `input` **不含**快取：`uncached = input`；`classified = input + cached + cache_write + output` |
| Copilot / Antigravity | 目前**無法**分類 → 全部 `None`（不猜） |

- `unclassified = total − classified`（有 total 時）；total 缺失 → 0；**total 小於 classified（下溢）→ 整個流量作廢**。
- **任何必要計數缺失 → 回傳全 `None`**，不以 0 補。`cached_input_tokens: null` 曾被當成 0 而製造出假的「快取重用偏低」訊號（教訓 2026-09-12）。
- 所有加總用 `checked_add`；溢位 → `None`，不飽和。
- Token 序列化為字串（避免 JS 53-bit 精度損失）。

## 3. 成本估算（僅作 API 等價參考）

- `pricing_basis = current_api_equivalent`，附 `price_as_of` 與價格來源 URL。**這是「若走 API 計價約多少」，不是帳單。**
- **Core v1 決策 D-01**：規格預設不計算／不持久化／不顯示估算成本；估算是獨立的 Derived Estimate，需 ADR 與獨立驗收才可成為產品功能。目前程式內的估算屬於分析報告的衍生值，**永遠不與 official 成本相加**。
- 費率以奈美元（nano-dollars）整數運算，輸出到 6 位小數（四捨五入）；每筆有 min/max 兩端（快取寫入有 5 分鐘/1 小時 TTL 兩種費率，無法分辨時給區間）。
- 價格表（每百萬 token，USD；`price_as_of 2026-09-12`，**使用前務必對照官方現價，這份表會過期**）：

| 模型 | input | cached | cache write (min–max) | output | 長 context 門檻 |
|---|---|---|---|---|---|
| codex gpt-5.3-codex / 5.2-codex | 1.75 | 0.175 | — | 14 | — |
| codex gpt-5-codex | 1.25 | 0.125 | — | 10 | — |
| codex gpt-5.6(-sol) | 4 | 0.4 | 5 | 20 | 272,000 |
| codex gpt-5.6-terra | 2 | 0.2 | 2.5 | 12 | 272,000 |
| codex gpt-5.6-luna | 0.2 | 0.02 | 0.25 | 1.2 | 272,000 |
| codex gpt-6-astra | 10 | 1 | 12.5 | 50 | 272,000 |
| claude sonnet-4-5/4-6 | 3 | 0.3 | 3.75–6 | 15 | — |
| claude sonnet-5 | 2 | 0.2 | 2.5–4 | 10 | — |
| claude haiku-4-5 | 1 | 0.1 | 1.25–2 | 5 | — |
| claude opus-4-5…4-8 / opus-5 | 5 | 0.5 | 6.25–10 | 25 | — |
| claude fable-5-1 | 10 | 0.25 | 12.5–20 | 50 | — |

- **不猜價**：未辨識的模型、缺少模型名、缺少 Token 分類、輸入超過長 context 門檻（價格結構不同）的回合，一律**不計價**。
- `cost.state`：`complete`（已定價 = 已觀察）· `partial` · `unavailable` · `no_usage`（已觀察為 0）。`coverage_percent = 已定價 / 已觀察`。
- **矛盾 fail closed**：`priced_tokens > observed_tokens` → `unavailable`，**隱藏金額與覆蓋率**，不夾住也不回報 100%（教訓 2026-09-12）。
- 百分比一律整數運算、四捨五入到 1 位小數；分母為 0 → `None`。

## 4. 效率訊號（保守門檻；是「線索」不是「判決」）

| kind | 觸發條件 | 嚴重度 |
|---|---|---|
| `low_cache_reuse` | 資料完整、回合 ≥ 3、輸入 ≥ 10,000、快取佔輸入 **< 20%** | review |
| `input_heavy` | 資料完整、回合 ≥ 3、輸入 ≥ 20,000、輸出 **≤ 5%** 輸入 | review |
| `incomplete_data` | `limited` / 有跳過檔案 / 拒收 / 部分紀錄 | info |
| `pricing_incomplete` | 成本 `partial` 或 `unavailable` | info |
| `no_obvious_signal` | 無其他訊號且總量 > 0 | ok |

文案原則：說出證據（「目前約 N% 的輸入來自快取」），並附「這是效率線索，不等於已證明浪費」「程式碼分析本來也可能是輸入密集」這類限定語。資料不完整時**不發**前兩種訊號。

## 5. 累積 counter（若實作持久化歷史）

| 情況 | delta 規則 |
|---|---|
| 首次看見 counter | 來源可證明起點為 0 才記當前值，否則 `delta = null` 等下一筆 |
| 同 session 增加 | `current − previous` |
| 值相同 | delta 0，ingestion 可去重不另寫 summary |
| 值下降 | 有 session/reset/rotation 證據 → 開新 segment；否則標 anomaly，**不產生負 delta** |
| 檔案重讀 | `source_event_key` 命中即跳過 |
| parser 版本改變 | 不自動跨版本接續，除非 migration 規則有測試 |

- 同 event key＋同 payload：duplicate；同 key＋不同 payload：quarantine 為 `identity_collision`，兩筆皆不進 summary。
- 晚到事件插入正確位置後，重算受影響 segment 與 summary。
- 負值、overflow、NaN/Infinity → 拒收並計 rejected，**不截斷**。
- input 是否含快取、reasoning 是否含於 output → 寫入 `accounting_policy_version`；只有相同 policy 才能直接加總。
- `fact_key = SHA-256("agentmeter.fact.v1\0" + 欄位序列)`；字串先 NFC＋UTF-8＋big-endian u32 長度前綴；由 History 核心重算，Collector 提供的 hash 不受信任。
- 檔案 identity 用可跨 rename 的檔案 ID，不只用 path；檔案縮短視為 truncation（新 generation 從 offset 0 讀並以 fact key 去重）；未完整的最後一行不更新 offset。

## 6. 保存與時區

| 資料 | 保存 |
|---|---|
| quota 原始快照 | 30 日；僅在數值改變或每 5 分鐘 heartbeat 才寫 |
| raw usage events | 預設 90 日（30/90/180/365 可選）；清理後 summary 保留 |
| Daily Summary | ≥ 180 日 |
| opt-in session 事件 | 預設 30 日、獨立開關 |
| 診斷 | 7 日＋總量上限 |

- 刪 raw 前須在**同一 transaction** 確認對應 summary 已完成、checksum 正確且 `rebuildable_until` 已更新；失敗則 raw 與 summary 一起 rollback。
- `bucket_date` 由使用者設定的 IANA `bucket_timezone` 從 **instant** 計算，不以檔名或 timestamp 前 10 字決定；statusline 檔名日期只是 discovery hint。DST、半小時/45 分鐘 offset、跨午夜都要有 fixture。
- 查詢遇到不同 `bucket_timezone` 的 summary：預設分組並標 `mixed_timezone=true`；不靜默合併。
- 備份：用 SQLite online backup interface（不直接複製 WAL 的 `.db`）；寫暫存 → fsync → atomic rename；manifest 含 schema 版本、時間、大小、SHA-256；還原前驗 checksum、integrity、相容性，並先另存現行 DB。
- **Migration 前必備份（保留最近 3 份）；備份或 migration 失敗 → fail closed**：不啟動 Collector、不寫入，提供診斷匯出與還原入口（ADR 0011）。
