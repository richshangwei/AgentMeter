use serde_json::Value;

fn config() -> Value {
    serde_json::from_str(include_str!("../desktop-p0/tauri.conf.json")).unwrap()
}

#[test]
fn nsis_bundle_is_enabled_with_approved_webview2_strategy() {
    let config = config();
    assert_eq!(config["bundle"]["active"], true);
    assert_eq!(config["bundle"]["targets"], "nsis");
    assert_eq!(
        config["bundle"]["windows"]["webviewInstallMode"]["type"],
        "downloadBootstrapper"
    );
    assert_eq!(
        config["bundle"]["windows"]["webviewInstallMode"]["silent"],
        true
    );
    assert!(config["bundle"]["windows"]["minimumWebview2Version"].is_null());
    assert_eq!(config["bundle"]["windows"]["allowDowngrades"], false);
    assert_eq!(
        config["bundle"]["windows"]["nsis"]["installMode"],
        "currentUser"
    );
    assert_eq!(
        config["bundle"]["windows"]["nsis"]["installerHooks"],
        "nsis/installer-hooks.nsh"
    );
}

#[test]
fn package_identity_version_and_frontend_are_explicit_and_consistent() {
    let config = config();
    assert_eq!(config["identifier"], "com.agentmeter.p0");
    assert_eq!(config["productName"], "AgentMeter P0");
    assert_eq!(config["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(config["build"]["frontendDist"], "ui");
    assert!(std::path::Path::new("desktop-p0/ui").is_dir());
}

#[test]
fn desktop_brand_assets_drive_header_window_bundle_and_tray_icons() {
    let config = config();
    assert_eq!(config["bundle"]["icon"][0], "icons/icon.ico");
    assert_eq!(config["bundle"]["icon"][1], "icons/icon.png");
    let html = include_str!("../desktop-p0/ui/index.html");
    assert!(html.contains("assets/agentmeter-icon.png"));
    assert!(html.contains("class=\"brand-name\">Agent<span>Meter</span>"));
    assert!(html.contains("assets/agentmeter-icon.png"));
    let icon = include_bytes!("../desktop-p0/icons/icon.ico");
    assert_eq!(&icon[..4], &[0, 0, 1, 0]);
    let png = include_bytes!("../desktop-p0/icons/icon.png");
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    let build = include_str!("../desktop-p0/build.rs");
    let main = include_str!("../desktop-p0/src/main.rs");
    assert!(build.contains("icons/icon.ico"));
    assert!(main.contains("include_bytes!(\"../icons/tray-icon.rgba\")"));
    assert!(!main.contains("[0x58, 0xa6, 0xff, 0xff].repeat"));
}

#[test]
fn release_startup_failures_have_a_webview_independent_diagnostic_path() {
    let main = include_str!("../desktop-p0/src/main.rs");
    let diagnostics = include_str!("../desktop-p0/src/diagnostics.rs");
    assert!(!main.contains(".expect(\"failed to run AgentMeter P0 desktop shell\")"));
    assert!(main.contains("desktop_startup_failed"));
    assert!(main.contains("startup_registration_failed"));
    assert!(diagnostics.contains("MessageBoxW"));
    assert!(diagnostics.contains("startup-error.log"));
    assert!(diagnostics.contains("agentmeter.desktop-startup-diagnostic/v1"));
    assert!(main.contains("重新執行安裝程式以修復 WebView2"));
}

#[test]
fn startup_registration_matches_the_nsis_product_name_and_cleans_the_legacy_name() {
    let config = config();
    let product = config["productName"].as_str().unwrap();
    let source = include_str!("../desktop-p0/src/main.rs");
    assert!(source.contains(&format!("const STARTUP_VALUE: &str = \"{product}\";")));
    assert!(source.contains("const LEGACY_STARTUP_VALUE: &str = \"AgentMeterP0\";"));
    assert!(source.contains("delete_value(LEGACY_STARTUP_VALUE)?;"));
}

#[test]
fn readiness_probe_is_side_effect_free_and_never_becomes_the_primary_instance() {
    let source = include_str!("../desktop-p0/src/main.rs");
    let windows = include_str!(
        "../desktop-p0/vendor/tauri-plugin-single-instance/src/platform_impl/windows.rs"
    );
    assert!(source.contains("arg == \"--probe-ready\""));
    assert!(source.contains("A readiness probe has no UI side effect"));
    assert!(windows.contains("let probe_ready ="));
    assert!(windows.contains("std::process::exit(if probe_ready { 3 } else { 0 })"));
}

#[test]
fn desktop_tablet_controls_keep_pair_codes_ephemeral_and_out_of_navigation() {
    let html = include_str!("../desktop-p0/ui/index.html");
    let script = include_str!("../desktop-p0/ui/dashboard.js");
    assert!(html.contains("id=\"tablet-code\""));
    assert!(html.contains("請勿將含配對碼的畫面截圖或分享"));
    assert!(script.contains("get('tablet-code').textContent = result.code"));
    assert!(script.contains("setTimeout(clearPairCode, result.expires_in_seconds * 1000)"));
    assert!(script.contains("tablet_clear_pairing"));
    assert!(script.contains("tablet_usb_connect"));
    assert!(script.contains("{adbPath, serial, devicePort}"));
    assert!(script.contains("mapping !== 'owned'"));
    assert!(script.contains("authenticated_session_established"));
    assert!(script.contains("tablet_activity"));
    assert!(script.contains("setInterval(pollTabletActivity, 2000)"));
    let bridge = include_str!("../desktop-p0/src/tablet_bridge.rs");
    assert!(bridge.contains("revoke_sessions_for_transport_change"));
    assert!(bridge.contains("ReverseMappingState::Changed"));
    assert!(bridge.contains("mapping_ownership_lost"));
    assert!(!script.contains("innerHTML"));
    assert!(!script.contains("clipboard"));
    for line in script.lines().filter(|line| line.contains("localStorage")) {
        assert!(
            line.contains("desktopMonitorKey"),
            "localStorage may persist display preferences only, never pairing material"
        );
    }
    assert!(!script.contains("location.href"));
}

#[test]
fn desktop_uses_the_proven_quota_paths_without_manual_sources() {
    let html = include_str!("../desktop-p0/ui/index.html");
    let script = include_str!("../desktop-p0/ui/dashboard.js");
    let collector = include_str!("../scripts/quota-smoke.mjs");
    let bridge = include_str!("../desktop-p0/src/tablet_bridge.rs");
    for old_control in [
        "gh-path",
        "copilot-context",
        "copilot-account",
        "copilot-meter",
        "claude-path",
    ] {
        assert!(!html.contains(&format!("id=\"{old_control}\"")));
    }
    assert!(script.contains("'refresh_quota'"));
    assert!(!script.contains("'refresh_copilot'"));
    assert!(!script.contains("'load_claude'"));
    assert!(!script.contains("innerHTML"));
    assert!(collector.contains("'account.getQuota'"));
    assert!(collector.contains("'account/rateLimits/read'"));
    assert!(collector.contains("'--print','/usage'"));
    assert!(bridge.contains("crate::auto_quota::collect"));
    assert_eq!(
        config()["bundle"]["resources"]["resources/quota-helper/"],
        "quota-helper/"
    );
}

#[test]
fn uninstall_hook_requires_explicit_purge_and_never_deletes_data_itself() {
    let hook = include_str!("../desktop-p0/nsis/installer-hooks.nsh");
    assert!(hook.contains("NSIS_HOOK_PREUNINSTALL"));
    assert!(hook.contains("$CMDLINE \"/PURGE\""));
    assert!(hook.contains("StrCpy $DeleteAppDataCheckboxState 1"));
    assert!(hook.contains("$UpdateMode <> 1"));
    assert!(hook.contains("\"AgentMeterP0\""));
    assert!(!hook.contains("RMDir"));
    assert!(!hook.contains("$APPDATA"));
    assert!(!hook.contains("$LOCALAPPDATA"));
}
