fn main() {
    println!("cargo:rerun-if-env-changed=AGENTMETER_UPDATE_ENDPOINT");
    println!("cargo:rerun-if-env-changed=AGENTMETER_UPDATE_PUBLIC_KEY");
    let icon = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("icons/icon.ico");
    let windows = tauri_build::WindowsAttributes::new().window_icon_path(icon);
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run Tauri build script");
}
