# 05 平板通道、配對與安全邊界

> 目錄：威脅模型 → 通道 → 配對 → Session → 路由與請求檢查 → 資料操作 → ADB 規則 → 平板 UI → 負向測試

## 1. 威脅模型（先承認它）

通道是 **loopback HTTP 經 ADB reverse**，**不是端對端 TLS**，也不宣稱傳輸加密（ADR 0004）。威脅包含：Windows 本機的其他程序、平板上其他能開 socket 的 App、惡意網頁偽造 Host/Origin。因此資料保護**完全靠 application 層 session 認證**；Host/Origin 檢查只是額外防護，不是身分。若未來要求傳輸機密性，需另導入 HTTPS 或原生 Android client。

## 2. 通道規則

- 只綁 `127.0.0.1:<host_port>`；**不建立 LAN listener、不加防火牆入站規則、不用 `adb -a`**。
- 伺服器只在平板功能啟用時運行。停用 / 清除配對 / 離開時順序：**先撤銷 session → 關閉 SSE → 移除自己建立的 mapping → 停 listener**。
- 平板 URL 用獨立 `device_port`（預設 8317）；host port 被占時映射 `tcp:8317 → tcp:<新 host port>`，只有 device port 衝突才換 tablet URL。
- 所有靜態資產與 API 回應：`Cache-Control: no-store`、`Referrer-Policy: no-referrer`、`X-Content-Type-Options: nosniff`、嚴格 CSP（`default-src 'none'; script-src 'self'; connect-src 'self'; img-src 'self'; frame-ancestors 'none'; form-action 'none'; base-uri 'none'`）。
- **任何 query string 一律 400**（`query_not_allowed`）——秘密永遠不放 URL。
- 請求解析逾時 2 秒；格式錯誤回 400 並只回報分類（`invalid_encoding` / `request_timeout` / `incomplete_request` / `request_parse_failed`）。
- Windows 陷阱：accepted socket 會繼承 listener 的非阻塞模式，必須明確 `set_nonblocking(false)`，並用**延遲、分段、不 shutdown 寫端**的請求測試（教訓 2026-09-10：立即寫入＋shutdown 的測試掩蓋了實機 USB 失敗）。看到一般 parse error 時**不要推論硬體或瀏覽器不相容**。

## 3. 配對（Device Pair）

流程（ADR 0003）：
1. 使用者**先在桌面選定 ADB serial**，桌面才顯示配對碼。
2. 配對碼：**8 位數**，由 OS 隨機源產生（`getrandom`，失敗就 panic——寧可不啟動也不用弱隨機），有效 **2 分鐘**，每個 mapping generation **最多 5 次**嘗試，超過必須由桌面重新產生。
3. App 開啟**不含秘密**的 landing page；使用者在平板輸入配對碼。
4. 成功後**立即使代碼失效（單次使用、防重放）**，設定 HttpOnly Device Pair cookie 與短效 Tablet Session。
5. 配對碼、pair material、token **不進 log、URL、診斷**。

Device Pair 持久化：DPAPI（Windows CurrentUser）保護的 verifier；原子替換；獨占鎖；密文損毀 → **fail closed、不退回明文**；明文上限 1 MiB、密文上限 16 MiB；token 64 hex、CSRF 32 hex；非 Windows 明確不支援。
Device Pair 存活到使用者撤銷、切換裝置或清除配對。

## 4. Tablet Session

- 預設 TTL **15 分鐘**，輪替；App 重啟、mapping generation 改變、帳號切換、裝置切換時失效，之後以 Device Pair 自動換新（`POST /api/v1/session`），正常情況**不再要求輸入配對碼**。
- 所有 dashboard / history / SSE / refresh 路由都要求高熵 session（`HttpOnly`、`SameSite`）。
- `POST /api/v1/refresh` 另需 **CSRF**（`x-csrf-token` 對應 session）。
- 不使用寬鬆 CORS。

## 5. 路由與檢查順序

私有路由：`/health`、`/api/v1/{dashboard,history,events,refresh,pairs}`、`/api/v1/monitor/ping`。公開：靜態資產、`POST /api/v1/pair`、`POST /api/v1/session`。

檢查順序（先便宜後昂貴，且每次拒絕都記錄**安全診斷碼**而不含內容）：
`Host` 白名單 → 拒絕 query → 靜態資產 → pair/session 專用處理 → 非私有路由 404 → `Origin` 必須等於伺服器 origin → session 認證（401）→ 紀錄已認證活動 → 方法分派（refresh 再檢 CSRF）。

