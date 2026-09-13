const test = require('node:test');
const assert = require('node:assert/strict');
const vm = require('node:vm');
const fs = require('node:fs');
const path = require('node:path');
function harness() {
  const elements = new Map(),calls=[];
  function element(id='') {
    if (!elements.has(id)) elements.set(id,{textContent:'',className:'',children:[],hidden:false,disabled:false,value:'',dataset:{},style:{},handlers:{},
      querySelectorAll(selector){return selector === '.window' ? this.children.filter(n=>n.className==='window') : [];},
      setAttribute(name,value){this[name]=value;},addEventListener(type,fn){this.handlers[type]=fn;},append(...nodes){this.children.push(...nodes);},replaceChildren(...nodes){this.children=nodes;this.textContent='';}});
    return elements.get(id);
  }
  const providers=['codex','claude','copilot','antigravity'];
  const buttons=providers.map(p=>{const e=element(p+'-button');e.dataset.provider=p;return e;});
  let serial=0;
  const context={document:{getElementById:element,createElement:()=>element('new'+serial++),
    querySelectorAll:selector=>selector==='[data-provider]'?buttons:[...buttons,element('refresh'),element('claude-enable')]},
    setTimeout(){},clearTimeout(){},setInterval(){},Date,Number,String,screen:{availWidth:1080,availHeight:900,availLeft:0,availTop:0},
    window:{innerWidth:1080,innerHeight:900,confirm:()=>true,localStorage:{getItem:()=>null,setItem(){}},__TAURI__:{core:{invoke:async(command,args)=>{calls.push({command,args});return command==='snapshot'||command==='refresh_quota'?{provider_states:[],refreshing:false}:{running:false};}}}}};
  vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../ui/layout.js'),'utf8'),context);
  vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../ui/hud-model.js'),'utf8'),context);
  vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../ui/dashboard.js'),'utf8'),context);
  return {context,element,calls};
}
const text = e => e.textContent+e.children.map(text).join(' ');
test('settings expose only backend-approved quota refresh intervals',()=>{
  const html=fs.readFileSync(path.join(__dirname,'../ui/index.html'),'utf8');
  assert.match(html,/id="quota-refresh-interval"/);
  const values=[...html.matchAll(/<option value="(\d+)">/g)].map(match=>Number(match[1]));
  assert.deepEqual(values,[120,300,600,900,1800,3600]);
  assert.doesNotMatch(html,/完成後 60 秒/);
});
test('renders four real quota cards, reset caveats and stale data without source forms',()=>{
  const {context,element}=harness();
  context.render({provider_states:['codex','claude','copilot','antigravity'].map(provider=>({provider,
    collection_state:'ready',freshness:'fresh',collected_at:1000,quota_windows:[{remaining_percent:24.7,label:'premium_interactions',entitlement:1500,used:1130}]}))});
  for (const p of ['codex','claude','copilot','antigravity']) assert.match(text(element(p+'-quota')),/24.7% 剩餘/);
  assert.match(text(element('copilot-quota')),/已用 1130 \/ 1500/);
  assert.match(text(element('copilot-quota')),/重設時間尚未確認/);
  context.render({provider_states:[{provider:'claude',collection_state:'error',freshness:'stale',failure_code:'authentication_required',quota_windows:[{remaining_percent:8}],collected_at:1000}]});
  assert.match(element('claude-status').textContent,/舊資料/);
  assert.equal(element('claude-status').className,'status-error');
  assert.match(element('claude-message').textContent,/登入/);
  context.render({provider_states:[{provider:'antigravity',collection_state:'error',freshness:'unknown',failure_code:'cli_not_found',quota_windows:[]}]});
  assert.match(element('antigravity-message').textContent,/重新執行 AgentMeter 安裝程式/);
});
test('quota bars use exact finite remaining values, clamp bounds and distinguish unknown',()=>{
  const {context,element}=harness();
  for (const [input,width,color] of [[24.7,24.7,'warning'],[30,30,'warning'],[30.1,30.1,'good'],[10.1,10.1,'warning'],[10,10,'low'],[0,0,'low'],[-2,0,'low'],[120,100,'good'],[null,null,null],[undefined,null,null],[NaN,null,null],[Infinity,null,null],['24.7',null,null]]) {
    context.render({provider_states:[{provider:'codex',collection_state:'ready',quota_windows:[{remaining_percent:input}]}]});
    const row=element('codex-quota').children[0];
    const track=row.children.find(n=>n.className.startsWith('quota-track'));
    if (width===null) {
      assert.equal(track.children.length,0);
      assert.match(text(row),/剩餘未知/);
      assert.match(track.className,/quota-unknown/);
    } else {
      assert.equal(track.children[0].style.width,width+'%');
      assert.equal(track.children[0].className,'quota-fill quota-'+color);
      assert.ok(text(row).includes(width+'% 剩餘'));
    }
  }
});
test('one refresh command has no source paths and explicit Claude trust only',async()=>{
  const {context,calls}=harness();
  await context.refreshQuota('all');
  const refresh=calls.find(c=>c.command==='refresh_quota');
  assert.equal(JSON.stringify(refresh.args),JSON.stringify({provider:'all',enableClaude:false}));
  assert.ok(!calls.some(c=>['load_claude','refresh_copilot','source_settings'].includes(c.command)));
});

