#![allow(clippy::collapsible_if)]
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize)]
struct Report {
    schema_version: &'static str,
    outcome: &'static str,
    observation: Option<Observation>,
    capabilities: BTreeMap<String, &'static str>,
    diagnostics: Vec<Diagnostic>,
    evidence: Evidence,
}
#[derive(Debug, Serialize)]
struct Observation {
    provider: &'static str,
    provider_account: Account,
    source: Source,
    quota_windows: Vec<QuotaWindow>,
    data_quality: &'static str,
    collector_maturity: &'static str,
    availability: &'static str,
    collection_state: &'static str,
    freshness: &'static str,
    source_timestamp: Option<i64>,
    collected_at_unix_ms: u128,
}
#[derive(Debug, Serialize)]
struct Account {
    kind: String,
    scope: Option<String>,
}
#[derive(Debug, Serialize)]
struct Source {
    kind: &'static str,
    version: Option<String>,
    mode: &'static str,
    replay: bool,
}
#[derive(Debug, Serialize)]
struct QuotaWindow {
    limit_id: Option<String>,
    limit_name: Option<String>,
    window: String,
    scope: String,
    limit: Option<f64>,
    used: Option<f64>,
    remaining: Option<f64>,
    used_percent: Option<i64>,
    remaining_percent: Option<i64>,
    resets_at: Option<i64>,
    unit: Option<String>,
    over_limit: bool,
}
#[derive(Debug, Serialize)]
struct Diagnostic {
    code: String,
    message: String,
}
#[derive(Debug, Serialize)]
struct Evidence {
    tested_version: Option<String>,
    version_evidence: &'static str,
    tested_at: &'static str,
    source_precedence: Vec<&'static str>,
    fallback_release_policy: &'static str,
    limitations: &'static str,
    replay: bool,
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() < 2 || args[0] != "antigravity" || args[1] != "collect" {
        eprintln!(
            "usage: agentmeter-antigravity-p0 antigravity collect --fixture <file> [--version <version>]"
        );
        return ExitCode::from(2);
    }
    let mut fixture = None;
    let mut version = None;
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--fixture" if i + 1 < args.len() => {
                fixture = Some(args[i + 1].clone());
                i += 2;
            }
            "--version" if i + 1 < args.len() => {
                version = Some(args[i + 1].clone());
                i += 2;
            }
            _ => {
                return emit_failure("command_failed", "invalid arguments");
            }
        }
    }
    let Some(path) = fixture else {
        return emit_failure("command_failed", "a fixture is required");
    };
    let input = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => return emit_failure("command_failed", &e.to_string()),
    };
    match collect(&input, version) {
        Ok(report) => {
            serde_json::to_writer_pretty(std::io::stdout(), &report).unwrap();
            println!();
            ExitCode::SUCCESS
        }
        Err((code, msg, version, replay)) => {
            let report = failure_report(code, msg, version, replay);
            serde_json::to_writer_pretty(std::io::stdout(), &report).unwrap();
            println!();
            ExitCode::from(1)
        }
    }
}
fn emit_failure(code: &'static str, message: &str) -> ExitCode {
    let report = failure_report(code.to_owned(), message.to_owned(), None, false);
    serde_json::to_writer_pretty(std::io::stdout(), &report).unwrap();
    println!();
    ExitCode::from(1)
}
fn failure_report(code: String, message: String, version: Option<String>, replay: bool) -> Report {
    Report {
        schema_version: "agentmeter.p0.collection-report.v1",
        outcome: "failure",
        observation: None,
        capabilities: BTreeMap::new(),
        diagnostics: vec![Diagnostic { code, message }],
        evidence: Evidence {
            tested_version: version,
            version_evidence: "fixture_declared",
            tested_at: "2026-09-07T00:00:00+08:00",
            source_precedence: vec!["structured_status_line", "headless_text_experimental"],
            fallback_release_policy: "prohibited_pending_real_version_validation",
            limitations: "fixture parser evidence; not a real Antigravity installation or account",
            replay,
        },
    }
}
fn collect(
    input: &str,
    cli_version: Option<String>,
) -> Result<Report, (String, String, Option<String>, bool)> {
    let value: Value = serde_json::from_str(input).map_err(|e| {
        (
            "malformed_response".into(),
            e.to_string(),
            cli_version.clone(),
            true,
        )
    })?;
    let fixture_version = value
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let version = cli_version.or(fixture_version);
    let replay = true;
    if let Some(kind) = value
        .pointer("/provider_error/kind")
        .and_then(Value::as_str)
    {
        let code = match kind {
            "authentication_required" => "authentication_failed",
            "timeout" => "timeout",
            "command_failed" => "command_failed",
            _ => "schema_changed",
        };
        return Err((
            code.into(),
            value
                .pointer("/provider_error/message")
                .and_then(Value::as_str)
                .unwrap_or("provider command failed")
                .into(),
            version,
            replay,
        ));
    }
    if let Some(schema) = value.get("schema_version").and_then(Value::as_str) {
        if schema != "antigravity.statusline.v1" {
            return Err((
                "schema_changed".into(),
                format!("unsupported structured schema: {schema}"),
                version,
                replay,
            ));
        }
    }
    if let Some(data) = value
        .get("statusLine")
        .or_else(|| value.get("status_line"))
        .or_else(|| value.get("structured"))
    {
        return normalize_structured(data, version, replay);
    }
    if let Some(text) = value.get("headless_text").and_then(Value::as_str) {
        let locale = value.get("locale").and_then(Value::as_str);
        let format_revision = value.get("format_revision").and_then(Value::as_str);
        return normalize_text(text, version, locale, format_revision, replay);
    }
    Err((
        "schema_changed".into(),
        "neither structured statusLine nor headless text was found".into(),
        version,
        replay,
    ))
}
fn normalize_structured(
    data: &Value,
    version: Option<String>,
    replay: bool,
) -> Result<Report, (String, String, Option<String>, bool)> {
    let account = data
        .get("account")
        .and_then(Value::as_object)
        .and_then(|a| a.get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let raw = data
        .get("quota")
        .or_else(|| data.get("quotas"))
        .ok_or_else(|| {
            (
                "schema_changed".into(),
                "structured statusLine quota map or array missing".into(),
                version.clone(),
                replay,
            )
        })?;
    let entries: Vec<(Option<&str>, &Value)> = match raw {
        Value::Array(items) => items.iter().map(|item| (None, item)).collect(),
        Value::Object(items) => items
            .iter()
            .map(|(bucket, item)| (Some(bucket.as_str()), item))
            .collect(),
        _ => {
            return Err((
                "schema_changed".into(),
                "structured statusLine quota is neither a map nor an array".into(),
                version,
                replay,
            ));
        }
    };
    let mut windows = Vec::new();
    let mut diagnostics = Vec::new();
    if account == "unknown" {
        diagnostics.push(Diagnostic {
            code: "account_scope_unknown".into(),
            message: "structured statusLine did not identify a Provider Account".into(),
        });
    }
    for (bucket, item) in entries {
        let Some(obj) = item.as_object() else {
            diagnostics.push(Diagnostic {
                code: "schema_changed".into(),
                message: "quota entry is not an object".into(),
            });
            continue;
        };
        let window = string(obj.get("window").or_else(|| obj.get("window_name")))
            .or_else(|| bucket.map(str::to_owned))
            .unwrap_or_else(|| "unknown".into());
        let scope = string(obj.get("scope")).unwrap_or_else(|| "account".into());
        let limit = number(obj.get("limit"));
        let used = number(obj.get("used"));
        let remaining = number(obj.get("remaining"));
        if limit.is_none() && used.is_none() && remaining.is_none() {
            diagnostics.push(Diagnostic {
                code: "missing_quota_values".into(),
                message: format!("{window} has no numeric quota value"),
            });
        }
        let used_percent = percent(used, limit)
            .or_else(|| number(obj.get("used_percent")).map(|x| x.round() as i64));
        let remaining_percent = percent(remaining, limit)
            .or_else(|| number(obj.get("remaining_percent")).map(|x| x.round() as i64));
        windows.push(QuotaWindow {
            limit_id: string(obj.get("id").or_else(|| obj.get("limit_id")))
                .or_else(|| bucket.map(str::to_owned)),
            limit_name: string(obj.get("name")),
            window,
            scope,
            limit,
            used,
            remaining,
            used_percent,
            remaining_percent,
            resets_at: timestamp(obj.get("reset_at").or_else(|| obj.get("resets_at"))),
            unit: string(obj.get("unit")),
            over_limit: remaining.map(|x| x < 0.0).unwrap_or(false)
                || used.zip(limit).map(|(u, l)| u > l).unwrap_or(false),
        });
    }
    if windows.is_empty() {
        return Err((
            "schema_changed".into(),
            "structured statusLine contained no quota windows".into(),
            version,
            replay,
        ));
    }
    let mut capabilities = BTreeMap::new();
    capabilities.insert("statusLine/structured_quota".into(), "supported");
    capabilities.insert("headless_text_quota".into(), "experimental");
    let has_quota_value = windows.iter().any(|window| {
        window.limit.is_some()
            || window.used.is_some()
            || window.remaining.is_some()
            || window.used_percent.is_some()
            || window.remaining_percent.is_some()
    });
    Ok(Report {
        schema_version: "agentmeter.p0.collection-report.v1",
        outcome: "success",
        observation: Some(Observation {
            provider: "antigravity",
            provider_account: Account {
                kind: account,
                scope: string(data.get("scope")),
            },
            source: Source {
                kind: "status_line",
                version: version.clone(),
                mode: "fixture_replay",
                replay,
            },
            quota_windows: windows,
            data_quality: "local_observed",
            collector_maturity: "experimental",
            availability: "available",
            collection_state: if has_quota_value { "ready" } else { "idle" },
            freshness: "unknown",
            source_timestamp: timestamp(data.get("timestamp")),
            collected_at_unix_ms: now_ms(),
        }),
        capabilities,
        diagnostics,
        evidence: Evidence {
            tested_version: version,
            version_evidence: "fixture_declared",
            tested_at: "2026-09-07T00:00:00+08:00",
            source_precedence: vec!["structured_status_line", "headless_text_experimental"],
            fallback_release_policy: "prohibited_pending_real_version_validation",
            limitations: "fixture parser evidence; not a real Antigravity installation or account",
            replay,
        },
    })
}
fn normalize_text(
    text: &str,
    version: Option<String>,
    locale: Option<&str>,
    format_revision: Option<&str>,
    replay: bool,
) -> Result<Report, (String, String, Option<String>, bool)> {
    let Some(v) = version.as_deref() else {
        return Err((
            "version_required".into(),
            "headless text fallback requires an explicit product version".into(),
            version,
            replay,
        ));
    };
    if v != "1.8.2" {
        return Err((
            "unsupported_version".into(),
            format!("headless text fallback is validated only for 1.8.2, not {v}"),
            version,
            replay,
        ));
    }
    if locale != Some("en-US") {
        return Err((
            "localization_unsupported".into(),
            format!(
                "headless text fallback is validated only for en-US, not {}",
                locale.unwrap_or("unknown")
            ),
            version,
            replay,
        ));
    }
    if format_revision != Some("1") {
        return Err((
            "text_changed".into(),
            format!(
                "headless text format revision {} is not the validated revision 1",
                format_revision.unwrap_or("unknown")
            ),
            version,
            replay,
        ));
    }
    let mut windows = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 && parts[1].ends_with('%') {
            if let Ok(rem) = parts[1].trim_end_matches('%').parse::<i64>() {
                windows.push(QuotaWindow {
                    limit_id: None,
                    limit_name: None,
                    window: parts[0].into(),
                    scope: "account".into(),
                    limit: None,
                    used: None,
                    remaining: None,
                    used_percent: Some(100 - rem),
                    remaining_percent: Some(rem),
                    resets_at: None,
                    unit: Some("percent".into()),
                    over_limit: rem < 0,
                });
            }
        }
    }
    if windows.is_empty() {
        return Err((
            "unexpected_output".into(),
            "headless output did not match the validated 1.8.2 en-US format".into(),
            version,
            replay,
        ));
    }
    let mut capabilities = BTreeMap::new();
    capabilities.insert("statusLine/structured_quota".into(), "unsupported");
    capabilities.insert("headless_text_quota".into(), "experimental");
    Ok(Report {
        schema_version: "agentmeter.p0.collection-report.v1",
        outcome: "success",
        observation: Some(Observation {
            provider: "antigravity",
            provider_account: Account {
                kind: "unknown".into(),
                scope: None,
            },
            source: Source {
                kind: "headless_text",
                version: version.clone(),
                mode: "fixture_replay",
                replay,
            },
            quota_windows: windows,
            data_quality: "estimated",
            collector_maturity: "experimental",
            availability: "available",
            collection_state: "ready",
            freshness: "unknown",
            source_timestamp: None,
            collected_at_unix_ms: now_ms(),
        }),
        capabilities,
        diagnostics: vec![Diagnostic {
            code: "experimental_fallback".into(),
            message: "parsed headless text is lower trust than structured statusLine data".into(),
        }],
        evidence: Evidence {
            tested_version: version,
            version_evidence: "fixture_declared",
            tested_at: "2026-09-07T00:00:00+08:00",
            source_precedence: vec!["structured_status_line", "headless_text_experimental"],
            fallback_release_policy: "prohibited_pending_real_version_validation",
            limitations: "fixture parser evidence; not a real Antigravity installation or account",
            replay,
        },
    })
}
fn string(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_owned)
}
fn number(v: Option<&Value>) -> Option<f64> {
    v.and_then(|x| x.as_f64().or_else(|| x.as_i64().map(|n| n as f64)))
}
fn percent(v: Option<f64>, l: Option<f64>) -> Option<i64> {
    v.zip(l)
        .filter(|(_, l)| *l > 0.0)
        .map(|(v, l)| (v / l * 100.0).round() as i64)
}
fn timestamp(v: Option<&Value>) -> Option<i64> {
    v.and_then(|x| x.as_i64().or_else(|| x.as_str()?.parse().ok()))
}
fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}
