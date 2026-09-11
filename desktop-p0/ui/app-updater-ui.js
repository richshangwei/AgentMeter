(function () {
  'use strict';
  const node = id => document.getElementById(id);
  const errors = {
    update_download_or_signature_failed: '下載或簽章驗證失敗，未安裝任何檔案。請重試下載；若持續失敗，請等候修正後的正式發布。',
    update_install_failed: '更新解壓縮或安裝準備失敗，已保留驗證完成的更新，請重試。',
    update_state_busy: '另一個更新作業正在進行，請稍候。',
    update_not_downloaded: '請先下載並完成簽章驗證，再安裝新版。',
    update_artifact_url_invalid: '更新檔來源不符合信任設定，已拒絕下載。',
  };
  function renderUpdate(state) {
    const version = state.version || '', current = state.current_version || '—';
    const downloaded = Math.max(0, Number(state.downloaded_bytes) || 0), total = Number(state.total_bytes) || 0;
    const percent = total > 0 ? Math.min(100, Math.floor(downloaded / total * 100)) : null;
    const messages = {
      idle: '尚未檢查新版。', checking: '正在安全檢查 GitHub Release…',
      not_configured: '此版本尚未設定正式更新來源與簽署公開金鑰，請先安裝已啟用更新的正式版本。',
      current: `目前已是最新版 ${current}。`, available: `發現新版 ${version}，可以下載。`,
      downloading: `正在下載並驗證新版 ${version}… ${percent === null ? (downloaded / 1048576).toFixed(1) + ' MB' : percent + '%'}`,
      ready: `新版 ${version} 已下載並通過簽章驗證。準備好後，按「重新啟動並更新」。`,
      installing: '正在啟動安裝程式，AgentMeter 即將關閉…',
      error: errors[state.error] || '目前無法完成更新作業；額度監控不受影響，請稍後重試。',
    };
    node('update-version').textContent = `目前版本 ${current}`;
    node('update-status').textContent = messages[state.status] || messages.idle;
    node('check-update').disabled = state.busy;
    node('download-update').hidden = !state.can_download && state.status !== 'downloading';
    node('download-update').disabled = state.busy || !state.can_download;
    node('install-update').hidden = !state.can_install && state.status !== 'installing';
    node('install-update').disabled = state.busy || !state.can_install;
    node('auto-app-check').checked = state.preferences.autoCheck;
    node('auto-app-download').checked = state.preferences.autoDownload;
    node('update-preference-warning').hidden = state.preferenceSaved;
    const progress = node('update-progress'); progress.hidden = state.status !== 'downloading';
    if (percent === null) progress.removeAttribute('value'); else progress.value = percent;
    const attention = Boolean(state.can_download || state.can_install);
    node('app-settings').classList.toggle('update-available', attention);
    node('app-settings').setAttribute('aria-label', attention ? '設定，有新版可更新' : '設定');
    node('app-settings').title = attention ? `新版 ${version} ${state.can_install ? '已可安裝' : '可下載'}` : '設定';
  }
  let storage;
  try { storage = window.localStorage; } catch { /* Session-only preferences still work. */ }
  const updater = createAppUpdater({ invoke: command => tabletCommand(command), storage, onChange: renderUpdate });
  node('check-update').addEventListener('click', () => { void updater.check(); });
  node('download-update').addEventListener('click', () => { void updater.download(); });
  node('install-update').addEventListener('click', () => {
    if (!updater.snapshot().can_install) return;
    if (window.confirm('新版已通過簽章驗證。安裝時會關閉 AgentMeter，並重新啟動程式。現在更新嗎？')) void updater.install(true);
  });
  node('auto-app-check').addEventListener('change', event => updater.setPreferences({ autoCheck: event.target.checked }));
  node('auto-app-download').addEventListener('change', event => updater.setPreferences({ autoDownload: event.target.checked }));
  window.addEventListener('beforeunload', () => updater.dispose());
  void updater.start();
})();
