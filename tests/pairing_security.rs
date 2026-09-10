use serde_json::Value;
use std::process::Command;

fn run(path: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-pairing-p0"))
        .args(["--fixture", path])
        .output()
        .expect("binary runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("JSON report")
}

#[test]
fn pairing_session_and_security_boundary_are_exercised() {
    let report = run("tests/fixtures/pairing/healthy.json");
    assert_eq!(report["pair"]["status"], "supported");
    assert_eq!(report["session"]["status"], "supported");
    assert_eq!(report["attack_cases"]["status"], "fail_closed");
    assert_eq!(report["csrf_origin"]["status"], "fail_closed");
    assert_eq!(report["revocation"]["status"], "supported");
    assert_eq!(report["secret_hygiene"]["status"], "supported");
    assert_eq!(report["persisted_secret"]["status"], "not_observed");
    assert_eq!(report["route_auth"]["status"], "fail_closed");
}

#[test]
fn unsafe_origin_is_blocked() {
    let report = run("tests/fixtures/pairing/bad-origin.json");
    assert_eq!(report["csrf_origin"]["status"], "blocked");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d.as_str().unwrap().contains("approved loopback"))
    );
}

#[test]
fn wrong_code_policy_expiry_and_incomplete_route_auth_cannot_claim_support() {
    let report = run("tests/fixtures/pairing/bad-policy.json");
    assert_eq!(report["pair"]["status"], "not_observed");
    assert_eq!(report["attack_cases"]["status"], "unsafe");
    assert_eq!(report["route_auth"]["status"], "blocked");
}
