use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::model::{
    Availability, CollectionReport, CollectionState, CollectorMaturity, CreditsSnapshot,
    DataQuality, Diagnostic, FailureReport, Freshness, Observation, ProviderAccount, QuotaWindow,
    Source,
};

#[derive(Clone, Copy)]
enum CollectionMode {
    Fixture,
    Live,
}

#[derive(Clone, Copy)]
enum ErrorKind {
    Authentication,
    MethodUnsupported,
    Other,
}

pub fn collect_fixture(path: &Path) -> Result<CollectionReport, Box<FailureReport>> {
    let file = File::open(path).map_err(|error| format!("fixture_unavailable: {error}"))?;
    let mut responses = BTreeMap::new();

    for (index, line) in BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| format!("fixture_unreadable: {error}"))?;
        let value: Value = serde_json::from_str(&line)
            .map_err(|error| format!("malformed_response at line {}: {error}", index + 1))?;
        let id = value.get("id").and_then(Value::as_i64).ok_or_else(|| {
            format!(
                "malformed_response at line {}: missing numeric id",
                index + 1
            )
        })?;
        responses.insert(id, value);
    }

    normalize_with_failure_evidence(responses, CollectionMode::Fixture)
}

pub fn collect_live(
    codex_bin: &Path,
    timeout: Duration,
) -> Result<CollectionReport, Box<FailureReport>> {
    match collect_live_once(codex_bin, timeout) {
        Ok(report) => Ok(report),
        Err(first) if first.failure_code == "process_exited" => {
            collect_live_once(codex_bin, timeout)
        }
        Err(error) => Err(error),
    }
}

fn collect_live_once(
    codex_bin: &Path,
    timeout: Duration,
) -> Result<CollectionReport, Box<FailureReport>> {
    let mut child = Command::new(codex_bin)
        .args(["app-server", "--stdio"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("process_exited: could not start app-server: {error}"))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "process_exited: app-server stdin unavailable".to_owned())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "process_exited: app-server stdout unavailable".to_owned())?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let value = line
                .map_err(|error| format!("process_exited: app-server stdout failed: {error}"))
                .and_then(|line| {
                    serde_json::from_str::<Value>(&line)
                        .map_err(|error| format!("malformed_response: {error}"))
                });
            if sender.send(value).is_err() {
                break;
            }
        }
    });

    let result: Result<BTreeMap<i64, Value>, String> = (|| {
        let mut responses = BTreeMap::new();
        send(
            &mut stdin,
            serde_json::json!({
                "id": 1,
                "method": "initialize",
                "params": {
                    "clientInfo": {
                        "name": "agentmeter-p0",
                        "title": "AgentMeter P0 Collector",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "capabilities": { "experimentalApi": false }
                }
            }),
        )?;
        responses.insert(1, receive(&receiver, 1, "initialize", timeout)?);
        if responses[&1].get("error").is_some() {
            return Ok(responses);
        }

        send(&mut stdin, serde_json::json!({ "method": "initialized" }))?;
        send(
            &mut stdin,
            serde_json::json!({
                "id": 2,
                "method": "account/read",
                "params": { "refreshToken": false }
            }),
        )?;
        responses.insert(2, receive(&receiver, 2, "account/read", timeout)?);
        if responses[&2].get("error").is_some() {
            return Ok(responses);
        }

        send(
            &mut stdin,
            serde_json::json!({
                "id": 3,
                "method": "account/rateLimits/read"
            }),
        )?;
        responses.insert(
            3,
            receive(&receiver, 3, "account/rateLimits/read", timeout)?,
        );
        if responses[&3].get("error").is_some() {
            return Ok(responses);
        }

        send(
            &mut stdin,
            serde_json::json!({
                "id": 4,
                "method": "account/usage/read",
                "params": {}
            }),
        )?;
        responses.insert(4, receive(&receiver, 4, "account/usage/read", timeout)?);
        Ok(responses)
    })();

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    normalize_with_failure_evidence(result.map_err(FailureReport::from)?, CollectionMode::Live)
}

fn send(stdin: &mut impl Write, message: Value) -> Result<(), String> {
    serde_json::to_writer(&mut *stdin, &message)
        .map_err(|error| format!("process_exited: request serialization failed: {error}"))?;
    stdin
        .write_all(b"\n")
        .and_then(|_| stdin.flush())
        .map_err(|error| format!("process_exited: request write failed: {error}"))
}

