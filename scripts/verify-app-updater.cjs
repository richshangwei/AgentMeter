// NODE_PATH must resolve Playwright; no real update server or installer is used.
const { chromium } = require('playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const root = path.resolve(__dirname, '..');

(async () => {
  const browser = await chromium.launch({ headless: true, channel: process.env.AGENTMETER_BROWSER_CHANNEL || 'msedge' });
  try {
    for (const [width,height] of [[1080,640],[640,520],[375,844]]) {
      const page = await browser.newPage({ viewport: { width,height }, reducedMotion: 'reduce' });
      const errors = []; page.on('pageerror', error => errors.push(error.message));
      await page.route('http://updater.local/**', route => {
        const name = new URL(route.request().url()).pathname.slice(1) || 'index.html';
        const file = path.join(root, 'desktop-p0/ui', name);
        const contentType = name.endsWith('.js') ? 'text/javascript' : name.endsWith('.css') ? 'text/css' : name.endsWith('.png') ? 'image/png' : 'text/html';
        return fs.existsSync(file) ? route.fulfill({ contentType, body: fs.readFileSync(file) }) : route.fulfill({ status:404 });
      });
      await page.addInitScript(() => {
        window.updateCalls = [];
        window.updateState = { configured:true, status:'idle', current_version:'0.1.0', can_download:false, can_install:false };
        window.__TAURI__ = { core: { invoke: async command => {
          if (['snapshot','refresh_quota'].includes(command)) return { provider_states:[] };
          if (!['update_status','check_update','download_update','install_update'].includes(command)) return {};
          window.updateCalls.push(command);
          if (command === 'check_update') window.updateState = { ...window.updateState, status:'available', version:'0.2.0', can_download:true };
          if (command === 'download_update') {
            window.updateState = { ...window.updateState, status:'downloading', downloaded_bytes:50, total_bytes:100 };
            return new Promise(resolve => { window.finishUpdateDownload = () => {
              window.updateState = { ...window.updateState, status:'ready', can_download:false, can_install:true };
              resolve(window.updateState);
            }; });
          }
          if (command === 'install_update') window.updateState = { ...window.updateState, status:'installing' };
          return window.updateState;
        } } };
      });
      await page.goto('http://updater.local/');
      await page.waitForFunction(() => document.getElementById('update-status').textContent.includes('發現新版'));
      assert.equal(await page.locator('#app-settings').getAttribute('aria-label'), '設定，有新版可更新');
      await page.locator('#app-settings').click();
      await page.locator('#download-update').click();
      await page.waitForFunction(() => document.getElementById('update-progress').value === 50);
      assert(await page.locator('#check-update').isDisabled());
      assert(await page.locator('#download-update').isDisabled());
      await page.evaluate(() => window.finishUpdateDownload());
      await page.waitForFunction(() => document.getElementById('update-status').textContent.includes('通過簽章驗證'));
      page.once('dialog', dialog => dialog.dismiss());
      await page.locator('#install-update').click();
      assert(!await page.evaluate(() => window.updateCalls.includes('install_update')));
      page.once('dialog', dialog => dialog.accept());
      await page.locator('#install-update').click();
      await page.waitForFunction(() => window.updateCalls.includes('install_update'));
      await page.locator('#auto-app-check').uncheck();
      await page.reload();
      await page.waitForFunction(() => window.updateCalls.includes('update_status'));
      assert(!await page.evaluate(() => window.updateCalls.includes('check_update')));
      await page.locator('#app-settings').click();
      assert(!await page.locator('#auto-app-check').isChecked());
      await page.locator('#auto-app-download').check();
      await page.locator('#check-update').click();
      await page.waitForFunction(() => window.updateCalls.includes('download_update'));
      await page.evaluate(() => window.finishUpdateDownload());
      await page.waitForFunction(() => !document.getElementById('install-update').hidden);
      assert(!await page.evaluate(() => window.updateCalls.includes('install_update')));
      await page.locator('#install-update').scrollIntoViewIfNeeded();
      assert(await page.locator('#install-update').evaluate(node => {
        const rect = node.getBoundingClientRect(); return rect.left >= 0 && rect.right <= innerWidth && rect.top >= 0 && rect.bottom <= innerHeight;
      }), 'update action clipped at small size');
      assert(await page.locator('#app-settings-dialog').evaluate(node => node.scrollWidth <= node.clientWidth + 1), 'dialog horizontal overflow');
      assert.deepEqual(errors, []);
      if (width === 640) await page.screenshot({ path:path.join(root,'.scratch/application-update-ready.png') });
      console.log(`PASS updater ${width}x${height}: check, progress, cancel/confirm, preferences, auto-download, reachable actions`);
      await page.close();
    }
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
