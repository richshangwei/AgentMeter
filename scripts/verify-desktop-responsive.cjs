// Run with NODE_PATH pointing to a runtime that provides Playwright.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');
const names = ['codex', 'claude', 'copilot', 'antigravity'];
const snapshot = { provider_states: names.map((provider, index) => ({
  provider, collection_state: 'ready', freshness: 'fresh', collected_at: 1789092648000,
  quota_windows: Array.from({ length: [8, 2, 1, 4][index] }, (_, n) => ({
    label: n ? `基本模型 codex_bengalfox secondary ${n}` : ['Codex 主視窗', '目前工作階段 5 小時', 'Premium interactions', 'Gemini 每週'][index],
    remaining_percent: n ? 100 : [92, 77, 24.7, 54][index],
    reset_display: '2026/9/18 上午7:56:04 (Etc/GMT-8)', used: 1130, entitlement: 1500,
  })),
})) };

async function verify(page, width, height, count) {
  await page.setViewportSize({ width, height });
  await page.evaluate(({ snapshot, names, count }) => {
    desktopSelection = names.slice(0, count); desktopPage = 0; render(snapshot);
    document.querySelectorAll('.quota').forEach(node => { node.scrollTop = 0; });
  }, { snapshot, names, count });
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
  const cards = page.locator('.monitor-card:not([hidden])');
  assert.equal(await cards.count(), count);
  for (let i = 0; i < count; i++) {
    const card = cards.nth(i);
    await card.scrollIntoViewIfNeeded();
    const errors = await card.evaluate(card => {
      const errors = [], quota = card.querySelector('.quota'), primary = quota.firstElementChild;
      const q = quota.getBoundingClientRect(), p = primary.getBoundingClientRect(), outer = card.getBoundingClientRect();
      const css = getComputedStyle(primary), ring = getComputedStyle(primary, '::before');
      const size = parseFloat(ring.width), col = parseFloat(css.gridTemplateColumns);
      const ringTop = p.top + (p.height - size) / 2, ringLeft = p.left + (col - size) / 2;
      if (ringTop < q.top - 1 || ringTop + size > q.bottom + 1 || ringLeft < q.left - 1 || ringLeft + size > q.right + 1) errors.push('primary ring clipped');
      const value = primary.children[1], range = document.createRange(); range.selectNode(value.firstChild);
      if (range.getBoundingClientRect().width > size * .69 * .94) errors.push('percentage exceeds ring opening');
      const label = primary.children[0].getBoundingClientRect(), v = value.getBoundingClientRect();
      if (Math.min(label.right, v.right) > Math.max(label.left, v.left) && Math.min(label.bottom, v.bottom) > Math.max(label.top, v.top)) errors.push('label/value overlap');
      for (const node of card.querySelectorAll('button, .hint')) {
        if (!node.getClientRects().length) continue;
        const r = node.getBoundingClientRect();
        if (r.top < q.bottom - 1 && r.bottom > q.top + 1) errors.push('footer overlaps quota');
        if (r.bottom > outer.bottom - 3 || r.right > outer.right || r.left < outer.left) errors.push('footer clipped by card');
      }
      if (quota.scrollWidth > quota.clientWidth + 1) errors.push('horizontal quota overflow');
      if (![...card.querySelectorAll('button')].some(node => node.getClientRects().length)) errors.push('actions hidden');
      return errors;
    });
    assert.deepEqual(errors, [], `${width}x${height}, count=${count}, ${names[i]}`);
    const rows = card.locator('.window');
    for (let n = 0; n < await rows.count(); n++) {
      const row = rows.nth(n);
      assert(await row.isVisible(), `quota ${n} silently hidden`);
      await row.scrollIntoViewIfNeeded();
      assert(await row.evaluate(node => {
        const r = node.getBoundingClientRect(), q = node.parentElement.getBoundingClientRect();
        return r.top >= q.top - 1 && r.bottom <= q.bottom + 1;
      }), `${width}x${height}: quota ${n} not fully reachable`);
    }
  }
  const scroll = await page.locator('#codex-quota').evaluate(node => node.scrollTop);
  await page.locator('#codex-quota .window').first().evaluate(node => { window.previousQuotaRow = node; });
  await page.evaluate(snapshot => render(snapshot), snapshot);
  assert(await page.evaluate(() => previousQuotaRow === document.querySelector('#codex-quota .window')), 'unchanged polling replaces quota nodes');
  assert(Math.abs(await page.locator('#codex-quota').evaluate(node => node.scrollTop) - scroll) < 2, 'refresh resets reading position');
  const changed = structuredClone(snapshot);
  changed.provider_states[0].quota_windows[0].remaining_percent = 91.9;
  await page.evaluate(snapshot => render(snapshot), changed);
  assert(Math.abs(await page.locator('#codex-quota').evaluate(node => node.scrollTop) - scroll) < 2, 'changed values reset reading position');
  await page.locator('#codex-quota').focus();
  await page.locator('#codex-quota').evaluate(node => { node.scrollTop = 0; });
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(resolve)));
  await page.keyboard.press('PageDown');
  await page.waitForTimeout(300);
  const keyboard = await page.locator('#codex-quota').evaluate(node => ({ top: node.scrollTop, overflow: node.scrollHeight > node.clientHeight, active: document.activeElement?.id }));
  assert(!keyboard.overflow || keyboard.top > 0, `${width}x${height} count=${count}: keyboard cannot scroll quota region ${JSON.stringify(keyboard)}`);
  assert(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth), 'document horizontal overflow');
}

