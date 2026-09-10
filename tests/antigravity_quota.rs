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
        r["capabilities"]["statusLine/structured_quota"],
        "supported"
    );
    assert_eq!(
        r["evidence"]["source_precedence"][0],
        "structured_status_line"
    );
    let windows = r["observation"]["quota_windows"].as_array().unwrap();
    let prompt = windows
        .iter()
        .find(|window| window["limit_id"] == "prompt")
        .unwrap();
    let credits = windows
        .iter()
        .find(|window| window["limit_id"] == "credits")
        .unwrap();
    assert_eq!(prompt["remaining_percent"], 75);
    assert_eq!(credits["over_limit"], true);
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
    assert_eq!(r["observation"]["collection_state"], "idle");
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
    assert_eq!(r["diagnostics"][0]["code"], "unexpected_output");
}

#[test]
fn localization_text_change_and_unexpected_output_are_distinct() {
    for (fixture, expected) in [
        (
            "tests/fixtures/antigravity/text-localized.json",
            "localization_unsupported",
        ),
        (
            "tests/fixtures/antigravity/text-format-changed.json",
            "text_changed",
        ),
        (
            "tests/fixtures/antigravity/text-unexpected.json",
            "unexpected_output",
        ),
    ] {
        let (ok, report) = run(fixture, None);
        assert!(!ok, "fixture {fixture}");
        assert_eq!(report["diagnostics"][0]["code"], expected);
        assert!(report["observation"].is_null());
    }
}

#[test]
fn authentication_and_command_failures_are_distinct() {
    let (ok, r) = run(
        "tests/fixtures/antigravity/authentication-failed.json",
        None,
    );
    assert!(!ok);
    assert_eq!(r["diagnostics"][0]["code"], "authentication_failed");
    assert!(r["observation"].is_null());

    let (ok, r) = run("tests/fixtures/antigravity/does-not-exist.json", None);
    assert!(!ok);
    assert_eq!(r["diagnostics"][0]["code"], "command_failed");
}

#[test]
fn fallback_rejects_other_minor_versions() {
    let (ok, r) = run("tests/fixtures/antigravity/text.json", Some("1.9.0"));
    assert!(!ok);
    assert_eq!(r["diagnostics"][0]["code"], "unsupported_version");
}

#[test]
fn evidence_is_versioned_and_records_release_boundary() {
    let (ok, report) = run("tests/fixtures/antigravity/text.json", None);
    assert!(ok);
    assert_eq!(report["evidence"]["tested_version"], "1.8.2");
    assert_eq!(report["evidence"]["version_evidence"], "fixture_declared");
    assert_eq!(
        report["evidence"]["fallback_release_policy"],
        "prohibited_pending_real_version_validation"
    );
    assert_eq!(report["evidence"]["replay"], true);
    assert!(report["evidence"]["limitations"].is_string());
}
