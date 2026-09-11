const monitorPreferenceKey = 'agentmeter.monitor-selection.v1';

function normalizeMonitorSelection(raw, available) {
  const ids = [...new Set((available || []).filter(id => typeof id === 'string' && id))];
  if (!Array.isArray(raw)) return ids;
  return [...new Set(raw)].filter(id => ids.includes(id));
}

function setMonitorEnabled(selected, provider, enabled, available) {
  const current = normalizeMonitorSelection(selected, available);
  if (!available.includes(provider)) return current;
  if (enabled) return current.includes(provider) ? current : [...current,provider];
  return current.filter(id => id !== provider);
}

function monitorSlots(selected, available) {
  return normalizeMonitorSelection(selected, available);
}

function tabletViewportGrid(_width, _height, selectedCount = 0) {
  const count=Math.max(0,Math.min(4,Number(selectedCount)||0));
  if(count<=1)return {columns:1,rows:1,pageSize:4};
  if(count===2)return {columns:2,rows:1,pageSize:4};
  if(count===3)return {columns:1,rows:3,pageSize:4};
  return {columns:2,rows:2,pageSize:4};
}

function pagedMonitorSlots(selected, available, page, pageSize) {
  const ids = normalizeMonitorSelection(selected, available);
  const safeSize = Math.max(1, Number(pageSize) || 1);
  const pages = Math.max(1, Math.ceil(ids.length / safeSize));
  const safePage = Math.max(0, Math.min(Number(page) || 0, pages - 1));
  return {
    page:safePage,
    pages,
    slots:ids.slice(safePage * safeSize, (safePage + 1) * safeSize)
  };
}

function moveMonitor(selected, provider, direction) {
  const values=Array.isArray(selected)?[...selected]:[];
  const index=values.indexOf(provider),target=index+(Number(direction)<0?-1:1);
  if(index<0||target<0||target>=values.length)return values;
  [values[index],values[target]]=[values[target],values[index]];
  return values;
}

function quotaPeriodLabel(item) {
  const minutes=Number(item?.window_duration_mins);
  if(Number.isFinite(minutes)&&minutes>0){
    if(minutes===10080)return '每週使用上限';
    if(minutes===300)return '5 小時用量限制';
    return Number.isInteger(minutes/60)?`${minutes/60} 小時用量限制`:`${minutes} 分鐘用量限制`;
  }
  if(item?.window==='five_hour')return '5 小時用量限制';
  if(item?.window==='seven_day')return '每週使用上限';
  return '用量限制';
}

function quotaLimitLabel(item) {
  const id=item?.limit_id||'';
  const name=typeof item?.limit_name==='string'?item.limit_name.trim():'';
  if(id==='codex'||id==='default')return '常規使用額度';
  if(id==='base_model_inference'||name.toLowerCase()==='gpt-reserve')return '備用模型額度';
  if(name)return name;
  if(id==='codex_bengalfox')return 'GPT-5.3-Codex-Spark';
  if(id.startsWith('codex_'))return '模型使用額度';
  return '';
}

function quotaLabel(item) {
  const group=quotaLimitLabel(item);
  if(group)return `${group} · ${quotaPeriodLabel(item)}`;
  return (item?.label||item?.bucket_key||'可用額度')
    .replace('premium_interactions','Premium interactions')
    .replace('five_hour','5 小時').replace('seven_day','每週')
    .replace('Gemini Models','Gemini').replace('Claude and GPT models','Claude / GPT')
    .replace('Weekly Limit Remaining','每週').replace('Five Hour Limit Remaining','5 小時')
    .replace(/([^·]) (?=(?:每週|5 小時)$)/,'$1 · ')
    .replace(/\b(?:primary|secondary)\b/gi,'用量限制').trim();
}

function primaryMetric(provider) {
  const quota = (provider?.quota_windows || [])[0];
  if (quota) return {
    label:quotaLabel(quota),
    value:typeof quota.remaining_percent === 'number' && Number.isFinite(quota.remaining_percent) ? quota.remaining_percent : null,
    unit:'%', reset:quota.resets_at ?? quota.reset_display ?? null, kind:'quota'
  };
  const usage = (provider?.source_usage || [])[0];
  if (usage) return {
    label:usage.label || usage.model || '使用量',
    value:typeof usage.used === 'number' && Number.isFinite(usage.used) ? usage.used : null,
    unit:usage.unit || '', reset:null, kind:'usage'
  };
  return {label:'尚未取得資料',value:null,unit:'',reset:null,kind:'unknown'};
}

