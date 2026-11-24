// Loaded synchronously in <head> so the saved theme applies before first paint.
(() => {
  const key = 'agentmeter.theme.v1';
  const themes = ['dark', 'light', 'aurora', 'pets', 'hamster'];
  const ecoKey = 'agentmeter.eco-motion.v1';
  const root = document.documentElement;
  const normalize = value => (themes.includes(value) ? value : 'dark');
  const apply = theme => {
    root.dataset.theme = normalize(theme);
  };
  // The HUD calls sync on every poll so it follows theme changes made in the main window.
  const sync = () => {
    let saved = 'dark', eco = 'false';
    try {
      saved = normalize(window.localStorage.getItem(key));
      eco = String(window.localStorage.getItem(ecoKey) === 'true');
    } catch { /* storage unavailable: keep defaults */ }
    if (root.dataset.theme !== saved) apply(saved);
    if (root.dataset.eco !== eco) root.dataset.eco = eco;
  };
  sync();
  window.agentMeterTheme = { sync };

  document.addEventListener('DOMContentLoaded', () => {
    const select = document.getElementById('theme-select');
    const status = document.getElementById('theme-status');
    if (!select) return;
    const eco = document.getElementById('eco-motion');
    eco.checked = root.dataset.eco === 'true';
    eco.addEventListener('change', () => {
      root.dataset.eco = String(eco.checked);
      try {
        window.localStorage.setItem(ecoKey, root.dataset.eco);
        if (status) status.textContent = '';
      } catch {
        if (status) status.textContent = '無法儲存設定，本次仍會套用。';
      }
    });
    select.value = root.dataset.theme;
    select.addEventListener('change', () => {
      apply(select.value);
      document.dispatchEvent(new Event('agentmeter-theme'));
      try {
        window.localStorage.setItem(key, root.dataset.theme);
        if (status) status.textContent = '';
      } catch {
        if (status) status.textContent = '無法儲存設定，本次仍會套用。';
      }
    });
  });
})();
