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
    const parts = [card.querySelector('.heading'), card.querySelector('.quota'), card.querySelector('.card-foot')];
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
      const kids = [...tile.children].filter(shown);
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
    await page.addInitScript(data => { window.__TAURI__ = { core: { invoke: async command => ['snapshot', 'refresh_quota'].includes(command) ? data : command === 'check_update' ? { configured: false } : {} } }; }, snapshot);
    await page.goto('http://responsive.local/');
    await page.waitForLoadState('networkidle');
    const sizes = [[2560,1440],[2491,1312],[2048,1190],[1993,1050],[1920,1080],[1873,1135],[1498,908],[1440,900],[1400,520],[1366,768],[1280,720],[1080,640],[952,1235],[900,560],[800,600],[762,988],[640,900],[640,520]];
    for (const [width, height] of sizes) console.log(`PASS ${width}x${height}  ${await verify(page, width, height)}`);
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
