const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const code = fs.readFileSync('desktop-p0/ui/resources.js', 'utf8');

const GiB = 1073741824;

function fakeElement(tag = 'div') {
  return {
    tag, hidden: false, checked: false, textContent: '', className: '', dataset: {}, style: {}, children: [],
    addEventListener(_type, fn) { this.change = fn; },
    appendChild(child) { this.children.push(child); return child; },
    append(...nodes) { this.children.push(...nodes); },
    replaceChildren(...nodes) { this.children = nodes; },
  };
}

function setup(responses) {
  const ids = ['resources-panel', 'resources-enabled', 'resources-preference-status', 'resource-disks', 'resources-status',
    'resource-cpu-tile', 'resource-cpu', 'resource-cpu-bar',
    'resource-app-tile', 'resource-app', 'resource-app-detail', 'resource-app-bar', 'resource-memory-tile', 'resource-memory', 'resource-memory-detail', 'resource-memory-bar'];
  const elements = Object.fromEntries(ids.map(id => [id, fakeElement()]));
  const stored = new Map();
  const state = { calls: 0, scheduled: null };
  const window = {
    localStorage: { getItem: key => stored.get(key), setItem: (key, value) => stored.set(key, value) },
    dispatchEvent() {},
    __TAURI__: { core: { invoke: async () => responses[Math.min(state.calls++, responses.length - 1)] } },
  };
  vm.runInNewContext(code, {
    window,
    document: { getElementById: id => elements[id], createElement: tag => fakeElement(tag) },
    Event: class {},
    setTimeout: fn => { state.scheduled = fn; return 1; },
    clearTimeout() {},
  });
  return { elements, stored, state };
}

const diskTiles = elements => elements['resource-disks'].children.map(tile => ({
  label: tile.children[0].textContent,
  value: tile.children[1].textContent,
  detail: tile.children[2].textContent,
  bar: tile.children[3].children[0].style.width,
  level: tile.dataset.level,
}));

test('resources show large percentages with detail, bars and a tile per disk', async () => {
  const { elements } = setup([{
    cpu_percent: 37.5,
    memory: { used: GiB, total: 2 * GiB },
    disk: { used: 2 * GiB, total: 4 * GiB }, disk_path: 'C:',
    disks: [{ path: 'C:', used: 2 * GiB, total: 4 * GiB }, { path: 'D:', used: 95 * GiB, total: 100 * GiB }],
  }]);
  await new Promise(setImmediate);
  assert.equal(elements['resources-panel'].hidden, false);
  assert.equal(elements['resource-cpu'].textContent, '37.5%');
  assert.equal(elements['resource-cpu-bar'].style.width, '37.5%');
  assert.equal(elements['resource-memory'].textContent, '50.0%');
  assert.equal(elements['resource-memory-detail'].textContent, '1.0 / 2.0 GiB');
  assert.deepEqual(diskTiles(elements), [
    { label: '磁碟 C:', value: '50.0%', detail: '2.0 / 4.0 GiB', bar: '50%', level: 'good' },
    { label: '磁碟 D:', value: '95.0%', detail: '95.0 / 100.0 GiB', bar: '95%', level: 'low' },
  ]);
});

test('older backends without a disk list still show the system disk', async () => {
  const { elements } = setup([{ cpu_percent: null, cpu_error: false, memory: null, disk: { used: 2 * GiB, total: 4 * GiB }, disk_path: 'C:' }]);
  await new Promise(setImmediate);
  assert.equal(elements['resource-cpu'].textContent, '取樣中…');
  assert.equal(elements['resource-memory'].textContent, '無法讀取');
  assert.deepEqual(diskTiles(elements).map(tile => [tile.label, tile.value]), [['磁碟 C:', '50.0%']]);
});

test('disk tiles are rebuilt only when the drive set changes', async () => {
  const one = { cpu_percent: 1, memory: null, disks: [{ path: 'C:', used: 1, total: 4 }] };
  const two = { cpu_percent: 1, memory: null, disks: [{ path: 'C:', used: 2, total: 4 }, { path: 'E:', used: 1, total: 2 }] };
  const { elements, state } = setup([one, one, two]);
  await new Promise(setImmediate);
  const first = elements['resource-disks'].children[0];
  state.scheduled();
  await new Promise(setImmediate);
  assert.equal(elements['resource-disks'].children[0], first);
  state.scheduled();
  await new Promise(setImmediate);
  assert.deepEqual(diskTiles(elements).map(tile => tile.label), ['磁碟 C:', '磁碟 E:']);
});

test('disabling the panel persists the preference and stops polling', async () => {
  const { elements, stored, state } = setup([{ cpu_percent: 1, memory: null, disks: [] }]);
  await new Promise(setImmediate);
  elements['resources-enabled'].checked = false;
  elements['resources-enabled'].change();
  assert.equal(elements['resources-panel'].hidden, true);
  assert.equal(stored.get('agentmeter.local-resources-enabled.v1'), 'false');
  state.scheduled();
  await new Promise(setImmediate);
  assert.equal(state.calls, 1); // A callback queued before disabling must not collect again.
});

test('the app tile shows its own CPU and memory, and stays honest without data', async () => {
  const base = { cpu_percent: 10, memory: null, disks: [] };
  const { elements, state } = setup([
    { ...base, app: { cpu_percent: null, memory: 200 * 1048576, processes: 7 } },
    { ...base, app: { cpu_percent: 2.34, memory: 312 * 1048576, processes: 8 } },
    base,
  ]);
  await new Promise(setImmediate);
  assert.equal(elements['resource-app'].textContent, '取樣中…');
  assert.equal(elements['resource-app-detail'].textContent, '記憶體 200 MB');
  state.scheduled();
  await new Promise(setImmediate);
  assert.equal(elements['resource-app'].textContent, '2.3%');
  assert.equal(elements['resource-app-detail'].textContent, '記憶體 312 MB · 8 個程序');
  assert.equal(elements['resource-app-bar'].style.width, '2.34%');
  state.scheduled();
  await new Promise(setImmediate);
  assert.equal(elements['resource-app'].textContent, '無法讀取');
  assert.equal(elements['resource-app-detail'].textContent, '');
});
