// Loaded synchronously in <head> so the saved theme applies before first paint.
(() => {
  const key = 'agentmeter.theme.v1';
  const themes = ['dark', 'light', 'aurora'];
  const root = document.documentElement;
  const normalize = value => (themes.includes(value) ? value : 'dark');
  const apply = theme => {
    root.dataset.theme = normalize(theme);
  };
  let saved = 'dark';
  try { saved = normalize(window.localStorage.getItem(key)); } catch { /* storage unavailable: keep default */ }
  apply(saved);

  document.addEventListener('DOMContentLoaded', () => {
    const select = document.getElementById('theme-select');
    const status = document.getElementById('theme-status');
    if (!select) return;
    select.value = root.dataset.theme;
    select.addEventListener('change', () => {
      apply(select.value);
      try {
        window.localStorage.setItem(key, root.dataset.theme);
        if (status) status.textContent = '';
      } catch {
        if (status) status.textContent = '無法儲存設定，本次仍會套用。';
      }
    });
  });
})();
