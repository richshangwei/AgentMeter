(() => {
  const byId = id => document.getElementById(id);
  const panel = byId('resources-panel');
  const toggle = byId('resources-enabled');
  const preferenceStatus = byId('resources-preference-status');
  const preferenceKey = 'agentmeter.local-resources-enabled.v1';
  let timer = null;
  let generation = 0;
  const capacity = value => {
    if (!value || !Number.isFinite(value.used) || !Number.isFinite(value.total) || value.total <= 0) return '無法讀取';
    const gib = bytes => (bytes / 1073741824).toFixed(1);
    return `${gib(value.used)} / ${gib(value.total)} GiB（${(value.used / value.total * 100).toFixed(1)}%）`;
  };
  async function refresh(current) {
    if (current !== generation || panel.hidden) return;
    try {
      const result = await window.__TAURI__.core.invoke('local_resources');
      if (current !== generation || panel.hidden) return;
      byId('resource-cpu').textContent = Number.isFinite(result.cpu_percent) ? `${result.cpu_percent.toFixed(1)}%` : (result.cpu_error ? '無法讀取' : '取樣中…');
      byId('resource-memory').textContent = capacity(result.memory);
      byId('resource-disk').textContent = capacity(result.disk);
      byId('resource-disk-label').textContent = result.disk_path ? `系統磁碟 ${result.disk_path}` : '系統磁碟';
      byId('resources-status').textContent = '';
    } catch {
      if (current !== generation || panel.hidden) return;
      for (const id of ['resource-cpu', 'resource-memory', 'resource-disk']) byId(id).textContent = '無法讀取';
      byId('resources-status').textContent = '無法取得本機資源，請稍後重試。';
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
