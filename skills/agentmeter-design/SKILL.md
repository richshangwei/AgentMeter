---
name: agentmeter-design
description: 設計、實作、審查「多家 AI 編碼助手（Codex、Claude Code、GitHub Copilot、Google Antigravity）用量／額度監控」的完整設計規範與邏輯規則，來自 AgentMeter（Windows 系統匣桌面程式＋USB 平板儀表板＋浮動視窗 HUD＋簽章自動更新＋Token 歷史分析）。只要使用者提到 AI 用量監控、quota／額度／剩餘百分比、rate limit 視窗、Codex app-server、Claude /usage 或 statusLine、Copilot getQuota 或 Billing、Antigravity /usage、Dashboard Snapshot／SSE、ADB reverse 平板配對、Tauri 系統匣常駐、Token 成本估算或快取重用、浮動額度視窗，或想做／審查／移植類似的 usage／quota 監控，即使沒有明說「AgentMeter」也要使用此 skill。涵蓋領域詞彙、五條正交狀態軸、各 Provider 收集與解析規則、fail-closed 原則、loopback 通道安全、桌面生命週期、更新簽章、版面演算法、驗證與教訓，並附快照檢查腳本。不適用於一般 APM／伺服器監控或與 AI 用量無關的儀表板。
---

# AgentMeter 設計規範

AgentMeter 是一套本機 Windows 監控產品：把 Codex、Claude Code、GitHub Copilot、Google Antigravity 的**用量與剩餘額度、重置時間、資料可信度**集中顯示在桌面與配對平板上。

這個 skill 把它所有「邏輯規則」與「設計決策」整理成可重用的規範。它的價值不在程式碼，而在於**每條規則背後的原因**——大多數規則是在真實帳號上、或因為事故才被加上去的。移植到別的專案時，你可以換語言、框架、傳輸層，但不要丟掉那些原因。

## 先決定你要做什麼

| 使用者想… | 先讀 | 再做 |
|---|---|---|
| 理解／溝通這個領域（用語、狀態怎麼分） | `references/01-domain-model.md` | 用其中的詞彙回答，避免禁用同義詞 |
| 新增或修改某家 Provider 的收集器／解析器 | `01` + `references/02-collectors.md` | 走下面「變更 Parser 的檢查清單」 |
| 決定一個數字怎麼算、怎麼顯示、何時通知 | `references/03-quota-semantics-and-display.md` | 對照不變量；用 `check-snapshot.mjs` 驗證輸出 |
| 做 Token／成本／效率分析、歷史保存 | `references/04-token-analytics.md` | 先確認隱私界線與 fail-closed 規則 |
| 做平板／遠端檢視、配對、ADB、HTTP API | `references/05-tablet-and-security.md` | 先過 §1 威脅模型與 §9 負向測試 |
| 做桌面常駐、更新、打包、發布 | `references/06-desktop-lifecycle-and-release.md` | 走「發布前檢查清單」 |
| 排版／響應式／圓環與磚的尺寸 | `references/07-ui-layout-rules.md` | 先定拓撲，再跑驗證矩陣 |
| 規劃驗證、寫回報、避免重蹈覆轍 | `references/08-verification-and-lessons.md` | 先查教訓表是否已有同類事故 |
| 判斷「規格說的」與「程式做的」哪個算數 | `references/09-decisions-and-drift.md` | 查偏離表；偏離時照實說明 |

**不要一次讀完全部。** 按上表挑 1–2 份；每份開頭都有目錄。

## 十二條不可妥協的規則（以及為什麼）

