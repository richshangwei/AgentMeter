use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

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
    assert_eq!(enabled["preserved"]["command"], "my-status");
    assert!(
        enabled["installed"]["command"]
            .as_str()
            .unwrap()
            .contains("agentmeter-claude-p0 status")
    );
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
    assert_eq!(o["provider"], "claude");
    assert_eq!(o["source"]["kind"], "status_line");
    assert_eq!(o["data_quality"], "local_observed");
    assert_eq!(o["collector_maturity"], "experimental");
    assert_eq!(o["observation"]["quota_windows"][0]["scope"], "five_hour");
    assert_eq!(o["observation"]["quota_windows"][0]["used"], 25);
    write(
        &event,
        json!({"type":"status","timestamp":"2026-09-06T01:02:03Z","model":"claude"}),
    );
    let missing = run(&["normalize", event.to_str().unwrap()]);
    assert_eq!(missing["collection_state"], "idle");
    assert_eq!(missing["failure_code"], "missing_quota");
    assert!(missing["observation"].is_null());
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn configuration_conflict_is_setup_required_and_never_overwritten() {
    let d = tempfile_dir();
    let c = d.join("settings.json");
    write(
        &c,
        json!({"statusLine":{"type":"command","command":"original"},"theme":"dark"}),
    );
    run(&["install", c.to_str().unwrap()]);
    run(&["enable", c.to_str().unwrap()]);
    write(
        &c,
        json!({"statusLine":{"type":"command","command":"user-new-command"},"theme":"light"}),
    );
    let result = run(&["disable", c.to_str().unwrap()]);
    assert_eq!(result["diagnostic"], "configuration_conflict");
    assert_eq!(result["availability"], "setup_required");
    assert_canonical_failure(&result, "configuration_conflict", "error");
    let unchanged: Value = serde_json::from_slice(&fs::read(&c).unwrap()).unwrap();
    assert_eq!(unchanged["statusLine"]["command"], "user-new-command");
    assert_eq!(unchanged["theme"], "light");
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn failure_matrix_has_complete_canonical_axes_and_diagnostics() {
    let d = tempfile_dir();
    let event = d.join("event.json");

    let missing = run(&["normalize", event.to_str().unwrap()]);
    assert_canonical_failure(&missing, "missing_event", "idle");

    fs::write(&event, b"not-json").unwrap();
    let malformed = run(&["normalize", event.to_str().unwrap()]);
    assert_canonical_failure(&malformed, "malformed_payload", "error");

    write(&event, json!({"type":"future-status","usage":{}}));
    let schema = run(&["normalize", event.to_str().unwrap()]);
    assert_canonical_failure(&schema, "schema_changed", "error");

    fs::remove_dir_all(d).unwrap();
}

#[test]
fn prior_command_failure_is_structured_on_stderr() {
    let d = tempfile_dir();
    let c = d.join("settings.json");
    let failing_command = if cfg!(windows) { "exit /b 7" } else { "exit 7" };
    write(
        &c,
        json!({"statusLine":{"type":"command","command":failing_command}}),
    );
    let installed = run(&["install", c.to_str().unwrap()]);
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0"))
        .args(["status", installed["manifest"].as_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_canonical_failure(&report, "command_failed", "error");
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
    assert_eq!(f["source"]["kind"], "log_fallback");
    assert_eq!(f["collector_maturity"], "experimental");
    assert_eq!(f["data_quality"], "estimated");
    assert_eq!(f["observation"]["remaining"], 42);
    fs::remove_dir_all(d).unwrap();
}

#[test]
fn wrapper_preserves_prior_statusline_output() {
    let d = tempfile_dir();
    let c = d.join("settings.json");
    write(
        &c,
        json!({"statusLine":{"type":"command","command":"more"}}),
    );
    let installed = run(&["install", c.to_str().unwrap()]);
    let manifest = installed["manifest"].as_str().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0"))
        .args(["status", manifest])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"status-event")
        .unwrap();
    child.stdin.take();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "status-event"
    );
    fs::remove_dir_all(d).unwrap();
}

fn tempfile_path() -> PathBuf {
    std::env::temp_dir().join(format!(
        "agentmeter-claude-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ))
}

fn tempfile_dir() -> PathBuf {
    let p = tempfile_path();
    fs::create_dir_all(&p).unwrap();
    p
}

#[test]
fn temporary_paths_remain_unique_across_parallel_tests() {
    use std::{collections::HashSet, thread};

    let workers: Vec<_> = (0..16)
        .map(|_| thread::spawn(|| (0..1_000).map(|_| tempfile_path()).collect::<Vec<_>>()))
        .collect();
    let all_paths: Vec<_> = workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap())
        .collect();
    let unique: HashSet<_> = all_paths.iter().collect();
    assert_eq!(unique.len(), all_paths.len());
}

#[test]
fn wrapper_collects_documented_event_without_leaking_input_into_report_or_display() {
    use std::{io::Write, process::Stdio};
    let d = tempfile_dir();
    let config = d.join("settings.json");
    write(
        &config,
        json!({"statusLine":{"type":"command","command":"echo ORIGINAL-DISPLAY"}}),
    );
    let installed = run(&["install", config.to_str().unwrap()]);
    run(&["enable", config.to_str().unwrap()]);
    let manifest = Path::new(installed["manifest"].as_str().unwrap());
    let sink = manifest.with_extension("observation.json");
    let cases = [
        (
            json!({"session_id":"PRIVATE-SESSION","cwd":"PRIVATE-PATH","rate_limits":{"five_hour":{"used_percentage":12.5,"resets_at":1788739200_u64},"seven_day":{"used_percentage":0,"resets_at":null}}}),
            "ready",
            Value::Null,
        ),
        (json!({"rate_limits":null}), "idle", json!("missing_quota")),
        (
            json!({"rate_limits":{"five_hour":null,"seven_day":{"used_percentage":null}}}),
            "idle",
            json!("missing_quota"),
        ),
        (
            json!({"rate_limits":{"five_hour":{"used_percentage":101}}}),
            "error",
            json!("schema_changed"),
        ),
        (
            json!({"rate_limits":{"five_hour":{"used_percentage":25,"resets_at":"PRIVATE-RESET"}}}),
            "error",
            json!("schema_changed"),
        ),
    ];
    for (event, state, failure) in cases {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0"))
            .args(["status", manifest.to_str().unwrap()])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(&serde_json::to_vec(&event).unwrap())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{:?}", output.stderr);
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "ORIGINAL-DISPLAY"
        );
        assert!(output.stderr.is_empty());
        let stored = fs::read_to_string(&sink).unwrap();
        assert!(!stored.contains("PRIVATE"));
        let report: Value = serde_json::from_str(&stored).unwrap();
        assert_eq!(report["collection_state"], state);
        assert_eq!(report["failure_code"], failure);
        assert_eq!(report["source"]["replay"], false);
        assert_eq!(report["data_quality"], "official");
        if state == "ready" {
            let observation = &report["observation"];
            assert!(observation["source_timestamp"].is_null());
            assert!(observation["observed_at_epoch_seconds"].as_u64().unwrap() > 0);
            assert_eq!(observation["quota_windows"][0]["used"], 12.5);
            assert_eq!(observation["quota_windows"][0]["scope"], "five_hour");
            assert_eq!(observation["quota_windows"][0]["unit"], "percent");
            assert_eq!(observation["quota_windows"][0]["resets_at"], 1788739200_u64);
            assert_eq!(observation["quota_windows"][1]["used"], 0);
        } else {
            assert!(report["observation"].is_null());
        }
    }
    assert_eq!(run(&["remove", config.to_str().unwrap()])["restored"], true);
    assert!(!manifest.exists());
    assert!(!sink.exists());
    let restored: Value = serde_json::from_slice(&fs::read(&config).unwrap()).unwrap();
    assert_eq!(restored["statusLine"]["command"], "echo ORIGINAL-DISPLAY");
    fs::remove_dir_all(d).unwrap();
}

fn assert_canonical_failure(report: &Value, code: &str, state: &str) {
    assert_eq!(report["provider"], "claude");
    assert_eq!(report["provider_account"]["kind"], "unknown");
    assert!(report["source"]["kind"].is_string());
    assert_eq!(report["source"]["replay"], true);
    assert_eq!(report["data_quality"], "local_observed");
    assert_eq!(report["collector_maturity"], "experimental");
    assert!(report["availability"].is_string());
    assert_eq!(report["collection_state"], state);
    assert_eq!(report["freshness"], "unknown");
    assert_eq!(report["failure_code"], code);
    assert_eq!(report["diagnostics"][0]["code"], code);
    assert!(report["observation"].is_null());
}
