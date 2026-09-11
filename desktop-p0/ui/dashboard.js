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
const desktopGuides = {
  codex:{name:'Codex',url:'https://learn.chatgpt.com/docs/codex/cli',steps:['依 Windows 官方說明安裝 Codex CLI。','在終端機執行 codex，使用 ChatGPT 帳號完成登入。','回到 AgentMeter 按「更新」，確認卡片出現額度。']},
  claude:{name:'Claude Code',url:'https://docs.anthropic.com/en/docs/claude-code/getting-started',steps:['依 Anthropic 官方說明安裝 Claude Code。','在終端機執行 claude，完成登入；若提示工作目錄信任，回 AgentMeter 按「啟用讀取」。','回到 AgentMeter 按「更新」，確認卡片出現額度。']},
  copilot:{name:'GitHub Copilot',url:'https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli',steps:['依 GitHub 官方說明安裝 Copilot CLI。','執行 copilot login；必要時先執行 gh auth login，完成帳號驗證。','確認方案或組織政策允許 Copilot CLI，再回 AgentMeter 按「更新」。']},
  antigravity:{name:'Antigravity',url:'https://codelabs.developers.google.com/antigravity-cli-hands-on',steps:['先重新執行 AgentMeter 安裝程式，補齊內建收集元件。','若仍缺少工具，依 Google 官方教學安裝並執行 agy 完成登入。','回到 AgentMeter 按「更新」，確認卡片出現額度。']}
};
function showDesktopGuide(provider){const guide=desktopGuides[provider]||{name:desktopProviderName(provider),url:'',steps:['安裝此監控來源的官方工具。','完成官方工具登入或授權。','回到 AgentMeter 按「更新」重試。']};get('desktop-guide-title').textContent=`${guide.name} 安裝與連線`;get('desktop-guide-steps').replaceChildren(...guide.steps.map(text=>{const item=document.createElement('li');item.textContent=text;return item;}));const link=get('desktop-guide-link');link.hidden=!guide.url;if(guide.url)link.href=guide.url;else link.removeAttribute('href');get('desktop-guide-dialog').showModal();}
function updateDesktopGuideButton(provider,visible){const card=get(provider+'-card');if(!card?.querySelector)return;let button=card.querySelector('.desktop-guide-button');if(!button){const update=card.querySelector('button[data-provider]');if(!update)return;let row=update.parentElement?.classList?.contains('button-row')?update.parentElement:null;if(!row){row=document.createElement('div');row.className='button-row';update.replaceWith(row);row.append(update);}button=document.createElement('button');button.className='ghost desktop-guide-button';button.textContent='安裝步驟';button.onclick=()=>showDesktopGuide(provider);row.append(button);}button.hidden=!visible;}
const desktopCatalog = {
  codex:{name:'Codex'},claude:{name:'Claude Code'},copilot:{name:'GitHub Copilot'},antigravity:{name:'Antigravity'},
  cursor:{name:'Cursor',catalogOnly:true},kiro:{name:'Kiro',catalogOnly:true}
};
const desktopNames=Object.fromEntries(Object.entries(desktopCatalog).map(([id,item])=>[id,item.name]));
const coreDesktopProviders=['codex','claude','copilot','antigravity'];
let desktopAvailable = [...coreDesktopProviders], desktopSelection = null, desktopPage = 0, desktopOptionPage = 0, desktopOptionsSignature = '';
function validDesktopProviderId(value) {
  return typeof value === 'string' && /^[a-z0-9][a-z0-9._-]{0,79}$/.test(value) && value !== 'constructor' && value !== '__proto__' && !value.startsWith('desktop-');
}
function desktopProviderName(provider) {
  return Object.prototype.hasOwnProperty.call(desktopNames,provider) ? desktopNames[provider] : provider;
}
function loadDesktopSelection() {
  if (desktopSelection !== null) return desktopSelection;
  try {
    const raw = window.localStorage?.getItem(desktopMonitorKey);
    desktopSelection = normalizeDesktopSelection(raw == null ? null : JSON.parse(raw),desktopAvailable);
  } catch { desktopSelection = normalizeDesktopSelection(null,desktopAvailable); }
  return desktopSelection;
}
function saveDesktopSelection() {
  try { window.localStorage?.setItem(desktopMonitorKey,JSON.stringify(desktopSelection)); } catch {}
}
function ensureDesktopCard(provider) {
  if (document.getElementById(provider+'-card')) return;
  const card=document.createElement('article');card.id=provider+'-card';card.className='monitor-card';card.dataset.providerCard=provider;
  const heading=document.createElement('div');heading.className='heading';const titleWrap=document.createElement('div');titleWrap.className='provider-title';
  const mark=document.createElement('span');mark.className='provider-mark';mark.textContent=(provider[0]||'?').toUpperCase();mark.setAttribute('aria-hidden','true');
  const titleBox=document.createElement('div'),title=document.createElement('h2'),subtitle=document.createElement('p');title.textContent=desktopProviderName(provider);subtitle.textContent='AI Agent';titleBox.append(title,subtitle);titleWrap.append(mark,titleBox);
  const status=document.createElement('span');status.id=provider+'-status';status.textContent='等待讀取';heading.append(titleWrap,status);
  const quota=document.createElement('div');quota.id=provider+'-quota';quota.className='quota';quota.textContent='—';quota.setAttribute('role','region');quota.setAttribute('aria-label',desktopProviderName(provider)+' 額度明細');
  const foot=document.createElement('div');foot.className='card-foot';
  const timeNode=document.createElement('p');timeNode.id=provider+'-time';timeNode.className='hint';timeNode.textContent='尚無資料';
  const row=document.createElement('div');row.className='button-row';
  const button=document.createElement('button');button.dataset.provider=provider;button.textContent='更新';button.addEventListener('click',()=>refreshQuota(provider));row.append(button);
  const message=document.createElement('p');message.id=provider+'-message';message.className='hint card-message';message.setAttribute('role','status');
  foot.append(timeNode,row,message);card.append(heading,quota,foot);get('desktop-cards').append(card);observeQuotaRegion(quota);
}
function renderDesktopOptions() {
  loadDesktopSelection();
  const optionIds=[...desktopSelection,...Object.keys(desktopCatalog).filter(id=>!desktopSelection.includes(id)),...desktopAvailable.filter(id=>!desktopSelection.includes(id)&&!Object.prototype.hasOwnProperty.call(desktopCatalog,id))];
  const optionHeight=Number(window.innerHeight)||640,pageSize=optionHeight<520?2:optionHeight<760?4:6,pages=Math.max(1,Math.ceil(optionIds.length/pageSize));
  desktopOptionPage=Math.max(0,Math.min(desktopOptionPage,pages-1));
  const signature=JSON.stringify([optionIds,desktopAvailable,desktopSelection,desktopOptionPage,pageSize]);
  if(signature===desktopOptionsSignature)return;
  desktopOptionsSignature=signature;
  const host=get('desktop-monitor-options');
  const visibleOptions=optionIds.slice(desktopOptionPage*pageSize,(desktopOptionPage+1)*pageSize);
  host.replaceChildren(...visibleOptions.map(id=>{
    const row=document.createElement('div');row.className='monitor-option';
    const monitorable=desktopAvailable.includes(id),selected=desktopSelection.includes(id);
    if(monitorable){const input=document.createElement('input');input.type='checkbox';input.checked=selected;input.setAttribute('aria-label',`${desktopProviderName(id)} 顯示在主畫面`);input.onchange=()=>{desktopSelection=input.checked?[...desktopSelection,id]:desktopSelection.filter(value=>value!==id);desktopSelection=normalizeDesktopSelection(desktopSelection,desktopAvailable);desktopPage=0;desktopOptionsSignature='';saveDesktopSelection();applyDesktopLayout();renderDesktopOptions();};row.append(input);}
    else{const pending=document.createElement('span');pending.className='catalog-dot';pending.setAttribute('aria-hidden','true');row.append(pending);}
    const info=document.createElement('span');info.className='monitor-option-info';const name=document.createElement('strong');name.textContent=desktopProviderName(id);const state=document.createElement('small');state.textContent=monitorable?(selected?'顯示中 · 可監控':'可監控 · 未顯示'):'尚未支援監控 · 安裝狀態未檢查';info.append(name,state);row.append(info);
    if(selected){const order=document.createElement('span');order.className='monitor-order';const index=desktopSelection.indexOf(id);for(const [direction,label] of [[-1,'上移'],[1,'下移']]){const button=document.createElement('button');button.type='button';button.className='ghost';button.textContent=label;button.setAttribute('aria-label',`${desktopProviderName(id)}${label}`);button.disabled=direction<0?index===0:index===desktopSelection.length-1;button.onclick=()=>{desktopSelection=moveDesktopMonitor(desktopSelection,id,direction);desktopPage=0;desktopOptionsSignature='';saveDesktopSelection();applyDesktopLayout();renderDesktopOptions();};order.append(button);}row.append(order);}
    else if(!monitorable){const badge=document.createElement('span');badge.className='catalog-badge';badge.textContent='候選 Agent';row.append(badge);}
    return row;
  }));
  get('desktop-monitor-count').textContent=`主畫面顯示 ${desktopSelection.length} 個監控；每頁最多 4 張，更多項目會自動分頁。`;
  const pager=get('desktop-options-pager');pager.hidden=pages<=1;get('desktop-options-page').textContent=`${desktopOptionPage+1} / ${pages}`;get('desktop-options-prev').disabled=desktopOptionPage===0;get('desktop-options-next').disabled=desktopOptionPage===pages-1;
}
function measureDesktopCards(cards) {
  const width=Number(cards.clientWidth),height=Number(cards.clientHeight);
  let gap=12;
  if(typeof getComputedStyle==='function'){const value=parseFloat(getComputedStyle(cards).rowGap);if(Number.isFinite(value))gap=value;}
  return {width,height,gap};
}
function layoutQuotaRegion(quota) {
  if(!quota||quota.hidden)return;
  const width=Number(quota.clientWidth),height=Number(quota.clientHeight);
  if(!(width>0&&height>0)||!quota.style?.setProperty)return;
  const tiles=[...(quota.children||[])].filter(node=>node.classList?.contains('window'));
  quota.style.setProperty('--q-h',height+'px');
  if(!tiles.length){
    quota.dataset.variant='empty';
    quota.dataset.emptyShape=height<96?'row':'column';
    return;
  }
  delete quota.dataset.emptyShape;
  const layout=quotaTileLayout(width,height,tiles.length);
  if(!layout)return;
  quota.dataset.variant=layout.variant;quota.dataset.used=String(layout.showUsed);quota.dataset.track=layout.inlineTrack?'inline':'none';
  quota.style.setProperty('--q-reset-lines',String(layout.resetLines));quota.style.setProperty('--q-label-lines',String(layout.labelLines));
  for(const [name,value] of [['--q-cols',layout.columns],['--q-rows',layout.rows],['--q-gap',layout.gap+'px'],['--q-tile-w',layout.tileWidth+'px'],['--q-tile-h',layout.tileHeight+'px'],
    ['--q-pad',layout.pad+'px'],['--q-ring',layout.ring+'px'],['--q-label',layout.label+'px'],['--q-value',layout.value+'px'],['--q-small',layout.small+'px']])quota.style.setProperty(name,String(value));
}
function layoutVisibleQuotaRegions() {
  if(!document.querySelectorAll)return;
  for(const quota of document.querySelectorAll('.monitor-card:not([hidden]) .quota'))layoutQuotaRegion(quota);
}
let quotaObserver=null;
function observeQuotaRegion(quota) {
  if(typeof ResizeObserver!=='function'||!quota)return;
  if(!quotaObserver)quotaObserver=new ResizeObserver(entries=>{for(const entry of entries)layoutQuotaRegion(entry.target);});
  quotaObserver.observe(quota);
}
function applyDesktopLayout() {
  loadDesktopSelection();
  const cards=get('desktop-cards'),area=measureDesktopCards(cards);
  const pageSize=desktopPageCapacity(area.width,area.height,desktopSelection.length,area.gap,Math.max(1,...desktopSelection.map(id=>desktopTileCounts[id]||1)));
  const page=pagedMonitorIds(desktopSelection,desktopPage,pageSize),grid=desktopViewportGrid(area.width,area.height,page.ids.length);desktopPage=page.page;
  const visible=new Set(page.ids);for(const id of desktopAvailable){const card=get(id+'-card');if(card){card.hidden=!visible.has(id);card.style.order=String(page.ids.indexOf(id));}}
  const empty=get('desktop-empty');if(empty)empty.hidden=page.ids.length!==0;
  cards.dataset.count=String(page.ids.length);
  if(area.width>0&&area.height>0){
    const cardWidth=(area.width-area.gap*(grid.columns-1))/grid.columns,cardHeight=(area.height-area.gap*(grid.rows-1))/grid.rows;
    const shape=desktopCardShape(cardWidth,cardHeight);
    cards.dataset.shape=shape;cards.dataset.density=desktopCardDensity(cardWidth,cardHeight,shape);
  }
  if(cards.style.setProperty){cards.style.setProperty('--grid-columns',grid.columns);cards.style.setProperty('--grid-rows',grid.rows);}
  const pager=get('desktop-pager');pager.hidden=page.pages<=1;get('desktop-page').textContent=`${page.page+1} / ${page.pages}`;get('desktop-prev').disabled=page.page===0;get('desktop-next').disabled=page.page===page.pages-1;
  layoutVisibleQuotaRegions();
}
const desktopTileCounts={};
let desktopLayoutFrame=0;
function scheduleDesktopLayout() {
  if(typeof requestAnimationFrame!=='function'){applyDesktopLayout();return;}
  if(desktopLayoutFrame)return;
  desktopLayoutFrame=requestAnimationFrame(()=>{desktopLayoutFrame=0;applyDesktopLayout();});
}
function quotaErrorMessage(provider, code) {
  if (code === 'cli_not_found') return providerSetup[provider] || quotaErrors.cli_not_found;
  if (code === 'authentication_required') return `${providerSetup[provider] || '請先完成官方工具登入。'} 若已登入仍失敗，請確認使用的是目前 Windows 帳號。`;
  return quotaErrors[code] || '暫時無法讀取。請依上方說明處理後按「重新整理」；若仍失敗，請重新啟動 AgentMeter。';
}
let localRefreshing = false;
let syncingSnapshot = false;
const shortTime = value => value == null ? '尚無資料' : new Date(Number(value)).toLocaleString('zh-TW',{month:'numeric',day:'numeric',hour:'2-digit',minute:'2-digit'});
function displayPercent(value) {
  return String(Math.round(value * 10) / 10);
}
function render(snapshot) {
  const busy = localRefreshing || snapshot.refreshing === true;
  for (const button of document.querySelectorAll('[data-provider], #refresh, #claude-enable')) button.disabled = busy;
  let ready = 0;
  const states=(snapshot.provider_states || []).filter(item=>validDesktopProviderId(item?.provider));
  const previousAvailable=[...desktopAvailable];
  const future=states.map(item=>item.provider).filter(name=>validDesktopProviderId(name)&&!coreDesktopProviders.includes(name));
  const nextAvailable=[...coreDesktopProviders,...new Set(future)];
  const added=nextAvailable.filter(name=>!previousAvailable.includes(name));
  loadDesktopSelection();
  for(const name of previousAvailable)if(!nextAvailable.includes(name)){const card=get(name+'-card');if(card)card.hidden=true;}
  desktopAvailable=nextAvailable;
  for(const name of added)ensureDesktopCard(name);
  desktopSelection=normalizeDesktopSelection(desktopSelection,desktopAvailable);
  saveDesktopSelection();
  for (const provider of states) {
    const name = provider.provider;
    const failed = provider.collection_state === 'error';
    const stale = provider.freshness === 'stale';
    const windows = provider.quota_windows || [];
    const usage = provider.source_usage || [],hasData=windows.length||usage.length;
    if (!failed && hasData) ready++;
    get(name+'-status').textContent = failed ? (stale ? '更新失敗 · 舊資料' : '需要處理') : hasData ? '已連線' : '等待讀取';
    get(name+'-status').className = failed ? 'status-error' : '';
    const quota = get(name+'-quota');
    const card = get(name+'-card');
    if (card?.dataset) card.dataset.failed = String(failed);
    const contentSignature = JSON.stringify([windows, usage]);
    if (quota.dataset.contentSignature !== contentSignature) {
      quota.dataset.contentSignature = contentSignature;
      quota.replaceChildren();
      if (!windows.length && !usage.length) {
        const empty=document.createElement('div');empty.className='empty-state';
        const image=document.createElement('img');image.src='assets/empty-cloud.png';image.alt='';
        const title=document.createElement('strong');title.textContent='尚未取得資料';
        const note=document.createElement('span');note.textContent='未同步';
        const legacy=document.createElement('span');legacy.className='sr-only';legacy.textContent='— 尚未取得額度';
        empty.append(image,title,note,legacy);quota.append(empty);
      }
      for (const item of windows) {
        const row = document.createElement('div'); row.className = 'window';
        const label = document.createElement('div'); label.className = 'q-label'; label.textContent = quotaLabel(item);
        const value = document.createElement('div'); value.className = 'q-value';
        const remaining = typeof item.remaining_percent === 'number' && Number.isFinite(item.remaining_percent)
          ? Math.max(0, Math.min(100, item.remaining_percent)) : null;
        if(row.style.setProperty)row.style.setProperty('--remaining',remaining === null ? 0 : remaining);else row.style['--remaining']=remaining === null ? 0 : remaining;
        const level = remaining === null ? 'unknown' : remaining > 30 ? 'good' : remaining > 10 ? 'warning' : 'low';
        row.dataset.level = level;
        value.textContent = remaining === null ? '—' : displayPercent(remaining)+'%';
        const valueMeaning=document.createElement('span');valueMeaning.className='sr-only';valueMeaning.textContent=remaining === null ? '剩餘未知' : ' 剩餘';value.append(valueMeaning);
        const track = document.createElement('div'); track.className = 'quota-track';
        if (remaining !== null) {
          const fill = document.createElement('div');
          fill.className = 'quota-fill quota-' + level;
          fill.style.width = remaining+'%';
          track.append(fill);
        } else { track.className += ' quota-unknown'; }
        const reset = document.createElement('div'); reset.className = 'detail q-reset';
        reset.textContent = item.reset_display ? '重設：'+item.reset_display :
          item.resets_at == null ? '重設時間尚未確認' : '重設：'+new Date(typeof item.resets_at === 'number' ? item.resets_at*1000 : item.resets_at).toLocaleString('zh-TW',{month:'numeric',day:'numeric',hour:'2-digit',minute:'2-digit'});
        row.append(label,value,track,reset);
        const details = [label.textContent, remaining === null ? '剩餘未知' : `剩餘 ${displayPercent(remaining)}%`, reset.textContent];
        if (typeof item.entitlement === 'number' && typeof item.used === 'number') {
          const used = document.createElement('div'); used.className = 'detail q-used';
          used.textContent = '已用 '+item.used+' / '+item.entitlement; row.append(used); details.push(used.textContent);
        }
        row.title = details.join('\n');
        quota.append(row);
      }
      if (!windows.length) for (const item of usage) {
        const row=document.createElement('div');row.className='window usage-row';const label=document.createElement('div');label.className='q-label';label.textContent=item.label||item.model||'使用量';const value=document.createElement('div');value.className='q-value';value.textContent=typeof item.used==='number'?`${item.used}${item.unit?' '+item.unit:''}`:'使用量未知';row.title=`${label.textContent}\n${value.textContent}`;row.append(label,value);quota.append(row);
      }
      layoutQuotaRegion(quota);
    }
    const rowCount = windows.length || usage.length;
    desktopTileCounts[name] = Math.max(1,rowCount);
    const timeText = '更新於 '+shortTime(provider.collected_at)+(stale ? ' · 舊資料，不代表目前額度':'')+(rowCount > 1 ? ` · 共 ${rowCount} 項` : '');
    get(name+'-time').textContent = timeText;
    get(name+'-time').title = '資料更新：'+time(provider.collected_at)+(stale ? '（舊資料，不代表目前額度）':'');
    const messageText = failed ? quotaErrorMessage(name, provider.failure_code) :
      name === 'copilot' ? '顯示 Premium interactions 權益，不是 Billing 或 AI credits 餘額。' :
      name === 'claude' ? '自動讀取官方 /usage；終端格式相容性仍屬實驗性。' :
      name === 'antigravity' ? '自動讀取官方 CLI 額度；不需要手動提供資料檔。' : '沿用本機 Codex 登入，自動取得官方額度。';
    get(name+'-message').textContent = messageText;
    get(name+'-message').title = messageText;
    if (!failed) get(name+'-time').title += '\n'+messageText;
    updateDesktopGuideButton(name,failed&&(!hasData||['cli_not_found','authentication_required','workspace_trust_required','collector_runtime_missing'].includes(provider.failure_code)));
    if (name === 'claude') get('claude-enable').hidden = provider.failure_code !== 'workspace_trust_required';
  }
  get('message').textContent = busy ? '正在讀取額度…' : ready+'/'+states.length+' 個服務已取得資料';
  const syncClock=get('sync-clock');if(syncClock)syncClock.textContent=new Date().toLocaleTimeString('zh-TW',{hour:'2-digit',minute:'2-digit'});
  applyDesktopLayout();renderDesktopOptions();
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

get('app-settings').addEventListener('click',()=>{renderDesktopOptions();get('app-settings-dialog').showModal();});
get('app-settings-close').addEventListener('click',()=>get('app-settings-dialog').close());
get('desktop-empty-settings').addEventListener('click',()=>{renderDesktopOptions();get('app-settings-dialog').showModal();});
get('desktop-prev').addEventListener('click',()=>{desktopPage--;applyDesktopLayout();});
get('desktop-next').addEventListener('click',()=>{desktopPage++;applyDesktopLayout();});
get('desktop-options-prev').addEventListener('click',()=>{desktopOptionPage--;desktopOptionsSignature='';renderDesktopOptions();});
get('desktop-options-next').addEventListener('click',()=>{desktopOptionPage++;desktopOptionsSignature='';renderDesktopOptions();});
get('desktop-guide-close').addEventListener('click',()=>get('desktop-guide-dialog').close());
get('tablet-settings-open').addEventListener('click',()=>{const note=get('pair-safety-note');note.hidden=false;get('tablet-settings-dialog').append(note);get('tablet-settings-dialog').showModal();});
get('tablet-settings-close').addEventListener('click',()=>get('tablet-settings-dialog').close());
if(get('desktop-monitor-options').parentElement)get('desktop-monitor-options').parentElement.append(get('desktop-options-pager'));
loadDesktopSelection();renderDesktopOptions();applyDesktopLayout();
if (window.addEventListener) window.addEventListener('resize',scheduleDesktopLayout);
if (typeof ResizeObserver === 'function') {
  new ResizeObserver(scheduleDesktopLayout).observe(get('desktop-cards'));
  for (const quota of document.querySelectorAll('.quota')) observeQuotaRegion(quota);
}
