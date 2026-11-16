const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const code = fs.readFileSync('desktop-p0/ui/resources.js', 'utf8');

test('resources appear with provider cards and preference controls polling', async () => {
  const ids = ['resources-panel','resources-enabled','resources-preference-status','resource-cpu','resource-memory','resource-disk','resource-disk-label','resources-status'];
  const elements = Object.fromEntries(ids.map(id => [id, { hidden: false, checked: false, textContent: '', addEventListener(_type, fn) { this.change = fn; } }]));
  const stored = new Map();
  let calls = 0, scheduled;
  const window = { localStorage: { getItem: key => stored.get(key), setItem: (key, value) => stored.set(key, value) }, dispatchEvent() {}, __TAURI__: { core: { invoke: async () => { calls++; return { cpu_percent: 37.5, memory: {used: 1073741824, total: 2147483648}, disk: {used: 2147483648, total: 4294967296}, disk_path: 'C:' }; } } } };
  vm.runInNewContext(code, { window, document: {getElementById: id => elements[id]}, Event: class {}, setTimeout: fn => { scheduled = fn; return 1; }, clearTimeout() {} });
  await new Promise(setImmediate);
  assert.equal(elements['resources-panel'].hidden, false);
  assert.equal(elements['resource-cpu'].textContent, '37.5%');
  assert.equal(elements['resource-memory'].textContent, '1.0 / 2.0 GiB（50.0%）');
  assert.equal(elements['resource-disk'].textContent, '2.0 / 4.0 GiB（50.0%）');
  assert.equal(elements['resource-disk-label'].textContent, '系統磁碟 C:');
  elements['resources-enabled'].checked = false;
  elements['resources-enabled'].change();
  assert.equal(elements['resources-panel'].hidden, true);
  assert.equal(stored.get('agentmeter.local-resources-enabled.v1'), 'false');
  scheduled();
  await new Promise(setImmediate);
  assert.equal(calls, 1); // A callback queued before disabling must not collect again.
});
