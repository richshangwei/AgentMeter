use std::process::Command;

use serde_json::Value;

#[test]
fn fixture_collection_reports_trusted_codex_observation() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/success.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(
        report["schema_version"],
        "agentmeter.p0.collection-report.v1"
    );
    assert_eq!(report["outcome"], "success");
    assert_eq!(report["observation"]["provider"], "codex");
    assert_eq!(report["observation"]["provider_account"]["kind"], "chatgpt");
    assert_eq!(report["observation"]["source"]["mode"], "fixture_replay");
    assert_eq!(report["observation"]["source"]["replay"], true);
    assert_eq!(report["observation"]["data_quality"], "local_observed");
    assert_eq!(report["observation"]["collector_maturity"], "experimental");
    assert_eq!(report["observation"]["availability"], "available");
    assert_eq!(report["observation"]["collection_state"], "ready");
    assert_eq!(report["observation"]["freshness"], "unknown");
    assert_eq!(
        report["observation"]["quota_windows"][0]["used_percent"],
        17
    );
    assert_eq!(
        report["observation"]["quota_windows"][0]["remaining_percent"],
        83
    );
    assert_eq!(
        report["observation"]["quota_windows"][0]["window_duration_mins"],
        300
    );
    assert_eq!(report["capabilities"]["account/read"], "supported");
    assert_eq!(
        report["capabilities"]["account/rateLimits/read"],
        "supported"
    );
    assert_eq!(report["capabilities"]["account/usage/read"], "unsupported");
}

#[test]
fn authentication_failure_is_a_diagnosable_report_without_observation() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/authentication-failed.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["outcome"], "failure");
    assert_eq!(report["observation"], Value::Null);
    assert_eq!(report["failure_code"], "authentication_failed");
    assert_eq!(report["availability"], "needs_login");
    assert_eq!(report["collection_state"], "error");
    assert_eq!(report["freshness"], "unknown");
}

#[test]
fn subprocess_collection_performs_handshake_and_matches_responses_by_id() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--codex-bin",
            env!("CARGO_BIN_EXE_fake-codex-app-server"),
            "--timeout-ms",
            "1000",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(
        output.status.success(),
        "stdout: {}; stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["outcome"], "success");
    assert_eq!(report["app_server_version"], "codex_cli_rs/test-fixture");
    assert_eq!(
        report["observation"]["quota_windows"][0]["used_percent"],
        23
    );
    assert_eq!(report["capabilities"]["account/usage/read"], "unsupported");
}

#[test]
fn subprocess_failures_remain_distinct_and_never_emit_an_observation() {
    for (mode, expected_code) in [
        ("exit", "process_exited"),
        ("malformed", "malformed_response"),
        ("timeout", "timeout"),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
            .env("AGENTMETER_FAKE_MODE", mode)
            .args([
                "codex",
                "collect",
                "--codex-bin",
                env!("CARGO_BIN_EXE_fake-codex-app-server"),
                "--timeout-ms",
                "50",
            ])
            .output()
            .expect("run AgentMeter P0 CLI");

        assert!(
            !output.status.success(),
            "mode {mode} unexpectedly succeeded"
        );
        let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
        assert_eq!(report["outcome"], "failure", "mode {mode}");
        assert_eq!(report["observation"], Value::Null, "mode {mode}");
        assert_eq!(report["failure_code"], expected_code, "mode {mode}");
    }
}

#[test]
fn multi_bucket_windows_preserve_scope_and_unknown_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/multi-bucket.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    let windows = report["observation"]["quota_windows"]
        .as_array()
        .expect("quota windows");
    assert_eq!(windows.len(), 2);
    assert_eq!(windows[0]["limit_id"], "codex");
    assert_eq!(windows[0]["window_duration_mins"], Value::Null);
    assert_eq!(windows[0]["resets_at"], Value::Null);
    assert_eq!(windows[1]["limit_id"], "codex-mini");
    assert_eq!(windows[1]["window_duration_mins"], 60);
}

#[test]
fn unsupported_required_method_is_not_reported_as_a_generic_failure() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/rate-limits-unsupported.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["failure_code"], "method_unsupported");
    assert_eq!(report["availability"], "unsupported");
    assert_eq!(report["observation"], Value::Null);
}

#[test]
fn partial_evidence_is_preserved_when_rate_limits_require_authentication() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .env("AGENTMETER_FAKE_MODE", "auth_rate_limits")
        .args([
            "codex",
            "collect",
            "--codex-bin",
            env!("CARGO_BIN_EXE_fake-codex-app-server"),
            "--timeout-ms",
            "1000",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["failure_code"], "authentication_failed");
    assert_eq!(report["app_server_version"], "codex_cli_rs/test-fixture");
    assert_eq!(report["provider_account_kind"], "chatgpt");
    assert_eq!(report["capabilities"]["account/read"], "supported");
    assert_eq!(
        report["capabilities"]["account/rateLimits/read"],
        "authentication_required"
    );
    assert_eq!(report["capabilities"]["account/usage/read"], "unsupported");
}

#[test]
fn over_limit_usage_preserves_raw_percent_and_floors_remaining_at_zero() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/over-limit.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    let window = &report["observation"]["quota_windows"][0];
    assert_eq!(window["used_percent"], 135);
    assert_eq!(window["remaining_percent"], 0);
    assert_eq!(window["over_limit"], true);
}

#[test]
fn successful_usage_and_credits_are_preserved_with_account_scope_and_source_time() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/usage-success.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    let observation = &report["observation"];
    assert_eq!(observation["quota_windows"][0]["scope"], "provider_account");
    assert_eq!(observation["credits"][0]["balance"], "12.50");
    assert_eq!(observation["credits"][0]["unit"], "credits");
    assert_eq!(observation["source_timestamp"], 1788739100_i64);
    assert_eq!(observation["source_usage"]["usage"]["inputTokens"], 1234);
    assert_eq!(report["capabilities"]["account/usage/read"], "supported");
}

#[test]
fn process_exit_reconnects_once_and_repeats_the_protocol_handshake() {
    let marker = std::env::temp_dir().join(format!(
        "agentmeter-reconnect-{}.marker",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&marker);
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .env("AGENTMETER_FAKE_MODE", "exit_once")
        .env("AGENTMETER_FAKE_MARKER", &marker)
        .args([
            "codex",
            "collect",
            "--codex-bin",
            env!("CARGO_BIN_EXE_fake-codex-app-server"),
            "--timeout-ms",
            "1000",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");
    let _ = std::fs::remove_file(marker);

    assert!(
        output.status.success(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    assert_eq!(report["outcome"], "success");
}

#[test]
fn remaining_only_window_is_normalized_without_fabricating_a_success_state() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-p0"))
        .args([
            "codex",
            "collect",
            "--fixture",
            "tests/fixtures/codex/remaining-only.jsonl",
        ])
        .output()
        .expect("run AgentMeter P0 CLI");

    assert!(output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).expect("valid JSON report");
    let window = &report["observation"]["quota_windows"][0];
    assert_eq!(window["remaining_percent"], 64);
    assert_eq!(window["used_percent"], 36);
    assert_eq!(window["over_limit"], false);
}
