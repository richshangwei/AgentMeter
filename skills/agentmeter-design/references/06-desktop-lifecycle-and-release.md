# 06 桌面生命週期、更新與發布

> 目錄：架構 → 視窗與程序生命週期 → 設定檔 → 簽章更新 → 打包 → 診斷 → 隱私與證據 → 發布閘門

## 1. 架構（ADR 0005、0006）

Rust core ＋ Tauri 2 shell ＋ 純 Web 前端（桌面與平板共用元件，不依賴 Tauri 私有 API）。四家 Collector **編譯進程式（或隨包的第一方 helper）**，只經受控子程序呼叫官方 CLI；**v1 沒有動態 plugin SDK，也不載入任意腳本**——以可稽核性與可重現封裝換取擴充性。

> 實作偏離：隨包的 Node/ConPTY helper 是對「全 Rust Collector」的刻意偏離，但它只接受**固定 Provider 識別碼＋App 專屬 workspace**，不接受使用者腳本或任意命令。

模組：core、collectors、storage、scheduler、usb、http、desktop、web-ui。Collector 介面提供 probe / collect / health / shutdown，回傳**能力旗標**而不假定 token / quota 都存在。

## 2. 視窗與程序生命週期

| 項目 | 規則 |
|---|---|
| 系統匣 | 啟用 tray；左鍵：show →（必要時）unminimize → focus；右鍵選單含儀表板、更新、USB、設定、暫停/恢復、離開。**不依賴 tray title 顯示狀態** |
| 關閉 | 一般 `CloseRequested` 必須 `prevent_close` 後 hide；首次提示一次；**只有明確「離開」才進入 terminating 並依序停止工作** |
| 單一實例 | single-instance plugin **第一個註冊**；第二次啟動只喚醒既有視窗，不得建立第二組 Collector、HTTP server、通知器或 ADB mapping。支援探針參數（`--probe-ready` 無副作用、`--request-exit` 要求完整離開） |
| 自動啟動 | 預設關閉；由 Rust backend 管理 enable / disable / isEnabled；`--hidden` 啟動時不顯示主視窗但建立 tray 與必要服務。Run key 值名稱改過時，要清掉**舊名稱**（legacy cleanup） |
| 視窗 | 可置頂、記憶位置；螢幕消失時回可見範圍；主視窗最小 640×520，預設 1080×640 |
| 離開順序 | `stop_and_wait`（設取消旗標 → 等運作中收集釋放 → 拒絕新工作）→ 撤銷 session → 關 SSE → 移除 owned mapping → 停 listener |
| 休眠 / 喚醒 | 休眠期間不更新；喚醒後先標舊 Observation 為 stale 再重新收集 |
| 暫停 | 停止收集、重試、通知；保留 tray / 桌面 / server / 既有 Observation，所有畫面標 `paused` |

跨 desktop 的第二次啟動曾因 IPC 範圍而失效（教訓：以本地 patch 的 single-instance 修正，並用 5 次真實第二次啟動驗證 fail-closed）；程序生命週期要用**真實程序**測，而不是模擬。

## 3. 設定檔

- 所有設定：**有大小上限、有版本號、損毀就明確報錯**（不靜默回預設，除非檔案不存在）。
- 寫入一律「同目錄暫存檔（`create_new`）→ `sync_all` → `rename`」；失敗清暫存。
- 路徑設定須為絕對路徑、≤ 8192、不含 NUL。
- v1 → v2 設定遷移是**只加不改**（v1 讀入後補 `copilot: null`）；不支援的版本回錯，不猜。
- 空值代表「持續自動探索」，不是猜測的安裝位置或身分。
- 禁止在設定中存憑證明文。

## 4. 簽章自動更新

使用者可見行為：啟動時與每 **6 小時**檢查；失敗 **30 分鐘**後重試；若有操作進行中，**60 秒**後再排；檢查不得阻塞監控。偏好**分開保存**：自動檢查（預設**開**）、自動下載（預設**關**）。下載顯示進度，**簽章驗證通過才進入 ready**；**安裝永遠需要明確確認**；已驗證的下載只在記憶體中，離開後須重下載。

