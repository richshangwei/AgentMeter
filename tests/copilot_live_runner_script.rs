#![cfg(windows)]

use serde_json::Value;
use std::{path::PathBuf, process::Command};

struct Temp(PathBuf);

impl Temp {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "agentmeter-copilot-runner-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn script() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/test-copilot-live.ps1")
}

fn run(temp: &Temp, name: &str, accounts: &[(&str, &str)], allow: bool) -> std::process::Output {
    let output = temp.0.join(format!("{name}.json"));
    let log = temp.0.join(format!("{name}.log"));
    let mut command = Command::new("powershell");
    command.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script().to_str().unwrap(),
        "-CollectorPath",
        env!("CARGO_BIN_EXE_agentmeter-copilot-p0"),
        "-GitHubCliPath",
        env!("CARGO_BIN_EXE_fake-gh"),
        "-OutputPath",
        output.to_str().unwrap(),
        "-Year",
        "2026",
        "-Month",
        "9",
    ]);
    for (parameter, account) in accounts {
        command.args([parameter, account]);
    }
    if allow {
        command.arg("-AllowProviderRequests");
    }
    command
        .env("AGENTMETER_FAKE_GH_MODE", "matrix")
        .env("AGENTMETER_FAKE_GH_LOG", log)
        .output()
        .unwrap()
}

#[test]
fn runner_requires_explicit_authorization_and_does_not_create_evidence_without_it() {
    let temp = Temp::new();
    let output = run(&temp, "denied", &[("-PersonalAccount", "octocat")], false);
    assert!(!output.status.success());
    assert!(!temp.0.join("denied.json").exists());
    assert!(!temp.0.join("denied.log").exists());
}

#[test]
fn runner_executes_six_separate_meter_requests_and_writes_sanitized_evidence() {
    let temp = Temp::new();
    let output = run(
        &temp,
        "matrix",
        &[
            ("-PersonalAccount", "octocat"),
            ("-BusinessAccount", "example-org"),
            ("-EnterpriseAccount", "example-enterprise"),
        ],
        true,
    );
    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let bytes = std::fs::read(temp.0.join("matrix.json")).unwrap();
    let evidence: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        evidence["schema_version"],
        "agentmeter.copilot-live-evidence/v1"
    );
    assert_eq!(evidence["outcome"], "success");
    assert_eq!(evidence["api_version"], "2026-03-10");
    assert_eq!(evidence["results"].as_array().unwrap().len(), 6);
    assert!(evidence["untested_contexts"].as_array().unwrap().is_empty());
    assert_eq!(evidence["credential_material_recorded"], false);
    for result in evidence["results"].as_array().unwrap() {
        assert_eq!(result["collector_exit"], 0);
        assert_eq!(result["report"]["evidence"]["mode"], "live_gh_api");
        assert_eq!(result["report"]["evidence"]["replay"], false);
        assert!(result["report"]["contexts"][0]["observation"].is_null());
    }
    let text = String::from_utf8(bytes).unwrap();
    assert!(!text.to_ascii_lowercase().contains("authorization: bearer"));
    assert!(!text.contains("PRIVATE"));

    let log = std::fs::read_to_string(temp.0.join("matrix.log")).unwrap();
    assert_eq!(log.lines().count(), 6);
    for prefix in ["/users/", "/organizations/", "/enterprises/"] {
        assert!(log.contains(prefix));
    }
    assert!(log.contains("/ai_credit/usage"));
    assert!(log.contains("/premium_request/usage"));
}

#[test]
fn runner_records_unprovided_contexts_as_evidence_gaps() {
    let temp = Temp::new();
    let output = run(&temp, "partial", &[("-PersonalAccount", "octocat")], true);
    assert!(output.status.success());
    let evidence: Value =
        serde_json::from_slice(&std::fs::read(temp.0.join("partial.json")).unwrap()).unwrap();
    assert_eq!(evidence["outcome"], "completed_with_gaps");
    assert_eq!(evidence["results"].as_array().unwrap().len(), 2);
    assert_eq!(evidence["untested_contexts"].as_array().unwrap().len(), 2);
    assert_eq!(
        evidence["untested_contexts"][0]["reason"],
        "account_slug_not_provided"
    );
}
