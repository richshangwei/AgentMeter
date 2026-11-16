const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const vm=require('node:vm');

function harness(){
  const elements=new Map();let serial=0;
  function element(id=''){
    if(!elements.has(id))elements.set(id,{id,textContent:'',className:'',hidden:false,disabled:false,open:false,checked:false,value:'',children:[],dataset:{},handlers:{},
      append(...nodes){this.children.push(...nodes);},replaceChildren(...nodes){this.children=nodes;this.textContent='';},
      addEventListener(type,handler){this.handlers[type]=handler;},setAttribute(name,value){this[name]=value;},
      showModal(){this.open=true;},close(){this.open=false;if(this.handlers.close)this.handlers.close();},focus(){this.focused=true;}});
    return elements.get(id);
  }
  element('history-provider').value='codex';
  const document={getElementById:element,createElement:tag=>{const node=element(`${tag}-${serial++}`);node.tagName=tag.toUpperCase();return node;}};
  const context=vm.createContext({document,Date,BigInt,Intl,Number,String,Array,Math,Promise});
  vm.runInContext(fs.readFileSync(path.join(__dirname,'../ui/history.js'),'utf8'),context);
  return {context,element};
}
const text=node=>node.textContent+node.children.map(text).join(' ');

function report(provider='codex',overrides={}){return {
  schema_version:'agentmeter.provider-history.v1',provider,status:'supported',data_quality:'local_observed',
  files_scanned:2,files_skipped:0,duplicate_sessions:1,records_rejected:1,partial_records:1,limited:true,
  totals:{input_tokens:'12000',cached_input_tokens:'9000',cache_write_input_tokens:null,output_tokens:'500',reasoning_output_tokens:'125',total_tokens:'12500'},
  analysis:{schema_version:'agentmeter.token-analysis.v1',provider,pricing_basis:'current_api_equivalent',price_as_of:'2026-09-12',price_sources:['https://developers.openai.com/api/docs/models/gpt-5.3-codex'],
    flow:{uncached_input_tokens:'3000',cached_input_tokens:'9000',cache_write_input_tokens:'0',output_tokens:'500',unclassified_tokens:'0'},cache_reuse_percent:'75.0',output_input_percent:'4.2',
    cost:{state:'complete',estimated_usd_min:'0.020000',estimated_usd_max:'0.020000',priced_tokens:'12500',observed_tokens:'12500',coverage_percent:'100.0'},
    sessions:[{session_ref:'session-1',total_tokens:'2100',share_percent:'16.8',cost:{state:'complete',estimated_usd_min:'0.004000',estimated_usd_max:'0.004000',priced_tokens:'2100',observed_tokens:'2100',coverage_percent:'100.0'}},{session_ref:'session-2',total_tokens:'10400',share_percent:'83.2',cost:{state:'partial',estimated_usd_min:'0.016000',estimated_usd_max:'0.017000',priced_tokens:'9000',observed_tokens:'10400',coverage_percent:'86.5'}}],
    signals:[{kind:'input_heavy',severity:'review',title:'輸入很多、輸出較少',evidence:'輸出約為輸入的 4.2%；這只是檢查線索。',action:'檢查是否重送大型檔案。'}]},
  sessions:[
    {session_ref:'session-1',started_at:'2026-09-12T01:00:00Z',ended_at:'2026-09-12T01:10:00Z',duration_ms:'600000',turn_count:1,content_available:true,totals:{input_tokens:'2000',cached_input_tokens:'1000',cache_write_input_tokens:'0',output_tokens:'100',reasoning_output_tokens:'20',total_tokens:'2100'},turns:[{turn_ref:'turn-1',started_at:'2026-09-12T01:00:00Z',ended_at:'2026-09-12T01:10:00Z',duration_ms:'600000',model:'gpt-fixture-a',tokens:{input_tokens:'2000',cached_input_tokens:'1000',cache_write_input_tokens:'0',output_tokens:'100',reasoning_output_tokens:'20',total_tokens:'2100'}}]},
    {session_ref:'session-2',started_at:'2026-09-12T02:00:00Z',ended_at:'2026-09-12T02:20:00Z',duration_ms:null,turn_count:201,turn_count_exact:false,turns_truncated:true,content_available:true,totals:{input_tokens:'10000',cached_input_tokens:'8000',cache_write_input_tokens:null,output_tokens:'400',reasoning_output_tokens:'105',total_tokens:'10400'},turns:[{turn_ref:'turn-1',started_at:'2026-09-12T02:00:00Z',ended_at:'2026-09-12T02:20:00Z',duration_ms:null,model:'gpt-fixture-b',tokens:{input_tokens:'10000',cached_input_tokens:'8000',cache_write_input_tokens:null,output_tokens:'400',reasoning_output_tokens:'105',total_tokens:'10400'}}],prompt:'PRIVATE_PROMPT_SENTINEL',path:'PRIVATE_PATH_SENTINEL'}
  ],response:'PRIVATE_RESPONSE_SENTINEL',...overrides
};}

