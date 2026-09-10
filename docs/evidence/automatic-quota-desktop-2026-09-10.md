# 自動額度桌面版 — 2026-09-10

## 已完成

將四家最小實帳測試成功的方式接進桌面主流程，沒有再要求使用者填寫來源路徑、報告檔、帳號 slug 或 Billing 設定。

- 四張額度卡片、全部／單家重新整理。
- 啟動後自動讀取；每次背景輪詢完成後等待 60 秒再次讀取，隱藏到通知區後仍繼續。
- 可關閉本次執行的自動更新；重新啟動預設開啟。
- 桌面與平板使用同一收集方法與 LiveDashboard；Antigravity 的平板刷新不再固定回覆 unsupported。
- 失敗時保留舊數值並標示 stale；不將未知額度補成 0% 或 100%。
- 收集程序有逾時、互斥及取消；Windows Job 管理收集子程序樹。
- 隨安裝包攜帶 Node、ConPTY/xterm 及核對過 SHA512 的 Antigravity CLI；使用者不必安裝測試套件。
- 舊事件接收器與 Billing 實驗程式保留作為相容／回歸用途，不再參與預設桌面操作。

這次依 codebase-design 的小介面原則，將差異集中在固定 Provider 的收集入口。
這是配合使用者要求、改採實測方法的取捨，與 ADR 0006 的全 Rust 收集器設計不同；不是開放任意腳本或動態插件。

## 正式執行檔實帳驗收

`scripts/test-native-quota.mjs` 對正式執行檔執行 `--quota-collect`，四家皆有非空額度才算 PASS。

證據：`native-quota-1788990213085.json`，時間 2026-09-10 05:43:33 Asia/Taipei。

| Provider | 剩餘額度 | 結果 |
| --- | --- | --- |
| Codex | 主／次視窗 46% / 76%；基本模型 95% | PASS |
| Copilot | premium_interactions 24.7%，已用 1130 / 1500 | PASS |
| Claude | 五小時 100%，週額度 2% | PASS |
| Antigravity | Gemini 每週 58%；其他此次回報視窗 100% | PASS |

不使用模型生成請求或 GitHub Billing endpoint。Copilot fallback 使用現有 GitHub CLI 的現用帳號，憑證僅在記憶體與官方子程序環境中使用，不寫入證據。

## 修正與驗證

diagnosing-bugs 的重現迴圈找出 Claude 新工作目錄與既有測試目錄的差異：

1. 首次提示繪製時輸入處理尚未就緒；新版本也可能預設選中 **No, exit**。現在明確授權啟用後，等待提示就緒、確認選項，再選 Yes。
2. 新額度視窗可回報 0% used 而暫無 reset 行。現在接受真實百分比，重設時間保持未知。

新增測試曾先重現失敗，再驗證修正；暫時診斷輸出已移除。

- 完整 `scripts/verify-local.ps1`：PASS（root/desktop tests、格式、語法、warnings-denied Clippy）。
- 桌面 Rust：18 tests PASS。
- JavaScript：既有 15 + 額度解析 6 + 自動額度 UI 2 tests PASS。
- 正式執行檔四家查詢：PASS。
- NSIS 建置與檢查：PASS；已確認包含 node.exe、agy.exe、固定收集入口、xterm 與 Windows ConPTY native modules。
- 新版程序已啟動，readiness probe 回覆成功。

## 產物

- 執行檔：`desktop-p0/target/release/agentmeter-desktop-p0.exe`
- 執行檔 SHA256：`07224B67FFC886EB10814811BBEBDA5CEBD47EC0109459376FC004D8EF6E1BA8`
- 安裝包：`desktop-p0/target/release/bundle/nsis/AgentMeter P0_0.1.0_x64-setup.exe`
- 安裝包大小：70,575,278 bytes。
- 安裝包 SHA256：`958E64E6470922DEFB44BD594CAD230CEFFF0F9304E5D11357A220F8DDCB2F0D`

執行檔必須連同旁邊的 quota-helper 目錄使用；不要只複製單一 exe。

## 仍有的限制

- 本機已驗證，不代表所有方案／所有電腦皆支援。
- Claude 終端與 Antigravity TSV 解析仍屬 experimental；格式變動會顯示失敗，不猜測數值。
- Copilot 是 premium interactions，不是 AI credits；其重設日期未驗證，畫面顯示未知。
- 首次 Claude 啟用僅信任 AgentMeter 的隔離目錄，且停用工具與自動更新，不改其他專案信任設定。
- 安裝包未簽章；未在乾淨 VM 執行安裝，也沒有本次實體平板端對端驗證。此產物供本機驗用，不宣稱正式公開發行。
