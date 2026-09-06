use serde_json::{Value, json};
use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

fn normalize(item: &Value) -> Value {
    let permission = item
        .pointer("/billing_api/permission")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = match permission {
        "granted" => "available",
        "denied" => "permission_denied",
        _ => "unknown",
    };
    let preview = item
        .get("preview_sdk")
        .cloned()
        .unwrap_or_else(|| json!({"status":"unknown"}));
    json!({"context":item.get("context").and_then(Value::as_str).unwrap_or("unknown"),"billing_api":{"permission":permission,"status":status},"quota":{"status":if permission=="denied"{"unknown_permission_denied"}else{item.pointer("/quota/status").and_then(Value::as_str).unwrap_or("unknown")},"raw":item.get("quota").cloned().unwrap_or(Value::Null)},"ai_credits":item.get("ai_credits").cloned().unwrap_or_else(||json!({"status":"unknown"})),"legacy_premium_requests":item.get("legacy_premium_requests").cloned().unwrap_or_else(||json!({"status":"unknown"})),"preview_sdk":{"status":preview.get("status").cloned().unwrap_or_else(||json!("unknown")),"experimental":true,"raw":preview},"unknowns":item.get("unknowns").cloned().unwrap_or_else(||json!([]))})
}
fn main() -> ExitCode {
    let mut a = env::args().skip(1);
    let input: Value = if a.next().as_deref() == Some("--fixture") {
        match a
            .next()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
        {
            Some(v) => v,
            None => return ExitCode::from(2),
        }
    } else {
        let mut s = String::new();
        if io::stdin().read_to_string(&mut s).is_err() {
            return ExitCode::from(2);
        };
        match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(_) => return ExitCode::from(2),
        }
    };
    let xs = input
        .get("contexts")
        .and_then(Value::as_array)
        .map(|v| v.iter().map(normalize).collect::<Vec<_>>())
        .unwrap_or_default();
    println!("{}",serde_json::to_string_pretty(&json!({"schema_version":"copilot-p0/v1","capability":"copilot_quota_boundaries","contexts":xs,"unknowns":input.get("unknowns").cloned().unwrap_or_else(||json!([])),"evidence":input.get("evidence").cloned().unwrap_or_else(||json!([]))})).unwrap());
    ExitCode::SUCCESS
}
