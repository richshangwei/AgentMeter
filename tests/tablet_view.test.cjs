const {test} = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');

const context = vm.createContext({URL});
vm.runInContext(fs.readFileSync(path.join(__dirname,'../tablet-ui/view.js'),'utf8'), context);

test('tablet dashboard has no add tile and routes monitor management through settings',()=>{
  const client=fs.readFileSync(path.join(__dirname,'../tablet-ui/client.js'),'utf8');
  assert.doesNotMatch(client,/className='card add-card'|空白新增格/);
  assert.match(client,/get\('settings'\)\.onclick/);
  assert.match(client,/Cursor/);
  assert.match(client,/Kiro/);
});

test('monitor selection is unique, future-provider friendly, and settings-owned', () => {
  const available = ['codex','claude','copilot','antigravity','future-provider'];
  assert.deepEqual(Array.from(context.normalizeMonitorSelection(null,available)), available);
  assert.deepEqual(Array.from(context.normalizeMonitorSelection(['claude','claude','missing','codex'],available)), ['claude','codex']);
  assert.deepEqual(Array.from(context.setMonitorEnabled(['codex'],'claude',true,available)), ['codex','claude']);
  assert.deepEqual(Array.from(context.setMonitorEnabled(['codex','claude'],'codex',false,available)), ['claude']);
  assert.deepEqual(Array.from(context.monitorSlots([],available)), []);
  assert.deepEqual(Array.from(context.monitorSlots(['codex'],available)), ['codex']);
});

test('adaptive monitor pages render at most four cards with exact composition and no reserved slot', () => {
  const matrix=[[1,{columns:1,rows:1,pageSize:4}],[2,{columns:2,rows:1,pageSize:4}],[3,{columns:1,rows:3,pageSize:4}],[4,{columns:2,rows:2,pageSize:4}]];
  for(const [width,height] of [[1280,800],[1024,768],[390,844],[844,390]]) for(const [count,expected] of matrix)
    assert.deepEqual({...context.tabletViewportGrid(width,height,count)},expected,`${width}x${height} count=${count}`);
  const available = Array.from({length:12},(_,index)=>`provider-${index+1}`);
  const first = context.pagedMonitorSlots(available,available,0,4);
  const last = context.pagedMonitorSlots(available,available,99,4);
  assert.deepEqual(Array.from(first.slots),available.slice(0,4));
  assert.equal(first.pages,3);
  assert.equal(last.page,2);
  assert.deepEqual(Array.from(last.slots),available.slice(8));
  assert.deepEqual(Array.from(context.pagedMonitorSlots([],available,0,4).slots),[]);
});

test('tablet settings can reorder selected monitors without changing membership', () => {
  const selected=['codex','claude','copilot','antigravity'];
  assert.deepEqual(Array.from(context.moveMonitor(selected,'codex',-1)),selected);
  assert.deepEqual(Array.from(context.moveMonitor(selected,'antigravity',1)),selected);
  assert.deepEqual(Array.from(context.moveMonitor(selected,'copilot',-1)),['codex','copilot','claude','antigravity']);
});

test('primary metric preserves unknown and zero rather than inventing quota', () => {
  assert.deepEqual({...context.primaryMetric({quota_windows:[{label:'5 小時',remaining_percent:0,resets_at:123}]})},
    {label:'5 小時',value:0,unit:'%',reset:123,kind:'quota'});
  assert.equal(context.primaryMetric({quota_windows:[{remaining_percent:null}]}).value,null);
  assert.deepEqual({...context.primaryMetric({quota_windows:[],source_usage:[{label:'Premium interactions',used:12,unit:'requests'}]})},
    {label:'Premium interactions',value:12,unit:'requests',reset:null,kind:'usage'});
  assert.equal(context.primaryMetric({quota_windows:[],source_usage:[]}).value,null);
});

test('tablet uses the same duration-based Codex quota labels as desktop', () => {
  assert.equal(context.quotaLabel({limit_id:'codex',window_duration_mins:10080}),'常規使用額度 · 每週使用上限');
  assert.equal(context.quotaLabel({limit_id:'codex_bengalfox',limit_name:'GPT-5.3-Codex-Spark',window_duration_mins:300}),'GPT-5.3-Codex-Spark · 5 小時用量限制');
  assert.equal(context.quotaLabel({limit_id:'base_model_inference',limit_name:'gpt-reserve',window_duration_mins:10080}),'備用模型額度 · 每週使用上限');
});

test('provider setup guides are stable, numbered, and only use official https links', () => {
  for (const provider of ['codex','claude','copilot','antigravity']) {
    const guide = context.setupGuide(provider,'cli_not_found');
    assert.equal(guide.provider,provider);
    assert.ok(guide.steps.length >= 3);
    assert.match(guide.url,/^https:\/\//);
  }
  assert.match(context.setupGuide('claude','workspace_trust_required').steps.join(' '),/桌機/);
  assert.equal(context.needsSetupGuide({setup:'required',availability:'unknown',collection_state:'idle',quota_windows:[],source_usage:[]}),true);
  assert.equal(context.needsSetupGuide({setup:'ready',availability:'available',collection_state:'collecting',quota_windows:[],source_usage:[]}),false);
  assert.equal(context.needsSetupGuide({setup:'ready',availability:'available',collection_state:'ready',quota_windows:[],source_usage:[{used:1}]}),false);
});

test('fullscreen controller is progressive and keeps button state synchronized', async () => {
  const handlers = {};
  const button = {textContent:'',hidden:false,setAttribute(name,value){this[name]=value;}};
  const root = {requestFullscreen:async()=>{doc.fullscreenElement=root;handlers.fullscreenchange();}};
  const doc = {documentElement:root,fullscreenElement:null,exitFullscreen:async()=>{doc.fullscreenElement=null;handlers.fullscreenchange();},
    addEventListener:(name,fn)=>{handlers[name]=fn;}};
  const toggle = context.createFullscreenController(doc,button);
  assert.equal(button.textContent,'全螢幕');
  await toggle(); assert.equal(button.textContent,'退出全螢幕'); assert.equal(button['aria-pressed'],'true');
  await toggle(); assert.equal(button.textContent,'全螢幕'); assert.equal(button['aria-pressed'],'false');
  const unsupported = {documentElement:{},addEventListener(){}};
  const unsupportedButton = {setAttribute(){}};
  context.createFullscreenController(unsupported,unsupportedButton);
  assert.equal(unsupportedButton.hidden,true);
});
