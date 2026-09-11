const {chromium} = require('C:/Users/richs/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname,'..');
const assets = new Map([
  ['/', ['text/html',fs.readFileSync(path.join(root,'tablet-ui/index.html'))]],
  ...['protocol.js','recovery.js','view.js','client.js'].map(name=>[`/${name}`,['text/javascript',fs.readFileSync(path.join(root,'tablet-ui',name))]])
]);
const snapshot = {stream_id:'visual-check',revision:1,provider_data:'live',providers:[
  {provider:'codex',availability:'available',collection_state:'ready',freshness:'fresh',collected_at:Date.now(),quota_windows:[{label:'5 小時',remaining_percent:82,reset_display:'2 小時 14 分'},{label:'每週',remaining_percent:61}]},
  {provider:'claude',availability:'available',collection_state:'ready',freshness:'fresh',collected_at:Date.now(),quota_windows:[{label:'目前工作階段',remaining_percent:47},{label:'本週',remaining_percent:28}]},
  {provider:'copilot',availability:'needs_login',collection_state:'error',freshness:'unknown',failure_code:'authentication_required',quota_windows:[]},
  {provider:'antigravity',availability:'available',collection_state:'error',freshness:'stale',failure_code:'timeout',collected_at:Date.now()-600000,quota_windows:[{label:'Gemini 每週',remaining_percent:8}]}
]};
(async()=>{
  const browser=await chromium.launch({headless:true,channel:'msedge'});
  for(const viewport of [{width:1024,height:768},{width:1280,height:800},{width:768,height:1024}]){
    const page=await browser.newPage({viewport});
    await page.route('http://agentmeter.local/**',route=>{const asset=assets.get(new URL(route.request().url()).pathname);asset?route.fulfill({status:200,contentType:asset[0],body:asset[1]}):route.fulfill({status:404,body:''});});
    await page.goto('http://agentmeter.local/');
    await page.evaluate(value=>{render(value,true);document.getElementById('pairing').hidden=true;document.getElementById('dashboard').hidden=false;},snapshot);
    const result=await page.evaluate(()=>({overflow:document.documentElement.scrollWidth>document.documentElement.clientWidth,cards:document.querySelectorAll('#cards article').length,add:document.querySelectorAll('.add-card').length,first:document.querySelector('.metric-value')?.textContent,buttons:[...document.querySelectorAll('button')].filter(button=>button.getClientRects().length>0).every(button=>button.getBoundingClientRect().height>=44)}));
    if(result.overflow||result.cards!==5||result.add!==1||result.first!=='82%'||!result.buttons)throw Error(`${viewport.width}x${viewport.height} ${JSON.stringify(result)}`);
    if(viewport.width===1280)await page.screenshot({path:path.join(root,'.scratch/tablet-dashboard.png'),fullPage:true});
    await page.close();
  }
  await browser.close();
  console.log('tablet UI: 3 viewports, five tiles with one add slot, no horizontal overflow, touch targets pass');
})().catch(error=>{console.error(error);process.exit(1);});