test('backend sync settings populate controls, save an allowed interval and use actual provider time',async()=>{
  const {context,element,calls}=harness();
  context.window.__TAURI__.core.invoke=async(command,args)=>{
    calls.push({command,args});
    if(command==='auto_quota_settings')return {enabled:false,interval_seconds:900};
    if(command==='set_auto_quota_settings')return {enabled:args.enabled,interval_seconds:args.intervalSeconds};
    return command==='snapshot'||command==='refresh_quota'?{provider_states:[],refreshing:false}:{running:false};
  };
  await context.loadAutoQuotaSettings();
  assert.equal(element('auto-quota').checked,false);
  assert.equal(element('quota-refresh-interval').value,'900');
  element('quota-refresh-interval').value='300';
  await context.saveAutoQuotaSettings();
  assert.equal(JSON.stringify(calls.at(-1)),JSON.stringify({command:'set_auto_quota_settings',args:{enabled:false,intervalSeconds:300}}));

  const collectedAt=Date.UTC(2026,8,12,3,4);
  context.render({provider_states:[{provider:'codex',collection_state:'ready',collected_at:collectedAt,quota_windows:[]}]});
  const first=element('sync-clock').textContent;
  context.render({provider_states:[{provider:'codex',collection_state:'ready',collected_at:collectedAt,quota_windows:[]}]});
  assert.equal(element('sync-clock').textContent,first);
  assert.notEqual(first,'尚未同步');
});

test('future providers cannot collide with reserved DOM ids and disappearing providers leave the selector',()=>{
  const {context,element}=harness();
  const value=provider=>({provider,collection_state:'ready',freshness:'fresh',quota_windows:[{remaining_percent:50}]});
  assert.doesNotThrow(()=>context.render({provider_states:[value('codex'),value('cursor'),value('future-1'),value('desktop-add'),value('constructor')]}));
  assert.deepEqual(Array.from(vm.runInContext('desktopAvailable',context)),['codex','claude','copilot','antigravity','cursor','future-1']);
  assert.deepEqual(Array.from(vm.runInContext('desktopSelection',context)),['codex','claude','copilot','antigravity']);
  assert.match(text(element('desktop-monitor-options')),/Cursor/);
  assert.match(text(element('desktop-monitor-options')),/Kiro/);
  context.render({provider_states:[value('codex')]});
  assert.deepEqual(Array.from(vm.runInContext('desktopAvailable',context)),['codex','claude','copilot','antigravity']);
});

test('unchanged polling preserves desktop settings option nodes and focus targets',()=>{
  const {context,element}=harness();
  const before=element('desktop-monitor-options').children[0];
  context.render({provider_states:[{provider:'codex',collection_state:'ready',freshness:'fresh',quota_windows:[{remaining_percent:50}]}]});
  const after=element('desktop-monitor-options').children[0];
  assert.equal(after,before);
});

