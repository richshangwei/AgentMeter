//! Allowlisted normalization of Claude statusLine events. Raw session input stays in memory.
use serde_json::{Value, json};

pub fn normalize_status_event(bytes: &[u8]) -> Value {
    let mut report = diagnostic_report(
        "missing_quota",
        "available",
        "idle",
        "status_line",
        "valid event contains no supported quota windows",
    );
    report["source"] = json!({"kind":"status_line","mode":"event_stdin","replay":false});
    report["data_quality"] = json!("official");
    report["outcome"] = json!("success");
    let received = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    report["collected_at_epoch_seconds"] = json!(received);
    let invalid = |mut report: Value, code: &str| {
        report["outcome"] = json!("failure");
        report["collection_state"] = json!("error");
        report["failure_code"] = json!(code);
        report["diagnostic"] = json!(code);
        report["diagnostics"] = json!([{"code":code,"message":"unsupported statusLine payload"}]);
        report
    };
    let event: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => return invalid(report, "malformed_payload"),
    };
    if !event.is_object() {
        return invalid(report, "schema_changed");
    }
    let Some(limits) = event.get("rate_limits").filter(|value| !value.is_null()) else {
        return report;
    };
    if !limits.is_object() {
        return invalid(report, "schema_changed");
    }
    let mut windows = Vec::new();
    for scope in ["five_hour", "seven_day"] {
        let Some(window) = limits.get(scope).filter(|value| !value.is_null()) else {
            continue;
        };
        if !window.is_object() {
            return invalid(report, "schema_changed");
        }
        let Some(used) = window
            .get("used_percentage")
            .filter(|value| !value.is_null())
        else {
            continue;
        };
        if !used.as_f64().is_some_and(|n| (0.0..=100.0).contains(&n)) {
            return invalid(report, "schema_changed");
        }
        let reset = window.get("resets_at").cloned().unwrap_or(Value::Null);
        if !reset.is_null() && reset.as_u64().is_none() {
            return invalid(report, "schema_changed");
        }
        windows.push(json!({"scope":scope,"used":used,"limit":100,"unit":"percent","resets_at":reset,"reset_time_unit":"unix_epoch_seconds"}));
    }
    if !windows.is_empty() {
        report["collection_state"] = json!("ready");
        report["failure_code"] = Value::Null;
        report["diagnostic"] = Value::Null;
        report["diagnostics"] = json!([]);
        // The documented payload has no measurement timestamp. Receipt time is
        // explicit and must not masquerade as Provider source time.
        report["observation"] = json!({"source_timestamp":null,"observed_at_epoch_seconds":received,"quota_windows":windows});
    }
    report
}

fn diagnostic_report(
    code: &str,
    availability: &str,
    collection_state: &str,
    source_kind: &str,
    message: &str,
) -> Value {
    json!({
        "schema_version":"agentmeter.p0.collection-report.v1",
        "outcome":"failure",
        "provider":"claude",
        "provider_account":{"kind":"unknown"},
        "source":{"kind":source_kind,"mode":"fixture_replay","replay":true},
        "data_quality":"local_observed",
        "collector_maturity":"experimental",
        "availability":availability,
        "collection_state":collection_state,
        "freshness":"unknown",
        "failure_code":code,
        "diagnostic":code,
        "diagnostics":[{"code":code,"message":message}],
        "observation":Value::Null
    })
}
