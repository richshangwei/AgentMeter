const historyMetricDefinitions=[
  ['total_tokens','總 Token'],['input_tokens','輸入'],['cached_input_tokens','快取輸入'],
  ['cache_write_input_tokens','快取寫入'],['output_tokens','輸出'],['reasoning_output_tokens','推理']
];
const historyProviderNames={codex:'Codex',claude:'Claude Code',copilot:'GitHub Copilot',antigravity:'Antigravity'};

function historyTokenValue(value){
  if(typeof value!=='string'||!/^(0|[1-9]\d*)$/.test(value))return null;
  try{return BigInt(value);}catch{return null;}
}
function formatHistoryToken(value){const parsed=historyTokenValue(value);return parsed===null?'—':parsed.toLocaleString('en-US');}
function formatHistoryWhen(value){
  if(typeof value!=='string'||value.length>64)return '時間未知';
  const date=new Date(value);return Number.isFinite(date.getTime())?date.toLocaleString('zh-TW'):'時間未知';
}
function formatHistoryDuration(value){
  const number=typeof value==='string'&&/^\d+$/.test(value)?Number(value):value;
  if(!Number.isFinite(number)||number<0)return '耗時未知';
  const seconds=Math.round(number/1000);if(seconds<60)return `${seconds} 秒`;
  const minutes=Math.floor(seconds/60),remainder=seconds%60;return remainder?`${minutes} 分 ${remainder} 秒`:`${minutes} 分`;
}
function historyTokenView(tokens={}){
  const view={};for(const [key] of historyMetricDefinitions)view[key]=formatHistoryToken(tokens?.[key]);return view;
}
function historyPercent(value){
  if(typeof value!=='string'||!/^\d{1,3}(?:\.\d)?$/.test(value))return null;
  const parsed=Number(value);return Number.isFinite(parsed)&&parsed>=0&&parsed<=100?parsed:null;
}
function historyUsd(value){return typeof value==='string'&&/^\d+\.\d{6}$/.test(value)?value:null;}
function historyCostView(cost){
  const min=historyUsd(cost?.estimated_usd_min),max=historyUsd(cost?.estimated_usd_max),coverage=historyPercent(cost?.coverage_percent);
  const state=['complete','partial','unavailable','no_usage'].includes(cost?.state)?cost.state:'unavailable';
  let amount='無法估價';
  if(state==='no_usage')amount='沒有可計價 Token';
  else if(min&&max)amount=`${state==='partial'?'已估 ':''}US$${min}${min===max?'':`–${max}`}`;
  return {state,amount,coverage,coverageLabel:coverage===null?'—':`${coverage.toFixed(1)}%`};
}
function historyAnalysisView(analysis,provider){
  if(analysis?.schema_version!=='agentmeter.token-analysis.v1'||analysis?.provider!==provider)return null;
  const definitions=[['uncached_input_tokens','未快取輸入','fresh'],['cached_input_tokens','快取輸入','cached'],['cache_write_input_tokens','快取寫入','write'],['output_tokens','輸出','output'],['unclassified_tokens','未分類','unknown']];
  const parts=definitions.map(([key,label,tone])=>({key,label,tone,value:historyTokenValue(analysis.flow?.[key])})).filter(part=>part.value!==null&&part.value>0n);
  const total=parts.reduce((sum,part)=>sum+part.value,0n);
  const flow=parts.map(part=>({...part,formatted:formatHistoryToken(part.value.toString()),percent:total>0n?Number(part.value*1000n/total)/10:0}));
  const cost=historyCostView(analysis.cost),cache=historyPercent(analysis.cache_reuse_percent);
  const sessionAnalysis=new Map((Array.isArray(analysis.sessions)?analysis.sessions:[]).filter(item=>typeof item?.session_ref==='string').map(item=>[item.session_ref,{share:historyPercent(item.share_percent),cost:historyCostView(item.cost)}]));
  const signals=(Array.isArray(analysis.signals)?analysis.signals:[]).filter(signal=>['ok','info','review'].includes(signal?.severity)&&typeof signal?.title==='string'&&typeof signal?.evidence==='string'&&typeof signal?.action==='string').slice(0,6);
  return {flow,cost,cacheLabel:cache===null?'—':`${cache.toFixed(1)}%`,sessionAnalysis,signals,priceAsOf:typeof analysis.price_as_of==='string'?analysis.price_as_of:'日期未知'};
}
function historyViewModel(report){
  if(report?.schema_version!=='agentmeter.provider-history.v1'||!historyProviderNames[report?.provider]||!Array.isArray(report.sessions))return null;
  const summary=historyTokenView(report.totals);summary.total=summary.total_tokens;
  const sessions=report.sessions.map((session,index)=>{
    const totals=historyTokenView(session?.totals);
    const turns=Array.isArray(session?.turns)?session.turns.map((turn,turnIndex)=>({
      turn_ref:typeof turn?.turn_ref==='string'?turn.turn_ref:`turn-${turnIndex+1}`,
      when:formatHistoryWhen(turn?.ended_at||turn?.started_at),duration:formatHistoryDuration(turn?.duration_ms),
      model:typeof turn?.model==='string'&&turn.model.length<=120?turn.model:'模型未知',tokens:historyTokenView(turn?.tokens)
    })):[];
    return {
      session_ref:typeof session?.session_ref==='string'?session.session_ref:`session-${index+1}`,
      started_at:typeof session?.started_at==='string'?session.started_at:null,
      ended_at:typeof session?.ended_at==='string'?session.ended_at:null,
      when:formatHistoryWhen(session?.ended_at||session?.started_at),duration:formatHistoryDuration(session?.duration_ms),
      turnCount:Number.isInteger(session?.turn_count)&&session.turn_count>=0?session.turn_count:turns.length,
      turnCountExact:session?.turn_count_exact!==false,turnsTruncated:session?.turns_truncated===true,
      contentAvailable:report.status==='supported'&&report.provider!=='antigravity',
      detailCount:turns.length,total:totals.total_tokens,totalValue:historyTokenValue(session?.totals?.total_tokens)??-1n,
      totals,turns,stableIndex:index
    };
  }).sort((left,right)=>left.totalValue===right.totalValue?left.stableIndex-right.stableIndex:(left.totalValue>right.totalValue?-1:1));
  const incomplete=report.limited===true||Number(report.files_skipped)>0||Number(report.records_rejected)>0||Number(report.partial_records)>0;
  return {provider:report.provider,name:historyProviderNames[report.provider],support:report.status==='supported'?'verified':report.status==='schema_unverified'?'unverified':'verified',
    state:report.status==='supported'?'ready':report.status,summary,sessions,incomplete,analysis:historyAnalysisView(report.analysis,report.provider),status:historyStatus(report,sessions.length,incomplete)};
}
function historyStatus(report,count,incomplete){
  const name=historyProviderNames[report.provider];
  if(report.status==='unsupported')return `${name} 尚無可驗證的本機對話 Token 來源；額度資料不能替代對話 Token。`;
  if(report.status==='schema_unverified')return `${name} 的本機對話格式尚未驗證；不會以額度資料代替對話 Token。`;
  if(report.status==='source_unavailable')return `尚未找到這個 Windows 使用者的 ${name} 本機對話紀錄。`;
  if(report.status==='permission_denied')return `${name} 本機對話紀錄無法讀取；請檢查檔案權限。`;
  return count?`已讀取 ${count} 個 ${name} 工作階段 · 依總 Token 排序${incomplete?' · 資料有截斷或未完整解析':''}`
    :`${name} 的來源可讀，但尚未找到可用的 Session Token 紀錄${incomplete?' · 資料有截斷或未完整解析':''}`;
}
function historyMetricNode(label,value){
  const node=document.createElement('div');node.className='history-metric';
  const name=document.createElement('span');name.textContent=label;
  const amount=document.createElement('strong');amount.textContent=value;node.append(name,amount);return node;
}
function historyMetricsNode(tokens,compact=false){
  const node=document.createElement('div');node.className=compact?'history-metrics compact':'history-metrics';
  for(const [key,label] of historyMetricDefinitions)node.append(historyMetricNode(label,tokens[key]));return node;
}
function historyKpiNode(label,value,note=''){
  const node=document.createElement('article');node.className='history-kpi';const name=document.createElement('span'),amount=document.createElement('strong'),hint=document.createElement('small');
  name.textContent=label;amount.textContent=value;hint.textContent=note;node.append(name,amount,hint);return node;
}
function renderHistoryAnalysis(view){
  const host=document.getElementById('history-analysis'),kpis=document.getElementById('history-kpis'),flowHost=document.getElementById('history-flow'),legend=document.getElementById('history-flow-legend');
  const signals=document.getElementById('history-signals'),top=document.getElementById('history-top-sessions'),note=document.getElementById('history-price-note');
  if(view.state!=='ready'||!view.analysis){host.hidden=true;kpis.replaceChildren();flowHost.replaceChildren();legend.replaceChildren();signals.replaceChildren();top.replaceChildren();note.textContent='';return;}
  const analysis=view.analysis;host.hidden=false;
  kpis.replaceChildren(historyKpiNode('總 Token',view.summary.total_tokens,'目前讀取範圍'),historyKpiNode('API 等值成本',analysis.cost.amount,analysis.cost.state==='partial'?'只包含已定價部分':'目前公開標準單價'),historyKpiNode('估價覆蓋率',analysis.cost.coverageLabel,'未辨識資料不猜價'),historyKpiNode('快取重用率',analysis.cacheLabel,'占全部輸入 Token'));
  flowHost.replaceChildren(...analysis.flow.map(part=>{const bar=document.createElement('span');bar.className=`history-flow-part ${part.tone}`;bar.setAttribute('style',`width:${Math.max(part.percent,.8)}%`);bar.setAttribute('aria-hidden','true');return bar;}));
  const flowDescription=analysis.flow.length?analysis.flow.map(part=>`${part.label} ${part.formatted}，占 ${part.percent.toFixed(1)}%`).join('；'):'沒有可分類的 Token';flowHost.ariaLabel=flowDescription;
  legend.replaceChildren(...analysis.flow.map(part=>{const item=document.createElement('li'),swatch=document.createElement('i'),label=document.createElement('span'),value=document.createElement('strong');swatch.className=part.tone;label.textContent=part.label;value.textContent=`${part.formatted} · ${part.percent.toFixed(1)}%`;item.append(swatch,label,value);return item;}));
  signals.replaceChildren(...(analysis.signals.length?analysis.signals.map(signal=>{const item=document.createElement('li'),title=document.createElement('strong'),evidence=document.createElement('p'),action=document.createElement('small');item.className=`history-signal ${signal.severity}`;title.textContent=signal.title;evidence.textContent=signal.evidence;action.textContent=signal.action;item.append(title,evidence,action);return item;}):[(()=>{const item=document.createElement('li');item.textContent='目前沒有足夠資料產生效率訊號。';return item;})()]));
  const ranked=view.sessions.slice(0,5);top.replaceChildren(...ranked.map(session=>{const item=document.createElement('li'),head=document.createElement('div'),name=document.createElement('strong'),value=document.createElement('span'),track=document.createElement('div'),bar=document.createElement('i'),cost=document.createElement('small');const detail=analysis.sessionAnalysis.get(session.session_ref);name.textContent=`工作階段 ${session.session_ref.replace(/^session-/,'')}`;value.textContent=`${session.total} Token`;head.append(name,value);track.className='history-top-track';bar.setAttribute('style',`width:${detail?.share??0}%`);track.append(bar);cost.textContent=detail?.cost.amount??'無法估價';item.append(head,track,cost);return item;}));
  note.textContent=`費率基準：${analysis.priceAsOf} 的公開標準 API 單價等值；不等於 ChatGPT、Claude 或 Copilot 訂閱帳單，也不回推歷史實際扣款。`;
}
function renderProviderHistory(report,onReveal){
  const status=document.getElementById('history-status'),summary=document.getElementById('history-summary');
  const sessionsHost=document.getElementById('history-sessions'),view=historyViewModel(report);
  if(!view){status.textContent='對話 Token 資料格式無法辨識。';summary.replaceChildren();sessionsHost.replaceChildren();document.getElementById('history-analysis').hidden=true;return;}
  status.textContent=view.status;
  renderHistoryAnalysis(view);
  summary.replaceChildren(...(view.state==='ready'?historyMetricDefinitions.map(([key,label])=>historyMetricNode(label,view.summary[key])):[]));
  sessionsHost.replaceChildren(...view.sessions.map((session,index)=>{
    const details=document.createElement('details');details.className='history-session';details.open=index===0;
    const heading=document.createElement('summary'),title=document.createElement('strong');title.textContent=`工作階段 ${session.session_ref.replace(/^session-/,'')}`;
    const turnLabel=session.turnsTruncated?`${session.turnCountExact?'共':'至少'} ${session.turnCount} 個回合，僅顯示最近 ${session.detailCount} 個`:`${session.turnCount} 個回合`;
    const meta=document.createElement('span');meta.textContent=`${session.when} · ${turnLabel} · ${session.total} Token`;heading.append(title,meta);
    details.append(heading,historyMetricsNode(session.totals,true));
    if(session.contentAvailable){
      const action=document.createElement('button');action.className='ghost history-content-open';action.textContent='檢視我送出的內容';
      action.dataset.sessionRef=session.session_ref;action.addEventListener('click',()=>onReveal(view.provider,session,action));details.append(action);
    }
    const turns=document.createElement('ol');turns.className='history-turns';
    for(const turn of session.turns){
      const item=document.createElement('li'),turnHead=document.createElement('div');turnHead.className='history-turn-head';
      const turnName=document.createElement('strong');turnName.textContent=`回合 ${turn.turn_ref.replace(/^turn-/,'')}`;
      const turnMeta=document.createElement('span');turnMeta.textContent=`${turn.model} · ${turn.when} · ${turn.duration}`;
      turnHead.append(turnName,turnMeta);item.append(turnHead,historyMetricsNode(turn.tokens,true));turns.append(item);
    }
    details.append(turns);return details;
  }));
}
function renderHistoryContent(result){
  const body=document.getElementById('history-content-body'),status=document.getElementById('history-content-status');
  if(result?.schema_version!=='agentmeter.provider-session-content.v1'||!Array.isArray(result.prompts)){
    body.replaceChildren();status.textContent='Prompt 資料格式無法辨識。';return;
  }
  body.replaceChildren(...result.prompts.map((prompt,index)=>{
    const article=document.createElement('article'),heading=document.createElement('strong'),content=document.createElement('pre');
    heading.textContent=`送出內容 ${index+1}`;content.textContent=typeof prompt?.text==='string'?prompt.text:'';
    article.append(heading,content);return article;
  }));
  const duplicates=Number.isInteger(result.duplicate_prompt_count)&&result.duplicate_prompt_count>0?` · 完全相同的 Prompt 額外出現 ${result.duplicate_prompt_count} 次（可能是刻意重試）`:'';
  status.textContent=result.prompts.length?`已顯示 ${result.prompts.length} 筆遮罩後的使用者 Prompt${result.limited?' · 內容已截斷':''}${duplicates}`:'此工作階段沒有可安全辨識的使用者 Prompt。';
}
function initHistoryUi(invoke){
  const open=document.getElementById('history-open'),dialog=document.getElementById('history-dialog'),close=document.getElementById('history-close');
  const refresh=document.getElementById('history-refresh'),providerSelect=document.getElementById('history-provider'),status=document.getElementById('history-status');
  const contentDialog=document.getElementById('history-content-dialog'),contentClose=document.getElementById('history-content-close');
  const consent=document.getElementById('history-content-consent'),contentLoad=document.getElementById('history-content-load');
  const contentStatus=document.getElementById('history-content-status'),contentBody=document.getElementById('history-content-body');
  let busy=false,pendingLoad=false,requestGeneration=0,selectedSession=null,returnFocus=null;
  const clearContent=()=>{requestGeneration++;selectedSession=null;contentBody.replaceChildren();contentStatus.textContent='尚未讀取。';consent.checked=false;contentLoad.disabled=true;if(contentDialog.open)contentDialog.close();};
  const reveal=(provider,session,trigger)=>{
    clearContent();selectedSession={provider,sessionRef:session.session_ref,expectedStartedAt:session.started_at,expectedEndedAt:session.ended_at};returnFocus=trigger;
    document.getElementById('history-content-session').textContent=`${historyProviderNames[provider]} · 工作階段 ${session.session_ref.replace(/^session-/,'')}`;
    contentDialog.showModal();contentLoad.disabled=true;consent.focus();
  };
  const load=async()=>{
    if(busy){pendingLoad=true;clearContent();return;}
    busy=true;refresh.disabled=true;clearContent();const generation=++requestGeneration;
    const provider=providerSelect.value;status.textContent=`正在讀取 ${historyProviderNames[provider]} 本機對話 Token…`;
    try{const report=await invoke('history_sessions', {provider});if(generation!==requestGeneration)return;renderProviderHistory(report,reveal);}
    catch(error){if(generation!==requestGeneration)return;const code=String(error);status.textContent=code.includes('source_unavailable')?'尚未找到這個 Provider 的本機對話紀錄。':'對話 Token 明細讀取失敗；沒有載入任何對話正文。';document.getElementById('history-summary').replaceChildren();document.getElementById('history-sessions').replaceChildren();document.getElementById('history-analysis').hidden=true;}
    finally{busy=false;refresh.disabled=false;if(pendingLoad){pendingLoad=false;await load();}}
  };
  const closeContent=()=>{const focus=returnFocus;clearContent();returnFocus=null;if(focus)focus.focus();};
  open.addEventListener('click',async()=>{dialog.showModal();await load();});
  close.addEventListener('click',()=>{clearContent();dialog.close();});dialog.addEventListener('close',clearContent);
  refresh.addEventListener('click',load);providerSelect.addEventListener('change',load);
  consent.addEventListener('change',()=>{contentLoad.disabled=!consent.checked;});
  contentClose.addEventListener('click',closeContent);contentDialog.addEventListener('close',()=>{contentBody.replaceChildren();contentStatus.textContent='尚未讀取。';consent.checked=false;contentLoad.disabled=true;selectedSession=null;requestGeneration++;});
  contentLoad.addEventListener('click',async()=>{
    if(!selectedSession||!consent.checked)return;const selected={...selectedSession},generation=++requestGeneration;
    contentLoad.disabled=true;contentStatus.textContent='正在按需讀取並遮罩 Prompt…';contentBody.replaceChildren();
    try{
      const result=await invoke('history_session_content',{provider:selected.provider,sessionRef:selected.sessionRef,expectedStartedAt:selected.expectedStartedAt,expectedEndedAt:selected.expectedEndedAt,consent:true});
      if(generation!==requestGeneration)return;
      if(result?.provider!==selected.provider||result?.session_ref!==selected.sessionRef){contentStatus.textContent='Prompt 回應與選取工作階段不一致。';return;}
      renderHistoryContent(result);
    }catch(error){if(generation===requestGeneration)contentStatus.textContent=String(error).includes('stale')?'工作階段已變更，請重新整理後再試。':'Prompt 讀取失敗或此格式尚未支援。';}
    finally{if(generation===requestGeneration)contentLoad.disabled=!consent.checked;}
  });
}
