(() => {
  const byId = id => document.getElementById(id);
  const panel = byId('resources-panel');
  const toggle = byId('resources-enabled');
  const preferenceStatus = byId('resources-preference-status');
  const disksHost = byId('resource-disks');
  const preferenceKey = 'agentmeter.local-resources-enabled.v1';
  let timer = null;
  let generation = 0;
  let diskTiles = new Map();
  let diskOrder = '';

  const gib = bytes => (bytes / 1073741824).toFixed(1);
  const validCapacity = value => value && Number.isFinite(value.used) && Number.isFinite(value.total) && value.total > 0;
  const level = percent => !Number.isFinite(percent) ? '' : percent >= 90 ? 'low' : percent >= 80 ? 'warning' : 'good';

  // Paints one tile: big percentage, secondary detail line, usage bar and level color.
  function paint(tile, percent, detail) {
    const known = Number.isFinite(percent);
    tile.value.textContent = known ? `${percent.toFixed(1)}%` : detail.fallback;
    if (tile.detail) tile.detail.textContent = known ? detail.text : '';
    tile.bar.style.width = known ? `${Math.min(100, Math.max(0, percent))}%` : '0%';
    tile.root.dataset.level = level(percent);
  }
  function paintCapacity(tile, value) {
    if (!validCapacity(value)) return paint(tile, NaN, { fallback: '無法讀取' });
    paint(tile, value.used / value.total * 100, { text: `${gib(value.used)} / ${gib(value.total)} GiB` });
  }

  const cpuTile = { root: byId('resource-cpu-tile'), value: byId('resource-cpu'), bar: byId('resource-cpu-bar') };
  const memoryTile = { root: byId('resource-memory-tile'), value: byId('resource-memory'), detail: byId('resource-memory-detail'), bar: byId('resource-memory-bar') };

  function createDiskTile(path) {
    const make = (tag, className, text) => {
      const node = document.createElement(tag);
      if (className) node.className = className;
      if (text) node.textContent = text;
      return node;
    };
    const root = make('div', 'resource-tile');
    const value = make('span', 'resource-value', '讀取中…');
    const detail = make('span', 'resource-detail');
    const track = make('span', 'resource-bar');
    const bar = make('i');
    track.appendChild(bar);
    root.append(make('strong', '', `磁碟 ${path}`), value, detail, track);
    return { root, value, detail, bar };
  }

  // Prefer the per-drive list; older backends only report the system drive.
  function disksOf(result) {
    if (Array.isArray(result.disks) && result.disks.length) return result.disks;
    return [{ path: result.disk_path || '系統', used: result.disk?.used, total: result.disk?.total }];
  }

  function renderDisks(disks) {
    const order = disks.map(disk => disk.path).join('|');
    if (order !== diskOrder) {
      diskOrder = order;
      diskTiles = new Map(disks.map(disk => [disk.path, createDiskTile(disk.path)]));
      disksHost.replaceChildren(...[...diskTiles.values()].map(tile => tile.root));
      window.dispatchEvent(new Event('resize'));
    }
    for (const disk of disks) paintCapacity(diskTiles.get(disk.path), disk);
  }

  function showFailure() {
    paint(cpuTile, NaN, { fallback: '無法讀取' });
    paintCapacity(memoryTile, null);
    for (const tile of diskTiles.values()) paintCapacity(tile, null);
    byId('resources-status').textContent = '無法取得本機資源，請稍後重試。';
  }

  async function refresh(current) {
    if (current !== generation || panel.hidden) return;
    try {
      const result = await window.__TAURI__.core.invoke('local_resources');
      if (current !== generation || panel.hidden) return;
      paint(cpuTile, result.cpu_percent, { text: '', fallback: result.cpu_error ? '無法讀取' : '取樣中…' });
      paintCapacity(memoryTile, result.memory);
      renderDisks(disksOf(result));
      byId('resources-status').textContent = '';
    } catch {
      if (current !== generation || panel.hidden) return;
      showFailure();
    }
    if (current === generation && !panel.hidden) timer = setTimeout(() => refresh(current), 2000);
  }
  function apply(enabled) {
    generation++;
    clearTimeout(timer);
    panel.hidden = !enabled;
    if (enabled) refresh(generation);
    window.dispatchEvent(new Event('resize'));
  }
  try { toggle.checked = window.localStorage.getItem(preferenceKey) !== 'false'; } catch { toggle.checked = true; }
  toggle.addEventListener('change', () => {
    try {
      window.localStorage.setItem(preferenceKey, String(toggle.checked));
      preferenceStatus.textContent = '';
    } catch { preferenceStatus.textContent = '無法儲存設定，本次仍會套用。'; }
    apply(toggle.checked);
  });
  apply(toggle.checked);
})();
