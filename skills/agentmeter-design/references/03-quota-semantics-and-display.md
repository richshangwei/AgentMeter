# 03 額度語意、顯示與通知規則

> 目錄：數值規則 → 多窗口卡片 → 標籤語言 → 浮動視窗 HUD → 通知 → Dashboard Snapshot

## 1. 數值規則

| 概念 | 規則 |
|---|---|
| 已用 / 剩餘 | 同一窗口：剩餘 = 100 − 已用。UI 衍生值夾在 0..100；**原始值完整保存** |
| 缺值 | `null` → UI「尚未取得」/「—」。**不是 0%、不是 100%** |
| Over Limit | 顯示 0% remaining，另標超額比例；不解讀為「額外付費」或「仍可使用」 |
| 重置時間 | 以 UTC 儲存、依顯示時區呈現；Copilot 的重置時間不可信，一律不顯示為已驗證 |
| 重置時間單位 | 明示：Claude statusLine 為 `unix_epoch_seconds`；終端解析只有顯示文字 |
| 上下文使用率 | 單一對話的占用，**不是**帳號剩餘額度 |
| 費用 | v1 只顯示 Provider 直接回傳的官方費用；估算另走 Token 分析（見 04） |
| Token vs 配額 | 兩者的範圍不同（session / 本機 / 帳號）必須標明；**不得把配額百分比呈現成 session/turn 的 Token 數**（教訓 2026-09-12） |
| 單位已知但無上限 | 顯示原單位，不套用百分比門檻 |

## 2. 多窗口卡片

- **精簡畫面**：顯示「有效窗口中最低的剩餘比例」並清楚標示是哪個窗口；**未知窗口不得參與最小值運算**。**明細**保留全部窗口。
- 真實資料量遠大於範例：Codex 可同時有 8 個窗口（多個 limit_id × 5 小時/每週），Antigravity 4 個。測試固定資料量必須貼近真實（教訓 2026-09-11：兩個窗口的 fixture 漏掉了裁切 bug）。
- 沒有 `quota_windows` 但有 `source_usage`：用文字列顯示，不畫百分比環。
- 輪詢結果未變 → **保留 DOM 節點與閱讀位置**，不重繪。

## 3. 標籤語言（產品用語 ≠ 傳輸欄位）

教訓：`primary`/`secondary`、`base_model_inference` 曾被直接當成產品文案顯示。

```
週期標籤  windowDurationMins === 10080 → 每週使用上限
          === 300              → 5 小時用量限制
          其他整除 60          → N 小時用量限制
          其他                 → N 分鐘用量限制
          無 duration：window=five_hour → 5 小時用量限制；seven_day → 每週使用上限；否則「用量限制」
群組標籤  limit_id ∈ {codex, default}            → 常規使用額度
          limit_id=base_model_inference 或 name=gpt-reserve → 備用模型額度
          有 limit_name                           → 原名
          codex_bengalfox                         → GPT-5.3-Codex-Spark
          codex_*                                 → 模型使用額度
完整標籤  「{群組} · {週期}」；任何產品文案不得出現 primary / secondary
```

由 API 回傳的列決定是否同時有 5 小時與每週——不要假定。

## 4. 浮動視窗 HUD（桌面右下角）

**目標：** 在不干擾工作的前提下，一眼看見選定 Provider 的主要剩餘值。

- **互動契約（現行，以測試 `compact_hud_is_a_draggable_hidden_secondary_window` 為準）**：HUD 是**不可聚焦**（`set_focusable(false)`）、**接收滑鼠**（`set_ignore_cursor_events(false)`）、只有「按住拖曳」這一種互動（原生 `start_dragging()`），**沒有 hover 樣式**（CSS 不得出現 `main:hover`）。
  - 演進史：2026-09-12 的版本是「被動、原生點擊穿透」，因為當時為了做 hover 取消穿透後，置頂 WebView 蓋住了 Windows 系統匣飛出視窗而無法點擊。2026-09-13 起為了支援跨螢幕拖曳，改成接收滑鼠。**這個取捨是否重新引入系統匣被遮擋的問題，本 skill 沒有證據**——移植時若你的 HUD 靠近系統匣，請先在真機驗證，或維持穿透版本（`set_ignore_cursor_events(true)`，並在設定文案揭露「點擊會穿透」）。
  - 不論選哪種：永遠不可聚焦；不要加 hover；互動只能是刻意的、明確的模式。
