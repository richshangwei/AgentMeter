// Fit-to-viewport gate for the desktop dashboard: no scrollbars, nothing clipped.
// Run with NODE_PATH pointing to a runtime that provides Playwright.
// Optional: AGENTMETER_BROWSER_CHANNEL=msedge (Windows) — defaults to bundled Chromium.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const names = ['codex', 'claude', 'copilot', 'antigravity'];
const shots = path.join(root, '.scratch');
const snapshot = { provider_states: names.map((provider, index) => ({
  provider, collection_state: 'ready', freshness: 'fresh', collected_at: 1789092648000,
  quota_windows: Array.from({ length: [8, 2, 1, 4][index] }, (_, n) => ({
    ...(provider === 'codex' ? n === 0
      ? {limit_id:'codex',window:'primary',window_duration_mins:10080}
      : {limit_id:n > 4 ? 'base_model_inference':'codex_bengalfox',limit_name:n > 4 ? 'gpt-reserve':'GPT-5.3-Codex-Spark',window:n % 2 ? 'primary':'secondary',window_duration_mins:n % 2 ? 300:10080}
      : {label:n ? `window ${n}`:['five_hour','premium_interactions','Gemini Models Weekly Limit Remaining'][index - 1]}),
    remaining_percent: n === 3 ? 100 : n ? 100 - n * 11.3 : [92, 77, 24.7, 54][index],
    reset_display: '2026/9/18 上午7:56:04 (Etc/GMT-8)', used: 1130, entitlement: 1500,
  })),
})) };
const edge = { provider_states: [
  { provider: 'codex', collection_state: 'ready', quota_windows: [], source_usage: [{ label: '模型使用量', used: 123456789, unit: 'tokens' }] },
  { provider: 'claude', collection_state: 'error', freshness: 'stale', collected_at: 1789092648000, failure_code: 'workspace_trust_required', quota_windows: [{ label: '目前工作階段', remaining_percent: null }] },
  { provider: 'copilot', collection_state: 'error', failure_code: 'cli_not_found', quota_windows: [] },
  { provider: 'antigravity', collection_state: 'error', failure_code: 'authentication_required', quota_windows: [] },
] };
const historySnapshot={
  schema_version:'agentmeter.provider-history.v1',provider:'codex',status:'supported',data_quality:'local_observed',
  files_scanned:2,files_skipped:0,duplicate_sessions:1,records_rejected:1,partial_records:1,limited:true,
  totals:{input_tokens:'12000',cached_input_tokens:'9000',cache_write_input_tokens:null,output_tokens:'500',reasoning_output_tokens:'125',total_tokens:'12500'},
  analysis:{schema_version:'agentmeter.token-analysis.v1',provider:'codex',pricing_basis:'current_api_equivalent',price_as_of:'2026-09-12',price_sources:['https://developers.openai.com/api/docs/models/gpt-5.3-codex'],flow:{uncached_input_tokens:'3000',cached_input_tokens:'9000',cache_write_input_tokens:'0',output_tokens:'500',unclassified_tokens:'0'},cache_reuse_percent:'75.0',output_input_percent:'4.2',cost:{state:'complete',estimated_usd_min:'0.020000',estimated_usd_max:'0.020000',priced_tokens:'12500',observed_tokens:'12500',coverage_percent:'100.0'},sessions:[{session_ref:'session-1',total_tokens:'2100',share_percent:'16.8',cost:{state:'complete',estimated_usd_min:'0.004000',estimated_usd_max:'0.004000',priced_tokens:'2100',observed_tokens:'2100',coverage_percent:'100.0'}},{session_ref:'session-2',total_tokens:'10400',share_percent:'83.2',cost:{state:'partial',estimated_usd_min:'0.016000',estimated_usd_max:'0.017000',priced_tokens:'9000',observed_tokens:'10400',coverage_percent:'86.5'}}],signals:[{kind:'input_heavy',severity:'review',title:'輸入很多、輸出較少',evidence:'輸出約為輸入的 4.2%；這只是檢查線索。',action:'檢查是否重送大型檔案。'}]},
  sessions:[
    {session_ref:'session-1',started_at:'2026-09-12T01:00:00Z',ended_at:'2026-09-12T01:10:00Z',duration_ms:600000,turn_count:1,totals:{input_tokens:'2000',cached_input_tokens:'1000',cache_write_input_tokens:'0',output_tokens:'100',reasoning_output_tokens:'20',total_tokens:'2100'},turns:[{turn_ref:'turn-1',started_at:'2026-09-12T01:00:00Z',ended_at:'2026-09-12T01:10:00Z',duration_ms:600000,model:'gpt-fixture-a',tokens:{input_tokens:'2000',cached_input_tokens:'1000',cache_write_input_tokens:'0',output_tokens:'100',reasoning_output_tokens:'20',total_tokens:'2100'}}]},
    {session_ref:'session-2',started_at:'2026-09-12T02:00:00Z',ended_at:'2026-09-12T02:20:00Z',duration_ms:null,turn_count:1,totals:{input_tokens:'10000',cached_input_tokens:'8000',cache_write_input_tokens:null,output_tokens:'400',reasoning_output_tokens:'105',total_tokens:'10400'},turns:[{turn_ref:'turn-1',started_at:'2026-09-12T02:00:00Z',ended_at:'2026-09-12T02:20:00Z',duration_ms:null,model:'gpt-fixture-b',tokens:{input_tokens:'10000',cached_input_tokens:'8000',cache_write_input_tokens:null,output_tokens:'400',reasoning_output_tokens:'105',total_tokens:'10400'}}],prompt:'PRIVATE_PROMPT_SENTINEL'}
  ]
};
const contentSnapshot={schema_version:'agentmeter.provider-session-content.v1',provider:'codex',session_ref:'session-2',limited:false,duplicate_prompt_count:1,prompts:[{turn_ref:'turn-1',text:'很長的本機提示 '.repeat(80)+' <img onerror=PRIVATE_XSS_SENTINEL>'}]};

