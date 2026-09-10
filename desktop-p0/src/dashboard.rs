use serde_json::{Value, json};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub struct Dashboard {
    pub live: agentmeter_p0::live_dashboard::LiveDashboard,
    pub lifecycle: std::sync::Arc<crate::lifecycle::CollectorLifecycle>,
    latest: Mutex<Option<Value>>,
    claude: Mutex<Option<Value>>,
    copilot: Mutex<Option<Value>>,
    refreshing: AtomicBool,
}

fn public_report(report: Value) -> Value {
    let observation = &report["observation"];
    json!({
        "availability": observation.get("availability").or_else(|| report.get("availability")).cloned().unwrap_or(json!("unknown")),
        "failure_code": report.get("failure_code").cloned().unwrap_or(Value::Null),
        "quota_windows": observation.get("quota_windows").cloned().unwrap_or(json!([])),
        "freshness": observation.get("freshness").cloned().unwrap_or(json!("unknown")),
        "data_quality":observation.get("data_quality").or_else(|| report.get("data_quality")).cloned().unwrap_or(json!("unknown")),
        "collected_at": observation.get("collected_at_unix_ms").cloned().unwrap_or(Value::Null),
        "checked_at": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
    })
}

#[tauri::command]
pub fn snapshot(state: tauri::State<'_, Dashboard>) -> Result<Value, String> {
    let latest = state.latest.lock().map_err(|_| "dashboard unavailable")?;
    let claude = state.claude.lock().map_err(|_| "dashboard unavailable")?;
    let copilot = state.copilot.lock().map_err(|_| "dashboard unavailable")?;
    Ok(json!({
        "codex":*latest,
        "claude":*claude,
        "copilot":*copilot,
        "provider_states":state.live.snapshot().1,
        "refreshing":state.refreshing.load(Ordering::Acquire) || state.lifecycle.is_running()
    }))
}

pub(crate) fn locate_codex() -> Option<std::path::PathBuf> {
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path).filter(|p| p.is_absolute()) {
            let candidate = directory.join("codex.exe");
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let root = std::path::PathBuf::from(std::env::var_os("LOCALAPPDATA")?).join("OpenAI/Codex/bin");
    let candidates: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path().join("codex.exe"))
        .filter(|p| p.is_file())
        .collect();
    if candidates.len() == 1 {
        candidates.into_iter().next()
    } else {
        None
    }
}

#[tauri::command]
pub fn load_claude(path: String, state: tauri::State<'_, Dashboard>) -> Result<Value, String> {
    use std::io::Read;
    let path = if path.trim().is_empty() {
        crate::discovery::claude_report().ok_or("尚未收到 Claude 事件。請先按「設定整合」啟用，接著照常使用 Claude；有新狀態事件後便會更新。")?
    } else {
        std::path::PathBuf::from(path)
    };
    if !path.is_absolute() {
        return Err("請輸入報告的完整路徑".into());
    }
    let file = std::fs::File::open(path).map_err(|_| "無法開啟報告")?;
    let mut bytes = Vec::new();
    file.take(1_048_577)
        .read_to_end(&mut bytes)
        .map_err(|_| "無法讀取報告")?;
    if bytes.len() > 1_048_576 {
        return Err("報告超過大小限制".into());
    }
    let report: Value = serde_json::from_slice(&bytes).map_err(|_| "報告不是有效 JSON")?;
    let view = claude_view(&report)?;
    state
        .live
        .publish("claude", &tablet_view("claude", &view))?;
    *state.claude.lock().map_err(|_| "dashboard unavailable")? = Some(view);
    snapshot(state)
}

fn claude_view(report: &Value) -> Result<Value, String> {
    if report["schema_version"] != "agentmeter.p0.collection-report.v1"
        || report["provider"] != "claude"
        || report["source"]["mode"] != "event_stdin"
        || report["source"]["replay"] != false
    {
        return Err("請使用 Claude wrapper 產生的事件報告".into());
    }
    if report["outcome"] == "failure" || report["collection_state"] == "error" {
        return Err("Claude 事件格式無法辨識；保留上次資料，等待新的有效事件。".into());
    }
    let mut windows = Vec::new();
    if let Some(items) = report["observation"]["quota_windows"].as_array() {
        for item in items {
            let used = item["used"].as_f64().filter(|n| (0.0..=100.0).contains(n));
            if item["unit"] != "percent" || used.is_none() {
                return Err("額度欄位格式不符".into());
            }
            let scope = item["scope"]
                .as_str()
                .filter(|s| ["five_hour", "seven_day"].contains(s))
                .ok_or("未知額度視窗")?;
            windows.push(json!({"window":scope,"remaining_percent":100.0-used.unwrap(),"resets_at":item["resets_at"].as_u64()}));
        }
    }
    let status = if windows.is_empty() {
        "已收到事件，尚無可用額度欄位"
    } else {
        "已載入 Claude 事件報告"
    };
    Ok(
        json!({"quota_windows":windows,"collected_at":report["collected_at_epoch_seconds"].as_u64().and_then(|t| t.checked_mul(1000)),"status":status}),
    )
}

