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
      addEventListener(type,fn){this.handlers[type]=fn;},append(...nodes){this.children.push(...nodes);},replaceChildren(...nodes){this.children=nodes;this.textContent='';}});
    return elements.get(id);
  }
  const providers=['codex','claude','copilot','antigravity'];
  const buttons=providers.map(p=>{const e=element(p+'-button');e.dataset.provider=p;return e;});
  let serial=0;
  const context={document:{getElementById:element,createElement:()=>element('new'+serial++),
    querySelectorAll:selector=>selector==='[data-provider]'?buttons:[...buttons,element('refresh'),element('claude-enable')]},
    setTimeout(){},clearTimeout(){},setInterval(){},Date,Number,String,
    window:{confirm:()=>true,__TAURI__:{core:{invoke:async(command,args)=>{calls.push({command,args});return command==='snapshot'||command==='refresh_quota'?{provider_states:[],refreshing:false}:{running:false};}}}}};
  vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../ui/dashboard.js'),'utf8'),context);
  return {context,element,calls};
}
const text = e => e.textContent+e.children.map(text).join(' ');
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
