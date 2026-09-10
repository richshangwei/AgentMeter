const get = id => document.getElementById(id);
let session = '', csrf = '', pairCsrf = '';
const acceptSnapshot = createSnapshotGate();
const recovery = createPairRecovery({
  getItem: key => localStorage.getItem(key),
  setItem: (key,value) => localStorage.setItem(key,value),
  removeItem: key => localStorage.removeItem(key)
});
let controller, connecting = false, retry;
async function api(path, body = {}, signal) {
  return fetch(path, {method:'POST', credentials:'same-origin', cache:'no-store',
    headers:{'Content-Type':'application/json', Authorization:`Bearer ${session}`, 'X-CSRF-Token':csrf},
    body:JSON.stringify(body), signal});
}
function render(value, allowNewStream = false) {
  if (!acceptSnapshot(value, allowNewStream)) return;
  get('revision').textContent = `版本 ${value.revision} · 完整快照 · ${value.provider_data === 'live' ? '桌面收集資料' : '模擬資料'}`;
  get('cards').replaceChildren();
  for (const provider of value.providers) {
    const card = document.createElement('article'), title = document.createElement('h2');
    title.textContent = provider.provider; card.append(title);
    const details = document.createElement('details'), summary = document.createElement('summary');
    summary.textContent = '資料來源與狀態'; details.append(summary);
    const names = {setup:'連線設定',paused:'暫停',availability:'來源狀態',collection_state:'收集狀態',freshness:'新鮮度',data_quality:'資料品質',collector_maturity:'支援程度',failure_code:'問題代碼'};
    for (const [key,label] of Object.entries(names)) {
      const row = document.createElement('p'); row.textContent = `${label}：${provider[key] ?? '未知'}`; details.append(row);
    }
    if (!(provider.quota_windows || []).length) {
      const row = document.createElement('p'); row.textContent = '剩餘額度：尚未取得'; card.append(row);
    }
    for (const quota of provider.quota_windows || []) {
      const row = document.createElement('p'); row.textContent = `${quota.label || quota.bucket_key || '額度'}：${quota.remaining_percent == null ? '未知' : `${quota.remaining_percent}% 剩餘`}`; card.append(row);
    }
    for (const usage of provider.source_usage || []) {
      const row = document.createElement('p'); row.textContent = `${usage.label || usage.model || '使用量'}：${usage.used ?? '未知'} ${usage.unit || ''}`; card.append(row);
    }
    const stamp = document.createElement('p'); stamp.textContent = `資料時間：${provider.collected_at ? new Date(Number(provider.collected_at)).toLocaleString('zh-TW') : '尚未取得'}`; card.append(stamp);
    card.append(details);
    const button = document.createElement('button'); button.textContent = '要求重新整理';
    button.onclick = async () => {
      button.disabled = true;
      try {
        const response = await api('/api/v1/refresh', {source:provider.provider});
        const result = await response.json();
        const messages = {accepted:'已接受，等待新快照。',coalesced:'正在更新中，請稍候。',throttled:'剛剛已更新，請稍後再試。',unsupported:'此來源尚未支援更新。',paused:'監控已暫停，請在桌面恢復。'};
        get('status').textContent = response.status === 202 ? (messages[result.result] || '請稍後重試。') : '重新整理未接受；請稍後重試。';
      } catch { get('status').textContent = '連線中斷，顯示的數值可能已過期。'; }
      finally { button.disabled = false; }
    };
    card.append(button); get('cards').append(card);
  }
}
async function connect() {
  if (connecting || !session) return;
  connecting = true; clearTimeout(retry); controller = new AbortController();
  const currentController = controller;
  const watchdog = createWatchdog(() => currentController.abort());
  watchdog.touch();
  try {
    let response = await api('/api/v1/dashboard', {}, controller.signal);
    if (response.status === 401 && pairCsrf) {
      const exchange = await fetch('/api/v1/session', {method:'POST',credentials:'same-origin',
        headers:{'Content-Type':'application/json','X-CSRF-Token':pairCsrf},body:'{}',signal:controller.signal});
      if (!exchange.ok) {
        if (exchange.status === 401 || exchange.status === 403) { recovery.clear(); pairCsrf = ''; session = ''; get('pairing').hidden = false; }
        throw Error('pair exchange failed');
      }
      const next = await exchange.json(); session = next.tablet_session; csrf = next.csrf_token;
      response = await api('/api/v1/dashboard', {}, controller.signal);
    }
    if (!response.ok) throw Error('dashboard unavailable');
    const initial = await response.json();
    render(initial, true); get('status').textContent = initial.provider_data === 'live' ? '已連接桌面 · 請核對各來源的資料時間' : '已連接 · 模擬資料';
    const events = await api('/api/v1/events', {}, controller.signal);
    if (!events.ok || !events.body) throw Error('stream unavailable');
    const reader = events.body.getReader(), decoder = new TextDecoder();
    const parse = createEventParser(value => render(value), () => watchdog.touch());
    while (true) {
      const {done,value} = await reader.read(); if (done) throw Error('stream closed');
      parse(decoder.decode(value, {stream:true}));
    }
  } catch {
    get('status').textContent = session ? '連線中斷；舊資料可能過期，正在重新連線。' : '需要重新配對。';
  } finally {
    watchdog.stop();
    controller.abort(); connecting = false;
    if (session) retry = setTimeout(connect, 3000);
  }
}
get('pair').onclick = async () => {
  get('pair').disabled = true;
  try {
    const code = get('code').value; get('code').value = '';
    if (!/^\d{8}$/.test(code)) throw Error('invalid code');
    const response = await api('/api/v1/pair', {code});
    if (!response.ok) throw Error('pair failed');
    const result = await response.json(); session = result.tablet_session; csrf = result.csrf_token; pairCsrf = result.pair_csrf_token;
    const saved = recovery.save(pairCsrf);
    get('recovery-status').textContent = saved ? '已保留此瀏覽器的配對恢復資料。' : '瀏覽器不允許儲存；重新開啟後需重新配對。';
    get('pairing').hidden = true; connect();
  } catch { get('status').textContent = '配對未成功，請在桌面確認有效配對碼。'; }
  finally { get('code').value = ''; get('pair').disabled = false; }
};
window.addEventListener('online', connect);
async function restorePair() {
  pairCsrf = recovery.read();
  if (!pairCsrf) return;
  get('pair').disabled = true;
  const attempt = new AbortController();
  const deadline = setTimeout(() => attempt.abort(), 10000);
  try {
    const response = await fetch('/api/v1/session', {method:'POST',credentials:'same-origin',cache:'no-store',
      headers:{'Content-Type':'application/json','X-CSRF-Token':pairCsrf},body:'{}',signal:attempt.signal});
    if (!response.ok) {
      if (response.status === 401 || response.status === 403) { recovery.clear(); pairCsrf = ''; }
      throw Error('restore failed');
    }
    const result = await response.json(); session = result.tablet_session; csrf = result.csrf_token;
    get('pairing').hidden = true; get('recovery-status').textContent = '已恢復配對，正在取得最新完整快照。';
    connect();
  } catch { get('status').textContent = '暫時無法恢復配對。確認主機已啟動且網址相同，再重新載入；配對已撤銷時請重新配對。'; }
  finally { clearTimeout(deadline); get('pair').disabled = false; }
}
restorePair();
