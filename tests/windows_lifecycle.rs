use serde_json::Value;
use std::process::Command;

fn run() -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-windows-p0"))
        .args(["--fixture", "tests/fixtures/windows/lifecycle.json"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn run_events(events: &[&str]) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-windows-p0"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    serde_json::to_writer(
        child.stdin.as_mut().unwrap(),
        &serde_json::json!({"events": events}),
    )
    .unwrap();
    child.stdin.take();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn repeated_launch_reuses_one_instance() {
    let result = run_events(&["launch", "launch"]);
    let state = &result["state"];
    assert_eq!(state["instance_count"], 1);
    assert_eq!(state["collector_count"], 1);
    assert_eq!(state["server_count"], 1);
    assert!(
        state["trace"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v == "activate-existing-instance")
    );
}

#[test]
fn close_hides_and_tray_show_restores_window() {
    let result = run_events(&["launch", "close-window", "tray-show"]);
    let state = &result["state"];
    assert!(state["window_visible"].as_bool().unwrap());
    assert_eq!(state["tray_icon_count"], 1);
}

#[test]
fn explicit_exit_removes_owned_background_work() {
    let state = &run()["state"];
    assert_eq!(state["collector_count"], 0);
    assert_eq!(state["server_count"], 0);
    assert_eq!(state["database_writer_count"], 0);
    assert!(!state["running"].as_bool().unwrap());
}

#[test]
fn startup_registration_is_reversible_and_no_duplicates_are_reported() {
    let result = run();
    assert_eq!(result["status"], "pass");
    assert_eq!(result["state"]["startup_registered"], false);
    assert!(result["state"]["anomalies"].as_array().unwrap().is_empty());
}
