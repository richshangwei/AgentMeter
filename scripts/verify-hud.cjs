const {chromium}=require('playwright');
const assert=require('node:assert/strict');
const fs=require('node:fs');
const path=require('node:path');
const root=path.resolve(__dirname,'..');
const names=['codex','claude','copilot','antigravity'];
const snapshot={provider_states:[
  {provider:'codex',quota_windows:[
    {limit_id:'codex_bengalfox',window_duration_mins:300,remaining_percent:100},
    {limit_id:'base_model_inference',window_duration_mins:10080,remaining_percent:98},
    {limit_id:'codex',window_duration_mins:10080,remaining_percent:92}
  ]},
  {provider:'claude',quota_windows:[{remaining_percent:77}]},
  {provider:'copilot',quota_windows:[{remaining_percent:24.7}]},
  {provider:'antigravity',quota_windows:[{remaining_percent:54}]}
]};

(async()=>{
  const channel=process.env.AGENTMETER_BROWSER_CHANNEL;
  const browser=await chromium.launch({headless:true,...(channel?{channel}:{})});
  try{
    const page=await browser.newPage({viewport:{width:240,height:132}});
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
        window.hudCalls.push({command,...args});
        if(command==='hud_monitors')return {monitors:Array.from({length:7},(_,i)=>({id:'monitor-'+i,label:'螢幕 '+(i+1)+' · 3840 × 2160',primary:i===0})),selectedMonitor:'monitor-0'};
        return {};
      }}};
    },{snapshot,names});
    await page.goto('http://hud.local/hud.html');
    await page.waitForFunction(()=>document.querySelectorAll('#hud-rows li').length===4);
    const result=await page.evaluate(()=>{
      const box=node=>node.getBoundingClientRect();
      const overlap=(a,b)=>Math.min(a.right,b.right)-Math.max(a.left,b.left)>1&&Math.min(a.bottom,b.bottom)-Math.max(a.top,b.top)>1;
      const alpha=color=>{const values=color.match(/[\d.]+/g).map(Number);return values.length===4?values[3]:1;};
      const rows=[...document.querySelectorAll('#hud-rows li')];
      return {
        text:rows.map(row=>[row.querySelector('.hud-name').textContent,row.querySelector('.hud-value').textContent]),
        overflow:document.documentElement.scrollWidth>innerWidth||document.documentElement.scrollHeight>innerHeight,
        rowOverflow:rows.some(row=>row.scrollWidth>row.clientWidth||row.scrollHeight>row.clientHeight),
        overlap:rows.some(row=>overlap(box(row.querySelector('.hud-name')),box(row.querySelector('.hud-value')))),
        nameSize:parseFloat(getComputedStyle(document.querySelector('.hud-name')).fontSize),
        valueSize:parseFloat(getComputedStyle(document.querySelector('.hud-value')).fontSize),
        nameOpacity:getComputedStyle(document.querySelector('.hud-name')).opacity,
        valueOpacity:getComputedStyle(document.querySelector('.hud-value')).opacity,
        panelAlpha:alpha(getComputedStyle(document.getElementById('hud-panel')).backgroundColor),
        controls:document.querySelectorAll('button,input,a').length,
        call:window.hudCalls.filter(call=>call.command==='configure_hud').at(-1)
      };
    });
    assert.deepEqual(result.text,[['Codex','92%'],['Claude Code','77%'],['GitHub Copilot','24.7%'],['Antigravity','54%']]);
    assert.equal(result.overflow,false);
    assert.equal(result.rowOverflow,false);
    assert.equal(result.overlap,false);
    assert.equal(result.controls,0);
    assert(result.nameSize>=12);
    assert(result.valueSize>=16);
    assert.equal(result.nameOpacity,'1');
    assert.equal(result.valueOpacity,'1');
    assert.equal(result.panelAlpha,0.72);
    assert.equal(result.call.enabled,true);
    assert.equal(result.call.width,240);
    assert.equal(result.call.height,132);
    assert.equal('x' in result.call,false);
    assert.equal('y' in result.call,false);
    await page.locator('#hud-panel').click();
    assert(await page.evaluate(()=>window.hudCalls.some(call=>call.command==='start_hud_drag')));
    const configurations=await page.evaluate(()=>window.hudCalls.filter(call=>call.command==='configure_hud').length);
    await page.evaluate(()=>pollHud());
    assert(await page.evaluate(count=>window.hudCalls.filter(call=>call.command==='configure_hud').length>count,configurations));
    for(const opacity of [35,50,72,100]){
      const actual=await page.evaluate(value=>{localStorage.setItem('agentmeter.hud-opacity.v1',String(value));renderHud();return getComputedStyle(document.documentElement).getPropertyValue('--surface-alpha').trim();},opacity);
      assert.equal(actual,String(opacity/100));
    }
    await page.evaluate(()=>localStorage.setItem('agentmeter.hud-opacity.v1','50'));
    await page.evaluate(()=>renderHud());
    await page.hover('#hud-panel');
    await page.waitForTimeout(220);
    const hover=await page.evaluate(()=>{
      const color=getComputedStyle(document.getElementById('hud-panel')).backgroundColor;
      const values=color.match(/[\d.]+/g).map(Number);
      return {
        panelAlpha:values.length===4?values[3]:1,
        nameOpacity:getComputedStyle(document.querySelector('.hud-name')).opacity,
        valueOpacity:getComputedStyle(document.querySelector('.hud-value')).opacity
      };
    });
    assert.equal(hover.panelAlpha,1);
    assert.equal(hover.nameOpacity,'1');
    assert.equal(hover.valueOpacity,'1');
    fs.mkdirSync(path.join(root,'.scratch'),{recursive:true});
    await page.screenshot({path:path.join(root,'.scratch/hud-240x132.png'),omitBackground:true});
    const positionFailure=await page.evaluate(async()=>{
      const invoke=window.__TAURI__.core.invoke;
      let snapshots=0;
      window.__TAURI__.core.invoke=async(command,args)=>{
        if(command==='configure_hud')throw new Error('position file locked');
        if(command==='snapshot')snapshots++;
        return invoke(command,args);
      };
      await pollHud();
      window.__TAURI__.core.invoke=invoke;
      return {snapshots,title:document.getElementById('hud-panel').title};
    });
    assert.equal(positionFailure.snapshots,1);
    assert.match(positionFailure.title,/數據仍會更新/);
    assert.deepEqual(errors,[]);

    await page.setViewportSize({width:640,height:520});
    await page.goto('http://hud.local/index.html');
    await page.waitForSelector('#app-settings');
    await page.click('#app-settings');
    await page.waitForFunction(()=>document.querySelectorAll('#hud-monitor option').length===7);
    await page.selectOption('#hud-monitor','monitor-5');
    await page.waitForFunction(()=>window.hudCalls.some(call=>call.command==='move_hud_monitor'&&call.monitorId==='monitor-5'));
    await page.locator('#hud-position-reset').click();
    assert(await page.evaluate(()=>window.hudCalls.some(call=>call.command==='reset_hud_position')));
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
        sliderMax:input.max,
        horizontalFit:box.left>=0&&box.right<=innerWidth,
        call:window.hudCalls.filter(call=>call.command==='configure_hud').at(-1),
        controlsFit:[...section.querySelectorAll('select,button,input')].every(node=>{const bounds=node.getBoundingClientRect();return bounds.left>=box.left&&bounds.right<=box.right;})
      };
    });
    assert.equal(settings.enabled,'true');
    assert.equal(settings.opacity,'48');
    assert.equal(settings.output,'48%');
    assert.equal(settings.sliderMax,'100');
    assert(settings.sliderHeight>=44);
    assert.equal(settings.horizontalFit,true);
    assert.equal(settings.controlsFit,true);
    assert.equal(settings.call.enabled,true);
    assert.deepEqual(errors,[]);
    await page.screenshot({path:path.join(root,'.scratch/hud-settings-640x520.png')});
    console.log('PASS HUD 240x132 · settings 640x520 · opacity 35/48/72/100 · opaque text · hover background 100% · multi-monitor selection/drag/reset · no overlap or overflow');
  }finally{await browser.close();}
})().catch(error=>{console.error(error);process.exitCode=1;});
