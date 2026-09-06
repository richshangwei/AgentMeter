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
    assert_eq!(report["observation"]["data_quality"], "official");
    assert_eq!(report["observation"]["collector_maturity"], "experimental");
    assert_eq!(report["observation"]["availability"], "available");
    assert_eq!(report["observation"]["collection_state"], "ready");
    assert_eq!(report["observation"]["freshness"], "fresh");
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
        ("malformed", "schema_changed"),
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
    assert_eq!(
        report["capabilities"]["account/usage/read"],
        "not_attempted"
    );
}