function inspect() {
  const errors = [], stats = { truncated: 0, variants: {}, minFont: 99 };
  const box = node => node.getBoundingClientRect();
  const inside = (a, b, t = 1) => a.left >= b.left - t && a.top >= b.top - t && a.right <= b.right + t && a.bottom <= b.bottom + t;
  const overlap = (a, b) => Math.min(a.right, b.right) - Math.max(a.left, b.left) > 1 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 1;
  const shown = node => node.getClientRects().length && getComputedStyle(node).visibility !== 'hidden';
  const doc = document.documentElement;
  if (doc.scrollWidth > innerWidth + 1 || doc.scrollHeight > innerHeight + 1) errors.push(`document overflow ${doc.scrollWidth}x${doc.scrollHeight}`);
  for (const node of document.querySelectorAll('.app-shell, .cards, .monitor-card:not([hidden]), .monitor-card:not([hidden]) .quota')) {
    if (node.scrollHeight > node.clientHeight + 1 || node.scrollWidth > node.clientWidth + 1) errors.push(`${node.id || node.className} content overflow ${node.scrollWidth}x${node.scrollHeight} in ${node.clientWidth}x${node.clientHeight}`);
    const style = getComputedStyle(node);
    if (/(auto|scroll)/.test(style.overflowY + style.overflowX)) errors.push(`${node.id || node.className} is scrollable (${style.overflowX}/${style.overflowY})`);
  }
  const cardsBox = box(document.querySelector('.cards'));
  for (const card of document.querySelectorAll('.monitor-card:not([hidden])')) {
    const outer = box(card), id = card.id;
    if (!inside(outer, cardsBox)) errors.push(`${id} outside card grid`);
    const style = getComputedStyle(card), pad = parseFloat(style.paddingTop);
    const content = { left: outer.left + pad, top: outer.top + pad, right: outer.right - pad, bottom: outer.bottom - pad };
    const lane = card.querySelector('.run-lane');
    const parts = [card.querySelector('.heading'), card.querySelector('.quota'), card.querySelector('.card-foot'), ...(lane && shown(lane) ? [lane] : [])];
    if (lane && shown(lane) && lane.scrollHeight > lane.clientHeight + 1) errors.push(`${id} lane content overflow`);
    for (const part of parts) if (!inside(box(part), content, 1.5)) errors.push(`${id} ${part.className} clipped`);
    for (let a = 0; a < parts.length; a++) for (let b = a + 1; b < parts.length; b++) if (overlap(box(parts[a]), box(parts[b]))) errors.push(`${id} ${parts[a].className}/${parts[b].className} overlap`);
    const quota = parts[1], q = box(quota);
    if (q.height < 28) errors.push(`${id} quota region collapsed to ${q.height}px`);
    stats.variants[quota.dataset.variant] = (stats.variants[quota.dataset.variant] || 0) + 1;
    const tiles = [...quota.querySelectorAll('.window')];
    tiles.forEach((tile, n) => {
      const t = box(tile);
      if (!shown(tile)) errors.push(`${id} tile ${n} hidden`);
      if (!inside(t, q)) errors.push(`${id} tile ${n} outside quota region`);
      tiles.slice(n + 1).forEach((other, m) => { if (overlap(t, box(other))) errors.push(`${id} tiles ${n}/${n + m + 1} overlap`); });
      const kids = [...tile.children].filter(node => shown(node) && !node.classList.contains('habitat'));
      const habitat = tile.querySelector('.habitat');
      if (habitat && shown(habitat)) {
        const wheel = box(habitat.querySelector('.wheel-back')), texts = kids.filter(kid => /q-(label|reset|used)/.test(kid.className)), value = tile.querySelector('.q-value');
        const inWheel = ['ring', 'stack'].includes(quota.dataset.variant);
        stats.wheels = (stats.wheels || 0) + 1;
        if (!inside(wheel, t, .5)) errors.push(`${id} tile ${n} wheel outside tile`);
        for (const kid of inWheel ? texts : [...texts, value]) if (overlap(box(kid), wheel)) errors.push(`${id} tile ${n} ${kid.className} over wheel`);
        if (inWheel && !inside(box(value), wheel)) errors.push(`${id} tile ${n} value outside wheel`);
        const before = [habitat.dataset.act, habitat.style.getPropertyValue('--x')];
        for (const [act, x] of [['run', 0], ['rest', 0], ['rest', 1]]) {
          habitat.dataset.act = act; habitat.style.setProperty('--x', x);
          const ham = box(habitat.querySelector('.ham-spot'));
          if (!inside(ham, t, 1)) errors.push(`${id} tile ${n} hamster ${act}${x} outside tile`);
          for (const kid of act === 'run' ? [value, ...texts] : texts) if (overlap(box(kid), ham)) errors.push(`${id} tile ${n} hamster ${act}${x} over ${kid.className}`);
        }
        habitat.dataset.act = before[0]; habitat.style.setProperty('--x', before[1]);
      }
      for (const kid of kids) {
        if (!inside(box(kid), t, .5)) errors.push(`${id} tile ${n} ${kid.className} clipped`);
        const font = parseFloat(getComputedStyle(kid).fontSize); if (kid.textContent.trim()) stats.minFont = Math.min(stats.minFont, font);
        if (kid.scrollWidth > kid.clientWidth + 1) { if (kid.classList.contains('q-value')) errors.push(`${id} tile ${n} value truncated`); else stats.truncated++; }
      }
      for (let a = 0; a < kids.length; a++) for (let b = a + 1; b < kids.length; b++) if (overlap(box(kids[a]), box(kids[b]))) errors.push(`${id} tile ${n} ${kids[a].className}/${kids[b].className} overlap`);
      const ring = getComputedStyle(tile, '::before');
      if (ring.content !== 'none' && ring.display !== 'none' && !tile.classList.contains('usage-row')) {
        const size = parseFloat(ring.width), value = tile.querySelector('.q-value');
        if (size > t.height + 1 || size > t.width + 1) errors.push(`${id} tile ${n} ring ${size}px larger than tile`);
        const range = document.createRange(); range.selectNodeContents(value.firstChild);
        const text = range.getBoundingClientRect(), v = box(value);
        if (text.width > size * .69 * .94) errors.push(`${id} tile ${n} percentage wider than ring opening`);
        const cx = v.left + v.width / 2, cy = v.top + v.height / 2;
        tile.querySelectorAll('.q-label,.q-reset,.q-used').forEach(node => { if (shown(node) && overlap(box(node), { left: cx - size / 2, right: cx + size / 2, top: cy - size / 2, bottom: cy + size / 2 })) errors.push(`${id} tile ${n} ${node.className} overlaps ring`); });
      }
    });
    const empty = quota.querySelector('.empty-state');
    if (empty) for (const kid of [empty, ...empty.children]) if (shown(kid) && !kid.classList.contains('sr-only') && !inside(box(kid), q)) errors.push(`${id} empty-state clipped`);
    const buttons = [...card.querySelectorAll('button')].filter(shown);
    if (!buttons.length) errors.push(`${id} actions hidden`);
    for (const node of [...buttons, ...card.querySelectorAll('.hint, .heading h2, .heading > span')].filter(shown)) {
      if (!inside(box(node), outer, .5)) errors.push(`${id} ${node.id || node.className || node.tagName} outside card`);
      if (node.tagName === 'BUTTON' && node.scrollWidth > node.clientWidth + 1) errors.push(`${id} button label clipped`);
    }
    const message = card.querySelector('.card-message');
    if (message.textContent && shown(message) && message.title !== message.textContent) errors.push(`${id} clamped message lacks full tooltip`);
  }
  for (const node of document.querySelectorAll('.command-bar button, footer button')) if (shown(node) && !inside(box(node), { left: 0, top: 0, right: innerWidth, bottom: innerHeight })) errors.push(`toolbar ${node.id} outside viewport`);
  return { errors, stats };
}