fn copilot_view(report: &Value) -> Result<Value, String> {
    if report["schema_version"] != "copilot-p0/v1"
        || report["capability"] != "copilot_authoritative_billing_usage"
        || report["evidence"]["mode"] != "live_gh_api"
        || report["evidence"]["replay"] != false
    {
        return Err("Copilot 回應來源不受支援".into());
    }
    let contexts = report["contexts"]
        .as_array()
        .filter(|items| items.len() == 1)
        .ok_or("Copilot 回應缺少唯一帳號範圍")?;
    let context = &contexts[0];
    if context["data_quality"] != "official"
        || context["collector_maturity"] != "experimental"
        || context["authoritative_usage"]["status"] != "reported"
        || context["quota"]["status"] != "not_exposed_by_billing_usage_endpoint"
    {
        return Err("Copilot 回應品質或額度邊界不符".into());
    }
    let account_kind = context["account_scope"]["kind"]
        .as_str()
        .ok_or("Copilot 回應缺少帳號類型")?;
    let account = context["account_scope"]["reported"]
        .as_str()
        .ok_or("Copilot 回應缺少帳號")?;
    let meter = context["authoritative_usage"]["meter"]
        .as_str()
        .ok_or("Copilot 回應缺少計量類型")?;
    let mut usage_items = Vec::new();
    for item in context["authoritative_usage"]["items"]
        .as_array()
        .ok_or("Copilot 回應缺少使用量清單")?
    {
        let sku = item["sku"].as_str().ok_or("Copilot SKU 格式不符")?;
        let unit = item["unit"].as_str().ok_or("Copilot 單位格式不符")?;
        let net_used = item["net_used"]
            .as_f64()
            .filter(|value| value.is_finite() && *value >= 0.0)
            .ok_or("Copilot 使用量格式不符")?;
        usage_items.push(json!({
            "sku":sku,
            "model":item["model"].as_str(),
            "unit":unit,
            "net_used":net_used
        }));
    }
    Ok(json!({
        "status":"已讀取官方 Billing 使用量",
        "account_kind":account_kind,
        "account":account,
        "meter":meter,
        "billing_period":context["authoritative_usage"]["billing_period"],
        "usage_items":usage_items,
        "quota_status":"remaining_unknown",
        "data_quality":"official",
        "collector_maturity":"experimental",
        "checked_at":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
    }))
}

fn copilot_failure_view(code: &str) -> Value {
    let status = match code {
        "authentication_failed" => "GitHub CLI 需要登入",
        "permission_denied" => "帳號或角色權限不足",
        "rate_limited" => "GitHub 暫時限制請求",
        "endpoint_unavailable" => "此帳號無法使用 Billing endpoint",
        "timeout" => "GitHub Billing 讀取逾時",
        "cancelled" => "讀取已取消",
        "schema_changed" | "malformed_response" => "GitHub 回應格式不受支援",
        _ => "GitHub Billing 讀取失敗",
    };
    json!({
        "status":status,
        "failure_code":code,
        "usage_items":[],
        "quota_status":"remaining_unknown",
        "data_quality":"unknown",
        "collector_maturity":"experimental",
        "checked_at":SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
    })
}

