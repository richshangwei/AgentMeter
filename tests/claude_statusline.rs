use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn run(args: &[&str]) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0"))
        .args(args)
        .output()
        .expect("run Claude experiment");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("json output")
}
fn write(p: &Path, v: Value) {
    fs::write(p, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

#[test]
fn reversible_config_lifecycle_preserves_existing_statusline() {
    let d = tempfile_dir();
    let c = d.join("settings.json");
    write(
        &c,
        json!({"statusLine":{"type":"command","command":"my-status"},"theme":"dark"}),
    );
    let installed = run(&["install", c.to_str().unwrap()]);
    assert_eq!(installed["preserved"]["command"], "my-status");
    assert!(!installed["manifest"].as_str().unwrap().is_empty());
    let enabled = run(&["enable", c.to_str().unwrap()]);
    assert_eq!(enabled["enabled"], true);
    let disabled = run(&["disable", c.to_str().unwrap()]);
    assert_eq!(disabled["restored"], true);
    let restored: Value = serde_json::from_slice(&fs::read(&c).unwrap()).unwrap();
    assert_eq!(restored["statusLine"]["command"], "my-status");
    assert_eq!(run(&["enable", c.to_str().unwrap()])["enabled"], true);
    assert_eq!(run(&["disable", c.to_str().unwrap()])["enabled"], false);
    assert_eq!(run(&["remove", c.to_str().unwrap()])["restored"], true);
    let restored: Value = serde_json::from_slice(&fs::read(&c).unwrap()).unwrap();
    assert_eq!(restored["statusLine"]["command"], "my-status");
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn structured_event_normalizes_quota_and_missing_quota_is_unknown() {
    let d = tempfile_dir();
    let event = d.join("event.json");
    write(
        &event,
        json!({"type":"status","timestamp":"2026-09-06T01:02:03Z","usage":{"window":"five_hour","used":25,"limit":100,"unit":"percent","reset_at":"2026-09-06T05:00:00Z"}}),
    );
    let o = run(&["normalize", event.to_str().unwrap()]);
    assert_eq!(o["source"], "claude.statusLine");
    assert_eq!(o["maturity"], "structured");
    assert_eq!(o["observation"]["quota_window"], "five_hour");
    assert_eq!(o["observation"]["used"], 25);
    write(
        &event,
        json!({"type":"status","timestamp":"2026-09-06T01:02:03Z","model":"claude"}),
    );
    let missing = run(&["normalize", event.to_str().unwrap()]);
    assert_eq!(missing["collection_state"], "unknown");
    assert!(missing["observation"].is_null());
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn diagnostics_distinguish_malformed_and_schema_change_and_log_fallback() {
    let d = tempfile_dir();
    let e = d.join("event.json");
    write(&e, json!({"type":"status","usage":{"used":1}}));
    assert_eq!(
        run(&["normalize", e.to_str().unwrap()])["diagnostic"],
        "schema_changed"
    );
    fs::write(&e, b"not-json").unwrap();
    assert_eq!(
        run(&["normalize", e.to_str().unwrap()])["diagnostic"],
        "malformed_payload"
    );
    fs::write(&e, b"Quota: 42% remaining, resets 2026-09-06T05:00:00Z").unwrap();
    let f = run(&["fallback", e.to_str().unwrap()]);
    assert_eq!(f["maturity"], "log_fallback");
    assert_eq!(f["quality"], "degraded");
    assert_eq!(f["observation"]["remaining"], 42);
    fs::remove_dir_all(d).unwrap();
}

fn tempfile_dir() -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "agentmeter-claude-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&p).unwrap();
    p
}
