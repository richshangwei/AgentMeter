use serde_json::Value;
use std::process::Command;
fn run() -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .args(["--fixture", "tests/fixtures/copilot/contexts.json"])
        .output()
        .unwrap();
    assert!(o.status.success());
    serde_json::from_slice(&o.stdout).unwrap()
}
#[test]
fn matrix_preserves_contexts() {
    let v = run();
    assert_eq!(v["schema_version"], "copilot-p0/v1");
    assert_eq!(v["contexts"].as_array().unwrap().len(), 3);
    assert_eq!(v["contexts"][0]["context"], "Personal");
    assert_eq!(v["contexts"][1]["context"], "Business");
    assert_eq!(v["contexts"][2]["context"], "Enterprise");
}
#[test]
fn permission_failure_is_distinct() {
    let v = run();
    assert_eq!(
        v["contexts"][1]["billing_api"]["status"],
        "permission_denied"
    );
    assert_eq!(
        v["contexts"][1]["quota"]["status"],
        "unknown_permission_denied"
    );
}
#[test]
fn metrics_separate_preview_experimental() {
    let v = run();
    let p = &v["contexts"][0];
    assert_eq!(p["ai_credits"]["remaining"], 120);
    assert_eq!(p["legacy_premium_requests"]["remaining"], 30);
    assert_eq!(p["preview_sdk"]["experimental"], true);
}
#[test]
fn unknowns_preserved() {
    let v = run();
    assert_eq!(v["unknowns"][0], "preview endpoint stability");
    assert_eq!(v["contexts"][2]["unknowns"][0], "enterprise contract limit");
}