硬性規則：
1. **公鑰與 endpoint 在編譯時注入**（環境變數），未注入的 build 無法更新；公鑰長度 32–4096、endpoint 須符合 `https://github.com/<owner>/<repo>/releases/latest/download/latest.json` 且 owner/repo 僅限 `[A-Za-z0-9-_.]`、不得為 `.`/`..`。
2. Manifest 本身**未簽章**，所以要把版本綁到 release tag 與**精確安裝檔名**：URL 必須是 `https://github.com/<repo>/releases/download/v<版本>/AgentMeter-P0_<版本>_x64-setup.exe`，無 port / 帳密 / query / fragment；版本為三段純數字且無前導 0。
3. **Release 資產用 ASCII 安全檔名**（字母、數字、連字號、底線、句點）：GitHub 會把空白正規化成句點，導致 allowlist 與實際資產不符、不可變的 0.2.0 用戶端請求 404（教訓 2026-09-11）。**先發 non-Latest prerelease**，用未驗證 HTTP 比對 manifest URL 與實際資產 URL，確認 200 後才設 Latest。
4. **更新器生命週期順序**：在更新器「確認的 pre-exit hook」才停止不可逆的收集器狀態；下載、驗證、解壓的任何錯誤都必須讓監控**保持運作**且更新可重試（教訓 2026-09-10）。已把控制權交給 Windows 安裝程式後，SDK 無法救回此程序——文件要寫明。
5. **新註冊的 Tauri plugin 必須有封裝後啟動的設定契約測試**，以及真實 release 執行檔的啟動探測（`--hidden` / `--request-exit`）。曾經：單元測試全綠，但裝好的 exe 在建窗前因 plugin 設定被反序列化成 `null` 而 `desktop_startup_failed`。
6. 簽章私鑰**絕不入庫**：只以 Windows CurrentUser DPAPI 密文存在使用者目錄；在記憶體產生金鑰、只落地密文；**不要宣稱 CurrentUser DPAPI 可攜備份**。擷取敏感 CLI 輸出用隱藏 ProcessStartInfo 並以 exit code 判斷（PowerShell `Stop` 會把 stderr 警告當失敗）。
7. 發布 API 的 `target_commitish` 用完整 `git rev-parse HEAD`，不用短 SHA；改版本只動應用程式自己的 package 區段，檢視 lockfile diff，不得連帶升級依賴。

## 5. 打包（NSIS）

- 目標 `nsis`、`installMode: currentUser`、`allowDowngrades: false`；WebView2 用 `downloadBootstrapper`（`silent`）——**不得用 `skip`**；缺 runtime 要偵測並提供修復指引。
- 安裝／升級／卸載：卸載預設保留資料；`/PURGE` 才清；升級流程為 all-or-none 兩套件契約，變動前先守住 identity / version。
- 隨包資源（Node runtime、官方 CLI）**不經 Git 再散布**，保留各自上游授權。
- 正式 v1 的 exe / installer / uninstaller 必須 Authenticode 簽章並提供 SHA-256、SBOM 與授權清單；未簽章只能叫技術預覽版，且要明示。
- 相對路徑的檢查命令要從**儲存庫根目錄**執行；桌面建置另外用 `--manifest-path`（教訓 2026-09-14）。

## 6. 啟動診斷

啟動失敗要寫「去識別化」診斷檔（`startup-error.log`，細節 ≤ 2048 字元，暫存檔＋序號避免碰撞），讓封裝後的失敗可被分類而不是只有一個閃退。

## 7. 隱私與證據去識別

- 儲存庫是**公開**的：證據文件記錄 schema、Provider、狀態、原因，但**遮罩**個人用量值（剩餘%、用量/額度、重置時間）與機器識別（PID、本機絕對路徑）。截圖與驗證 log 不入版控（`.gitignore`）。
- 無預設遙測、無自動 crash upload。診斷離開本機只經「明確、預覽、遮罩」的匯出；ZIP 不含原始 DB、憑證、pair material、對話內容、完整個人路徑。
- 紀錄 Failure Code、時間與版本；不記 Authorization、cookie、配對碼、verifier、原始協定全文。
- 共享稽核檔（如 `tasks/todo.md`）提交前要比對 staged 內容，只暫存本任務的 hunk（教訓 2026-09-12：曾把別的任務的規格計畫一起 commit）。

## 8. 發布閘門（誠實分級）

任何下列項目沒有證據，就只能寫成「未驗證／未達成」，不得寫「完成」：真實 Provider 帳號成功讀取、乾淨 VM 安裝／升級／卸載、實體平板 8 小時運行、真實簽章更新安裝、Authenticode。**模擬資料與 fixture 只能驗證 UI，不能取代四家真實來源驗收。**
