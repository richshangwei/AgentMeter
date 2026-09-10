use agentmeter_p0::claude_setup::Plan;
use serde_json::{Value, json};
use std::sync::Mutex;

#[derive(Default)]
pub struct SetupState(Mutex<Option<Plan>>);

#[tauri::command]
pub fn inspect_setup() -> Value {
    let claude = crate::discovery::claude_settings()
        .map(|p| agentmeter_p0::claude_setup::integration_state(&p))
        .unwrap_or_else(|| Err("找不到使用者設定位置。".into()));
    json!({
        "codex_installed":crate::dashboard::locate_codex().is_some(),
        "github_installed":crate::discovery::github_cli().is_some(),
        "claude_installed":crate::discovery::claude_cli().is_some(),
        "claude_integration":claude.as_ref().map(|s| *s).unwrap_or("unavailable"),
        "claude_message":claude.err(),
        "claude_report_available":crate::discovery::claude_report().is_some(),
        "adb_path":crate::discovery::adb(),
        "antigravity":"unsupported"
    })
}

#[tauri::command]
pub fn preview_claude_setup(
    enable: bool,
    state: tauri::State<'_, SetupState>,
) -> Result<Value, String> {
    let mut pending = state.0.lock().map_err(|_| "設定忙碌，請稍後重試。")?;
    *pending = None;
    let path = crate::discovery::claude_settings().ok_or("找不到 Claude 設定位置。")?;
    let executable = std::env::current_exe().map_err(|_| "找不到內建接收器。")?;
    let plan = Plan::preview(path, &executable, crate::discovery::claude_shell(), enable)?;
    let view = plan.view();
    *pending = Some(plan);
    Ok(view)
}

#[tauri::command]
pub fn apply_claude_setup(state: tauri::State<'_, SetupState>) -> Result<Value, String> {
    let mut pending = state.0.lock().map_err(|_| "設定忙碌，請稍後重試。")?;
    pending.take().ok_or("請先預覽變更。")?.apply()
}

#[tauri::command]
pub fn cancel_claude_setup(state: tauri::State<'_, SetupState>) -> Result<(), String> {
    *state.0.lock().map_err(|_| "設定忙碌，請稍後重試。")? = None;
    Ok(())
}
