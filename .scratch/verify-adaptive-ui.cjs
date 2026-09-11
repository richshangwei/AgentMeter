const {chromium} = require('C:/Users/richs/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname,'..');

const names = ['codex','claude','copilot','antigravity',...Array.from({length:8},(_,index)=>`future-${index+1}`)];
const tabletSnapshot = {stream_id:'adaptive-check',revision:1,provider_data:'live',providers:names.map((provider,index)=>({
  provider,availability:index===2?'needs_login':'available',collection_state:index===2?'error':'ready',freshness:'fresh',collected_at:Date.now(),
  failure_code:index===2?'authentication_required':null,quota_windows:index===2?[]:[{label:index%2?'目前工作階段':'5 小時',remaining_percent:Math.max(4,82-index*6),reset_display:'2 小時 14 分'},{label:'每週',remaining_percent:61}]
}))};
const desktopSnapshot = {refreshing:false,provider_states:tabletSnapshot.providers};
const desktopTextStressSnapshot = {refreshing:false,provider_states:tabletSnapshot.providers.map((provider,index)=>index===0?{...provider,quota_windows:provider.quota_windows.map((item,windowIndex)=>windowIndex===0?{...item,remaining_percent:100}:item)}:provider)};

function assets(folder, files) {
  const map = new Map([['/',['text/html',fs.readFileSync(path.join(root,folder,'index.html'))]]]);
  for (const file of files) {
    const type=file.endsWith('.css')?'text/css':file.endsWith('.png')?'image/png':'text/javascript';
    map.set('/'+file,[type,fs.readFileSync(path.join(root,folder,file))]);
  }
  return map;
}
const tabletAssets=assets('tablet-ui',['protocol.js','recovery.js','view.js','client.js','assets/agentmeter-icon.png','assets/codex-icon.png','assets/claude-icon.png','assets/copilot-icon.png','assets/antigravity-icon.png','assets/empty-cloud.png']);
const desktopAssets=assets('desktop-p0/ui',['style.css','style-overrides.css','layout.js','dashboard.js','assets/operations-room-bg.png','assets/agentmeter-icon.png','assets/codex-icon.png','assets/claude-icon.png','assets/copilot-icon.png','assets/antigravity-icon.png','assets/empty-cloud.png']);

async function routeStatic(page,host,map){
  await page.route(`http://${host}/**`,route=>{const asset=map.get(new URL(route.request().url()).pathname);return asset?route.fulfill({status:200,contentType:asset[0],body:asset[1]}):route.fulfill({status:404,body:''});});
}
async function geometry(page,selector){return page.evaluate(selector=>{
  const isDesktop=selector.includes('desktop-cards');
  // Check relationships between visible content, not just containment in a card.
  for (const primary of document.querySelectorAll('.quota > .window:first-child')) {
    if (!primary.getClientRects().length) continue;
    const label=primary.children[0], value=primary.children[1];
    if (!label || !value) continue;
    const a=label.getBoundingClientRect(), b=value.getBoundingClientRect();
    if (Math.min(a.right,b.right)>Math.max(a.left,b.left)+1 && Math.min(a.bottom,b.bottom)>Math.max(a.top,b.top)+1) {
      throw Error(`Period label overlaps percentage at ${innerWidth}x${innerHeight}: ${label.textContent}`);
    }
    if (a.width<1 || a.height<1) throw Error('Period label has no readable layout space');
  }
  const html=document.documentElement,body=document.body,visible=[...document.querySelectorAll(selector)].filter(node=>node.getClientRects().length);
  const rects=visible.map(node=>node.getBoundingClientRect());
  const offenders=[];const contentInside=visible.every(card=>{const cardRect=card.getBoundingClientRect(),required=[card.querySelector('.heading,.card-head'),card.querySelector('.window,.metric,.unknown'),...card.querySelectorAll('button')].filter(Boolean).filter(node=>node.getClientRects().length);return required.every(node=>{const rect=node.getBoundingClientRect(),inside=rect.top>=cardRect.top-1&&rect.bottom<=cardRect.bottom+1&&rect.left>=cardRect.left-1&&rect.right<=cardRect.right+1;if(!inside)offenders.push({card:card.dataset.provider||card.dataset.providerCard,node:node.className||node.tagName,cardRect:{t:cardRect.top,b:cardRect.bottom,l:cardRect.left,r:cardRect.right},rect:{t:rect.top,b:rect.bottom,l:rect.left,r:rect.right}});return inside;});});
  const quotaOffenders=[];const quotaInside=visible.every(card=>{const quota=card.querySelector('.quota');if(!quota||!quota.getClientRects().length)return true;const quotaRect=quota.getBoundingClientRect();return [...quota.querySelectorAll('.window')].filter(node=>node.getClientRects().length).every(node=>{const rect=node.getBoundingClientRect(),inside=rect.top>=quotaRect.top-1&&rect.bottom<=quotaRect.bottom+1&&rect.left>=quotaRect.left-1&&rect.right<=quotaRect.right+1;if(!inside)quotaOffenders.push({card:card.dataset.provider||card.dataset.providerCard,node:'quota-window',quotaRect:{t:quotaRect.top,b:quotaRect.bottom,l:quotaRect.left,r:quotaRect.right},rect:{t:rect.top,b:rect.bottom,l:rect.left,r:rect.right}});return inside;});});
  const ringOffenders=[],valueOffenders=[];let valueInside=true;const ringInside=visible.every(card=>{const quota=card.querySelector('.quota'),primary=quota?.querySelector('.window:first-child');if(!primary||!primary.getClientRects().length)return true;const pseudo=getComputedStyle(primary,'::before'),ringSize=parseFloat(pseudo.width)||0,primaryStyle=getComputedStyle(primary),primaryRect=primary.getBoundingClientRect(),quotaRect=quota.getBoundingClientRect(),cardRect=card.getBoundingClientRect(),firstTrack=parseFloat(primaryStyle.gridTemplateColumns)||primaryRect.width,halo=card.closest('.cards')?.dataset.count==='4'&&innerWidth>=1200?8:0,ringLeft=primaryRect.left+(firstTrack-ringSize)/2,ringTop=primaryRect.top+(primaryRect.height-ringSize)/2,paint={l:ringLeft-halo,r:ringLeft+ringSize+halo,t:ringTop-halo,b:ringTop+ringSize+halo},inside=paint.l>=quotaRect.left-1&&paint.r<=quotaRect.right+1&&paint.t>=quotaRect.top-1&&paint.b<=quotaRect.bottom+1&&paint.l>=cardRect.left-1&&paint.r<=cardRect.right+1&&paint.t>=cardRect.top-1&&paint.b<=cardRect.bottom+1;const value=primary.querySelector(':scope > div:nth-child(2)'),textNode=value?.firstChild,range=textNode&&document.createRange();if(range){range.setStart(textNode,0);range.setEnd(textNode,textNode.length);}const textWidth=range?.getBoundingClientRect().width||0,mustFitCenter=isDesktop&&card.closest('.cards')?.dataset.count==='4'&&innerWidth>=1200&&innerHeight>=800,valueLimit=ringSize*.69*.9,valueFits=!mustFitCenter||textWidth<=valueLimit+1;if(!valueFits){valueInside=false;valueOffenders.push({card:card.dataset.provider||card.dataset.providerCard,node:'primary-value',value:textNode?.textContent,textWidth,valueLimit,ringSize,ratio:textWidth/(ringSize*.69),fontSize:getComputedStyle(value).fontSize});}if(!inside)ringOffenders.push({card:card.dataset.provider||card.dataset.providerCard,node:'primary-ring-paint',ringSize,halo,paint,quotaRect:{t:quotaRect.top,b:quotaRect.bottom,l:quotaRect.left,r:quotaRect.right},filter:pseudo.filter});return inside;});
  const grid=document.querySelector(selector.includes('desktop-cards')?'#desktop-cards':'#cards');
  return {scrollX:Math.max(html.scrollWidth,body.scrollWidth)>innerWidth,scrollY:Math.max(html.scrollHeight,body.scrollHeight)>innerHeight,
    count:visible.length,add:visible.filter(node=>node.classList.contains('add-card')).length,
    columns:Number(grid?.style.getPropertyValue('--grid-columns')),rows:Number(grid?.style.getPropertyValue('--grid-rows')),
    inside:rects.every(rect=>rect.left>=-1&&rect.top>=-1&&rect.right<=innerWidth+1&&rect.bottom<=innerHeight+1),contentInside,quotaInside,ringInside,valueInside,offenders,quotaOffenders,ringOffenders,valueOffenders};
},selector);}
function expectedGrid(count){return count<=1?[1,1]:count===2?[2,1]:count===3?[1,3]:[2,2];}
function assertGeometry(kind,width,height,state){const [columns,rows]=expectedGrid(state.count);if(state.scrollX||state.scrollY||!state.inside||!state.contentInside||!state.quotaInside||!state.ringInside||!state.valueInside||state.add!==0||state.count>4||state.columns!==columns||state.rows!==rows)throw Error(`${kind} ${width}x${height} ${JSON.stringify(state)} expected=${columns}x${rows}`);}
async function dialogInside(page,dialogSelector,requiredSelectors){return page.evaluate(({dialogSelector,requiredSelectors})=>{const dialog=document.querySelector(dialogSelector),outer=dialog.getBoundingClientRect();return requiredSelectors.every(selector=>[...dialog.querySelectorAll(selector)].filter(node=>node.getClientRects().length).every(node=>{const rect=node.getBoundingClientRect();return rect.top>=outer.top-1&&rect.bottom<=outer.bottom+1&&rect.left>=outer.left-1&&rect.right<=outer.right+1;}));},{dialogSelector,requiredSelectors});}

(async()=>{
  const browser=await chromium.launch({headless:true,channel:'msedge'});
  const desktopSizes=[[2491,1312],[2440,1288],[1993,1050],[1952,1030],[1452,1086],[1440,900],[1080,640],[640,520],[520,400]];
  for(const [width,height] of desktopSizes){
    const page=await browser.newPage({viewport:{width,height}});await routeStatic(page,'desktop.local',desktopAssets);
    await page.addInitScript(snapshot=>{window.__TAURI__={core:{invoke:async command=>command==='snapshot'||command==='refresh_quota'?snapshot:command==='check_update'?{configured:false,available:false,current_version:'0.1.0'}:{running:false}}};},desktopSnapshot);
    await page.goto('http://desktop.local/');await page.waitForFunction(()=>document.getElementById('desktop-cards'));
    for(const count of [1,2,3,4]){await page.evaluate(({ids,snapshot,count})=>{desktopSelection=ids.slice(0,count);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot,count});assertGeometry('desktop',width,height,await geometry(page,'#desktop-cards > article'));}
    if(width>=1200&&height>=800){await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopTextStressSnapshot});assertGeometry('desktop-100-percent',width,height,await geometry(page,'#desktop-cards > article'));}
    await page.evaluate(({ids,snapshot})=>{desktopSelection=ids;desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});
    const seen=new Set();let pages=1;
    do {const state=await geometry(page,'#desktop-cards > article');assertGeometry('desktop',width,height,state);
      (await page.locator('#desktop-cards > article:not([hidden])').evaluateAll(nodes=>nodes.map(node=>node.dataset.providerCard))).forEach(id=>seen.add(id));
      const info=await page.locator('#desktop-page').textContent();pages=Number(info.split('/')[1]);if((await page.locator('#desktop-next').isEnabled()))await page.locator('#desktop-next').click();else break;
    } while(seen.size<names.length);
    if(seen.size!==names.length)throw Error(`desktop ${width}x${height} saw ${seen.size}/${names.length} across ${pages} pages`);
    if(width===2491||width===1452)await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;desktopOptionPage=0;desktopOptionsSignature='';render(snapshot);},{ids:names,snapshot:desktopSnapshot});
    await page.locator('#app-settings').click();
    if(!await dialogInside(page,'#app-settings-dialog',['.dialog-title','.monitor-option','#check-update','#desktop-options-pager:not([hidden])']))throw Error(`desktop settings clipped at ${width}x${height}`);
    if(width===1452){const options=await page.locator('#desktop-monitor-options').textContent();if(!options.includes('Cursor')||!options.includes('Kiro')||!options.includes('尚未支援監控'))throw Error(`desktop catalog missing at ${width}x${height}`);}
    if(width===1452){await page.screenshot({path:path.join(root,'.scratch/desktop-settings.png')});}
    await page.locator('#app-settings-close').click();
    await page.evaluate(()=>showDesktopGuide('copilot'));
    if(!await dialogInside(page,'#desktop-guide-dialog',['.dialog-title','.guide-steps li','.official-link']))throw Error(`desktop setup guide clipped at ${width}x${height}`);
    await page.locator('#desktop-guide-close').click();
    if(width===1452) {await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});await page.screenshot({path:path.join(root,'.scratch/desktop-reference-size.png')});}
    if(width===2491) {await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});await page.screenshot({path:path.join(root,'.scratch/desktop-ultrawide.png')});}
    if(width===2440) {await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});await page.screenshot({path:path.join(root,'.scratch/desktop-percentage-fixed.png')});}
    if(width===1993) {await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});await page.screenshot({path:path.join(root,'.scratch/desktop-hidpi-equivalent.png')});}
    if(width===1952) {await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});await page.screenshot({path:path.join(root,'.scratch/desktop-percentage-hidpi-fixed.png')});}
    if(width===1440) {await page.evaluate(({ids,snapshot})=>{desktopSelection=ids.slice(0,4);desktopPage=0;render(snapshot);},{ids:names,snapshot:desktopSnapshot});await page.screenshot({path:path.join(root,'.scratch/desktop-adaptive.png')});}
    await page.close();
  }
  const tabletSizes=[[1280,800],[1024,768],[768,1024],[390,844],[844,390],[320,480],[320,400]];
  for(const [width,height] of tabletSizes){
    const page=await browser.newPage({viewport:{width,height}});await routeStatic(page,'tablet.local',tabletAssets);await page.goto('http://tablet.local/');
    await page.evaluate(snapshot=>{render(snapshot,true);document.getElementById('pairing').hidden=true;document.getElementById('dashboard').hidden=false;},tabletSnapshot);
    for(const count of [1,2,3,4]){await page.evaluate(({snapshot,count})=>{selectedMonitors=snapshot.providers.slice(0,count).map(item=>item.provider);monitorPage=0;renderCards();},{snapshot:tabletSnapshot,count});assertGeometry('tablet',width,height,await geometry(page,'#cards > article'));}
    await page.evaluate(snapshot=>{selectedMonitors=snapshot.providers.map(item=>item.provider);monitorPage=0;renderCards();},tabletSnapshot);
    const seen=new Set();
    do {const state=await geometry(page,'#cards > article');assertGeometry('tablet',width,height,state);
      (await page.locator('#cards > article').evaluateAll(nodes=>nodes.map(node=>node.dataset.provider))).forEach(id=>seen.add(id));
      if((await page.locator('#next-page').isEnabled())&&(await page.locator('#next-page').isVisible()))await page.locator('#next-page').click();else break;
    } while(seen.size<names.length);
    if(seen.size!==names.length)throw Error(`tablet ${width}x${height} saw ${seen.size}/${names.length}`);
    const touchTargets=await page.locator('button').evaluateAll(nodes=>nodes.filter(node=>node.getClientRects().length).every(node=>node.getBoundingClientRect().height>=44));
    if(!touchTargets)throw Error(`tablet touch target below 44px at ${width}x${height}`);
    if(width===1024&&height===768)await page.evaluate(snapshot=>{selectedMonitors=snapshot.providers.slice(0,4).map(item=>item.provider);monitorPage=0;settingsOptionPage=0;renderCards();renderSettings();},tabletSnapshot);
    await page.locator('#settings').click();
    if(!await dialogInside(page,'#settings-dialog',['.dialog-head','.monitor-option','#settings-options-pager:not([hidden])']))throw Error(`tablet settings clipped at ${width}x${height}`);
    if(width===1024&&height===768){const options=await page.locator('#monitor-options').textContent();if(!options.includes('Cursor')||!options.includes('Kiro')||!options.includes('尚未支援監控'))throw Error(`tablet catalog missing at ${width}x${height}`);}
    if(width===1024&&height===768)await page.screenshot({path:path.join(root,'.scratch/tablet-settings.png')});
    await page.locator('[data-close="settings-dialog"]').click();
    if(width===1024&&height===768){await page.evaluate(snapshot=>{selectedMonitors=snapshot.providers.slice(0,4).map(item=>item.provider);monitorPage=0;renderCards();},tabletSnapshot);await page.screenshot({path:path.join(root,'.scratch/tablet-adaptive.png')});}
    if(width===390){await page.evaluate(snapshot=>{selectedMonitors=snapshot.providers.slice(0,4).map(item=>item.provider);monitorPage=0;renderCards();},tabletSnapshot);await page.screenshot({path:path.join(root,'.scratch/phone-adaptive.png')});}
    await page.close();
  }
  await browser.close();console.log('adaptive UI: 9 desktop + 7 tablet/phone viewports, exact 1/2/3/4 layouts, max four cards per page, zero add tiles, zero document scroll, quota rings and primary percentage text contained');
})().catch(error=>{console.error(error);process.exit(1);});