test('polling preserves quota nodes while changed data still renders every quota without scrolling',()=>{
  const {context,element}=harness();
  const snapshot={provider_states:[{provider:'codex',collection_state:'ready',quota_windows:Array.from({length:8},(_,index)=>({label:`window ${index}`,remaining_percent:24.7}))}]};
  context.render(snapshot);
  const quota=element('codex-quota'),before=quota.children[0];
  context.render(snapshot);
  assert.equal(quota.children[0],before);
  assert.match(element('codex-time').textContent,/共 8 項/);
  assert.doesNotMatch(element('codex-time').textContent+element('codex-message').textContent,/捲動/);
  assert.notEqual(quota.tabIndex,0,'a non-scrolling region must not become a tab stop');
  snapshot.provider_states[0].quota_windows[0].remaining_percent=23.5;
  context.render(snapshot);
  assert.notEqual(quota.children[0],before);
  assert.match(text(quota.children[0]),/23.5%/);
  assert.equal(quota.children.length,8);
});

test('quota tiles carry the full detail in a tooltip and a level for the gauge colour',()=>{
  const {context,element}=harness();
  context.render({provider_states:[{provider:'antigravity',collection_state:'ready',quota_windows:[
    {label:'Gemini Models Weekly Limit Remaining',remaining_percent:8,reset_display:'Monday'},
    {limit_id:'codex_bengalfox',limit_name:'GPT-5.3-Codex-Spark',window:'secondary',window_duration_mins:10080,remaining_percent:66.66,entitlement:10,used:3}]}]});
  const [first,second]=element('antigravity-quota').children;
  assert.equal(first.dataset.level,'low');
  assert.match(first.title,/Gemini · 每週/);
  assert.match(first.title,/重設：Monday/);
  assert.match(text(second),/66.7%/);
  assert.match(second.title,/GPT-5.3-Codex-Spark · 每週使用上限/);
  assert.match(second.title,/已用 3 \/ 10/);
});

test('HUD screen settings list all monitors and use backend positioning without browser coordinates',async()=>{
  const {context,element,calls}=harness();
  const monitors=Array.from({length:7},(_,index)=>({id:'display-'+index,label:'螢幕 '+(index+1),primary:index===0}));
  context.window.__TAURI__.core.invoke=async(command,args)=>{
    calls.push({command,args});
    if(command==='hud_monitors')return {monitors,selectedMonitor:'display-5'};
    return {};
  };
  await context.loadHudMonitors();
  assert.equal(element('hud-monitor').children.length,7);
  assert.equal(element('hud-monitor').value,'display-5');
  assert.match(element('hud-monitor').children[0].textContent,/主要/);
  element('hud-monitor').value='display-6';
  await context.changeHudPosition();
  const move=calls.find(call=>call.command==='move_hud_monitor');
  assert.equal(JSON.stringify(move.args),JSON.stringify({monitorId:'display-6'}));
  await context.changeHudPosition(true);
  assert.ok(calls.some(call=>call.command==='reset_hud_position'));
  element('hud-enabled').checked=true;
  context.applyHudPreference();
  const configure=calls.filter(call=>call.command==='configure_hud').at(-1);
  assert.equal(configure.args.width,240);
  assert.equal('x' in configure.args,false);
  assert.equal('y' in configure.args,false);
});

test('HUD monitor errors are actionable and failed moves reload the current selection',async()=>{
  const {context,element}=harness();
  context.window.__TAURI__.core.invoke=async()=>{throw new Error('disconnected');};
  await context.loadHudMonitors();
  assert.equal(element('hud-monitor').disabled,true);
  assert.equal(element('hud-position-reset').disabled,true);
  assert.match(element('hud-position-status').textContent,/重新整理螢幕/);
  context.window.__TAURI__.core.invoke=async(command)=>{
    if(command==='hud_monitors')return {monitors:[{id:'primary',label:'Primary',primary:true}],selectedMonitor:'primary'};
    throw new Error('disconnected');
  };
  await context.changeHudPosition();
  assert.equal(element('hud-monitor').value,'primary');
  assert.equal(element('hud-monitor').disabled,false);
  assert.match(element('hud-position-status').textContent,/移動失敗/);
});
