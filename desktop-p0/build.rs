fn main() {
    let output = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR"));
    let icon = output.join("agentmeter-p0.ico");
    let bytes: [u8; 70] = [
        0, 0, 1, 0, 1, 0, // ICONDIR
        1, 1, 0, 0, 1, 0, 32, 0, 48, 0, 0, 0, 22, 0, 0, 0, // entry
        40, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 32, 0, // bitmap header
        0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, // bitmap header tail
        0xff, 0xa6, 0x58, 0xff, // one BGRA pixel
        0, 0, 0, 0, // AND mask row
    ];
    std::fs::write(&icon, bytes).expect("write generated build icon");
    let escaped_icon = icon.to_string_lossy().replace('\\', "\\\\");
    let config_override = format!(r#"{{"bundle":{{"icon":["{escaped_icon}"]}}}}"#);
    // SAFETY: build scripts run single-threaded here before Tauri reads its configuration.
    unsafe { std::env::set_var("TAURI_CONFIG", &config_override) };
    println!("cargo:rustc-env=TAURI_CONFIG={config_override}");
    let windows = tauri_build::WindowsAttributes::new().window_icon_path(icon);
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}
