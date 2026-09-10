#[test]
fn desktop_process_runner_is_explicit_bounded_and_evidence_bearing() {
    let script = include_str!("../scripts/test-desktop-process-lifecycle.ps1");
    for required in [
        "AllowProcessLaunch",
        "--hidden",
        "--probe-ready",
        "--request-exit",
        "WaitForExit(8000)",
        "resident_count",
        "resident_after_exit",
        "abnormal_termination",
        "post_abnormal_relaunch",
        "executable_sha256",
        "agentmeter.desktop-process-lifecycle/v1",
        "reject_without_starting_a_second_instance",
    ] {
        assert!(
            script.contains(required),
            "missing lifecycle contract: {required}"
        );
    }
    assert!(script.contains("Stop-Process -Id $abnormalPrimary.Id -Force"));
    assert!(script.contains("Stop-Process -Id $ownedPrimary.Id"));
    assert!(!script.contains("Stop-Process -Name"));
}