async function show(page, data, selection, pageIndex = 0) {
  await page.evaluate(({ data, selection, pageIndex }) => { desktopSelection = selection; desktopPage = pageIndex; render(data); }, { data, selection, pageIndex });
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
}

async function verify(page, width, height) {
  await page.setViewportSize({ width, height });
  const notes = [];
  for (const count of [4, 1, 2, 3]) {
    const selection = names.slice(0, count);
    await show(page, snapshot, selection);
    const pages = await page.evaluate(() => Number(document.getElementById('desktop-page').textContent.split('/')[1]));
    const seen = [];
    for (let index = 0; index < pages; index++) {
      await show(page, snapshot, selection, index);
      const { errors, stats } = await page.evaluate(inspect);
      assert.deepEqual(errors, [], `${width}x${height} count=${count} page=${index + 1}`);
      assert(stats.minFont >= 10, `${width}x${height} count=${count}: ${stats.minFont}px text`);
      seen.push(...await page.locator('.monitor-card:not([hidden])').evaluateAll(cards => cards.map(card => card.dataset.providerCard)));
      if (index === 0) notes.push(`${count}:${Object.entries(stats.variants).map(([k, v]) => `${k}×${v}`).join('/')}${pages > 1 ? `(${pages}p)` : ''}${stats.truncated ? ` ${stats.truncated}…` : ''}`);
    }
    assert.deepEqual(seen.sort(), [...selection].sort(), `${width}x${height} count=${count}: pager must reach every monitor exactly once`);
    if (pages > 1) assert(await page.locator('#desktop-pager').isVisible(), 'pager hidden while monitors are paged');
  }
  // Polling with unchanged data must keep nodes; changed data must re-render in place.
  await show(page, snapshot, names);
  await page.locator('#codex-quota .window').first().evaluate(node => { window.previousQuotaRow = node; });
  await page.evaluate(data => render(data), snapshot);
  assert(await page.evaluate(() => previousQuotaRow === document.querySelector('#codex-quota .window')), 'unchanged polling replaces quota nodes');
  const pageSize = await page.locator('.monitor-card:not([hidden])').count();
  for (let index = 0; index < Math.ceil(4 / pageSize); index++) {
    await show(page, edge, names, index);
    const { errors } = await page.evaluate(inspect);
    assert.deepEqual(errors, [], `${width}x${height} edge states page=${index + 1}`);
  }
  return notes.join('  ');
}

