use serde_json::Value;
use std::process::Command;

fn run(path: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-usb-p0"))
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
fn selected_device_channel_reports_supported_mapping() {
    let report = run("tests/fixtures/usb/healthy.json");
    assert_eq!(report["selected_serial"], "USB-001");
    assert_eq!(report["bind"]["status"], "supported");
    assert_eq!(report["reverse"]["status"], "supported");
    assert_eq!(report["reachability"]["status"], "supported");
    assert_eq!(report["browser"]["status"], "observed");
}
#[test]
fn browser_launch_alone_does_not_mean_online() {
    let report = run("tests/fixtures/usb/browser_only.json");
    assert_eq!(report["browser"]["status"], "not_online");
    assert_eq!(report["reachability"]["status"], "blocked");
}
#[test]
fn selected_device_mapping_conflict_fails_closed() {
    let report = run("tests/fixtures/usb/conflict.json");
    assert_eq!(report["bind"]["status"], "supported");
    assert_eq!(report["reverse"]["status"], "blocked");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d.as_str().unwrap().contains("no-rebind"))
    );
}

#[test]
fn explicit_serial_wins_among_physical_emulator_tcp_and_offline_devices() {
    let report = run("tests/fixtures/usb/multiple-devices.json");
    assert_eq!(report["selected_serial"], "PHYSICAL-2");
    assert_eq!(report["selection"]["status"], "supported");
    assert_eq!(report["reverse"]["status"], "supported");
}

#[test]
fn unauthorized_selected_device_has_actionable_failure() {
    let report = run("tests/fixtures/usb/unauthorized.json");
    assert_eq!(report["selection"]["status"], "blocked");
    assert!(
        report["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| { value.as_str().unwrap().contains("authorized=false") })
    );
}
#[test]
fn reconnect_matrix_and_owned_teardown_are_explicit() {
    let report = run("tests/fixtures/usb/healthy.json");
    assert_eq!(report["reconnect"]["status"], "exercised");
    assert_eq!(report["teardown"]["status"], "supported");
    assert_eq!(report["host_port"], 43111);
    assert_eq!(report["device_port"], 43111);
}
