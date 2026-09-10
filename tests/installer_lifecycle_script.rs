#[test]
fn lifecycle_runner_has_hash_baseline_mutation_and_cleanup_guards() {
    let script = include_str!("../scripts/test-nsis-lifecycle.ps1");
    for required in [
        "ExpectedSha256",
        "ExpectedVersion",
        "UpgradeInstallerPath",
        "UpgradeExpectedSha256",
        "UpgradeExpectedVersion",
        "AllowSystemMutation",
        "Evidence parent directory does not exist",
        "Refusing to run while an AgentMeter process already exists",
        "Refusing to run over an existing AgentMeter installation",
        "Refusing to run where AgentMeter user data already exists",
        "WaitForExit",
        "Wait-AppReady",
        "--probe-ready",
        "--request-exit",
        "--startup-enable",
        "--startup-disable",
        "'/PURGE'",
        "Same-version reinstall did not preserve application data",
        "Upgrade installer must not be identical to the base installer",
        "UpgradeExpectedVersion must be greater than ExpectedVersion",
        "Upgrade did not preserve application data",
        "Upgraded product version",
        "same_version_reinstall_and_restart",
        "supported_upgrade",
        "upgrade_ready_ms",
        "initial_install_elapsed_ms",
        "installed_size_bytes",
        "first_launch_ready_ms",
        "idle_sample_seconds",
        "idle_working_set_bytes",
        "idle_cpu_percent",
        "agentmeter.windows-install-lifecycle/v1",
        "status = 'fail'",
        "roaming_data_marker_preserved",
        "local_data_marker_preserved",
    ] {
        assert!(
            script.contains(required),
            "missing lifecycle guard: {required}"
        );
    }
    assert!(script.contains("Get-FileHash"));
    assert!(!script.contains("Get-AuthenticodeSignature"));
    assert!(!script.contains("$IsWindows"));
}

#[test]
fn installer_identity_and_upgrade_contract_are_checked_before_first_install() {
    let script = include_str!("../scripts/test-nsis-lifecycle.ps1");
    let first_install = script
        .find("Invoke-BoundedProcess $resolvedInstaller @('/S'")
        .expect("initial installer invocation");
    for guard in [
        "Installer SHA-256 mismatch",
        "Upgrade installer SHA-256 mismatch",
        "UpgradeExpectedVersion must be greater than ExpectedVersion",
        "Upgrade installer must not be identical to the base installer",
        "Evidence parent directory does not exist",
        "Refusing to run while an AgentMeter process already exists",
        "Refusing to run where AgentMeter user data already exists",
    ] {
        let guard_position = script.find(guard).expect("upgrade/lifecycle guard");
        assert!(
            guard_position < first_install,
            "guard must run before installer: {guard}"
        );
    }
}
