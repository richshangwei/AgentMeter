use serde_json::{Value, json};
use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
    sync::atomic::AtomicBool,
    time::Duration,
};

use agentmeter_p0::copilot::{UsageRequest, collect_usage};

fn array_or_empty(value: Option<&Value>) -> Vec<Value> {
    value.and_then(Value::as_array).cloned().unwrap_or_default()
}

fn missing_values(required: &[Value], observed: &[Value]) -> Vec<Value> {
    required
        .iter()
        .filter(|required| !observed.iter().any(|observed| observed == *required))
        .cloned()
        .collect()
}

fn permission_requirements(item: &Value) -> (Value, bool) {
    let contract_present = item.get("permission_contract").is_some();
    let contract = item
        .get("permission_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let required_scopes = array_or_empty(contract.get("required_scopes"));
    let required_permissions = array_or_empty(contract.get("required_permissions"));
    let required_roles = array_or_empty(contract.get("required_roles"));
    let observed_scopes = array_or_empty(contract.get("observed_scopes"));
    let observed_permissions = array_or_empty(contract.get("observed_permissions"));
    let observed_roles = array_or_empty(contract.get("observed_roles"));
    let missing_scopes = missing_values(&required_scopes, &observed_scopes);
    let missing_permissions = missing_values(&required_permissions, &observed_permissions);
    // The documented account/organization roles are alternatives: one
    // authorized role is sufficient for the endpoint.
    let role_satisfied = required_roles.is_empty()
        || required_roles
            .iter()
            .any(|required| observed_roles.iter().any(|observed| observed == required));
    let missing_roles = if role_satisfied {
        Vec::new()
    } else {
        required_roles.clone()
    };
    let contract_satisfied = missing_scopes.is_empty()
        && missing_permissions.is_empty()
        && missing_roles.is_empty()
        && (!required_scopes.is_empty()
            || !required_permissions.is_empty()
            || !required_roles.is_empty());
    // Older fixture contexts intentionally predate the permission contract.
    // Keep their existing replay behavior, but label the contract untested so
    // this compatibility path cannot be mistaken for live authorization.
    let complete = !contract_present || contract_satisfied;
    let mut diagnostics = Vec::new();
    if !missing_scopes.is_empty() {
        diagnostics.push("missing required OAuth/classic scope(s)".to_owned());
    }
    if !missing_permissions.is_empty() {
        diagnostics.push("missing required fine-grained permission(s)".to_owned());
    }
    if !missing_roles.is_empty() {
        diagnostics.push("missing required account or organization role(s)".to_owned());
    }
    if diagnostics.is_empty() && !contract_present {
        diagnostics.push("permission contract untested in fixture".to_owned());
    }
    (
        json!({
            "endpoint":contract.get("endpoint").cloned().unwrap_or(Value::Null),
            "required_scopes":required_scopes,
            "required_permissions":required_permissions,
            "required_roles":required_roles,
            "observed_scopes":observed_scopes,
            "observed_permissions":observed_permissions,
            "observed_roles":observed_roles,
            "missing_scopes":missing_scopes,
            "missing_permissions":missing_permissions,
            "missing_roles":missing_roles,
            "status":if !contract_present { "untested_fixture" } else if contract_satisfied { "satisfied_fixture" } else { "missing_or_untested_fixture" },
            "diagnostics":diagnostics,
        }),
        complete,
    )
}

fn meter_boundary(item: &Value, key: &str) -> Value {
    let source = item
        .get(key)
        .cloned()
        .unwrap_or_else(|| json!({"status":"unknown"}));
    json!({
        "status":source.get("status").cloned().unwrap_or_else(||json!("unknown")),
        "included_allowance":source.get("included_allowance").cloned().unwrap_or(Value::Null),
        "used":source.get("used").cloned().unwrap_or(Value::Null),
        "remaining":source.get("remaining").cloned().unwrap_or(Value::Null),
        "unit":source.get("unit").cloned().unwrap_or(Value::Null),
        "billing_period":source.get("billing_period").cloned().unwrap_or(Value::Null),
        "overage":source.get("overage").cloned().unwrap_or_else(||json!({
            "enabled":null,
            "used":null,
            "extra_paid":null,
            "unit":null
        })),
        "scope":source.get("scope").cloned().unwrap_or_else(||json!({
            "pool_kind":"unknown",
            "shared_pool_remaining":null,
            "individual_limit":null
        })),
    })
}

fn normalize(item: &Value) -> Value {
    let permission = item
        .pointer("/billing_api/permission")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let status = match permission {
        "granted" => "available",
        "denied" => "permission_denied",
        "unauthenticated" => "authentication_failed",
        "rate_limited" => "rate_limited",
        "endpoint_unavailable" => "endpoint_unavailable",
        _ => "unknown",
    };
    let preview = item
        .get("preview_sdk")
        .cloned()
        .unwrap_or_else(|| json!({"status":"unknown"}));
    let quota = item.get("quota").cloned().unwrap_or(Value::Null);
    let quota_status = if permission == "denied" {
        "unknown_permission_denied"
    } else {
        quota
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
    };
    let shape_complete = permission == "granted"
        && quota_status == "reported"
        && quota.get("remaining").and_then(Value::as_f64).is_some()
        && quota.get("unit").and_then(Value::as_str).is_some()
        && quota
            .get("billing_period")
            .and_then(Value::as_str)
            .is_some();
    let base_failure_code = match permission {
        "denied" => Some("permission_denied"),
        "unauthenticated" => Some("authentication_failed"),
        "rate_limited" => Some("rate_limited"),
        "endpoint_unavailable" => Some("endpoint_unavailable"),
        _ if permission != "granted" => Some("permission_unknown"),
        _ if quota_status == "schema_changed" => Some("schema_changed"),
        _ => None,
    };
    let availability = match permission {
        "granted" | "rate_limited" => "available",
        "denied" => "permission_denied",
        "unauthenticated" => "needs_login",
        "endpoint_unavailable" => "unsupported",
        _ => "unknown",
    };
    let billing_remaining = quota.get("remaining").and_then(Value::as_f64);
    let preview_remaining = preview.get("remaining").and_then(Value::as_f64);
    let preview_comparison = match (billing_remaining, preview_remaining) {
        (Some(billing), Some(preview)) if (billing - preview).abs() < f64::EPSILON => "matched",
        (Some(_), Some(_)) => "diverged",
        _ => "not_comparable",
    };
    let (permission_requirements, permission_contract_complete) = permission_requirements(item);
    let ai_credits_boundary = meter_boundary(item, "ai_credits");
    let legacy_boundary = meter_boundary(item, "legacy_premium_requests");
    let mut diagnostics = permission_requirements["diagnostics"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    if permission == "granted" && !permission_contract_complete {
        diagnostics.push(json!(
            "billing API permission contract is not satisfied by fixture"
        ));
    }
    if let (Some(ai_unit), Some(legacy_unit)) = (
        ai_credits_boundary["unit"].as_str(),
        legacy_boundary["unit"].as_str(),
    ) && ai_unit == legacy_unit
    {
        diagnostics.push(json!(
            "AI Credits and legacy premium requests must use distinct units"
        ));
    }
    let units_distinct = match (
        ai_credits_boundary["unit"].as_str(),
        legacy_boundary["unit"].as_str(),
    ) {
        (Some(ai_unit), Some(legacy_unit)) => ai_unit != legacy_unit,
        _ => true,
    };
    let complete = shape_complete && permission_contract_complete && units_distinct;
    let failure_code = if base_failure_code.is_some() {
        base_failure_code
    } else if !permission_contract_complete {
        Some("permission_requirements_missing")
    } else if !units_distinct {
        Some("schema_changed")
    } else {
        None
    };
    let collection_state = if complete {
        "ready"
    } else if permission == "rate_limited" {
        "backing_off"
    } else {
        "error"
    };
    let observation = complete.then(|| {
        json!({
            "provider":"copilot",
            "provider_account":{"kind":item.get("account_scope").and_then(Value::as_str).unwrap_or("unknown")},
            "source":{"kind":"billing_api","api_version":item.pointer("/billing_api/api_version").cloned().unwrap_or(Value::Null),"mode":"fixture_replay","replay":true},
            "data_quality":"local_observed",
            "collector_maturity":"experimental",
            "availability":"available",
            "collection_state":"ready",
            "freshness":"unknown",
            "quota_windows":[{"scope":quota["billing_period"],"remaining":quota["remaining"],"limit":quota.get("limit").cloned().unwrap_or(Value::Null),"unit":quota["unit"]}]
        })
    });
    json!({
        "context":item.get("context").and_then(Value::as_str).unwrap_or("unknown"),
        "plan":item.get("plan").cloned().unwrap_or(Value::Null),
        "account_scope":item.get("account_scope").cloned().unwrap_or(Value::Null),
        "billing_api":{"permission":permission,"status":status,"api_version":item.pointer("/billing_api/api_version").cloned().unwrap_or(Value::Null)},
        "permission_requirements":permission_requirements,
        "quota":{"status":quota_status,"raw":quota},
        "observation":observation,
        "availability":availability,
        "collection_state":collection_state,
        "freshness":"unknown",
        "failure_code":failure_code,
        "ai_credits":item.get("ai_credits").cloned().unwrap_or_else(||json!({"status":"unknown"})),
        "legacy_premium_requests":item.get("legacy_premium_requests").cloned().unwrap_or_else(||json!({"status":"unknown"})),
        "quota_boundaries":{"ai_credits":ai_credits_boundary,"legacy_premium_requests":legacy_boundary,"aggregation":"never_sum_across_units"},
        "preview_sdk":{"status":preview.get("status").cloned().unwrap_or_else(||json!("unknown")),"experimental":true,"stability":preview.get("stability").cloned().unwrap_or_else(||json!("unknown")),"permission":preview.get("permission").cloned().unwrap_or_else(||json!("unknown")),"permission_status":preview.get("permission_status").cloned().unwrap_or_else(||json!("unknown")),"sdk_version":preview.get("sdk_version").cloned().unwrap_or(Value::Null),"billing_api_comparison":{"status":preview_comparison,"billing_remaining":billing_remaining,"preview_remaining":preview_remaining},"raw":preview},
        "unknowns":item.get("unknowns").cloned().unwrap_or_else(||json!([])),
        "diagnostics":diagnostics
    })
}

fn parse_live_options(args: &[String]) -> Result<UsageRequest, String> {
    if args.first().map(String::as_str) != Some("collect") {
        return Err(live_usage());
    }
    let mut gh_bin = None;
    let mut context = None;
    let mut account = None;
    let mut meter = None;
    let mut year = None;
    let mut month = None;
    let mut timeout = Duration::from_secs(10);
    let mut index = 1;
    while index < args.len() {
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{} requires a value", args[index]))?;
        match args[index].as_str() {
            "--gh-bin" => gh_bin = Some(value.clone()),
            "--context" => context = Some(value.clone()),
            "--account" => account = Some(value.clone()),
            "--meter" => meter = Some(value.clone()),
            "--year" => {
                let parsed = value
                    .parse::<u16>()
                    .map_err(|_| "--year must be a four-digit year".to_owned())?;
                if !(2000..=9999).contains(&parsed) {
                    return Err("--year must be between 2000 and 9999".into());
                }
                year = Some(parsed);
            }
            "--month" => {
                let parsed = value
                    .parse::<u8>()
                    .map_err(|_| "--month must be between 1 and 12".to_owned())?;
                if !(1..=12).contains(&parsed) {
                    return Err("--month must be between 1 and 12".into());
                }
                month = Some(parsed);
            }
            "--timeout-ms" => {
                let millis = value
                    .parse::<u64>()
                    .map_err(|_| "--timeout-ms must be a positive integer".to_owned())?;
                if millis == 0 || millis > 60_000 {
                    return Err("--timeout-ms must be between 1 and 60000".into());
                }
                timeout = Duration::from_millis(millis);
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        index += 2;
    }
    UsageRequest::new(
        gh_bin.ok_or_else(|| "--gh-bin is required".to_owned())?,
        &context.ok_or_else(|| "--context is required".to_owned())?,
        &account.ok_or_else(|| "--account is required".to_owned())?,
        &meter.ok_or_else(|| "--meter is required".to_owned())?,
        year,
        month,
        timeout,
    )
    .map_err(|failure| failure.message)
}

fn live_usage() -> String {
    "usage: agentmeter-copilot-p0 collect --gh-bin <absolute-path> --context <personal|business|enterprise> --account <slug> --meter <ai-credits|premium-requests> [--year <YYYY>] [--month <1-12>] [--timeout-ms <1-60000>]".into()
}

fn emit_failure(code: &str, message: &str) -> ExitCode {
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"schema_version":"copilot-p0/v1","outcome":"failure","observation":Value::Null,"availability":"unsupported","collection_state":"error","freshness":"unknown","failure_code":code,"message":message})).unwrap()
    );
    ExitCode::from(1)
}
fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().map(String::as_str) == Some("collect") {
        let options = match parse_live_options(&args) {
            Ok(options) => options,
            Err(message) => return emit_failure("invalid_arguments", &message),
        };
        return match collect_usage(options, &AtomicBool::new(false)) {
            Ok(report) => {
                println!("{}", serde_json::to_string_pretty(&report).unwrap());
                ExitCode::SUCCESS
            }
            Err(failure) => emit_failure(failure.code, &failure.message),
        };
    }
    let input: Value = if args.first().map(String::as_str) == Some("--fixture") {
        let Some(path) = args.get(1) else {
            return emit_failure("command_failed", "fixture path is required");
        };
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) => return emit_failure("command_failed", &e.to_string()),
        };
        match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return emit_failure("malformed_response", &e.to_string()),
        }
    } else {
        let mut s = String::new();
        if io::stdin().read_to_string(&mut s).is_err() {
            return emit_failure("command_failed", "could not read stdin");
        };
        match serde_json::from_str(&s) {
            Ok(v) => v,
            Err(e) => return emit_failure("malformed_response", &e.to_string()),
        }
    };
    let Some(contexts) = input.get("contexts").and_then(Value::as_array) else {
        return emit_failure("schema_changed", "contexts array is required");
    };
    let xs = contexts.iter().map(normalize).collect::<Vec<_>>();
    println!("{}",serde_json::to_string_pretty(&json!({"schema_version":"copilot-p0/v1","outcome":"success","capability":"copilot_quota_boundaries","contexts":xs,"unknowns":input.get("unknowns").cloned().unwrap_or_else(||json!([])),"evidence":input.get("evidence").cloned().unwrap_or_else(||json!([]))})).unwrap());
    ExitCode::SUCCESS
}