診斷碼：`malformed_request`、`host_rejected`、`query_rejected`、`origin_rejected`、`authorization_rejected`、`csrf_rejected`。
**`/health` 也需要 session**，且只回最少資訊（`{"status":"ok"}`），不洩漏帳號、usage 或 serial。

### 刷新語意

`POST /api/v1/refresh {source}` → 一律 `202 Accepted`，body `{result, source}`，`result` ∈ `accepted | coalesced | throttled | paused | unsupported`；未知 source → 400。實際結果由後續 Snapshot 傳送。**不把任何 HTTP 提供的參數或憑證傳給收集器**；收集器 panic 以 `catch_unwind` 捕捉並標 `failure`。**Paused 時不執行、也不暗中解除暫停。**

### SSE

- 完整 Snapshot，15 秒 heartbeat（不增 revision）；寫入逾時 250 ms；session 失效時主動關閉串流。
- 客戶端：45 秒沒訊息 → 標示中斷；片段化事件要能正確組裝；重連後先取完整快照。

## 6. 資料操作要分開、範圍要精確

| 操作 | 影響 | 保留 |
|---|---|---|
| Clear History | usage、quota、health、alert | 設定、Provider Account、Account Link、Device Pair |
| Forget Account | 該帳號＋其 Link＋其全部 History | 其他帳號 |
| Clear Pairing | 撤銷全部 Tablet Session 與 Device Pair | 其他一切 |
| Reset AgentMeter | 全部 | —（**需二次確認**） |
| 卸載 | 預設**保留** LocalAppData；明確選項（`/PURGE`）才刪 | — |

刪除前回傳預估列數，完成後回傳實際列數；用 transaction＋scope guard。

## 7. ADB 規則

- 所有 `reverse`/`list`/`remove`/`shell` 都帶 `-s <serial>`；每次操作前確認 state 為 `device` 且符合 **USB-only policy**（`transport_kind = usb`；`tcpip`/`emulator`/`unknown` 不當成功）。多裝置時**禁止任選第一台**；`-d` 不能取代序號。
- 一律用**參數陣列**：`adb -s <serial> reverse --no-rebind tcp:<device_port> tcp:<host_port>`。`--no-rebind` 讓未知 mapping 衝突原子失敗，而不是覆蓋。
- 移除只做 `reverse --remove tcp:<owned_device_port>`；**絕不** `kill-server`、`--remove-all` 或動別人的 mapping。mapping 以 generation 識別所有權；owned / missing / changed 三種狀態分開處理，外部 mapping 一律拒絕接管。
- 開啟瀏覽器：僅在 state / transport / reverse health 驗證通過後，`am start -W -a android.intent.action.VIEW -d <url>`；結果只記 `browser_launch_requested`，**不等於頁面已載入或 browser online**。
- 輸出上限（64 KiB）、有界逾時。
- 四個連線狀態分開：`usb_attached` → `adb_authorized` → `reverse_healthy` → `browser_session_online`；**最後一項只能由已認證的 HTTP/SSE heartbeat 決定**（區分「瀏覽器啟動」與「真有認證請求」）。
- USB 重插、adbd 重啟、Windows 喚醒後都重新驗證並按需重建，失敗採退避。受管理平板可能禁止 USB 偵錯——列為環境阻礙，不繞過政策。

## 8. 平板 UI（Monitor-only）

- 可：檢視、展開、切換主題、請求受節流刷新、選擇要顯示的 Provider 卡（**永遠保留一個空的新增槽**）、全螢幕（需使用者手勢）。
- 不可：修改 Source、登入、清資料、執行命令、控制 Provider、退出 App。
- 直向為首要，旋轉橫向仍可用；觸控目標 ≥ 44 CSS px；一般文字 ≥ 16 CSS px，主數值 36–48 CSS px；深色預設＋亮色；異常不單靠紅綠辨識（加文字標籤）。
- 依實際 CSS viewport／縮放驗收，不用面板像素寫死。保持亮屏是 best-effort，需附 Android「充電時保持喚醒」說明。
- 敏感值顯示：配對碼畫面保留「請勿截圖／分享」的安全文案——**安全文案是被測試保護的產品契約**，不能被視覺改版順手刪掉（教訓 2026-09-11）。

## 9. 必做的負向測試（AC-15、24、25）

未配對的本機程序、另一個 App／browser profile、惡意 Host/Origin、缺少或錯誤的 CSRF、重放配對碼、舊 generation 的 session、query 中夾帶 secret、同 LAN 主機連線——**全部必須拿不到資料也不能 refresh**；並用哨兵字串驗證回應與診斷不含 secret。