1. **Unknown ≠ 0。** 缺值是 `null`，UI 顯示「尚未取得」。補 0 會讓使用者以為額度用完，補 100% 會讓他們以為還有——兩者都會讓人做出錯誤決定。
2. **狀態分軸，不壓成單一 status。** `availability`、`collection_state`、`freshness`、`failure_code`、`collector_maturity` 各管一件事；`data_quality` 另外獨立。「被限流且過期」是真實狀態，單一 enum 表達不了。
3. **Official ≠ Stable。** 權威來源（Data Quality）與整合穩定度（Collector Maturity）回答不同問題。官方的 preview 介面可以權威但實驗性。
4. **Quota 是快照，不相加。** 只能看趨勢、期末值、期間最低值；Token／Credits／請求數／百分比分開呈現，不做「四家總剩餘量」。
5. **Parser 看不懂就 fail closed。** 保留舊值、標 `stale`、帶 failure code；絕不把舊數字顯示為成功。錯的數字比沒有數字更危險。
6. **矛盾資料整份拒絕，不挑一邊。** 例如 `usedPercent` 與 `remainingPercent` 不相加為 100 → `schema_changed`；已定價 Token > 已觀察 Token → 隱藏成本。
7. **只做唯讀探測，不碰憑證。** 不發模型請求、不複製／保存憑證、子程序用 argv 陣列與固定白名單、輸出有上限與逾時、Dashboard 只收白名單欄位。
8. **通道認證不靠網路位置。** loopback 也要 session；URL 不帶秘密；配對碼單次、2 分鐘、5 次；Host／Origin 只是額外防護，不是身分。
9. **改使用者既有設定前：預覽 → 同意 → 備份 → 原子寫入 → 只還原自己加的部分。**
10. **背景程序不得出現視窗。** `CREATE_NO_WINDOW`／`windowsHide`，並關閉第三方 CLI 的自更新（用文件記載的**精確字面值**）。
11. **證據誠實分級。** Fixture ≠ 真實帳號；模擬 ≠ 驗收；沒驗證就寫「未驗證」並附要跑的指令。
12. **每個修正都留一個 tripwire。** 一條測試、grep 規則或不變量斷言，讓同類錯誤回來時會失敗。

## 工作流程

### 開新監控專案（建議順序，每階段都要有可重現的證據）

1. **P0 可行性**：對每家 Provider 做唯讀 spike，產出去識別化 fixture、欄位／scope 說明、支援版本。**未知條件不得猜測**（例如帳號方案、CLI 版本）——列為待驗證。
2. **P1 核心骨架**：正交狀態模型、Dashboard Snapshot 契約、單一實例、系統匣。用 mock 可以跑通流程，但必須明確標示模擬。
3. **P2 真實來源**：四家收集器、白名單輸出、失敗保留舊值並標 stale。不得以 mock 宣稱完成。
4. **P3 平板／遠端顯示**：認證、配對、SSE、ADB；先寫負向測試。
5. **P4 產品化**：設定、通知、診斷、資料清除、安裝／升級／卸載。
6. **P5 發布驗收**：簽章、效能、8 小時運行、真實帳號對照。

分階段發布：技術預覽版可含 `experimental`／`unsupported` 的 Provider，但必須顯示具體阻礙與影響，不得用空白卡片宣稱完成。

### 變更 Parser 的檢查清單

- [ ] 輸入先做型別／範圍驗證；任一欄位不合法 → 整份失敗（`schema_changed`），不留半套結果
- [ ] 缺值保留為 `null`；矛盾值拒絕而非擇一
- [ ] 失敗訊息只放行已知代碼，其餘折成 `collector_failed`
- [ ] 重置時間的單位與可信度有標示（Copilot 一律 `null`；Claude 終端只有顯示文字）
- [ ] 版本敏感的文字解析標 `experimental`，並在 UI 揭露限制
- [ ] 加正常／缺欄位／超額／逾時／未登入／schema 變更六類 fixture
- [ ] 跑 `node scripts/check-snapshot.mjs` 驗證端到端輸出

### 新增一種 UI 顯示的檢查清單

