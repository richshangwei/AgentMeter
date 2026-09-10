# 四家真實額度最小測試 — 2026-09-09

## 結論

**GO：本機目前的四家帳號環境，都可以自動取得真實額度。**
兩次重新啟動測試均為 4/4 PASS。這只通過「取得額度」可行性關卡，
不是正式產品、所有方案、長期穩定性或重設時間準確性的驗收。
沒有執行模型生成請求，沒有呼叫 GitHub Billing endpoint，沒有用 fixture 或 token 統計代替額度。

最終測試時間：2026-09-09 22:24:52 Asia/Taipei。

| Provider | 最終結果 | 取得的剩餘額度 | 實際入口 |
| --- | --- | --- | --- |
| Codex | PASS | 主視窗 48%，次視窗 92%；base_model_inference 95% | App Server `account/rateLimits/read` |
| Copilot | PASS | premium_interactions 24.7%；已用 1130 / entitlement 1500 | 官方 CLI stdio RPC `account.getQuota` |
| Claude | PASS | 五小時 90%，週額度 8% | 官方 CLI `/usage`，ConPTY + 終端畫面解析 |
| Antigravity | PASS | Gemini 週額度 58%、五小時 100%；Claude/GPT 兩窗 100% | 官方 CLI `--print /usage` 的四欄 TSV |

## 證據與重跑

- 第一個全通過結果：`quota-smoke-1788963812057.json`。
- 最終全通過結果：`quota-smoke-1788963892492.json`。
- 早期 `quota-smoke-*.json` 保留排查歷程，不代表最終結論。
- 在專案根目錄執行 `powershell -File scripts/test-four-quota.ps1`；不必提供任何資料來源路徑。
- 退出碼 0 表示四家都有額度；1 表示至少一家 FAIL/BLOCKED；2 表示 launcher 缺少 Node.js。
- 自動定位只涵蓋目前 Windows 安裝方式；跨機器安裝體驗未完成。
- parser 邊界測試：`node --test tests/quota_smoke.test.mjs`，4 tests PASS。

## 排查結果與限制

1. 沙箱最初無法讀取 AppData 下的安裝資料，因此「未找到 CLI」不等於未安裝。
   實帳測試需要在正常使用者權限與可連網環境執行。
2. Copilot CLI 本身回報未登入，但 GitHub CLI 的現用帳號已登入。
   測試透過 `gh auth token` 在記憶體取得現用憑證，只透過子程序環境傳給官方 Copilot runtime；
   不寫入報告、不放入命令列、不更改持久登入。這條路確實避開 Billing endpoint。
3. Copilot 此次取得的是 **premium_interactions 權益**，不是新制 AI credits 的完整帳務餘額。
   entitlement 為零的 chat/completions 100% placeholder 被排除。
   回傳的 resetDate 每次接近查詢時間，**尚不能信任為實際重設時間**；
   最終證據保留 `reported_reset_at` 並標示 `reset_time_verified: false`。
4. Claude 採英文終端格式解析，屬 experimental；只有登入成功不算 PASS。
   必須同時解析到 Current session 與 Current week (all models) 的百分比及 Resets 行。
   重設資訊目前是官方畫面字串，尚未轉成可驗證的絕對時間。
   首次測試需要信任本專案新建的獨立暫用目錄；不自動同意其他目錄的 trust prompt。
5. Claude 初始版本是 2.1.222，之後觀察到 2.1.266。沒有執行更新命令，
   但首次互動啟動可能觸發其內建更新；不能宣稱既有安裝完全未變。
   後續 runner 對所有 Claude 子程序設定 `DISABLE_AUTOUPDATER=1`。
6. Antigravity 桌面版不是官方 `agy` CLI。測試將官方 CLI 1.1.28 暫放在
   `.scratch/quota-smoke-runtime/agy.exe`，依官方 installer manifest 核對 SHA512，
   未執行安裝器或修改 PATH。它成功使用既有登入取得資料。
   尚未與 IDE 額度畫面逐項對帳，不自動推定所有 IDE/CLI 額度池完全相同。
7. 終端解析與 TSV 格式皆可能隨版本改變；目前實驗不能代替長期相容性驗證。
   缺少資料、格式變動或無權限必須停止／顯示 unknown，不補成 0% 或 100%。

## 測試檔案與相依項

- `scripts/quota-smoke.mjs`：真實查詢、有限逾時、去識別化報告及明確退出碼。
- `scripts/test-four-quota.ps1`：不需資料路徑的本機入口。
- `scripts/prepare-quota-smoke.mjs`：固定版本 Antigravity 下載與 SHA512 驗證；拒絕覆寫。
- `scripts/quota-smoke-support/package.json` / `package-lock.json`：固定 node-pty 1.1.0、xterm/headless 6.0.0。
  安裝時使用 `npm install --ignore-scripts --no-audit --no-fund`，沒有全域安裝或執行套件安裝腳本。
- `.scratch/quota-smoke-runtime` 及測試 node_modules 已加入 ignore，保留在本機以利重跑。

## 官方來源

- [Codex App Server](https://learn.chatgpt.com/docs/app-server)
- [Copilot account quota](https://docs.github.com/en/copilot/how-tos/copilot-sdk/features/usage-and-billing#account-quota-and-premium-interactions)
- [Copilot SDK transport implementation](https://github.com/github/copilot-sdk/blob/main/nodejs/src/client.ts)
- [Claude statusline](https://code.claude.com/docs/en/statusline)
- [Antigravity headless slash commands](https://antigravity.google/docs/cli/headless/)
- [Antigravity Windows installation](https://antigravity.google/docs/cli/install/)

本次到此為止；尚未把測試移植進桌面／平板產品，也不自動繼續其餘功能。
