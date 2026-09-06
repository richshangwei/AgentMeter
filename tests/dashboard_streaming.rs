use serde_json::Value;
use std::process::Command;

fn run(path: &str) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_agentmeter-dashboard-p0"))
        .args(["--fixture", path])
        .output()
        .expect("binary runs");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("JSON report")
}

#[test]
fn complete_snapshot_stream_and_refresh_are_supported() {
    let r = run("tests/fixtures/dashboard/healthy.json");
    for key in [
        "complete_snapshot",
        "source_scoping",
        "sse_ordering",
        "reconnect",
        "async_refresh",
        "failure_isolation",
        "monitor_boundary",
    ] {
        assert_eq!(r[key]["status"], "supported", "{key}");
    }
}

#[test]
fn incomplete_or_stale_delivery_is_constrained() {
    let r = run("tests/fixtures/dashboard/stale.json");
    assert_eq!(r["sse_ordering"]["status"], "constrained");
    assert_eq!(r["reconnect"]["status"], "constrained");
    assert_eq!(r["async_refresh"]["status"], "constrained");
}
