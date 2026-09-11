const { test } = require('node:test');
const assert = require('node:assert/strict');
const { createAppUpdater, preferenceKey } = require('../desktop-p0/ui/app-updater.js');

function harness(saved = null) {
  const calls = [], timers = new Map(), polls = new Map(), changes = [];
  let sequence = 0;
  const disk = new Map(saved ? [[preferenceKey, JSON.stringify(saved)]] : []);
  const current = { configured: true, status: 'current', current_version: '0.1.0', can_download: false, can_install: false };
  const replies = { update_status: current, check_update: current,
    download_update: { ...current, status: 'ready', can_install: true }, install_update: { ...current, status: 'installing' } };
  const controller = createAppUpdater({
    invoke: async command => { calls.push(command); const reply = replies[command]; return typeof reply === 'function' ? reply() : reply; },
    storage: { getItem: key => disk.get(key), setItem: (key, value) => disk.set(key, value) },
    setTimer: (fn, delay) => { const id = ++sequence; timers.set(id, { fn, delay }); return id; }, clearTimer: id => timers.delete(id),
    setPoll: fn => { const id = ++sequence; polls.set(id, fn); return id; }, clearPoll: id => polls.delete(id),
    onChange: state => changes.push(state),
  });
  return { controller, calls, replies, timers, polls, changes, disk, current };
}
const available = { configured: true, status: 'available', current_version: '0.1.0', version: '0.2.0', can_download: true, can_install: false };

test('startup checks once and schedules six-hour checks without automatic installation', async () => {
  const h = harness(); await h.controller.start(); await h.controller.start();
  assert.deepEqual(h.calls, ['update_status','check_update']);
  assert.equal([...h.timers.values()][0].delay, 6 * 60 * 60 * 1000);
  assert.equal(h.polls.size, 0);
});
test('disabled automatic checks persist, but manual checks remain available', async () => {
  const h = harness({ autoCheck: false }); await h.controller.start();
  assert.deepEqual(h.calls, ['update_status']); assert.equal(h.timers.size, 0);
  await h.controller.check(); assert.equal(h.calls.at(-1), 'check_update');
  h.controller.setPreferences({ autoCheck: true });
  assert.equal([...h.timers.values()][0].delay, 0);
  assert.equal(JSON.parse(h.disk.get(preferenceKey)).autoCheck, true);
  h.controller.setPreferences({ autoCheck: false }); assert.equal(h.timers.size, 0);
});
test('automatic download stops at verified-ready and install requires explicit confirmation', async () => {
  const h = harness({ autoDownload: true }); h.replies.check_update = available;
  await h.controller.start();
  assert(h.calls.includes('download_update')); assert(!h.calls.includes('install_update'));
  assert.equal(h.controller.snapshot().status, 'ready');
  assert.equal(await h.controller.install(), false);
  await h.controller.install(true); assert.equal(h.calls.at(-1), 'install_update');
});
test('unconfigured builds never download or install, even with saved automatic download', async () => {
  const h = harness({ autoDownload: true });
  h.replies.update_status = h.replies.check_update = { configured: false, status: 'not_configured' };
  await h.controller.start(); await h.controller.download(); await h.controller.install(true);
  assert(!h.calls.includes('download_update')); assert(!h.calls.includes('install_update'));
});
test('overlapping commands are rejected and progress is polled without duplicate jobs', async () => {
  const h = harness(); h.replies.check_update = available; await h.controller.start();
  let finish;
  h.replies.download_update = () => new Promise(resolve => { finish = resolve; });
  const downloading = h.controller.download();
  await h.controller.check(); await h.controller.download(); await h.controller.install(true);
  assert.equal(h.calls.filter(command => command === 'download_update').length, 1);
  assert.equal(h.controller.snapshot().busy, true);
  h.replies.update_status = { ...available, status: 'downloading', downloaded_bytes: 50, total_bytes: 100 };
  await h.controller.sync(); assert.equal(h.controller.snapshot().downloaded_bytes, 50);
  finish({ ...available, status: 'ready', can_download: false, can_install: true }); await downloading;
  assert.equal(h.polls.size, 0); assert.equal(h.controller.snapshot().busy, false);
});
test('failed checks back off for 30 minutes and failed downloads remain retryable', async () => {
  const h = harness(); h.replies.check_update = () => { throw new Error('network'); };
  await h.controller.start(); assert.equal([...h.timers.values()][0].delay, 30 * 60 * 1000);
  h.replies.check_update = available; await h.controller.check();
  h.replies.update_status = { ...available, status: 'error', error: 'update_download_or_signature_failed' };
  h.replies.download_update = () => { throw 'update_download_or_signature_failed'; };
  assert.equal(await h.controller.download(), false);
  assert.equal(h.controller.snapshot().can_download, true);
  assert.equal(h.controller.snapshot().error, 'update_download_or_signature_failed');
  assert(!h.calls.includes('install_update'));
});
test('dispose cancels timers and storage errors do not break monitoring', async () => {
  const h = harness(); await h.controller.start(); h.controller.dispose();
  assert.equal(h.timers.size, 0); assert.equal(h.polls.size, 0);
  const controller = createAppUpdater({ invoke: async () => ({}), storage: { getItem() { throw Error(); }, setItem() { throw Error(); } } });
  controller.setPreferences({ autoCheck: false }); assert.equal(controller.snapshot().preferenceSaved, false);
  controller.dispose();
});

test('late progress cannot replace verified-ready state', async () => {
  const h = harness(); h.replies.check_update = available; await h.controller.start();
  let finishDownload, finishPoll;
  h.replies.download_update = () => new Promise(resolve => { finishDownload = resolve; });
  const downloading = h.controller.download();
  h.replies.update_status = () => new Promise(resolve => { finishPoll = resolve; });
  const polling = h.controller.sync();
  finishDownload({ ...available, status: 'ready', can_download: false, can_install: true });
  await downloading;
  finishPoll({ ...available, status: 'downloading', downloaded_bytes: 10 });
  await polling;
  assert.equal(h.controller.snapshot().status, 'ready');
  assert.equal(h.controller.snapshot().can_install, true);
});

test('scheduled checks defer while downloading instead of losing the recurring timer', async () => {
  const h = harness(); h.replies.check_update = available; await h.controller.start();
  let finish;
  h.replies.download_update = () => new Promise(resolve => { finish = resolve; });
  const downloading = h.controller.download();
  assert.equal(await h.controller.check(true), false);
  assert.equal([...h.timers.values()][0].delay, 60000);
  finish({ ...available, status: 'ready', can_download: false, can_install: true });
  await downloading;
  assert(!h.calls.includes('install_update'));
});
