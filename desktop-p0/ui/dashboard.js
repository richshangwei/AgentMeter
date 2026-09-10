const get = id => document.getElementById(id);
const labels = {available:'可讀取',not_installed:'未找到 Codex',needs_login:'需要登入',setup_required:'待設定',permission_denied:'權限不足',unsupported:'尚不支援',unknown:'狀態未知'};
const time = value => value == null ? '尚無資料' : new Date(Number(value)).toLocaleString('zh-TW');
let pairCodeTimer;
let tabletRunning = false;
function clearPairCode() {
  clearTimeout(pairCodeTimer);
  pairCodeTimer = undefined;
  get('tablet-code').textContent = '••••••••';
}
function renderTablet(status) {
  const running = status.running === true;
  tabletRunning = running;
  const usb = running ? status.usb : null;
  const mapping = usb?.mapping?.state || 'not_configured';
  const authenticated = status.transport?.authenticated_session_established === true;
  get('tablet-status').textContent = running
    ? `服務已啟動 · 同步桌面收集資料 · ${authenticated ? '已有認證平板活動' : '尚未觀察到認證平板活動'}`
    : '服務未啟動';
  get('tablet-origin').textContent = running
    ? `本機端點：${status.origin}；需另行建立選定裝置的 ADB reverse。`
    : '尚未啟動。啟動服務不會自動建立 ADB mapping。';
  get('tablet-usb-status').textContent = usb
    ? `USB serial：${usb.selected_serial} · device ${usb.device_port} → host ${usb.host_port} · mapping ${mapping} · browser ${usb.browser_launch_requested ? '已要求開啟' : '未要求'} · authenticated ${authenticated ? 'online' : 'not observed'}`
    : 'USB 尚未設定；瀏覽器啟動與已認證在線狀態會分開顯示。';
  get('tablet-start').disabled = running;
  get('tablet-stop').disabled = !running;
  get('tablet-pair').disabled = !running;
  get('tablet-clear').disabled = !running;
  get('tablet-usb-connect').disabled = !running || !!usb;
  get('tablet-usb-open').disabled = !usb || mapping !== 'owned';
  get('tablet-usb-recover').disabled = !usb || mapping === 'changed';
  get('tablet-usb-disconnect').disabled = !usb;
  for (const id of ['adb-path','device-serial','device-port']) get(id).disabled = !running || !!usb;
  if (!running) clearPairCode();
}
function renderTabletActivity(status) {
  if (!tabletRunning || status.running !== true) return;
  const transport = status.transport || {};
  const authenticated = transport.authenticated_session_established === true;
  get('tablet-status').textContent = `服務已啟動 · 同步桌面收集資料 · ${authenticated ? `已有認證平板活動（${transport.authenticated_request_count} 次請求）` : '尚未觀察到認證平板活動'}`;
}
async function tabletCommand(command, args = {}) {
  const invoke = window.__TAURI__?.core?.invoke;
  if (!invoke) throw new Error('desktop_bridge_unavailable');
  return invoke(command, args);
}
async function restoreTabletStatus() {
  try { renderTablet(await tabletCommand('tablet_status')); }
  catch { get('tablet-status').textContent = '無法讀取平板服務狀態。'; }
}
async function pollTabletActivity() {
  if (!tabletRunning) return;
  try { renderTabletActivity(await tabletCommand('tablet_activity')); }
  catch { get('tablet-status').textContent = '平板活動狀態暫時無法讀取。'; }
}
get('tablet-start').addEventListener('click', async () => {
  get('tablet-start').disabled = true;
  try { renderTablet(await tabletCommand('tablet_start')); }
  catch { get('tablet-status').textContent = '平板服務啟動失敗；請確認本機資料目錄與連接埠。'; }
});
get('tablet-stop').addEventListener('click', async () => {
  get('tablet-stop').disabled = true;
  try { renderTablet(await tabletCommand('tablet_stop')); }
  catch { get('tablet-status').textContent = '平板服務停止失敗；請使用通知區的完整退出。'; }
});
get('tablet-pair').addEventListener('click', async () => {
  get('tablet-pair').disabled = true;
  clearPairCode();
  try {
    const result = await tabletCommand('tablet_pair_code');
    get('tablet-code').textContent = result.code;
    pairCodeTimer = setTimeout(clearPairCode, result.expires_in_seconds * 1000);
  } catch { get('tablet-status').textContent = '無法產生配對碼；請先啟動服務。'; }
  finally { get('tablet-pair').disabled = false; }
});
get('tablet-clear').addEventListener('click', async () => {
  if (!window.confirm('確定要撤銷所有平板配對與目前工作階段？')) return;
  get('tablet-clear').disabled = true;
  try {
    await tabletCommand('tablet_clear_pairing');
    clearPairCode();
    get('tablet-status').textContent = '所有 Device Pair 與 Tablet Session 已撤銷。服務仍在執行。';
  } catch { get('tablet-status').textContent = '配對撤銷未能安全寫入；授權服務已停止，請完整退出後檢查資料目錄。'; }
  finally { get('tablet-clear').disabled = false; }
});
get('tablet-usb-connect').addEventListener('click', async () => {
  const adbPath = get('adb-path').value.trim();
  const serial = get('device-serial').value.trim();
  const devicePort = Number(get('device-port').value);
  if (!adbPath || !serial || !Number.isInteger(devicePort) || devicePort < 1 || devicePort > 65535) {
    get('tablet-usb-status').textContent = '請輸入 ADB 完整路徑、明確 serial 與有效 device port。';
    return;
  }
  get('tablet-usb-connect').disabled = true;
  try { renderTablet(await tabletCommand('tablet_usb_connect', {adbPath, serial, devicePort})); }
  catch (error) {
    const detail = String(error);
    get('tablet-usb-status').textContent = detail.includes('non_usb_transport')
      ? 'ADB 未回報可用的實體 USB transport。請關閉平板「無線偵錯」、確認 USB 線可傳資料並重新接受 USB 偵錯授權，再執行 adb kill-server、adb start-server 後重試。'
      : `USB mapping 失敗：${detail}`;
  }
  finally { if (!get('device-serial').disabled) get('tablet-usb-connect').disabled = false; }
});
get('tablet-usb-open').addEventListener('click', async () => {
  get('tablet-usb-open').disabled = true;
  try { renderTablet(await tabletCommand('tablet_usb_open')); }
  catch (error) { get('tablet-usb-status').textContent = `瀏覽器開啟要求失敗：${String(error)}`; }
});
get('tablet-usb-recover').addEventListener('click', async () => {
  get('tablet-usb-recover').disabled = true;
  try { renderTablet(await tabletCommand('tablet_usb_recover')); }
  catch (error) { get('tablet-usb-status').textContent = `USB 復原未完成：${String(error)}`; }
});
get('tablet-usb-disconnect').addEventListener('click', async () => {
  get('tablet-usb-disconnect').disabled = true;
  try { renderTablet(await tabletCommand('tablet_usb_disconnect')); }
  catch (error) {
    await restoreTabletStatus();
    get('tablet-usb-status').textContent = `USB 中斷結果：${String(error)}`;
  }
});

