use serde_json::Value;
use std::process::Command;

fn fixture(name: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-windows-install-p0"))
        .args(["--fixture", name])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn clean_install_lifecycle_has_actionable_release_gate() {
    let result = fixture("tests/fixtures/windows/install-pass.json");
    assert_eq!(result["status"], "needs-info");
    assert_eq!(result["webview2"]["ready"], true);
    assert_eq!(result["release_gate"], "VM-evidence-required");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| {
                value
                    .as_str()
                    .unwrap()
                    .contains("measurements are incomplete")
            })
    );
}

#[test]
fn missing_webview2_does_not_silently_launch() {
    let result = fixture("tests/fixtures/windows/install-webview-missing.json");
    assert_eq!(result["status"], "blocked");
    assert_eq!(result["webview2"]["ready"], false);
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| v.as_str().unwrap().contains("WebView2"))
    );
    assert_eq!(result["release_gate"], "block-Windows-v1-distribution");
}

#[test]
fn a_fixture_cannot_claim_clean_vm_success_without_every_lifecycle_step() {
    let result = fixture("tests/fixtures/windows/install-pass.json");
    assert!(
        result["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| { value.as_str().unwrap().contains("restart") })
    );
}