#[tauri::command]
pub async fn refresh_copilot(
    gh_path: String,
    context: String,
    account: String,
    meter: String,
    state: tauri::State<'_, Dashboard>,
) -> Result<Value, String> {
    let permit = state.lifecycle.start()?;
    let gh_path = if gh_path.trim().is_empty() {
        crate::discovery::github_cli()
            .ok_or("尚未找到 GitHub CLI。請安裝 GitHub CLI 並完成登入，再按連線重試。")?
    } else {
        std::path::PathBuf::from(gh_path.trim())
    };
    if context != "personal" && account.trim().is_empty() {
        return Err("公司帳務需要選定組織或企業；請在進階設定指定帳務範圍。".into());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        let account = if account.trim().is_empty() {
            agentmeter_p0::copilot::authenticated_account(&gh_path, permit.cancelled())?
        } else {
            account
        };
        let request = agentmeter_p0::copilot::UsageRequest::new(
            &gh_path,
            context.trim(),
            account.trim(),
            meter.trim(),
            None,
            None,
            Duration::from_secs(10),
        )?;
        agentmeter_p0::copilot::collect_usage(request, permit.cancelled())
    })
    .await
    .map_err(|_| "Copilot collector stopped")?;
    let view = match result {
        Ok(report) => copilot_view(&report)?,
        Err(failure) => copilot_failure_view(failure.code),
    };
    state
        .live
        .publish("copilot", &tablet_view("copilot", &view))?;
    *state.copilot.lock().map_err(|_| "dashboard unavailable")? = Some(view);
    snapshot(state)
}

#[tauri::command]
pub async fn refresh(state: tauri::State<'_, Dashboard>) -> Result<Value, String> {
    let permit = state.lifecycle.start()?;
    if state.refreshing.swap(true, Ordering::AcqRel) {
        return Err("refresh_in_progress".into());
    }
    let result = tauri::async_runtime::spawn_blocking(move || {
        let Some(executable) = locate_codex() else {
            return Ok(public_report(json!({"availability":"setup_required","failure_code":"codex_executable_not_found_or_ambiguous"})));
        };
        match agentmeter_p0::codex::collect_live_cancellable(
            &executable,
            Duration::from_secs(5),
            permit.cancelled(),
        ) {
            Ok(report) => serde_json::to_value(report),
            Err(report) => serde_json::to_value(report),
        }
        .map(public_report)
    })
    .await;
    state.refreshing.store(false, Ordering::Release);
    let report = result
        .map_err(|_| "collector stopped")?
        .map_err(|_| "invalid collection report")?;
    state
        .live
        .publish("codex", &tablet_view("codex", &report))?;
    *state.latest.lock().map_err(|_| "dashboard unavailable")? = Some(report);
    snapshot(state)
}