const quotaErrors = {
  authentication_required:'請先在對應服務登入，完成後按重新整理。',
  cli_not_found:'尚未找到官方 CLI。請先安裝並登入該服務的官方命令列工具，再按「重新整理」。',
  workspace_trust_required:'請按「啟用 Claude 讀取」，只需同意一次。',
  collector_runtime_missing:'程式的收集元件不完整，請重新安裝 AgentMeter。',
  terminal_probe_unavailable:'Claude 終端元件無法啟動，請確認 Claude 可正常使用。',
  terminal_quota_timeout:'Claude 未及時回傳額度，請確認登入後重試。',
  terminal_schema_changed:'Claude 畫面格式變更，暫時無法辨識額度。',
  quota_schema_unrecognized:'官方額度格式變更，暫時無法辨識。',
  cli_quota_request_failed:'Antigravity 額度讀取失敗，請確認官方工具的登入狀態。',
  permission_denied:'目前帳號沒有讀取此額度的權限。',
  timeout:'讀取逾時，請稍後重試。',
  quota_missing:'服務沒有回傳可用額度。',
  refresh_failed:'更新失敗，請稍後重試。'
};
const providerSetup = {
  codex:'Codex：安裝 Codex CLI 並完成登入（codex login），再按「重新整理」。',
  claude:'Claude Code：安裝 Claude Code CLI 並完成登入（claude login），再按「重新整理」。',
  copilot:'GitHub Copilot：安裝 GitHub Copilot CLI，執行 copilot login（或 gh auth login）後按「重新整理」。',
  antigravity:'Antigravity：請重新執行 AgentMeter 安裝程式以補齊內建官方工具，確認後再按「重新整理」。'
};
function quotaErrorMessage(provider, code) {
  if (code === 'cli_not_found') return providerSetup[provider] || quotaErrors.cli_not_found;
  if (code === 'authentication_required') return `${providerSetup[provider] || '請先完成官方工具登入。'} 若已登入仍失敗，請確認使用的是目前 Windows 帳號。`;
  return quotaErrors[code] || '暫時無法讀取。請依上方說明處理後按「重新整理」；若仍失敗，請重新啟動 AgentMeter。';
}
let localRefreshing = false;
let syncingSnapshot = false;
function quotaLabel(item) {
  return (item.label || item.bucket_key || '額度')
    .replace('base_model_inference','基本模型').replace('premium_interactions','Premium interactions')
    .replace('codex primary','Codex 主視窗').replace('codex secondary','Codex 次視窗')
    .replace('five_hour','5 小時').replace('seven_day','每週')
    .replace('Gemini Models','Gemini').replace('Claude and GPT models','Claude / GPT')
    .replace('Weekly Limit Remaining','每週').replace('Five Hour Limit Remaining','5 小時');
}
function render(snapshot) {
  const busy = localRefreshing || snapshot.refreshing === true;
  for (const button of document.querySelectorAll('[data-provider], #refresh, #claude-enable')) button.disabled = busy;
  let ready = 0;
  for (const provider of snapshot.provider_states || []) {
    const name = provider.provider;
    const failed = provider.collection_state === 'error';
    const stale = provider.freshness === 'stale';
    const windows = provider.quota_windows || [];
    if (!failed && windows.length) ready++;
    get(name+'-status').textContent = failed ? (stale ? '更新失敗 · 舊資料' : '需要處理') : windows.length ? '已連線' : '等待讀取';
    get(name+'-status').className = failed ? 'status-error' : '';
    const quota = get(name+'-quota');
    quota.replaceChildren();
    if (!windows.length) quota.textContent = '— 尚未取得額度';
    for (const item of windows) {
      const row = document.createElement('div'); row.className = 'window';
      const label = document.createElement('div'); label.className = 'detail'; label.textContent = quotaLabel(item);
      const value = document.createElement('div');
      const remaining = typeof item.remaining_percent === 'number' && Number.isFinite(item.remaining_percent)
        ? Math.max(0, Math.min(100, item.remaining_percent)) : null;
      value.textContent = remaining === null ? '剩餘未知' : remaining+'% 剩餘';
      const track = document.createElement('div'); track.className = 'quota-track';
      if (remaining !== null) {
        const fill = document.createElement('div');
        fill.className = 'quota-fill ' + (remaining > 30 ? 'quota-good' : remaining > 10 ? 'quota-warning' : 'quota-low');
        fill.style.width = remaining+'%';
        track.append(fill);
      } else { track.className += ' quota-unknown'; }
      const reset = document.createElement('div'); reset.className = 'detail';
      reset.textContent = item.reset_display ? '重設：'+item.reset_display :
        item.resets_at == null ? '重設時間尚未確認' : '重設：'+new Date(typeof item.resets_at === 'number' ? item.resets_at*1000 : item.resets_at).toLocaleString('zh-TW');
      row.append(label,value,track,reset);
      if (typeof item.entitlement === 'number' && typeof item.used === 'number') {
        const used = document.createElement('div'); used.className = 'detail';
        used.textContent = '已用 '+item.used+' / '+item.entitlement; row.append(used);
      }
      quota.append(row);
    }
    get(name+'-time').textContent = '資料更新：'+time(provider.collected_at)+(stale ? '（舊資料，不代表目前額度）':'');
    get(name+'-message').textContent = failed ? quotaErrorMessage(name, provider.failure_code) :
      name === 'copilot' ? '顯示 Premium interactions 權益，不是 Billing 或 AI credits 餘額。' :
      name === 'claude' ? '自動讀取官方 /usage；終端格式相容性仍屬實驗性。' :
      name === 'antigravity' ? '自動讀取官方 CLI 額度；不需要手動提供資料檔。' : '沿用本機 Codex 登入，自動取得官方額度。';
    if (name === 'claude') get('claude-enable').hidden = provider.failure_code !== 'workspace_trust_required';
  }
  get('message').textContent = busy ? '正在讀取額度…' : ready+'/4 個服務已取得額度';
}
async function refreshQuota(provider = 'all', enableClaude = false) {
  if (localRefreshing) return;
  localRefreshing = true;
  get('message').textContent = '正在讀取額度，請稍候…';
  try {
    const result = await tabletCommand('refresh_quota',{provider,enableClaude});
    localRefreshing = false; render(result);
  } catch(error) {
    get('message').textContent = String(error).includes('busy') ? '正在更新中，請稍候。' : '更新未完成，保留先前資料；請稍後重試。';
  } finally { localRefreshing = false; await pollQuota(); }
}
async function pollQuota() {
  if (syncingSnapshot) return;
  syncingSnapshot = true;
  try { render(await tabletCommand('snapshot')); }
  catch { get('message').textContent = '桌面服務暫時無法使用，請重新啟動 AgentMeter。'; }
  finally { syncingSnapshot = false; }
}
get('refresh').addEventListener('click',() => refreshQuota());
for (const button of document.querySelectorAll('[data-provider]')) {
  button.addEventListener('click',() => refreshQuota(button.dataset.provider));
}
get('claude-enable').addEventListener('click',() => {
  if (window.confirm('允許 Claude 信任 AgentMeter 專用的隔離工作目錄，以讀取額度？只需同意一次；不變更其他專案的權限，不開放模型工具。')) refreshQuota('claude',true);
});
get('auto-quota').addEventListener('change',async event => {
  try { await tabletCommand('set_auto_quota',{enabled:event.target.checked}); }
  catch { event.target.checked = !event.target.checked; }
});
restoreTabletStatus();
pollQuota();
setInterval(pollTabletActivity, 2000);
setInterval(pollQuota,2000);
