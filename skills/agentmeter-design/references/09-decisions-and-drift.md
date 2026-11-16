# 09 架構決策（ADR）與「規格 vs 實作」偏離表

> 目錄：ADR 摘要 → 權威順序 → 偏離表 → 未達成的閘門 → 把這套設計移植到新專案時的取捨

## 1. 架構決策（為什麼這樣做；難以逆轉）

| # | 決策 | 理由 |
|---|---|---|
| 0001 | 帳號 quota 以「帳號＋provider bucket」去重，本機 usage 保留 Source scope；跨 Source 合併只憑穩定 identity 或可撤銷的 Link | 避免重複計算共享配額，也避免暗示某個 Source 看到了另一環境的活動 |
| 0002 | Data Quality 與 Collector Maturity 分開 | 「官方」不等於「穩定」：官方的 preview 介面可以權威但實驗性；穩定的本機 parser 也只能是 local_observed |
| 0003 | 長效 Device Pair ＋ 短效輪替 Tablet Session；配對從桌面選定 ADB serial 開始，8 位碼 2 分鐘、5 次嘗試 | 正常重啟與 USB 重插能自動復原，同時能在 mapping 變動／換裝置／換帳號／撤銷時失效 |
| 0004 | Loopback HTTP over ADB reverse，所有私有端點要高熵認證；不稱端對端加密 | 避免 LAN 暴露；誠實描述保護邊界 |
| 0005 | Rust core ＋ Tauri 2 ＋ Web 前端 | 收集、持久化、排程、USB、API 同在原生核心；桌面與平板共用 Web 元件 |
| 0006 | v1 把四家第一方 Collector 編進程式，只用受控子程序呼叫官方 CLI；無動態 plugin SDK | 以有界權限模型、可重現封裝與可稽核的資料處理，換掉第三方可擴充性 |
| 0007 | 每個 Daily Summary 保存分桶時區 | 切換時區只影響新彙總；資料過期後不假裝已重算 |
| 0008 | 不做 SQLite 全庫加密，依賴 Windows 使用者邊界與磁碟保護；Provider ID 只存 keyed hash；pair verifier 等機密用 DPAPI；無預設遙測；允許不阻塞、簽章的 GitHub 發行檢查，不上傳診斷、不自動安裝 | 在不增加金鑰管理負擔下，守住明確的資料離機邊界 |
| 0009 | Availability / Collection State / Freshness / Failure Code / Maturity 為正交軸 | 同時被限流且過期、可用但退避，這些組合不應丟資訊 |
| 0010 | API 與 SSE 傳完整 Dashboard Snapshot，不傳增量 | 四家 payload 小；完整狀態使漏事件復原具確定性 |
| 0011 | Migration 前備份（留 3 份），失敗則全面停寫（fail closed） | 繼續跑在殘缺 schema 上不算「降級運作」 |

## 2. 權威順序（資訊衝突時）

可重現測試 → 公開 Interface／型別 → ADR／正式需求 → 實際實作 → 註解與敘述文件 → UI 觀察。
需求書、`CONTEXT.md`、ADR 彼此衝突時：**先停止實作並修正基線，不得自行選一份**。UI 走查只能證明使用者看到什麼，不能單獨證明內部資料正確。

## 3. 規格 vs 實作偏離表（2026-11 盤點）

使用這個 skill 時要分清楚：哪些是**已實作並驗證的規則**，哪些是**規格中的目標**。

