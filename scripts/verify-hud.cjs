const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const root=path.resolve(__dirname,'..');
const names=['codex','claude','copilot','antigravity'];
const snapshot={provider_states:[
  {provider:'codex',quota_windows:[{remaining_percent:92}]},
  {provider:'claude',quota_windows:[{remaining_percent:77}]},
  {provider:'copilot',quota_windows:[{remaining_percent:24.7}]},
  {provider:'antigravity',quota_windows:[{remaining_percent:54}]}
]};

(async()=>{
  const channel=process.env.AGENTMETER_BROWSER_CHANNEL;
  const browser=await chromium.launch({headless:true,...(channel?{channel}:{})});
  try{
    const page=await browser.newPage({viewport:{width:360,height:204}});
    const errors=[];
    page.on('pageerror',error=>errors.push(String(error)));
    await page.route('http://hud.local/**',route=>{
      const name=new URL(route.request().url()).pathname.slice(1)||'hud.html';
      const file=path.join(root,'desktop-p0/ui',name);
      const type=name.endsWith('.css')?'text/css':name.endsWith('.js')?'text/javascript':'text/html';
      return fs.existsSync(file)?route.fulfill({contentType:type,body:fs.readFileSync(file)}):route.fulfill({status:404});
    });
    await page.addInitScript(({snapshot,names})=>{
      localStorage.setItem('agentmeter.desktop-monitor-selection.v1',JSON.stringify(names));
      localStorage.setItem('agentmeter.hud-enabled.v1','true');
      localStorage.setItem('agentmeter.hud-opacity.v1','72');
      window.hudCalls=[];
      window.__TAURI__={core:{invoke:async(command,args)=>{
        if(command==='snapshot')return snapshot;
        if(command==='configure_hud')window.hudCalls.push(args);
        return {};
      }}};
    },{snapshot,names});
    await page.goto('http://hud.local/hud.html');
    await page.waitForFunction(()=>document.querySelectorAll('#hud-rows li').length===4);
    const result=await page.evaluate(()=>{
      const box=node=>node.getBoundingClientRect();
      const overlap=(a,b)=>Math.min(a.right,b.right)-Math.max(a.left,b.left)>1&&Math.min(a.bottom,b.bottom)-Math.max(a.top,b.top)>1;
      const rows=[...document.querySelectorAll('#hud-rows li')];
      return {
        text:rows.map(row=>[row.querySelector('.hud-name').textContent,row.querySelector('.hud-value').textContent]),
        overflow:document.documentElement.scrollWidth>innerWidth||document.documentElement.scrollHeight>innerHeight,
        rowOverflow:rows.some(row=>row.scrollWidth>row.clientWidth||row.scrollHeight>row.clientHeight),
        overlap:rows.some(row=>overlap(box(row.querySelector('.hud-name')),box(row.querySelector('.hud-value')))),
        nameSize:parseFloat(getComputedStyle(document.querySelector('.hud-name')).fontSize),
        valueSize:parseFloat(getComputedStyle(document.querySelector('.hud-value')).fontSize),
        controls:document.querySelectorAll('button,input,a').length,
        call:window.hudCalls.at(-1)
      };
    });
    assert.deepEqual(result.text,[['Codex','92%'],['Claude Code','77%'],['GitHub Copilot','24.7%'],['Antigravity','54%']]);
    assert.equal(result.overflow,false);
    assert.equal(result.rowOverflow,false);
    assert.equal(result.overlap,false);
    assert.equal(result.controls,0);
    assert(result.nameSize>=14);
    assert(result.valueSize>=20);
    assert.equal(result.call.enabled,true);
    assert.equal(result.call.height,204);
    for(const opacity of [35,72,95]){
      const actual=await page.evaluate(value=>{localStorage.setItem('agentmeter.hud-opacity.v1',String(value));renderHud();return getComputedStyle(document.documentElement).getPropertyValue('--surface-alpha').trim();},opacity);
      assert.equal(actual,String(opacity/100));
    }
    fs.mkdirSync(path.join(root,'.scratch'),{recursive:true});
    await page.screenshot({path:path.join(root,'.scratch/hud-360x204.png'),omitBackground:true});
    assert.deepEqual(errors,[]);

    await page.setViewportSize({width:640,height:520});
    await page.goto('http://hud.local/index.html');
    await page.waitForSelector('#app-settings');
    await page.click('#app-settings');
    await page.evaluate(()=>{
      localStorage.setItem('agentmeter.hud-enabled.v1','false');
      loadHudPreference();
    });
    await page.check('#hud-enabled');
    await page.locator('#hud-opacity').evaluate(input=>{input.value='48';input.dispatchEvent(new Event('input',{bubbles:true}));});
    const settings=await page.evaluate(()=>{
      const input=document.getElementById('hud-opacity'),section=document.querySelector('.hud-settings');
      section.scrollIntoView({block:'center'});
      const box=section.getBoundingClientRect();
      return {
        enabled:localStorage.getItem('agentmeter.hud-enabled.v1'),
        opacity:localStorage.getItem('agentmeter.hud-opacity.v1'),
        output:document.getElementById('hud-opacity-value').textContent,
        sliderHeight:input.getBoundingClientRect().height,
        horizontalFit:box.left>=0&&box.right<=innerWidth,
        call:window.hudCalls.at(-1)
      };
    });
    assert.equal(settings.enabled,'true');
    assert.equal(settings.opacity,'48');
    assert.equal(settings.output,'48%');
    assert(settings.sliderHeight>=44);
    assert.equal(settings.horizontalFit,true);
    assert.equal(settings.call.enabled,true);
    assert.deepEqual(errors,[]);
    await page.screenshot({path:path.join(root,'.scratch/hud-settings-640x520.png')});
    console.log('PASS HUD 360x204 · settings 640x520 · opacity 35/48/72/95 · no overlap or overflow');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
