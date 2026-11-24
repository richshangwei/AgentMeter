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
function hudRow(id) {
  const row=document.createElement('li');row.dataset.provider=id;
  const pet=document.createElement('span');pet.className='hud-pet';pet.append(createPet(id),createHabitat(false));
  const label=document.createElement('span');label.className='hud-label';
  const name=document.createElement('span');name.className='hud-name';
  const eta=document.createElement('small');eta.className='hud-eta';
  label.append(name,eta);
  const value=document.createElement('strong');value.className='hud-value';
  row.append(pet,label,value);return row;
}
function renderHud() {
  window.agentMeterTheme?.sync();
  const selection=hudSelection(),rows=hudRows(hudSnapshot,selection,hudNames),host=document.getElementById('hud-rows');
  const samples=loadQuotaSamples(),now=Date.now();
  const states=new Map((hudSnapshot?.provider_states||[]).map(item=>[item?.provider,item]));
  // Reuse rows between polls so running animations are not restarted.
  const existing=[...host.children];
  if(existing.map(row=>row.dataset.provider).join()!==rows.map(item=>item.id).join())host.replaceChildren(...rows.map(item=>hudRow(item.id)));
  rows.forEach((item,index)=>{
    const row=host.children[index],forecast=providerForecast(samples,states.get(item.id),now);
    const set=(selector,text)=>{const node=row.querySelector(selector);if(node.textContent!==text)node.textContent=text;};
    set('.hud-name',item.name);set('.hud-value',item.value);set('.hud-eta',forecastShortText(forecast.primary));
    row.dataset.level=forecastLevel(forecast.primary);
    if(document.documentElement.dataset.theme==='hamster')updateHabitat(row.querySelector('.habitat'),item.id,hamsterActivity(item.id,forecast.primary,forecast.rate,now),forecast.primary?forecastRemaining(forecast.primary.item):null,petStrideSeconds(forecast.rate));
    else updatePet(row.querySelector('.pet'),petMood(forecast.primary,forecast.rate),forecast.rate);
  });
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
    document.documentElement.dataset.idle=String(!hudEnabled()||selection.length===0);
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
