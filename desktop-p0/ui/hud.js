const hudNames={codex:'Codex',claude:'Claude Code',copilot:'GitHub Copilot',antigravity:'Antigravity',cursor:'Cursor',kiro:'Kiro'};
let hudSnapshot=null,hudBusy=false,lastHudGeometry='';

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
  const width=Math.max(260,Math.min(360,(Number(screen.availWidth)||360)-32));
  const height=hudHeight(rows.length);
  const left=(Number(screen.availLeft)||0)+(Number(screen.availWidth)||width)-width-16;
  const top=(Number(screen.availTop)||0)+(Number(screen.availHeight)||height)-height-16;
  const enabled=hudEnabled()&&rows.length>0,signature=JSON.stringify([enabled,width,height,left,top]);
  if(signature!==lastHudGeometry){
    lastHudGeometry=signature;
    hudInvoke('configure_hud',{enabled,width,height,x:left,y:top}).catch(()=>{});
  }
}
async function pollHud() {
  renderHud();
  if(!hudEnabled()||hudBusy)return;
  hudBusy=true;
  try { hudSnapshot=await hudInvoke('snapshot');renderHud(); }
  finally { hudBusy=false; }
}
pollHud();
setInterval(pollHud,2000);