test('analysis leads with cost coverage, token flow, signals and top sessions',async()=>{
  const {context,element}=harness();
  context.initHistoryUi(async()=>report());
  await element('history-open').handlers.click();
  assert.equal(element('history-analysis').hidden,false);
  const rendered=[text(element('history-kpis')),text(element('history-flow-legend')),text(element('history-signals')),text(element('history-top-sessions')),element('history-price-note').textContent].join(' ');
  assert.match(rendered,/API 等值成本/);
  assert.match(rendered,/US\$0\.020000/);
  assert.match(rendered,/估價覆蓋率/);
  assert.match(rendered,/100\.0%/);
  assert.match(fs.readFileSync(path.join(__dirname,'../ui/index.html'),'utf8'),/>Token 流向</);
  assert.match(rendered,/快取輸入/);
  assert.match(rendered,/輸入很多、輸出較少/);
  assert.match(rendered,/工作階段 2/);
  assert.match(rendered,/US\$0\.016000–0\.017000/);
  assert.match(element('history-flow').ariaLabel,/未快取輸入.*24\.0%.*快取輸入.*72\.0%/);
});

test('unavailable providers hide stale analysis instead of showing zero cost',async()=>{
  const {context,element}=harness();
  context.initHistoryUi(async(command,args)=>args.provider==='copilot'
    ?report('copilot',{status:'supported',analysis:{schema_version:'agentmeter.token-analysis.v1',provider:'copilot',pricing_basis:'current_api_equivalent',price_as_of:'2026-09-12',price_sources:[],flow:{},cache_reuse_percent:null,output_input_percent:null,cost:{state:'unavailable',estimated_usd_min:null,estimated_usd_max:null,priced_tokens:'0',observed_tokens:null,coverage_percent:null},sessions:[],signals:[]},sessions:[],totals:{}})
    :report(args.provider));
  await element('history-open').handlers.click();
  element('history-provider').value='copilot';
  await element('history-provider').handlers.change();
  assert.equal(element('history-analysis').hidden,false);
  assert.match(text(element('history-kpis')),/無法估價/);
  assert.doesNotMatch(text(element('history-kpis')),/US\$0\.000000/);
});

test('provider history formats decimal-string tokens and accepts every provider',()=>{
  const {context}=harness();
  assert.equal(context.formatHistoryToken('12345678901234567890'),'12,345,678,901,234,567,890');
  assert.equal(context.formatHistoryToken(null),'—');
  for(const provider of ['codex','claude','copilot','antigravity']){
    const view=context.historyViewModel(report(provider));
    assert.equal(view.provider,provider);
    assert.equal(view.sessions[0].session_ref,'session-2');
    assert.equal(view.summary.total,'12,500');
  }
});

