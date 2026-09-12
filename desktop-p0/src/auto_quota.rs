//! One fixed first-party collector entry shared by desktop, scheduler and tablet.
use agentmeter_p0::live_dashboard::PROVIDERS;
use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

pub struct AutoQuota {
    pub runtime: PathBuf,
    pub workdir: PathBuf,
    pub settings_file: PathBuf,
    pub enabled: AtomicBool,
    pub interval_seconds: AtomicU64,
    pub queue: std::sync::Mutex<()>,
    pub settings_queue: std::sync::Mutex<()>,
}

pub fn polling_due(last_completed: Option<Instant>, interval_seconds: u64) -> bool {
    last_completed
        .map(|completed| completed.elapsed() >= Duration::from_secs(interval_seconds))
        .unwrap_or(true)
}

fn node_compatible_path(path: &Path) -> PathBuf {
    // Tauri may return a verbatim local path (`\\?\C:\...`). Node's Windows
    // module loader rejects that prefix while resolving the bundled entrypoint.
    // Keep UNC verbatim paths intact; only local drive paths are normalized.
    let text = path.to_string_lossy();
    if let Some(rest) = text.strip_prefix(r"\\?\")
        && rest.as_bytes().get(1) == Some(&b':')
    {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(windows)]
struct Job(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl Job {
    fn attach(child: &std::process::Child) -> Result<Self, String> {
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::System::JobObjects::*;
        // SAFETY: valid owned job/process handles; structures are initialized and sized.
        unsafe {
            let handle = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if handle.is_null() {
                return Err("collector_job_failed".into());
            }
            let job = Self(handle);
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as _,
                std::mem::size_of_val(&info) as u32,
            ) == 0
                || AssignProcessToJobObject(handle, child.as_raw_handle() as _) == 0
            {
                return Err("collector_job_failed".into());
            }
            Ok(job)
        }
    }
}
#[cfg(windows)]
impl Drop for Job {
    fn drop(&mut self) {
        // SAFETY: this is the owned handle created by attach, closed exactly once.
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

pub fn run(
    runtime: &Path,
    workdir: &Path,
    provider: &str,
    trust: bool,
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    if provider != "all" && !PROVIDERS.contains(&provider) {
        return Err("unknown_provider".into());
    }
    let runtime = node_compatible_path(runtime);
    let workdir = node_compatible_path(workdir);
    let node = runtime.join("node.exe");
    let entry = runtime.join("quota-desktop.mjs");
    if !node.is_file() || !entry.is_file() {
        return Err("collector_runtime_missing".into());
    }
    std::fs::create_dir_all(&workdir).map_err(|_| "collector_directory_unavailable")?;
    let mut command = Command::new(node);
    command
        .arg(entry)
        .arg(provider)
        .arg(workdir)
        .current_dir(runtime)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    if trust {
        command.arg("--allow-workspace-trust");
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command.spawn().map_err(|_| "collector_start_failed")?;
    #[cfg(windows)]
    let job = match Job::attach(&child) {
        Ok(job) => job,
        Err(error) => {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    };
    let stdout = child.stdout.take().ok_or("collector_output_unavailable")?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(262_145).read_to_end(&mut bytes).map(|_| bytes)
    });
    let started = Instant::now();
    let result = loop {
        if cancelled.load(Ordering::Acquire) {
            break Err("cancelled".to_string());
        }
        if started.elapsed() > Duration::from_secs(150) {
            break Err("timeout".to_string());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err("collector_failed".into())
                };
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break Err("collector_wait_failed".into()),
        }
    };
    if result.is_err() {
        let _ = child.kill();
    }
    #[cfg(windows)]
    drop(job); // Reap descendants even on timeout, cancellation, or parent exit.
    let _ = child.wait();
    let bytes = reader
        .join()
        .map_err(|_| "collector_output_failed")?
        .map_err(|_| "collector_output_failed")?;
    result?;
    if bytes.len() > 262_144 {
        return Err("collector_output_too_large".into());
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|_| "collector_response_invalid")?;
    if report["schema"] != "agentmeter.live-quota/v1" {
        return Err("collector_response_invalid".into());
    }
    if !report["results"].is_array() {
        return Err("collector_response_invalid".into());
    }
    Ok(report)
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
pub fn view(result: &Value) -> Result<Value, String> {
    let provider = result["provider"]
        .as_str()
        .filter(|p| PROVIDERS.contains(p))
        .ok_or("unknown_provider")?;
    if result["status"] != "PASS" {
        return Err("quota_not_received".into());
    }
    let items = result["quota"]
        .as_array()
        .filter(|q| !q.is_empty() && q.len() <= 32)
        .ok_or("quota_missing")?;
    let mut windows = Vec::new();
    for item in items {
        let remaining = item["remaining_percent"]
            .as_f64()
            .filter(|n| n.is_finite() && (0.0..=100.0).contains(n))
            .ok_or("quota_invalid")?;
        let bucket = item["bucket"].as_str().unwrap_or("");
        let window = item["window"].as_str().unwrap_or("");
        let limit_id = item["limit_id"].as_str().unwrap_or("");
        let limit_name = item["limit_name"].as_str().filter(|s| s.len() <= 100);
        let plan_type = item["plan_type"].as_str().filter(|s| s.len() <= 50);
        let window_duration_mins = item["window_duration_mins"]
            .as_f64()
            .filter(|n| n.is_finite() && (0.0..=525_600.0).contains(n));
        if bucket.len() > 100 || window.len() > 100 || limit_id.len() > 100 {
            return Err("quota_invalid".into());
        }
        let reset_display = item["reset_display"].as_str().filter(|s| s.len() < 150);
        let resets_at = if provider == "copilot" {
            Value::Null
        } else {
            item.get("resets_at").cloned().unwrap_or(Value::Null)
        };
        windows.push(json!({"bucket_key":format!("{bucket}:{window}"),"label":format!("{bucket} {window}").trim(),
            "limit_id":limit_id,"limit_name":limit_name,"plan_type":plan_type,"window":window,"window_duration_mins":window_duration_mins,
            "remaining_percent":remaining,"unit":"percent","resets_at":resets_at,"reset_display":reset_display,
            "entitlement":item["entitlement"].as_f64(),"used":item["used"].as_f64()}));
    }
    Ok(
        json!({"setup":"ready","availability":"available","collection_state":"ready","freshness":"fresh",
        "failure_code":null,"data_quality":"official","quota_windows":windows,"source_usage":[],"collected_at":result["collected_at"].as_u64().map(u128::from).unwrap_or_else(now),"checked_at":now()}),
    )
}

fn publish_failure(
    store: &agentmeter_p0::live_dashboard::LiveDashboard,
    provider: &str,
    reason: &str,
) {
    let reason = match reason {
        "authentication_required"
        | "cli_not_found"
        | "workspace_trust_required"
        | "collector_runtime_missing"
        | "terminal_probe_unavailable"
        | "terminal_quota_timeout"
        | "terminal_schema_changed"
        | "quota_schema_unrecognized"
        | "cli_quota_request_failed"
        | "permission_denied"
        | "timeout"
        | "quota_missing"
        | "quota_invalid"
        | "collector_response_invalid"
        | "cancelled" => reason,
        _ => "collector_failed",
    };
    let mut view = store
        .snapshot()
        .1
        .into_iter()
        .find(|p| p["provider"] == provider)
        .unwrap_or(json!({}));
    view["collection_state"] = json!("error");
    view["freshness"] = json!(if view["collected_at"].is_null() {
        "unknown"
    } else {
        "stale"
    });
    view["failure_code"] = json!(reason);
    view["checked_at"] = json!(now());
    view["availability"] = json!(match reason {
        "authentication_required" => "needs_login",
        "cli_not_found" => "not_installed",
        "workspace_trust_required" => "setup_required",
        _ => "available",
    });
    let _ = store.publish(provider, &view);
}

pub fn collect(app: &tauri::AppHandle, provider: &str, trust: bool) -> Result<(), String> {
    if provider != "all" && !PROVIDERS.contains(&provider) {
        return Err("unknown_provider".into());
    }
    let state = app.state::<crate::dashboard::Dashboard>();
    let runtime = app.state::<AutoQuota>();
    let _queue = runtime.queue.lock().map_err(|_| "collector_busy")?;
    let permit = state.lifecycle.start()?;
    let requested: Vec<&str> = if provider == "all" {
        PROVIDERS.to_vec()
    } else {
        vec![provider]
    };
    match run(
        &runtime.runtime,
        &runtime.workdir,
        provider,
        trust,
        permit.cancelled(),
    ) {
        Ok(report) => {
            let results = report["results"]
                .as_array()
                .ok_or("collector_response_invalid")?;
            for provider in requested {
                let matching: Vec<_> = results
                    .iter()
                    .filter(|r| r["provider"] == provider)
                    .collect();
                if matching.len() != 1 {
                    publish_failure(&state.live, provider, "collector_response_invalid");
                    continue;
                }
                let result = matching[0];
                match view(result) {
                    Ok(view) => state.live.publish(provider, &view)?,
                    Err(error) => publish_failure(
                        &state.live,
                        provider,
                        if error == "quota_not_received" {
                            result["reason"].as_str().unwrap_or("quota_invalid")
                        } else {
                            &error
                        },
                    ),
                }
            }
        }
        Err(error) => {
            for provider in requested {
                publish_failure(&state.live, provider, &error);
            }
            return Err(error);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn refresh_quota(
    app: tauri::AppHandle,
    provider: String,
    enable_claude: bool,
) -> Result<Value, String> {
    let worker = app.clone();
    tauri::async_runtime::spawn_blocking(move || collect(&worker, &provider, enable_claude))
        .await
        .map_err(|_| "collector_stopped")??;
    crate::dashboard::snapshot(app.state::<crate::dashboard::Dashboard>())
}

#[tauri::command]
pub fn auto_quota_settings(app: tauri::AppHandle) -> Result<Value, String> {
    let state = app.state::<AutoQuota>();
    let _guard = state.settings_queue.lock().map_err(|_| "設定忙碌")?;
    Ok(json!({
        "enabled": state.enabled.load(Ordering::Acquire),
        "interval_seconds": state.interval_seconds.load(Ordering::Acquire)
    }))
}

fn persist_settings(
    state: &AutoQuota,
    enabled: bool,
    interval_seconds: u64,
) -> Result<Value, String> {
    let _guard = state.settings_queue.lock().map_err(|_| "設定忙碌")?;
    let saved =
        crate::settings::save_sync_preferences(&state.settings_file, enabled, interval_seconds)?;
    if !saved.enabled {
        state.enabled.store(false, Ordering::Release);
    }
    state
        .interval_seconds
        .store(saved.interval_seconds, Ordering::Release);
    if saved.enabled {
        state.enabled.store(true, Ordering::Release);
    }
    Ok(json!({
        "enabled": saved.enabled,
        "interval_seconds": saved.interval_seconds
    }))
}

#[tauri::command]
pub fn set_auto_quota(app: tauri::AppHandle, enabled: bool) -> Result<Value, String> {
    let state = app.state::<AutoQuota>();
    persist_settings(
        &state,
        enabled,
        state.interval_seconds.load(Ordering::Acquire),
    )
}

#[tauri::command]
pub fn set_auto_quota_settings(
    app: tauri::AppHandle,
    enabled: bool,
    interval_seconds: u64,
) -> Result<Value, String> {
    persist_settings(&app.state::<AutoQuota>(), enabled, interval_seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn node_path_normalization_only_removes_verbatim_prefix_for_local_drives() {
        assert_eq!(
            node_compatible_path(Path::new(r"\\?\C:\AgentMeter")),
            PathBuf::from(r"C:\AgentMeter")
        );
        assert_eq!(
            node_compatible_path(Path::new(r"\\?\UNC\server\share")),
            PathBuf::from(r"\\?\UNC\server\share")
        );
    }
    #[test]
    fn quota_projection_requires_actual_numeric_values_and_ignores_private_data() {
        let input = json!({"provider":"copilot","status":"PASS","token":"PRIVATE","quota":[{"bucket":"premium_interactions","remaining_percent":24.7,"reported_reset_at":"today","entitlement":1500,"used":1130}]});
        let projected = view(&input).unwrap();
        assert_eq!(projected["quota_windows"][0]["remaining_percent"], 24.7);
        assert!(projected["quota_windows"][0]["resets_at"].is_null());
        assert!(!projected.to_string().contains("PRIVATE"));
        for invalid in [Value::Null, json!("24"), json!(101), json!(-1)] {
            let mut bad = input.clone();
            bad["quota"][0]["remaining_percent"] = invalid;
            assert!(view(&bad).is_err());
        }
    }
    #[test]
    fn codex_projection_preserves_period_metadata_for_product_facing_labels() {
        let input = json!({"provider":"codex","status":"PASS","quota":[{"bucket":"codex_bengalfox","limit_id":"codex_bengalfox","limit_name":"GPT-5.3-Codex-Spark","plan_type":"prolite","window":"primary","window_duration_mins":300,"remaining_percent":100} ]});
        let projected = view(&input).unwrap();
        let quota = &projected["quota_windows"][0];
        assert_eq!(quota["limit_id"], "codex_bengalfox");
        assert_eq!(quota["limit_name"], "GPT-5.3-Codex-Spark");
        assert_eq!(quota["plan_type"], "prolite");
        assert_eq!(quota["window_duration_mins"], 300.0);
    }
    #[test]
    fn failed_refresh_retains_previous_quota_but_marks_it_stale() {
        let store = agentmeter_p0::live_dashboard::LiveDashboard::default();
        let input = json!({"provider":"claude","status":"PASS","quota":[{"window":"five_hour","remaining_percent":70,"reset_display":"3am"}]});
        store.publish("claude", &view(&input).unwrap()).unwrap();
        publish_failure(&store, "claude", "authentication_required");
        let failed = store
            .snapshot()
            .1
            .into_iter()
            .find(|p| p["provider"] == "claude")
            .unwrap();
        assert_eq!(failed["quota_windows"][0]["remaining_percent"], 70.0);
        assert_eq!(failed["freshness"], "stale");
        assert_eq!(failed["availability"], "needs_login");
    }
    #[test]
    fn polling_is_immediate_at_start_and_uses_the_latest_interval() {
        assert!(polling_due(None, 3600));
        let completed = Instant::now() - Duration::from_secs(301);
        assert!(polling_due(Some(completed), 300));
        assert!(!polling_due(Some(completed), 600));
    }
}