- 視窗：預設隱藏、透明、無邊框、置頂、不入工作列、無陰影、不可調整大小（240×132 為上限包絡）。
- 選擇的 Provider：最多 4 個；識別碼須符合 `^[a-z0-9][a-z0-9._-]{0,79}$`，排除 `constructor`/`__proto__`/`desktop-*` 前綴，去重。
- 高度 = `20 + rows × 28`，rows 夾在 1..4。
- **背景不透明度** 35..100（預設 72）；**只套用在面板背景，文字永遠不透明**（容器級透明會讓文字一起變淡）。
- 主值選擇（教訓 2026-09-12：曾用 `quota_windows[0]`，重新排序後顯示 Spark 或備用額度）：
  - 非 Codex → 第一個窗口。
  - Codex → 先過濾 `limit_id ∈ {codex, codex/default}`（常規額度），在其中優先取 `window_duration_mins === 10080`，否則取第一個，再否則退回 `quotas[0]`。**以穩定的 limit_id 與權威的 duration 選，不以陣列順序選。**
  - 沒有 `remaining_percent` 時依序退回：`source_usage[0].remaining_percent` → `used / entitlement(limit)` → `used + unit` → `—`。
  - 顯示數字四捨五入到 1 位小數，夾在 0..100。
- 位置：存「螢幕識別＋座標」；螢幕消失時回到可見工作區；提供「重設至螢幕右下角」與「重新整理螢幕」。
- 位置寫入與 snapshot 輪詢**分開處理錯誤**：位置檔被鎖不得阻止即時資料（教訓 2026-09-13）。
- 原生移動要序列化；拖曳期間延後週期性的工作區夾限；去抖後才讀取當前位置；還原儲存位置之前忽略啟動時的移動事件。

## 5. 通知

- 預設 **20% 提醒、10% 警示**，可調。
- 只對「可比較且新鮮」的 quota 觸發。
- 同帳號＋窗口＋門檻只提醒一次；回升超過門檻 **5 個百分點**或出現新窗口才重新武裝（防止在門檻附近抖動重複通知）。
- App 內保留事件記錄（Windows 通知會受作業系統設定影響）；支援靜音，預設不發聲。
- Paused 時停止通知。

## 6. Dashboard Snapshot 契約

`GET /api/v1/dashboard` 與 SSE `dashboard` 事件都送**完整**快照（ADR 0010）——四家 payload 很小，完整狀態讓漏事件的復原是確定性的。

欄位：`schema_version`、`stream_id`（每次 App 啟動唯一）、`revision`（同 stream 內遞增）、`generated_at`、`paused`、`providers[]`。

- Heartbeat（15 秒）**不增加** revision；45 秒沒收到 → 標示通道斷線。
- 重連、前景恢復、`stream_id` 改變 → 讀取完整快照；**不跨 App 重啟比較 revision**；不假定背景分頁計時器正常。
- Provider view 只含白名單欄位：`setup`、`availability`、`collection_state`、`freshness`、`failure_code`、`data_quality`、`collected_at`、`checked_at`、`quota_windows[]`（`bucket_key`、`label`、`remaining_percent`、`used_percent`、`unit`、`resets_at`、`reset_display`、`entitlement`、`used`）、`source_usage[]`。
- 一個視圖若尚未收集：`quota_windows: []`，**絕不放 mock 數值**。

可用 `scripts/check-snapshot.mjs` 機械化驗證這份契約（見 SKILL.md）。