test('dialog queries all four providers with no path and keeps unsupported truthful',async()=>{
  const {context,element}=harness();const calls=[];
  context.initHistoryUi(async(command,args)=>{calls.push({command,args});return args.provider==='antigravity'
    ?report('antigravity',{status:'schema_unverified',sessions:[],totals:{}})
    :report(args.provider);});
  await element('history-open').handlers.click();
  for(const provider of ['claude','copilot','antigravity']){
    element('history-provider').value=provider;
    await element('history-provider').handlers.change();
  }
  assert.equal(JSON.stringify(calls.map(call=>call.args)),JSON.stringify(['codex','claude','copilot','antigravity'].map(provider=>({provider}))));
  assert.ok(calls.every(call=>call.command==='history_sessions'&&!('path' in call.args)));
  assert.match(element('history-status').textContent,/格式尚未驗證/);
  assert.match(element('history-status').textContent,/不會以額度資料代替對話 Token/);
  assert.doesNotMatch(element('history-status').textContent,/%/);
});

test('prompt content is read only after per-session consent and is cleared on close',async()=>{
  const {context,element}=harness();const calls=[];
  context.initHistoryUi(async(command,args)=>{
    calls.push({command,args});
    if(command==='history_sessions')return report(args.provider);
    return {schema_version:'agentmeter.provider-session-content.v1',provider:args.provider,session_ref:args.sessionRef,limited:false,duplicate_prompt_count:2,prompts:[{turn_ref:'turn-1',text:'[已遮罩] <img onerror=PRIVATE_XSS_SENTINEL>'}]};
  });
  await element('history-open').handlers.click();
  const reveal=findByDataset(element('history-sessions'),'sessionRef','session-2');
  assert.ok(reveal);
  await reveal.handlers.click();
  assert.equal(calls.filter(call=>call.command==='history_session_content').length,0);
  assert.equal(element('history-content-dialog').open,true);
  element('history-content-consent').checked=true;
  await element('history-content-load').handlers.click();
  const contentCalls=calls.filter(call=>call.command==='history_session_content');
  assert.equal(contentCalls.length,1);
  assert.equal(JSON.stringify(contentCalls[0].args),JSON.stringify({provider:'codex',sessionRef:'session-2',expectedStartedAt:'2026-09-12T02:00:00Z',expectedEndedAt:'2026-09-12T02:20:00Z',consent:true}));
  assert.match(text(element('history-content-body')),/PRIVATE_XSS_SENTINEL/);
  assert.match(element('history-content-status').textContent,/完全相同的 Prompt 額外出現 2 次/);
  assert.equal(findTag(element('history-content-body'),'IMG'),false);
  element('history-content-close').handlers.click();
  assert.doesNotMatch(text(element('history-content-body')),/PRIVATE_XSS_SENTINEL/);
  assert.equal(element('history-content-consent').checked,false);
});

test('provider switch clears prompt DOM and invalidates late content responses',async()=>{
  const {context,element}=harness();let resolveContent;
  context.initHistoryUi((command,args)=>command==='history_sessions'?Promise.resolve(report(args.provider)):new Promise(resolve=>{resolveContent=resolve;}));
  await element('history-open').handlers.click();
  await findByDataset(element('history-sessions'),'sessionRef','session-2').handlers.click();
  element('history-content-consent').checked=true;
  const loading=element('history-content-load').handlers.click();
  element('history-provider').value='claude';
  await element('history-provider').handlers.change();
  resolveContent({schema_version:'agentmeter.provider-session-content.v1',provider:'codex',session_ref:'session-2',prompts:[{turn_ref:'turn-1',text:'LATE_PRIVATE_SENTINEL'}]});
  await loading;
  assert.doesNotMatch(text(element('history-content-body')),/LATE_PRIVATE_SENTINEL/);
  assert.equal(element('history-content-dialog').open,false);
  assert.equal(element('history-content-consent').checked,false);
});

test('history renderer has no dynamic HTML or browser persistence sinks',()=>{
  const source=fs.readFileSync(path.join(__dirname,'../ui/history.js'),'utf8');
  assert.doesNotMatch(source,/innerHTML|outerHTML|insertAdjacentHTML|document\.write|localStorage|sessionStorage|indexedDB|console\./);
});

function findByDataset(node,key,value){
  if(node.dataset?.[key]===value)return node;
  for(const child of node.children||[]){const match=findByDataset(child,key,value);if(match)return match;}
  return null;
}
function findTag(node,tag){
  if(node.tagName===tag)return true;
  return (node.children||[]).some(child=>findTag(child,tag));
}