async function verifyEdgeStates(page) {
  const edge = { provider_states: [
    { provider: 'codex', collection_state: 'ready', quota_windows: [], source_usage: [{ label: '模型使用量', used: 123456789, unit: 'tokens' }] },
    { provider: 'claude', collection_state: 'error', freshness: 'stale', failure_code: 'workspace_trust_required', quota_windows: [{ label: '目前工作階段', remaining_percent: null }] },
    { provider: 'copilot', collection_state: 'error', failure_code: 'cli_not_found', quota_windows: [] },
    { provider: 'antigravity', collection_state: 'error', failure_code: 'authentication_required', quota_windows: [] },
  ] };
  await page.evaluate(snapshot => { desktopSelection = ['codex','claude','copilot','antigravity']; render(snapshot); }, edge);
  const errors = await page.locator('.monitor-card:not([hidden])').evaluateAll(cards => cards.flatMap(card => {
    const errors = [], outer = card.getBoundingClientRect();
    for (const node of card.querySelectorAll('button, .hint, .window, .empty-state')) {
      if (!node.getClientRects().length) continue;
      const r = node.getBoundingClientRect();
      if (r.bottom > outer.bottom - 2 || r.left < outer.left || r.right > outer.right) errors.push(`${card.id}: ${node.className} clipped`);
      if (node.scrollWidth > node.clientWidth + 1) errors.push(`${card.id}: ${node.className} text overflow`);
    }
    return errors;
  }));
  assert.deepEqual(errors, [], 'error/empty/usage states must remain reachable');
}

(async () => {
  const browser = await chromium.launch({ headless: true, channel: 'msedge' });
  try {
    const page = await browser.newPage();
    await page.route('http://responsive.local/**', route => {
      const name = new URL(route.request().url()).pathname.slice(1) || 'index.html';
      const file = path.join(root, 'desktop-p0/ui', name);
      const type = name.endsWith('.css') ? 'text/css' : name.endsWith('.js') ? 'text/javascript' : name.endsWith('.png') ? 'image/png' : 'text/html';
      return fs.existsSync(file) ? route.fulfill({ contentType: type, body: fs.readFileSync(file) }) : route.fulfill({ status: 404 });
    });
    await page.addInitScript(snapshot => { window.__TAURI__ = { core: { invoke: async command => ['snapshot', 'refresh_quota'].includes(command) ? snapshot : {} } }; }, snapshot);
    await page.goto('http://responsive.local/');
    for (const [width, height] of [[2048,1190],[1873,1135],[1498,908],[952,1235],[762,988],[1080,640],[640,520],[1440,900],[1280,720],[375,844],[844,390]]) {
      for (const count of [4, 1, 2, 3]) await verify(page, width, height, count);
      await verifyEdgeStates(page);
      if (width === 640) await page.screenshot({ path: path.join(root, '.scratch/responsive-edge-states-640.png') });
      console.log(`PASS ${width}x${height}: 1/2/3/4 monitors, 8 quota rows, fractional values, refresh continuity`);
      if (width === 952 || width === 1498) {
        await verify(page, width, height, 4);
        await page.evaluate(() => { document.querySelectorAll('.quota').forEach(node => { node.scrollTop = 0; }); document.querySelector('.cards').scrollTop = 0; });
        await page.screenshot({ path: path.join(root, `.scratch/responsive-real-data-${width}.png`) });
      }
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
