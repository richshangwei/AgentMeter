const { chromium } = require('C:/Users/richs/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const { pathToFileURL } = require('node:url');
const assert = require('node:assert/strict');
(async()=>{
 const browser=await chromium.launch({headless:true,channel:'msedge'});
 try { const page=await browser.newPage({viewport:{width:1080,height:640}});
 await page.addInitScript(()=>{window.__TAURI__={core:{invoke:async c=>c==='snapshot'?{provider_states:[]}:{running:false}}};window.setInterval=()=>0;});
 await page.goto(pathToFileURL(require('node:path').resolve('desktop-p0/ui/index.html')).href);
 const results=[];
 for(const scenario of ['ready','stale','error']){
 await page.evaluate(scenario=>render({provider_states:['codex','claude','copilot','antigravity'].map(provider=>({provider,collection_state:scenario==='ready'?'ready':'error',freshness:scenario==='stale'?'stale':'fresh',failure_code:'cli_not_found',collected_at:1789010000000,quota_windows:scenario==='error'?[]:Array.from({length:{codex:3,claude:2,copilot:1,antigravity:4}[provider]},(_,i)=>({label:['5 小時','每週','基本模型','Claude / GPT'][i],remaining_percent:[24.7,72,8,100][i],reset_display:'2026/9/17 下午 10:00'}))}))}),scenario);
 const metrics=await page.evaluate(()=>({height:document.querySelector('main').getBoundingClientRect().height,width:document.documentElement.scrollWidth,tops:[...document.querySelectorAll('.cards article')].map(e=>e.getBoundingClientRect().top)}));
 console.log(scenario,await page.locator("#antigravity-quota .window").evaluateAll(es=>es.map(e=>({height:e.getBoundingClientRect().height,children:[...e.children].map(c=>({text:c.textContent,height:c.getBoundingClientRect().height}))}))));assert.equal(metrics.width,1080);assert.equal(new Set(metrics.tops).size,1);
 results.push({scenario,...metrics});await page.screenshot({path:'.scratch/compact-ui-'+scenario+'.png'});
 }
 for(const width of [640,390]){await page.setViewportSize({width,height:640});assert.equal(await page.evaluate(()=>document.documentElement.scrollWidth),width);}
 console.log(JSON.stringify(results)); } finally { await browser.close(); }
})().catch(e=>{console.error(e);process.exitCode=1;});

