use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::model::{
    Availability, CollectionReport, CollectionState, CollectorMaturity, CreditsSnapshot,
    DataQuality, Diagnostic, FailureReport, Freshness, Observation, ProviderAccount, QuotaWindow,
    Source,
};

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[test]
    fn pre_cancelled_collection_never_attempts_spawn() {
        let result = collect_live_cancellable(
            Path::new("missing-test-executable"),
            Duration::from_secs(5),
            &AtomicBool::new(true),
        );
        assert_eq!(result.unwrap_err().failure_code, "cancelled");
    }
    #[test]
    fn cancellation_interrupts_a_pending_response_without_waiting_for_deadline() {
        let (_sender, receiver) = mpsc::channel();
        let cancelled = std::sync::Arc::new(AtomicBool::new(false));
        let worker_cancelled = std::sync::Arc::clone(&cancelled);
        let worker = thread::spawn(move || {
            receive(
                &receiver,
                1,
                "initialize",
                Duration::from_secs(30),
                &worker_cancelled,
            )
        });
        cancelled.store(true, Ordering::Release);
        assert!(
            worker
                .join()
                .unwrap()
                .unwrap_err()
                .starts_with("cancelled:")
        );
    }
}

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
    collect_live_cancellable(codex_bin, timeout, &AtomicBool::new(false))
}

