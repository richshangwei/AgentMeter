const hudNames={codex:'Codex',claude:'Claude Code',copilot:'GitHub Copilot',antigravity:'Antigravity',cursor:'Cursor',kiro:'Kiro'};
let hudSnapshot=null,hudBusy=false;

function hudStorage(key,fallback) {
  try { const value=window.localStorage?.getItem(key);return value===null?fallback:value; }
  catch { return fallback; }
}
function hudSelection() {
  try { return normalizeHudSelection(JSON.parse(hudStorage(desktopMonitorKey,'null')),Object.keys(hudNames)); }
  catch { return Object.keys(hudNames); }
}
function hudEnabled() { return hudStorage(hudEnabledKey,'false')==='true'; }
function hudOpacity() { return normalizeHudOpacity(hudStorage(hudOpacityKey,hudDefaultOpacity)); }
function hudInvoke(command,args={}) {
  const invoke=window.__TAURI__?.core?.invoke;
  return invoke?invoke(command,args):Promise.resolve();
}
function renderHud() {
  const selection=hudSelection(),rows=hudRows(hudSnapshot,selection,hudNames),host=document.getElementById('hud-rows');
  host.replaceChildren(...rows.map(item=>{
    const row=document.createElement('li');row.dataset.provider=item.id;
    const name=document.createElement('span');name.className='hud-name';name.textContent=item.name;
    const value=document.createElement('strong');value.className='hud-value';value.textContent=item.value;
    row.append(name,value);return row;
  }));
  document.documentElement.style.setProperty('--surface-alpha',String(hudOpacity()/100));
}
async function pollHud() {
  if(hudBusy)return;
  hudBusy=true;
  try {
    renderHud();
    const selection=hudSelection();
    try {
      await hudInvoke('configure_hud',{enabled:hudEnabled()&&selection.length>0,width:240,height:hudHeight(selection.length)});
      document.getElementById('hud-panel').title='按住拖曳，可跨螢幕自由擺放';
    } catch {
      document.getElementById('hud-panel').title='視窗位置設定或保存失敗，將自動重試；數據仍會更新。';
    }
    if(hudEnabled()){hudSnapshot=await hudInvoke('snapshot');renderHud();}
  }
  catch { document.getElementById('hud-panel').title='更新浮動視窗失敗，將自動重試。'; }
  finally { hudBusy=false; }
}
document.getElementById('hud-panel').addEventListener('pointerdown',event=>{
  if(event.button!==0||event.isPrimary===false)return;
  hudInvoke('start_hud_drag').catch(()=>{
    document.getElementById('hud-panel').title='拖曳失敗，請重試或在設定中切換螢幕。';
  });
});
pollHud();
setInterval(pollHud,2000);
