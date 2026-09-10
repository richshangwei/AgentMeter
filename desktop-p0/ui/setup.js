// Local capability inspection does not make provider requests or modify settings.
(() => {
  const el = id => document.getElementById(id);
  const invoke = (command, args = {}) => window.__TAURI__.core.invoke(command, args);
  let applying = false;
  async function scan() {
    el('setup-scan').disabled = true;
    try {
      const state = await invoke('inspect_setup');
      const lines = [
        `Codex：${state.codex_installed ? '已找到，按重新整理檢查登入與額度。' : '尚未找到，請先安裝或確認官方工具可用。'}`,
        `GitHub Copilot：${state.github_installed ? '已找到 GitHub CLI，可按連線檢查。' : '尚未找到 GitHub CLI，請先完成安裝與登入。'}`,
        `Claude：${state.claude_installed ? '已找到工具。' : '尚未找到工具；非標準安裝仍可設定使用者整合。'} ${state.claude_integration === 'enabled' ? '整合已啟用。' : state.claude_integration === 'conflict' ? '設定已被修改，請先確認既有整合。' : '可按「設定整合」預覽並啟用。'} ${state.claude_report_available ? '已有事件報告。' : '尚未收到事件。'}`,
        `USB 平板：${state.adb_path ? '已找到 ADB。' : '尚未找到 Android Platform Tools。'}`,
        'Antigravity：尚未支援自動連線。'
      ];
      el('setup-results').replaceChildren(...lines.map(text => { const item = document.createElement('li'); item.textContent = text; return item; }));
      // Preserve a manually selected ADB source.
      if (state.adb_path && !el('adb-path').value.trim()) el('adb-path').value = state.adb_path;
      el('claude-setup-enable').textContent = state.claude_integration === 'enabled' ? '檢視整合' : '設定整合';
      el('claude-setup-disable').disabled = !['enabled','conflict'].includes(state.claude_integration);
    } catch {
      el('setup-results').textContent = '無法檢查桌面環境，請從 AgentMeter 桌面程式開啟後重試。';
    } finally { el('setup-scan').disabled = false; }
  }
  async function preview(enable) {
    el('claude-setup-enable').disabled = el('claude-setup-disable').disabled = true;
    try {
      const plan = await invoke('preview_claude_setup', {enable});
      el('claude-setup-summary').textContent = plan.message;
      el('claude-setup-path').textContent = plan.settings_path;
      el('claude-setup-before').textContent = JSON.stringify(plan.before, null, 2);
      el('claude-setup-after').textContent = JSON.stringify(plan.after, null, 2);
      el('claude-setup-result').textContent = '';
      el('claude-setup-apply').textContent = enable ? '備份並啟用' : '備份並還原';
      el('claude-setup-apply').disabled = false;
      el('claude-setup-dialog').showModal();
    } catch (error) { el('claude-message').textContent = String(error); }
    finally { el('claude-setup-enable').disabled = el('claude-setup-disable').disabled = false; }
  }
  el('setup-scan').addEventListener('click', scan);
  el('claude-setup-enable').addEventListener('click', () => preview(true));
  el('claude-setup-disable').addEventListener('click', () => preview(false));
  el('claude-setup-apply').addEventListener('click', async () => {
    applying = true;
    el('claude-setup-apply').disabled = el('claude-setup-cancel').disabled = true;
    try {
      const result = await invoke('apply_claude_setup');
      el('claude-setup-result').textContent = result.message;
      el('claude-message').textContent = result.message;
      // Stop any old watcher before changing its source, including on disable.
      el('claude-path').dispatchEvent(new Event('input'));
      el('claude-message').textContent = result.message;
      if (result.enabled) {
        el('claude-path').value = result.report_path;
        try { await invoke('save_claude_path', {path:result.report_path}); }
        catch { el('claude-settings-message').textContent = '整合已啟用，但監看來源未能儲存。'; }
        el('claude-watch').click();
      }
      await scan();
      el('claude-setup-cancel').textContent = '完成';
    } catch (error) {
      el('claude-setup-result').textContent = `${String(error)} 請關閉此視窗後重新預覽。`;
    } finally { applying = false; el('claude-setup-cancel').disabled = false; }
  });
  el('claude-setup-cancel').addEventListener('click', () => el('claude-setup-dialog').close());
  el('claude-setup-dialog').addEventListener('cancel', event => { if (applying) event.preventDefault(); });
  el('claude-setup-dialog').addEventListener('close', () => {
    el('claude-setup-cancel').textContent = '取消';
    for (const id of ['claude-setup-before','claude-setup-after','claude-setup-path']) el(id).textContent = '';
    void invoke('cancel_claude_setup').catch(() => {});
  });
  scan();
})();