async function verifyHistory(page,width,height){
  await page.setViewportSize({width,height});
  await page.evaluate(()=>{window.historyContentCalls=0;});
  await page.click('#history-open');
  await page.waitForFunction(()=>document.getElementById('history-status').textContent.includes('已讀取'));
  const result=await page.evaluate(()=>{
    const dialog=document.getElementById('history-dialog'),box=dialog.getBoundingClientRect();
    const text=dialog.textContent;
    return {
      open:dialog.open,
      horizontalOverflow:dialog.scrollWidth>dialog.clientWidth+1,
      insideViewport:box.left>=0&&box.top>=0&&box.right<=innerWidth+1&&box.bottom<=innerHeight+1,
      summaryCount:document.querySelectorAll('#history-summary .history-metric').length,
      kpiCount:document.querySelectorAll('#history-kpis .history-kpi').length,
      flowParts:document.querySelectorAll('#history-flow .history-flow-part').length,
      costShown:document.getElementById('history-kpis').textContent.includes('US$0.020000'),
      sessionCount:document.querySelectorAll('#history-sessions .history-session').length,
      highestImpactFirst:(document.querySelector('#history-sessions .history-session summary span')?.textContent||'').includes('10,400 Token'),
      incomplete:document.getElementById('history-status').textContent.includes('資料有截斷或未完整解析'),
      leaked:text.includes('PRIVATE_PROMPT_SENTINEL'),
      visibleClose:document.getElementById('history-close').getBoundingClientRect().width>0
    };
  });
  assert.deepEqual(result,{open:true,horizontalOverflow:false,insideViewport:true,summaryCount:6,kpiCount:4,flowParts:3,costShown:true,sessionCount:2,highestImpactFirst:true,incomplete:true,leaked:false,visibleClose:true},`${width}x${height} history dialog`);
  assert.equal(await page.evaluate(()=>window.historyContentCalls),0,'history open must not pre-read prompts');
  await page.locator('.history-content-open').first().click();
  assert.equal(await page.evaluate(()=>window.historyContentCalls),0,'prompt consent gate must not pre-read prompts');
  await page.check('#history-content-consent');
  await page.click('#history-content-load');
  await page.waitForFunction(()=>document.getElementById('history-content-status').textContent.includes('已顯示'));
  const content=await page.evaluate(()=>{
    const dialog=document.getElementById('history-content-dialog'),box=dialog.getBoundingClientRect();
    return {open:dialog.open,horizontalOverflow:dialog.scrollWidth>dialog.clientWidth+1,insideViewport:box.left>=0&&box.top>=0&&box.right<=innerWidth+1&&box.bottom<=innerHeight+1,
      calls:window.historyContentCalls,imgCount:dialog.querySelectorAll('img').length,sentinel:dialog.textContent.includes('PRIVATE_XSS_SENTINEL')};
  });
  assert.deepEqual(content,{open:true,horizontalOverflow:false,insideViewport:true,calls:1,imgCount:0,sentinel:true},`${width}x${height} prompt dialog`);
  await page.click('#history-content-close');
  assert.equal(await page.locator('#history-content-body').evaluate(node=>node.textContent.includes('PRIVATE_XSS_SENTINEL')),false,'closing prompt must clear content');
  await page.click('#history-close');
  assert.equal(await page.locator('#history-dialog').evaluate(dialog=>dialog.open),false);
}