fn tablet_view(provider: &str, report: &Value) -> Value {
    let failed = !report["failure_code"].is_null();
    let mut view = json!({
        "setup":if failed {"required"} else {"ready"},
        "availability":report.get("availability").cloned().unwrap_or(json!(if failed {"unknown"} else {"available"})),
        "collection_state":if failed {"error"} else {"ready"},
        "freshness":report.get("freshness").cloned().unwrap_or(json!(if provider == "copilot" {"provider_reported_period"} else {"unknown"})),
        "data_quality":report.get("data_quality").cloned().unwrap_or(json!(if provider == "claude" {"official"} else {"unknown"})),
        "failure_code":report["failure_code"],"collected_at":report["collected_at"],"checked_at":report["checked_at"],
        "quota_windows":[],"source_usage":[]
    });
    view["quota_windows"] = json!(report["quota_windows"].as_array().map(|items| items.iter().map(|q| json!({
        "bucket_key":q.get("window").or_else(|| q.get("limit_name")).cloned().unwrap_or(Value::Null),
        "label":q.get("window").or_else(|| q.get("limit_name")).cloned().unwrap_or(Value::Null),
        "unit":"percent","remaining_percent":q["remaining_percent"],"used_percent":q["used_percent"],"resets_at":q["resets_at"]
    })).collect::<Vec<_>>()).unwrap_or_default());
    if provider == "copilot" {
        view["source_usage"] = json!(
            report["usage_items"]
                .as_array()
                .map(|items| items
                    .iter()
                    .map(|q| json!({
                        "used":q["net_used"],"unit":q["unit"],"model":q["model"],"label":q["sku"]
                    }))
                    .collect::<Vec<_>>())
                .unwrap_or_default()
        );
    }
    view
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tablet_projection_keeps_usage_distinct_from_quota_and_drops_identity() {
        let store = agentmeter_p0::live_dashboard::LiveDashboard::default();
        let view = tablet_view(
            "copilot",
            &json!({"account":"PRIVATE","gh_path":"PRIVATE","data_quality":"official","usage_items":[{"sku":"AI Credits","net_used":15,"unit":"credits"}],"quota_status":"remaining_unknown"}),
        );
        store.publish("copilot", &view).unwrap();
        let providers = store.snapshot().1;
        assert_eq!(providers[2]["source_usage"][0]["used"], 15);
        assert_eq!(providers[2]["quota_windows"], json!([]));
        assert!(
            !serde_json::to_string(&providers)
                .unwrap()
                .contains("PRIVATE")
        );
        let codex = tablet_view(
            "codex",
            &json!({"availability":"available","freshness":"fresh","quota_windows":[{"window":"five_hour","remaining_percent":null,"used_percent":0}]}),
        );
        assert!(codex["quota_windows"][0]["remaining_percent"].is_null());
        assert_eq!(codex["quota_windows"][0]["used_percent"], 0);
    }
    #[test]
    fn claude_import_rejects_fixture_and_strips_unrelated_data() {
        let mut report = json!({"schema_version":"agentmeter.p0.collection-report.v1","provider":"claude","source":{"mode":"event_stdin","replay":false},"collected_at_epoch_seconds":42,"secret":"PRIVATE","observation":{"quota_windows":[{"scope":"five_hour","unit":"percent","used":25}]}});
        let result = claude_view(&report).unwrap();
        assert_eq!(result["quota_windows"][0]["remaining_percent"], 75.0);
        assert_eq!(result["collected_at"], 42000);
        assert!(!result.to_string().contains("PRIVATE"));
        report["source"]["replay"] = json!(true);
        assert!(claude_view(&report).is_err());
    }
    #[test]
    fn failure_does_not_fabricate_quota_or_expose_diagnostics() {
        let report = public_report(
            json!({"availability":"needs_login","failure_code":"authentication_required","observation":null,"diagnostics":[{"message":"PRIVATE"}]}),
        );
        assert_eq!(report["availability"], "needs_login");
        assert_eq!(report["quota_windows"], json!([]));
        assert!(report["collected_at"].is_null());
        assert!(!report.to_string().contains("PRIVATE"));
    }

    #[test]
    fn invalid_claude_event_does_not_look_like_a_successful_empty_report() {
        let event = agentmeter_p0::claude::normalize_status_event(b"broken");
        assert!(claude_view(&event).is_err());
        let event = agentmeter_p0::claude::normalize_status_event(b"{}");
        let view = claude_view(&event).unwrap();
        assert_eq!(view["status"], "已收到事件，尚無可用額度欄位");
        assert!(view["quota_windows"].as_array().unwrap().is_empty());
    }
    #[test]
    fn quota_retains_unknown_values_and_timestamp() {
        let report = public_report(
            json!({"observation":{"collected_at_unix_ms":42,"quota_windows":[{"remaining_percent":null,"used_percent":0}]}}),
        );
        assert_eq!(report["quota_windows"][0]["used_percent"], 0);
        assert!(report["quota_windows"][0]["remaining_percent"].is_null());
        assert_eq!(report["collected_at"], 42);
    }

    #[test]
    fn copilot_view_exposes_usage_without_credentials_money_or_invented_remaining_quota() {
        let report = json!({
            "schema_version":"copilot-p0/v1",
            "capability":"copilot_authoritative_billing_usage",
            "evidence":{"mode":"live_gh_api","replay":false},
            "contexts":[{
                "account_scope":{"kind":"personal","reported":"octocat"},
                "authoritative_usage":{"status":"reported","meter":"ai_credits","billing_period":{"year":2026,"month":9},"items":[{"sku":"Copilot AI Credits","model":"GPT-5","unit":"ai-credits","net_used":12,"netAmount":0.12,"secret":"PRIVATE"}]},
                "quota":{"status":"not_exposed_by_billing_usage_endpoint"},
                "data_quality":"official",
                "collector_maturity":"experimental"
            }]
        });
        let view = copilot_view(&report).unwrap();
        assert_eq!(view["account"], "octocat");
        assert_eq!(view["usage_items"][0]["net_used"], 12.0);
        assert_eq!(view["quota_status"], "remaining_unknown");
        assert!(!view.to_string().contains("PRIVATE"));
        assert!(!view.to_string().contains("netAmount"));
        assert!(view.get("remaining").is_none());
    }

    #[test]
    fn copilot_failure_view_is_sanitized_and_keeps_quota_unknown() {
        let view = copilot_failure_view("permission_denied");
        assert_eq!(view["failure_code"], "permission_denied");
        assert_eq!(view["quota_status"], "remaining_unknown");
        assert!(view["usage_items"].as_array().unwrap().is_empty());
    }
}