- [ ] 「有資料」的判斷式與渲染分支一一對應，各有 fixture
- [ ] 用**真實資料量**（8 個窗口、小數、`100%`、長重置文字、unknown／error／usage-only）
- [ ] 產品文案不得出現傳輸欄位名（`primary`／`secondary`／內部 limit id）
- [ ] 桌面不得有捲動區；放不下就分頁（見 07）
- [ ] 在多個尺寸與 DPI 縮放等價尺寸下量實際幾何，不只量外框

### 發布前檢查清單

- [ ] 簽章私鑰不在儲存庫；公鑰與 endpoint 在編譯時注入
- [ ] Release 資產為 ASCII 安全檔名；先發 prerelease，比對 manifest URL 與實際資產 URL 皆 HTTP 200，才設 Latest
- [ ] 更新器：下載／驗證／解壓失敗時監控仍運作且可重試；安裝需明確確認
- [ ] 新註冊的 plugin 有封裝後啟動設定契約測試＋真實 release exe 啟動探測
- [ ] 證據文件已去識別（個人用量值、PID、本機路徑）
- [ ] 未驗證的閘門（真實帳號、乾淨 VM、實體平板、簽章安裝）如實列出

## 隨附的腳本與資產

`scripts/check-snapshot.mjs` 把第 1–6 條不變量與 Snapshot 契約（`references/03` §6）做成可執行檢查。無依賴，Node ≥ 18。

```bash
node scripts/check-snapshot.mjs assets/dashboard-snapshot.example.json
node scripts/check-snapshot.mjs next.json --previous prev.json     # 另檢查 revision 規則
node scripts/check-snapshot.mjs snap.json --providers codex,claude  # 自訂 Provider 集合
node --test scripts/check-snapshot.test.mjs                         # 13 個測試，含每條規則的負向案例
```

退出碼：0 通過（可有警告）、1 有違規、2 輸入不可用。`assets/dashboard-snapshot.example.json` 是通過檢查的示範快照（數值為虛構）。

它檢查：envelope 欄位、四家齊全且不重複、五軸列舉值、`failure_code` 與 `collection_state` 的一致性、`fresh`／`stale` 需有時間戳、剩餘值 0..100 且與已用值相加為 100、超額顯示 0、Copilot 重置時間為 `null`、Paused 時不得 `collecting`、同 stream 內 revision 單調且僅在內容變動時遞增、以及快照中不得出現憑證／配對材料／序號／絕對路徑／bearer token／未遮罩 e-mail。

## 範圍與限制（請誠實傳達給使用者）

- 這是**設計與規則**的整理，不是可直接執行的完整產品。AgentMeter 原始碼是 Rust＋Tauri＋Node helper；本 skill 不附帶它。
- 規格（需求書、分析規格）與實作**有已知偏離**，例如目前沒有 SQLite 持久化、沒有任何 Collector 標為 `stable`，Claude／Antigravity 預設走 `/usage` 文字解析而非 statusLine。請以 `references/09` 的偏離表為準，回答時區分「已實作並驗證」與「目標設計」。
- Provider 的 CLI、輸出格式、價格都會變。`references/04` 的價格表標有 `price_as_of 2026-09-12`，使用前必須對照官方現價；文字解析器必須鎖定版本並在失敗時 fail closed。
- 目前實作只針對 Windows 11 x64；WSL 視為獨立 Source，未驗證。
- 未做的驗證：真實四家帳號逐一對照、乾淨 VM 安裝、實體平板 8 小時、真實簽章更新安裝。

## 出處

整理自 AgentMeter 儲存庫：`CONTEXT.md`（詞彙）、`AgentMeter-Requirements-v1.1.md`、`docs/AgentMeter-Token-Analytics-Spec-v1.0.md`、`docs/adr/0001–0011`、`tasks/lessons.md`、`design-qa.md`，以及 `src/`、`desktop-p0/`、`scripts/`、`tablet-ui/` 的實際程式。規則若與這些來源衝突，以可重現的測試與程式為準，再回頭修正本 skill。
