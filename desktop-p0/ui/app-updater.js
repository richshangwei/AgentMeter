(function (root) {
  'use strict';
  const preferenceKey = 'agentmeter.application-updates.v1';
  const statuses = new Set(['idle','checking','current','available','downloading','ready','installing','error','not_configured']);

  function createAppUpdater({ invoke, storage, onChange = () => {},
    setTimer = setTimeout, clearTimer = clearTimeout,
    setPoll = setInterval, clearPoll = clearInterval,
    intervalMs = 6 * 60 * 60 * 1000, retryMs = 30 * 60 * 1000 }) {
    let preferences = { autoCheck: true, autoDownload: false }, preferenceSaved = true;
    try {
      const saved = JSON.parse(storage?.getItem(preferenceKey) || 'null');
      if (typeof saved?.autoCheck === 'boolean') preferences.autoCheck = saved.autoCheck;
      if (typeof saved?.autoDownload === 'boolean') preferences.autoDownload = saved.autoDownload;
    } catch { preferenceSaved = false; }
    let state = { configured: null, current_version: '', status: 'idle', available: false,
      can_download: false, can_install: false, downloaded_bytes: 0, total_bytes: null, error: null };
    let active = null, timer = null, poll = null, started = false, disposed = false, polling = false, generation = 0;
    const snapshot = () => ({ ...state, busy: active !== null, preferences: { ...preferences }, preferenceSaved });
    function publish(change = {}) {
      if (disposed) return;
      state = { ...state, ...change };
      onChange(snapshot());
    }
    function accept(result) {
      if (!result || typeof result !== 'object') return;
      const status = statuses.has(result.status) ? result.status : result.configured === false ? 'not_configured' : undefined;
      publish({ ...result, ...(status ? { status } : {}) });
    }
    async function sync() {
      if (disposed || polling) return;
      polling = true;
      const requestedGeneration = generation;
      try { const result = await invoke('update_status'); if (generation === requestedGeneration) accept(result); }
      finally { polling = false; }
    }
    function schedule(delay = intervalMs) {
      if (timer !== null) clearTimer(timer);
      timer = null;
      if (!disposed && preferences.autoCheck) timer = setTimer(() => { timer = null; void check(true); }, delay);
    }
    async function perform(command, status) {
      if (active || disposed) return false;
      active = command; generation++;
      publish({ status, error: null });
      poll = setPoll(() => { void sync().catch(() => {}); }, 500);
      try {
        const result = await invoke(command); generation++; accept(result);
        return true;
      } catch (error) {
        try { await sync(); } catch { /* Keep the last usable state during IPC failure. */ }
        const code = String(error);
        publish({ status: 'error', error: /^update_[a-z_]+$/.test(code) ? code : 'update_unavailable' });
        return false;
      } finally {
        if (poll !== null) clearPoll(poll);
        poll = null; active = null; generation++; publish();
      }
    }
    async function check(automatic = false) {
      if (disposed || (automatic && !preferences.autoCheck)) return false;
      if (active) { if (automatic) schedule(60 * 1000); return false; }
      const ok = await perform('check_update', 'checking');
      if (ok && state.can_download && preferences.autoDownload) await download();
      schedule(ok && state.status !== 'error' ? intervalMs : retryMs);
      return ok;
    }
    async function download() {
      if (!state.configured || !state.can_download || active || disposed) return false;
      return perform('download_update', 'downloading');
    }
    async function install(confirmed = false) {
      // A timer or a downloaded artifact is never authority to terminate the app.
      if (!confirmed || !state.configured || !state.can_install || active || disposed) return false;
      return perform('install_update', 'installing');
    }
    function setPreferences(change) {
      const wasAutoCheck = preferences.autoCheck;
      for (const name of ['autoCheck','autoDownload']) {
        if (typeof change[name] === 'boolean') preferences[name] = change[name];
      }
      try { storage?.setItem(preferenceKey, JSON.stringify(preferences)); preferenceSaved = Boolean(storage); }
      catch { preferenceSaved = false; }
      publish();
      if (started) schedule(!wasAutoCheck && preferences.autoCheck ? 0 : intervalMs);
      if (preferences.autoDownload && state.can_download) void download();
    }
    async function start() {
      if (started || disposed) return;
      started = true; publish();
      try { await sync(); } catch { publish({ status: 'error', error: 'update_unavailable' }); }
      if (preferences.autoCheck) await check(true);
    }
    function dispose() {
      disposed = true;
      if (timer !== null) clearTimer(timer);
      if (poll !== null) clearPoll(poll);
    }
    return { start, check, download, install, setPreferences, sync, dispose, snapshot };
  }
  if (typeof module === 'object' && module.exports) module.exports = { createAppUpdater, preferenceKey };
  else root.createAppUpdater = createAppUpdater;
})(typeof globalThis === 'object' ? globalThis : this);
