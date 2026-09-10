#[test]
fn nsis_inspection_requires_packaged_startup_diagnostics_and_copilot_billing() {
    let script = include_str!("../scripts/inspect-nsis.ps1");
    for required in [
        "desktop_startup_failed",
        "startup_registration_failed",
        "startup-error.log",
        "agentmeter.desktop-startup-diagnostic/v1",
        "startup_diagnostic_markers_present = $true",
        "refresh_copilot",
        "save_copilot_source",
        "clear_copilot_source",
        "copilot_authoritative_billing_usage",
        "not_exposed_by_billing_usage_endpoint",
        "2026-03-10",
        "copilot_billing_markers_present = $true",
    ] {
        assert!(
            script.contains(required),
            "missing inspection marker: {required}"
        );
    }
    assert!(script.contains("[IO.File]::ReadAllBytes($embeddedPath)"));
    assert!(script.contains("Remove-Item -LiteralPath $resolvedInspection -Recurse -Force"));
}
