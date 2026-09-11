# Windows 簽署更新發佈

此流程只準備本機發佈草稿，不建立金鑰、不上傳、不發佈 GitHub Release，也不安裝更新。正式啟用前，維護者必須決定簽署金鑰的保管者、備份方式與發佈權限。

## 信任邊界

- 更新來源固定為 `https://github.com/richshangwei/AgentMeter/releases/latest/download/latest.json`，安裝包固定來自同一儲存庫的版本 Release。
- Tauri 更新簽章與 Windows Authenticode 不同；有更新簽章不代表已取得 Windows 信任憑證。
- 私鑰與密碼只能透過受保護的建置環境提供，不得放入 Git、聊天、命令列參數、截圖或公開紀錄。公鑰必須對應同一私鑰。
- 遺失簽署私鑰將使既有版本無法信任後續更新。更換公鑰需要另行設計信任遷移，不能直接覆寫公鑰。

## 本機準備

### 本機 Windows 金鑰保管（2026-09-11）

目前本機簽署入口為 `scripts/local-signing-build.ps1 -Version <版本>`，限持有既有金鑰的 Windows 使用者環境。金鑰放在 Git 外的使用者設定檔 `.agentmeter-signing` 目錄：`updater.key.dpapi` 是 Windows CurrentUser DPAPI 加密私鑰，`updater.key.pub` 是公鑰。私鑰僅在記憶體產生，沒有明文私鑰檔；建置時暫時透過子行程環境傳遞，完成後清除變數。程式拒絕覆寫已有私鑰。

DPAPI 不是可攜式密碼備份：其他電腦／重建帳戶可能無法解密。正式發布前必須另行安排可攜式加密備份及還原演練；不要只複製 `.dpapi` 就假設能跨電腦復原。同一 Windows 帳戶下的程式仍可解密，因此這不是防止該帳戶遭入侵的保護。更新簽章不等於 Windows Authenticode 憑證。

1. 選定比已發佈版本大的穩定版號（例如 `0.2.0`），更新 `desktop-p0/tauri.conf.json`、`desktop-p0/Cargo.toml` 與受影響的 lockfile。腳本不會自動修改版本。
2. 由安全的建置環境提供 `TAURI_SIGNING_PRIVATE_KEY`、加密私鑰所需的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`、`AGENTMETER_UPDATE_PUBLIC_KEY`、`AGENTMETER_UPDATE_ENDPOINT`。Endpoint 必須等於上述固定網址；不要輸出環境變數內容。私鑰可使用 Tauri 支援的私鑰檔案路徑或編碼內容。
3. 先執行完整測試，再準備草稿：

```powershell
powershell -ExecutionPolicy Bypass -File scripts/verify-local.ps1
node --test scripts/create-update-manifest.test.mjs
powershell -ExecutionPolicy Bypass -File scripts/build-signed-update.ps1 -Version 0.2.0
```

需要已安裝的 Cargo、Tauri CLI、Node.js、7-Zip 與本機已快取的鎖定依賴。建置使用 `--offline --locked`。缺少金鑰或版本不一致即停止，不降級為無簽章更新。

成功輸出在 `desktop-p0/target/update-drafts/<版本>-<唯一識別碼>/`，包含 NSIS `.exe`、相鄰 `.exe.sig` 和 `latest.json`。草稿不覆寫前次結果，全部留在忽略的 target 目錄。

## 發佈前人工驗收

在隔離 Windows 環境安裝帶正確公鑰與 endpoint 的前一版，驗證檢查新版本、下載、簽章驗證、安裝及重啟；另外驗證拒絕被修改的安裝包、離線、同版／舊版、下載失敗及重試。測試用 fixture 只檢查 manifest 格式，不證明真實私鑰／公鑰相符或更新可安裝。

取得發佈授權後，維護者才把三個檔案上傳至同一儲存庫的 `v<版本>` Release，確認 manifest 的 URL 與實際資產名稱完全相符，最後才將 Release 公開並設為 latest。不要先發佈 manifest 再補安裝包。未裝入公鑰與來源的舊預覽版需要手動安裝一次啟用版。

回復方式：暫停新 Release 的 latest 指向或移除有問題的 manifest，停止新增下載；不可依賴自動降版。已安裝新版的裝置需另行評估修正版或人工復原。

## 格式依據

本機 Tauri CLI `2.11.4` 的 `helpers/updater_signature.rs::sign_file` 將 minisign 四行簽章文字編碼成 Base64，寫入安裝包旁的 `.exe.sig`；`bundle.rs` 在非 v1 相容模式下直接簽署 NSIS 安裝包。Tauri updater `2.10.1` 使用靜態 manifest 的 `platforms.windows-x86_64.signature/url` 並在安裝前驗證簽章。`create-update-manifest.mjs` 檢查結構與固定 URL；真正的密碼學驗證由更新程式執行。