pub fn collect_live_cancellable(
    codex_bin: &Path,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<CollectionReport, Box<FailureReport>> {
    match collect_live_once(codex_bin, timeout, cancelled) {
        Ok(report) => Ok(report),
        Err(first) if first.failure_code == "process_exited" => {
            collect_live_once(codex_bin, timeout, cancelled)
        }
        Err(error) => Err(error),
    }
}

fn collect_live_once(
    codex_bin: &Path,
    timeout: Duration,
    cancelled: &AtomicBool,
) -> Result<CollectionReport, Box<FailureReport>> {
    if cancelled.load(Ordering::Acquire) {
        return Err("cancelled: collection stopped".to_owned().into());
    }
    let mut command = Command::new(codex_bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW for background collection.
    }
    let mut child = command
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
        let _ = sender.send(Err("process_exited: app-server stdout closed".to_owned()));
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
        responses.insert(1, receive(&receiver, 1, "initialize", timeout, cancelled)?);
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
        responses.insert(
            2,
            receive(&receiver, 2, "account/read", timeout, cancelled)?,
        );
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
            receive(&receiver, 3, "account/rateLimits/read", timeout, cancelled)?,
        );

        send(
            &mut stdin,
            serde_json::json!({
                "id": 4,
                "method": "account/usage/read",
                "params": {}
            }),
        )?;
        match receive(&receiver, 4, "account/usage/read", timeout, cancelled) {
            Ok(response) => {
                responses.insert(4, response);
            }
            Err(error) => {
                responses.insert(
                    4,
                    serde_json::json!({
                        "id": 4,
                        "error": {"code": -32099, "message": error}
                    }),
                );
            }
        }
        Ok(responses)
    })();

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
    if cancelled.load(Ordering::Acquire) {
        return Err("cancelled: collection stopped".to_owned().into());
    }
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
    cancelled: &AtomicBool,
) -> Result<Value, String> {
    let deadline = Instant::now() + timeout;
    loop {
        if cancelled.load(Ordering::Acquire) {
            return Err("cancelled: collection stopped".to_owned());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(format!("timeout: no response for {method}"));
        }
        match receiver.recv_timeout(remaining.min(Duration::from_millis(50))) {
            Ok(Ok(value)) if value.get("id").and_then(Value::as_i64) == Some(expected_id) => {
                return Ok(value);
            }
            Ok(Ok(_notification_or_other_response)) => continue,
            Ok(Err(error)) => return Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
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
    let usage_response = responses.get(&4);
    let (usage_capability, usage_diagnostic) = classify_optional_usage_response(usage_response);
    capabilities.insert("account/usage/read".to_owned(), usage_capability);
    if let Some(diagnostic) = usage_diagnostic {
        diagnostics.push(diagnostic);
    }

    let app_server_version = initialize
        .get("userAgent")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let account_value = account.get("account").unwrap_or(&Value::Null);
    let account_kind = account_value
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| "schema_changed: account.type is missing or invalid".to_owned())?
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
        .filter(|usage| usage.is_object())
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

fn classify_optional_usage_response(
    response: Option<&Value>,
) -> (&'static str, Option<Diagnostic>) {
    let Some(response) = response else {
        return (
            "timed_out",
            Some(Diagnostic {
                code: "timeout",
                message: "account/usage/read produced no response".into(),
            }),
        );
    };
    if let Some(result) = response.get("result") {
        if result.is_object() {
            return ("supported", None);
        }
        return (
            "schema_changed",
            Some(Diagnostic {
                code: "schema_changed",
                message: "account/usage/read result is not an object".into(),
            }),
        );
    }
    let Some(error) = response.get("error") else {
        return (
            "schema_changed",
            Some(Diagnostic {
                code: "schema_changed",
                message: "account/usage/read response has neither result nor error".into(),
            }),
        );
    };
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("unknown account/usage/read error")
        .to_owned();
    let lowercase = message.to_ascii_lowercase();
    let (capability, code) = if error.get("code").and_then(Value::as_i64) == Some(-32601) {
        ("unsupported", "method_unsupported")
    } else if lowercase.contains("auth") || lowercase.contains("login") {
        ("authentication_required", "authentication_failed")
    } else if lowercase.contains("permission") || lowercase.contains("forbidden") {
        ("permission_denied", "permission_denied")
    } else if lowercase.contains("timeout") {
        ("timed_out", "timeout")
    } else if lowercase.contains("schema") || lowercase.contains("malformed") {
        ("schema_changed", "schema_changed")
    } else if lowercase.contains("process_exited") || lowercase.contains("process exited") {
        ("process_exited", "process_exited")
    } else if lowercase.contains("rate limit") || lowercase.contains("429") {
        ("rate_limited", "rate_limited")
    } else {
        ("error", "request_failed")
    };
    (
        capability,
        Some(Diagnostic {
            code,
            message: format!("account/usage/read: {message}"),
        }),
    )
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
    let used_percent = window.get("usedPercent").and_then(Value::as_i64);
    let remaining_percent = window.get("remainingPercent").and_then(Value::as_i64);
    let (used_percent, display_remaining_percent, raw_remaining_percent) =
        match (used_percent, remaining_percent) {
            (Some(used), Some(remaining)) if used >= 0 && remaining == 100 - used => {
                (Some(used), Some(remaining.max(0)), Some(remaining))
            }
            (Some(used), None) if used >= 0 => (Some(used), Some((100 - used).max(0)), None),
            (None, Some(remaining)) if (0..=100).contains(&remaining) => {
                (Some(100 - remaining), Some(remaining), Some(remaining))
            }
            (None, None) => (None, None, None),
            (Some(_), Some(_)) => {
                return Err(format!(
                    "schema_changed: {window_name}.usedPercent and remainingPercent disagree"
                ));
            }
            (Some(_), None) | (None, Some(_)) => {
                return Err(format!(
                    "schema_changed: {window_name} percentage is invalid or missing"
                ));
            }
        };
    windows.push(QuotaWindow {
        limit_id: optional_string(snapshot, "limitId"),
        limit_name: optional_string(snapshot, "limitName"),
        window: window_name,
        scope: optional_string(window, "scope")
            .or_else(|| optional_string(snapshot, "scope"))
            .unwrap_or_else(|| "provider_account".to_owned()),
        used_percent,
        remaining_percent: display_remaining_percent,
        raw_remaining_percent,
        over_limit: used_percent.is_some_and(|used| used > 100),
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