fn receive(
    receiver: &mpsc::Receiver<Result<Value, String>>,
    expected_id: i64,
    method: &str,
    timeout: Duration,
) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("timeout: no response for {method}"));
        }
        match receiver.recv_timeout(remaining) {
            Ok(Ok(value)) if value.get("id").and_then(Value::as_i64) == Some(expected_id) => {
                return Ok(value);
            }
            Ok(Ok(_notification_or_other_response)) => continue,
            Ok(Err(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err(format!("timeout: no response for {method}"));
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!("process_exited: before response for {method}"));
            }
        }
    }
}

fn normalize_with_failure_evidence(
    responses: BTreeMap<i64, Value>,
    mode: CollectionMode,
) -> Result<CollectionReport, Box<FailureReport>> {
    normalize(&responses, mode).map_err(|error| Box::new(failure_with_evidence(error, &responses)))
}

fn normalize(
    responses: &BTreeMap<i64, Value>,
    mode: CollectionMode,
) -> Result<CollectionReport, String> {
    let initialize = result(responses, 1, "initialize")?;
    let account = result(responses, 2, "account/read")?;
    let rate_limits = result(responses, 3, "account/rateLimits/read")?;

    let mut capabilities = BTreeMap::from([
        ("account/read".to_owned(), "supported"),
        ("account/rateLimits/read".to_owned(), "supported"),
    ]);
    let mut diagnostics = Vec::new();
    match responses.get(&4).and_then(|response| response.get("error")) {
        Some(error) if error.get("code").and_then(Value::as_i64) == Some(-32601) => {
            capabilities.insert("account/usage/read".to_owned(), "unsupported");
            diagnostics.push(Diagnostic {
                code: "method_unsupported",
                message: "account/usage/read is not supported by this app-server version".into(),
            });
        }
        Some(_) => {
            capabilities.insert("account/usage/read".to_owned(), "error");
        }
        None => {
            capabilities.insert("account/usage/read".to_owned(), "supported");
        }
    }

    let app_server_version = initialize
        .get("userAgent")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let account_value = account.get("account").unwrap_or(&Value::Null);
    let account_kind = account_value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_owned();
    let plan_type = account_value
        .get("planType")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let display_hint = account_value
        .get("email")
        .and_then(Value::as_str)
        .map(mask_email);

    let mut quota_windows = Vec::new();
    let mut credits = Vec::new();
    match rate_limits
        .get("rateLimitsByLimitId")
        .and_then(Value::as_object)
        .filter(|snapshots| !snapshots.is_empty())
    {
        Some(snapshots) => {
            for snapshot in snapshots.values() {
                append_window(&mut quota_windows, snapshot, "primary")?;
                append_window(&mut quota_windows, snapshot, "secondary")?;
                append_credits(&mut credits, snapshot)?;
            }
        }
        None => {
            let snapshot = rate_limits
                .get("rateLimits")
                .ok_or_else(|| "schema_changed: rateLimits is missing".to_owned())?;
            append_window(&mut quota_windows, snapshot, "primary")?;
            append_window(&mut quota_windows, snapshot, "secondary")?;
            append_credits(&mut credits, snapshot)?;
        }
    }
    let source_usage = responses
        .get(&4)
        .and_then(|response| response.get("result"))
        .cloned();
    let source_timestamp = source_usage
        .as_ref()
        .and_then(|usage| usage.get("sourceTimestamp"))
        .and_then(Value::as_i64)
        .or_else(|| rate_limits.get("sourceTimestamp").and_then(Value::as_i64));

    Ok(CollectionReport {
        schema_version: "agentmeter.p0.collection-report.v1",
        outcome: "success",
        app_server_version: app_server_version.clone(),
        capabilities,
        observation: Observation {
            provider: "codex",
            provider_account: ProviderAccount {
                kind: account_kind,
                plan_type,
                display_hint,
            },
            source: Source {
                kind: "app_server",
                version: app_server_version,
                mode: match mode {
                    CollectionMode::Fixture => "fixture_replay",
                    CollectionMode::Live => "live",
                },
                replay: matches!(mode, CollectionMode::Fixture),
            },
            quota_windows,
            credits,
            source_usage,
            data_quality: match mode {
                CollectionMode::Fixture => DataQuality::LocalObserved,
                CollectionMode::Live => DataQuality::Official,
            },
            collector_maturity: CollectorMaturity::Experimental,
            availability: Availability::Available,
            collection_state: CollectionState::Ready,
            freshness: match mode {
                CollectionMode::Fixture => Freshness::Unknown,
                CollectionMode::Live => Freshness::Fresh,
            },
            source_timestamp,
            collected_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|error| format!("clock_error: {error}"))?
                .as_millis(),
        },
        diagnostics,
    })
}