function needsSetupGuide(provider) {
  const hasData = Boolean((provider?.quota_windows || []).length || (provider?.source_usage || []).length);
  return provider?.collection_state === 'error' || provider?.setup === 'required' ||
    ['not_installed','needs_login','setup_required'].includes(provider?.availability) ||
    (!hasData && provider?.collection_state !== 'collecting');
}

const providerGuides = {
  codex:{name:'Codex',url:'https://learn.chatgpt.com/docs/codex/cli',steps:[
    '在桌機開啟官方 Codex CLI 安裝說明，依 Windows 頁籤完成安裝。',
    '開啟終端機執行 codex；第一次啟動時選擇「使用 ChatGPT 登入」。',
    '登入完成後回到桌機 AgentMeter，按 Codex 的「重新整理」。',
    '確認桌機已顯示額度，再回平板按重新整理。'
  ]},
  claude:{name:'Claude Code',url:'https://docs.anthropic.com/en/docs/claude-code/getting-started',steps:[
    '在桌機開啟 Anthropic 官方安裝說明，依 Windows 需求安裝 Claude Code。',
    '在終端機執行 claude，選擇適合的官方帳號方式完成登入。',
    '回到桌機 AgentMeter；若出現提示，按「啟用 Claude 讀取」並確認專用工作目錄。',
    '桌機取得額度後，回平板按重新整理。'
  ]},
  copilot:{name:'GitHub Copilot',url:'https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli',steps:[
    '在桌機開啟 GitHub 官方 Copilot CLI 安裝說明，使用 WinGet 或官方 npm 套件安裝。',
    '執行 copilot login，依瀏覽器流程完成 GitHub 驗證。',
    '確認帳號方案可使用 Copilot CLI；組織帳號也需管理員啟用政策。',
    '回到桌機 AgentMeter 按 Copilot 的「重新整理」，再回平板更新。'
  ]},
  antigravity:{name:'Antigravity',url:'https://codelabs.developers.google.com/antigravity-cli-hands-on',steps:[
    '先在桌機重新執行 AgentMeter 安裝程式，確認內建收集元件完整。',
    '若仍缺少工具，開啟 Google 官方 Antigravity CLI 教學並完成 Windows 安裝。',
    '執行 agy，使用 Google OAuth 登入並完成條款／工作目錄確認。',
    '回到桌機 AgentMeter 按 Antigravity 的「重新整理」，再回平板更新。'
  ]}
};

function setupGuide(provider, failureCode) {
  const guide = providerGuides[provider] || {name:provider || '此服務',url:'',steps:[
    '先確認這個監控來源的官方工具已安裝。','完成官方工具的登入或授權。','回到 AgentMeter 按「更新」重試；若仍無資料，請確認官方工具本身可正常回報用量。'
  ]};
  if (provider === 'claude' && failureCode === 'workspace_trust_required') return {...guide,steps:[
    '回到桌機 AgentMeter。','按 Claude Code 卡片上的「啟用 Claude 讀取」。','確認只信任 AgentMeter 專用工作目錄。','完成後回平板重新整理。'
  ]};
  return {provider,...guide};
}

function createFullscreenController(doc, button) {
  if (!doc?.documentElement?.requestFullscreen || !doc?.exitFullscreen) {
    button.hidden = true;
    return async () => false;
  }
  const sync = () => {
    const active = Boolean(doc.fullscreenElement);
    button.textContent = active ? '退出全螢幕' : '全螢幕';
    button.setAttribute('aria-pressed',String(active));
  };
  doc.addEventListener('fullscreenchange',sync);
  sync();
  return async () => {
    try {
      if (doc.fullscreenElement) await doc.exitFullscreen();
      else await doc.documentElement.requestFullscreen();
      return true;
    } catch { sync(); return false; }
  };
}
