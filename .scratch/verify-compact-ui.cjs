const { chromium } = require('C:/Users/richs/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const { pathToFileURL } = require('node:url');
const assert = require('node:assert/strict');
(async()=>{
 const browser=await chromium.launch({headless:true,channel:'msedge'});
 try { const page=await browser.newPage({viewport:{width:1080,height:640}});
 await page.addInitScript(()=>{window.__TAURI__={core:{invoke:async c=>c==='snapshot'?{provider_states:[]}:{running:false}}};window.setInterval=()=>0;});
 await page.goto(pathToFileURL(require('node:path').resolve('desktop-p0/ui/index.html')).href);
 const results=[];
 for(const scenario of ['ready','stale','error','trust']){
 await page.evaluate(scenario=>render({provider_states:['codex','claude','copilot','antigravity'].map(provider=>({provider,collection_state:scenario==='ready'?'ready':'error',freshness:scenario==='stale'?'stale':'fresh',failure_code:scenario==='trust'&&provider==='claude'?'workspace_trust_required':'cli_not_found',collected_at:1789010000000,quota_windows:['error','trust'].includes(scenario)?[]:Array.from({length:{codex:3,claude:2,copilot:1,antigravity:4}[provider]},(_,i)=>({label:{codex:['codex primary','codex secondary','base_model_inference'],claude:['five_hour','seven_day'],copilot:['premium_interactions'],antigravity:['Gemini Models Weekly Limit Remaining','Gemini Models Five Hour Limit Remaining','Claude and GPT models Weekly Limit Remaining','Claude and GPT models Five Hour Limit Remaining']}[provider][i],remaining_percent:[24.7,72,8,100][i],reset_display:provider==='claude'?'Sep 17 at 10pm (Asia/Taipei)':'2026/9/17 下午 10:00',...(provider==='copilot'?{entitlement:1500,used:1130}:{})}))}))}),scenario);
 const metrics=await page.evaluate(()=>({height:document.querySelector('main').getBoundingClientRect().height,width:document.documentElement.scrollWidth,tops:[...document.querySelectorAll('.cards article')].map(e=>e.getBoundingClientRect().top)}));
 assert.ok(metrics.height<=640,JSON.stringify({scenario,...metrics}));assert.equal(metrics.width,1080);assert.equal(new Set(metrics.tops).size,1);
 results.push({scenario,...metrics});await page.screenshot({path:'.scratch/compact-ui-'+scenario+'.png'});
 }
 for(const width of [640,390]){await page.setViewportSize({width,height:640});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),width);}
 await page.setViewportSize({width:640,height:640}); await page.locator('.tablet-settings summary').click(); const expanded=await page.evaluate(()=>({width:document.documentElement.scrollWidth,height:document.documentElement.scrollHeight,tabletVisible:document.querySelector('.tablet-settings').open})); assert.equal(expanded.width,640); assert.ok(expanded.tabletVisible); results.push({scenario:'tablet-expanded-640',...expanded}); console.log(JSON.stringify(results)); } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exitCode=1;});
