use serde_json::Value;
use std::process::Command;

fn run(fixture: &str, version: Option<&str>) -> (bool, Value) {
    let mut args = vec!["antigravity", "collect", "--fixture", fixture];
    if let Some(v) = version {
        args.extend(["--version", v]);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-antigravity-p0"))
        .args(args)
        .output()
        .unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn structured_statusline_is_preferred_and_normalized() {
    let (ok, r) = run("tests/fixtures/antigravity/structured.json", None);
    assert!(ok);
    assert_eq!(r["observation"]["source"]["kind"], "status_line");
    assert_eq!(r["observation"]["provider_account"]["kind"], "google");
    assert_eq!(
        r["observation"]["quota_windows"][0]["remaining_percent"],
        75
    );
    assert_eq!(r["observation"]["quota_windows"][1]["over_limit"], true);
}
#[test]
fn missing_fields_are_unknown_and_diagnosed() {
    let (ok, r) = run("tests/fixtures/antigravity/missing.json", None);
    assert!(ok);
    assert_eq!(
        r["observation"]["quota_windows"][0]["remaining"],
        Value::Null
    );
    assert_eq!(r["diagnostics"][0]["code"], "missing_quota_values");
}
#[test]
fn text_fallback_is_version_bounded_and_lower_trust() {
    let (ok, r) = run("tests/fixtures/antigravity/text.json", None);
    assert!(ok);
    assert_eq!(r["observation"]["source"]["kind"], "headless_text");
    assert_eq!(r["observation"]["data_quality"], "estimated");
    assert_eq!(r["diagnostics"][0]["code"], "experimental_fallback");
    let (ok, r) = run("tests/fixtures/antigravity/text-changed.json", None);
    assert!(!ok);
    assert_eq!(r["diagnostics"][0]["code"], "unsupported_version");
}
#[test]
fn schema_drift_and_unparseable_text_are_distinct() {
    let (ok, r) = run("tests/fixtures/antigravity/schema-changed.json", None);
    assert!(!ok);
    assert_eq!(r["diagnostics"][0]["code"], "schema_changed");
    let (ok, r) = run(
        "tests/fixtures/antigravity/text-changed.json",
        Some("1.8.2"),
    );
    assert!(!ok);
    assert_eq!(r["diagnostics"][0]["code"], "text_changed");
}