fn failure_with_evidence(error: String, responses: &BTreeMap<i64, Value>) -> FailureReport {
    let app_server_version = responses
        .get(&1)
        .and_then(|response| response.get("result"))
        .and_then(|result| result.get("userAgent"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let provider_account_kind = responses
        .get(&2)
        .and_then(|response| response.get("result"))
        .and_then(|result| result.get("account"))
        .and_then(|account| account.get("type"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let capabilities = BTreeMap::from([
        (
            "account/read".to_owned(),
            capability_status(responses.get(&2)),
        ),
        (
            "account/rateLimits/read".to_owned(),
            capability_status(responses.get(&3)),
        ),
        (
            "account/usage/read".to_owned(),
            capability_status(responses.get(&4)),
        ),
    ]);

    FailureReport::from_error(error).with_evidence(
        app_server_version,
        provider_account_kind,
        capabilities,
    )
}

fn capability_status(response: Option<&Value>) -> String {
    let Some(response) = response else {
        return "not_attempted".to_owned();
    };
    if response.get("result").is_some() {
        return "supported".to_owned();
    }
    let Some(error) = response.get("error") else {
        return "schema_changed".to_owned();
    };
    match classify_error(error) {
        ErrorKind::MethodUnsupported => "unsupported".to_owned(),
        ErrorKind::Authentication => "authentication_required".to_owned(),
        ErrorKind::Other => "error".to_owned(),
    }
}

fn classify_error(error: &Value) -> ErrorKind {
    if error.get("code").and_then(Value::as_i64) == Some(-32601) {
        return ErrorKind::MethodUnsupported;
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if message.contains("auth") || message.contains("login") {
        ErrorKind::Authentication
    } else {
        ErrorKind::Other
    }
}

fn result<'a>(
    responses: &'a BTreeMap<i64, Value>,
    id: i64,
    method: &str,
) -> Result<&'a Value, String> {
    let response = responses
        .get(&id)
        .ok_or_else(|| format!("timeout: no response for {method}"))?;
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown app-server error");
        match classify_error(error) {
            ErrorKind::Authentication => {
                return Err(format!("authentication_failed: {method}: {message}"));
            }
            ErrorKind::MethodUnsupported => {
                return Err(format!("method_unsupported: {method}: {message}"));
            }
            ErrorKind::Other => {}
        }
        return Err(format!("request_failed for {method}: {error}"));
    }
    response
        .get("result")
        .ok_or_else(|| format!("malformed_response for {method}: missing result"))
}

fn append_window(
    windows: &mut Vec<QuotaWindow>,
    snapshot: &Value,
    window_name: &'static str,
) -> Result<(), String> {
    let Some(window) = snapshot.get(window_name).filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let used_percent = window
        .get("usedPercent")
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("schema_changed: {window_name}.usedPercent is missing"))?;
    if used_percent < 0 {
        return Err(format!(
            "schema_changed: {window_name}.usedPercent cannot be negative"
        ));
    }

    windows.push(QuotaWindow {
        limit_id: optional_string(snapshot, "limitId"),
        limit_name: optional_string(snapshot, "limitName"),
        window: window_name,
        scope: "provider_account",
        used_percent,
        remaining_percent: (100 - used_percent).max(0),
        over_limit: used_percent > 100,
        window_duration_mins: optional_i64(window, "windowDurationMins"),
        resets_at: optional_i64(window, "resetsAt"),
        unit: "percent",
    });
    Ok(())
}

fn append_credits(credits: &mut Vec<CreditsSnapshot>, snapshot: &Value) -> Result<(), String> {
    let Some(value) = snapshot.get("credits").filter(|value| !value.is_null()) else {
        return Ok(());
    };
    let has_credits = value
        .get("hasCredits")
        .and_then(Value::as_bool)
        .ok_or_else(|| "schema_changed: credits.hasCredits is missing".to_owned())?;
    let unlimited = value
        .get("unlimited")
        .and_then(Value::as_bool)
        .ok_or_else(|| "schema_changed: credits.unlimited is missing".to_owned())?;
    let balance = value
        .get("balance")
        .filter(|value| !value.is_null())
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    credits.push(CreditsSnapshot {
        limit_id: optional_string(snapshot, "limitId"),
        has_credits,
        unlimited,
        balance,
        unit: "credits",
    });
    Ok(())
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn optional_i64(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn mask_email(email: &str) -> String {
    let Some((local, domain)) = email.split_once('@') else {
        return "***".to_owned();
    };
    let first = local.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
}