(async () => {
  const channel = process.env.AGENTMETER_BROWSER_CHANNEL;
  const browser = await chromium.launch({ headless: true, ...(channel ? { channel } : {}) });
  try {
    const page = await browser.newPage();
    page.on('pageerror', error => { throw error; });
    await page.route('http://responsive.local/**', route => {
      const name = new URL(route.request().url()).pathname.slice(1) || 'index.html';
      const file = path.join(root, 'desktop-p0/ui', name);
      const type = name.endsWith('.css') ? 'text/css' : name.endsWith('.js') ? 'text/javascript' : name.endsWith('.png') ? 'image/png' : 'text/html';
      return fs.existsSync(file) ? route.fulfill({ contentType: type, body: fs.readFileSync(file) }) : route.fulfill({ status: 404 });
    });
    await page.addInitScript(({dashboard,history,content}) => { window.historyContentCalls=0;window.__TAURI__ = { core: { invoke: async command => ['snapshot', 'refresh_quota'].includes(command) ? dashboard : command === 'history_sessions' ? history : command === 'history_session_content' ? (window.historyContentCalls++,content) : command === 'check_update' ? { configured: false } : {} } }; }, {dashboard:snapshot,history:historySnapshot,content:contentSnapshot});
    await page.goto('http://responsive.local/');
    await page.waitForLoadState('networkidle');
    const sizes = [[2560,1440],[2491,1312],[2048,1190],[1993,1050],[1920,1080],[1873,1135],[1498,908],[1440,900],[1400,520],[1366,768],[1280,720],[1080,640],[952,1235],[900,560],[800,600],[762,988],[640,900],[640,520]];
    for (const [width, height] of sizes) console.log(`PASS ${width}x${height}  ${await verify(page, width, height)}`);
    // Depletion lanes take extra height in every theme; pets adds the runner track.
    await page.evaluate(() => { providerForecast = (samples, state, now) => state?.collection_state === 'error' ? {primary:null,windows:[],rate:0} : (windows => ({primary:windows[0]||null,windows,rate:38}))((state?.quota_windows||[]).map(item => ({item,state:'running',rate:38,etaMs:2100000,exhaustAt:now+2100000,resetAt:null,basisMs:2160000}))); });
    for (const theme of ['dark', 'pets', 'hamster']) {
      await page.emulateMedia({ reducedMotion: theme === 'hamster' ? 'reduce' : 'no-preference' });
      await page.evaluate(value => { document.documentElement.dataset.theme = value; }, theme);
      for (const [width, height] of sizes) console.log(`PASS LANE ${theme} ${width}x${height}  ${await verify(page, width, height)}`);
    }
    await page.evaluate(() => { document.documentElement.dataset.theme = 'dark'; });
    for(const [width,height] of [[1080,640],[640,520]]){await verifyHistory(page,width,height);console.log(`PASS HISTORY ${width}x${height}`);}
    if (process.env.AGENTMETER_SCREENSHOTS !== '0') {
      fs.mkdirSync(shots, { recursive: true });
      for (const [width, height, count, data] of [[1920,1080,4,snapshot],[1498,908,4,snapshot],[1080,640,4,snapshot],[1080,640,3,snapshot],[640,520,4,snapshot],[1280,720,4,edge],[2560,1440,2,snapshot]]) {
        await page.setViewportSize({ width, height });
        await show(page, data, names.slice(0, count));
        await page.screenshot({ path: path.join(shots, `fit-${width}x${height}-${count}${data === edge ? '-edge' : ''}.png`) });
      }
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
