const hudEnabledKey = 'agentmeter.hud-enabled.v1';
const hudOpacityKey = 'agentmeter.hud-opacity.v1';
const hudDefaultOpacity = 72;

function normalizeHudOpacity(value) {
  const number=Number(value);
  if(!Number.isFinite(number))return hudDefaultOpacity;
  return Math.max(35,Math.min(100,Math.round(number)));
}

function normalizeHudSelection(raw, fallback = []) {
  const values=Array.isArray(raw)?raw:fallback;
  return [...new Set(values)].filter(value=>typeof value==='string'&&/^[a-z0-9][a-z0-9._-]{0,79}$/.test(value)&&value!=='constructor'&&value!=='__proto__'&&!value.startsWith('desktop-')).slice(0,4);
}

function hudNumber(value) {
  return String(Math.round(value * 10) / 10);
}

function hudPrimaryQuota(provider) {
  const quotas=Array.isArray(provider?.quota_windows)?provider.quota_windows:[];
  if(provider?.provider!=='codex')return quotas[0]||null;
  const regular=quotas.filter(quota=>quota?.limit_id==='codex'||quota?.limit_id==='codex/default');
  return regular.find(quota=>Number(quota?.window_duration_mins)===10080)||regular[0]||quotas[0]||null;
}

function hudPrimaryValue(provider) {
  const quota=hudPrimaryQuota(provider);
  const remaining=Number(quota?.remaining_percent);
  if(quota&&quota.remaining_percent!==null&&Number.isFinite(remaining))return hudNumber(Math.max(0,Math.min(100,remaining)))+'%';
  const usage=Array.isArray(provider?.source_usage)?provider.source_usage[0]:null;
  if(!usage)return '—';
  const usageRemaining=Number(usage.remaining_percent);
  if(usage.remaining_percent!==null&&usage.remaining_percent!==undefined&&Number.isFinite(usageRemaining))return hudNumber(Math.max(0,Math.min(100,usageRemaining)))+'%';
  const used=Number(usage.used),entitlement=Number(usage.entitlement??usage.limit);
  if(Number.isFinite(used)&&Number.isFinite(entitlement))return hudNumber(used)+' / '+hudNumber(entitlement);
  if(Number.isFinite(used))return hudNumber(used)+(usage.unit?' '+usage.unit:'');
  return '—';
}

function hudRows(snapshot, selection, names = {}) {
  const states=new Map((snapshot?.provider_states||[]).filter(item=>item&&typeof item.provider==='string').map(item=>[item.provider,item]));
  return normalizeHudSelection(selection).map(id=>({id,name:names[id]||id,value:hudPrimaryValue(states.get(id))}));
}

function hudHeight(count) {
  const rows=Math.max(1,Math.min(4,Math.floor(Number(count)||0)));
  return 20+rows*28;
}