| 面向 | 規格（需求書 v1.1／分析規格） | 目前實作 | 判讀 |
|---|---|---|---|
| Claude 主要來源 | statusLine JSON | 預設用 `/usage` 終端解析（node-pty）；statusLine 接收器與設定整合已實作，但不是預設即時路徑 | 實作務實偏離；`/usage` 為 experimental |
| Antigravity 主要來源 | statusLine JSON `quota` map | `agy --print /usage` TSV 解析 | 規格視為 experimental fallback，實作把它當預設 |
| Copilot 主要來源 | 符合權限的 Billing API | 官方 CLI `account.getQuota`（含 `gh auth token` fallback）；Billing 收集器保留為回歸／舊制 | README：預設不需要 Billing 設定 |
| Copilot 重置時間 | 取自 SDK | **一律視為不可信**，Dashboard `resets_at=null` | 比規格更保守 |
| Collector 成熟度 | 正式 v1 四家需 `stable` | 目前**沒有任何 Collector 標為 `stable`**（四家皆 experimental） | 發布閘門未達 |
| 持久化 | SQLite（含 provider_accounts、usage_events、quota_snapshots、daily_summaries…）、migration、備份 | **尚無 SQLite／migration**；設定為小型 JSON 檔，配對為 DPAPI 檔；歷史分析是對 Provider 本機日誌的**唯讀有界掃描** | 第 04 章的 counter／保存／時區／備份規則是**目標設計**，尚未有程式驗證 |
| HTTP 傳輸 | Tokio + Axum | `std::net` 手寫最小 HTTP（Axum 不在離線 crate 快取）；行為在 HTTP seam，可替換 | 安全／狀態模型不變 |
| `/api/v1/history` | 依 Provider／Account／Source／期間查詢 | 回傳 `{"items":[],"bounded":true}` 的佔位 | 未實作 |
| 平板卡片 | 固定四家、可設順序 | 動態可選 Provider 卡，永遠保留一個空新增槽，附全螢幕 | 產品演進 |
| 狀態值 | 嚴格列舉 | 初始視圖有 `availability:"unknown"`、`data_quality:"unknown"`、`setup:"required"`；Copilot 出現 `freshness:"provider_reported_period"` | 在第一次收集之前／Copilot 特例的暫態值；`check-snapshot.mjs` 以受限方式容許 |
| HUD 點擊穿透 | `tasks/lessons.md` 2026-09-12 條目要求「原生點擊穿透」 | 現行程式與測試為**可拖曳、接收滑鼠、不可聚焦**（`set_ignore_cursor_events(false)`＋`start_dragging()`） | 該教訓條目已過時（儲存庫內仍未標示被取代）；以測試為準，見 03 §4 |
| Fixture 與 live | 必須明確區分 | Codex 以 `fixture_replay`／`replay:true` 標示 | 一致 |
| 成本估算 | Core v1 不計算（D-01） | `token_analysis` 計算 API 等價估算區間，附 `price_as_of` | 屬於分析報告衍生值，永不與 official 相加 |

**判讀原則：** 偏離若更保守（Copilot 重置時間不可信）就維持；若更樂觀（任何標 stable 的主張）則視為缺陷。

## 4. 未達成的閘門（不得宣稱完成）

- 四家 Provider 的**真實帳號**逐一對照證據（同時間官方畫面）與支援版本矩陣。
- 乾淨 VM 的安裝／升級／卸載（preserve 與 purge）與 WebView2 bootstrapper。
- 實體 Android 平板：認證流程、8 小時運行、USB 拔插復原。
- 真實的簽章更新安裝（隔離 Windows 環境）；正式版 Authenticode＋SBOM。
- SQLite 持久化與 migration fail-closed 的實作與測試。

## 5. 把這套設計移植到別的專案時，哪些可以改、哪些不能

**可以改（取決於你的環境）**：實作語言與 UI 框架、傳輸層（Axum / std::net / 其他）、平板通道（USB reverse → 其他）、視覺主題、更新通道、價格表與模型清單、監控的 Provider 清單（但沿用 Provider capability matrix 的做法）。

**不要改（改了就失去這套設計的價值）**：
1. Unknown ≠ 0，缺值為 null。
2. 正交狀態軸，以及 Data Quality ≠ Collector Maturity。
3. Quota 快照不相加；Token／Credits／請求數／百分比分開。
4. Parser 看不懂就 fail closed，保留舊快照並標 stale＋失敗代碼。
5. 只做唯讀探測；不複製憑證；子程序用 argv 陣列；白名單輸出。
6. 通道認證不依賴網路位置：每個私有路由都要 session，URL 不帶秘密，配對碼單次且有限次。
7. 修改使用者既有設定前：預覽、同意、備份、只還原自己加的部分。
8. 證據要誠實分級：fixture ≠ 真實帳號；模擬 ≠ 驗收；沒驗證就寫沒驗證。
