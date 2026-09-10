#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::env;
mod auto_quota;
mod dashboard;
mod diagnostics;
mod discovery;
mod lifecycle;
mod settings;
mod setup;
mod tablet_bridge;
use tauri::{
    Manager, WindowEvent,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};

const STARTUP_VALUE: &str = "AgentMeter P0";
const LEGACY_STARTUP_VALUE: &str = "AgentMeterP0";

fn show_main(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn exit_app(app: &tauri::AppHandle) {
    let lifecycle = std::sync::Arc::clone(&app.state::<dashboard::Dashboard>().lifecycle);
    let app = app.clone();
    std::thread::spawn(move || {
        let _ = app.state::<tablet_bridge::TabletBridge>().stop();
        lifecycle.stop_and_wait();
        app.exit(0);
    });
}

#[cfg(windows)]
fn set_startup(enabled: bool) -> Result<(), String> {
    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu
        .create_subkey("Software\\Microsoft\\Windows\\CurrentVersion\\Run")
        .map_err(|error| format!("cannot open startup registration: {error}"))?;
    let delete_value = |name: &str| match run.delete_value(name) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "cannot remove startup registration '{name}': {error}"
        )),
    };
    if enabled {
        delete_value(LEGACY_STARTUP_VALUE)?;
        let executable =
            env::current_exe().map_err(|error| format!("cannot resolve executable: {error}"))?;
        let command = format!("\"{}\" --hidden", executable.display());
        run.set_value(STARTUP_VALUE, &command)
            .map_err(|error| format!("cannot enable startup: {error}"))
    } else {
        delete_value(STARTUP_VALUE)?;
        delete_value(LEGACY_STARTUP_VALUE)
    }
}

#[cfg(not(windows))]
fn set_startup(_enabled: bool) -> Result<(), String> {
    Err("startup registration is supported only on Windows".into())
}

fn startup_command() -> Option<bool> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--startup-enable") {
        Some(true)
    } else if args.iter().any(|arg| arg == "--startup-disable") {
        Some(false)
    } else {
        None
    }
}

fn main() {
    let args: Vec<_> = env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "--quota-collect") {
        let runtime = env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("quota-helper");
        let workdir = std::path::PathBuf::from(env::var_os("LOCALAPPDATA").unwrap())
            .join("com.agentmeter.p0/quota-workspace");
        let result = auto_quota::run(
            &runtime,
            &workdir,
            "all",
            args.iter().any(|a| a == "--enable-claude"),
            &std::sync::atomic::AtomicBool::new(false),
        );
        match result {
            Ok(value) => println!("{value}"),
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
        return;
    }
    if args.get(1).is_some_and(|arg| arg == "--claude-status") {
        let result = args
            .get(2)
            .ok_or_else(|| "missing manifest".to_string())
            .and_then(|path| agentmeter_p0::claude_setup::receive(std::path::Path::new(path)));
        if result.is_err() {
            std::process::exit(2);
        }
        return;
    }
    if let Some(enabled) = startup_command() {
        match set_startup(enabled) {
            Ok(()) => println!("startup registration enabled={enabled}"),
            Err(error) => {
                diagnostics::report_failure(
                    "startup_registration_failed",
                    "請檢查目前使用者的開機啟動登錄權限後重試。",
                    &error,
                );
                std::process::exit(1);
            }
        }
        return;
    }

    let launch_hidden = env::args().any(|arg| arg == "--hidden");
    let result = tauri::Builder::default()
        .manage(dashboard::Dashboard::default())
        .manage(setup::SetupState::default())
        .manage(tablet_bridge::TabletBridge::default())
        .invoke_handler(tauri::generate_handler![
            dashboard::snapshot,
            auto_quota::refresh_quota,
            auto_quota::set_auto_quota,
            setup::inspect_setup,
            setup::preview_claude_setup,
            setup::apply_claude_setup,
            setup::cancel_claude_setup,
            dashboard::refresh,
            dashboard::refresh_copilot,
            dashboard::load_claude,
            settings::source_settings,
            settings::save_claude_path,
            settings::save_copilot_source,
            settings::clear_copilot_source,
            tablet_bridge::tablet_status,
            tablet_bridge::tablet_activity,
            tablet_bridge::tablet_start,
            tablet_bridge::tablet_stop,
            tablet_bridge::tablet_pair_code,
            tablet_bridge::tablet_clear_pairing,
            tablet_bridge::tablet_usb_connect,
            tablet_bridge::tablet_usb_disconnect,
            tablet_bridge::tablet_usb_recover,
            tablet_bridge::tablet_usb_open
        ])
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            if argv.iter().any(|arg| arg == "--probe-ready") {
                // Reaching this callback proves that the existing instance's IPC
                // window is accepting messages. A readiness probe has no UI side effect.
            } else if argv.iter().any(|arg| arg == "--request-exit") {
                exit_app(app);
            } else {
                show_main(app);
            }
        }))
        .setup(move |app| {
            let runtime = app.path().resource_dir()?.join("quota-helper");
            #[cfg(debug_assertions)]
            let runtime = if runtime.join("node.exe").is_file() {
                runtime
            } else {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/quota-helper")
            };
            app.manage(auto_quota::AutoQuota {
                runtime,
                workdir: app.path().app_local_data_dir()?.join("quota-workspace"),
                enabled: std::sync::atomic::AtomicBool::new(true),
                queue: std::sync::Mutex::new(()),
            });
            let polling_app = app.handle().clone();
            std::thread::spawn(move || {
                use std::{
                    sync::atomic::Ordering,
                    time::{Duration, Instant},
                };
                let mut next = Instant::now();
                loop {
                    if polling_app
                        .state::<dashboard::Dashboard>()
                        .lifecycle
                        .cancelled
                        .load(Ordering::Acquire)
                    {
                        break;
                    }
                    if Instant::now() >= next {
                        if polling_app
                            .state::<auto_quota::AutoQuota>()
                            .enabled
                            .load(Ordering::Acquire)
                        {
                            let _ = auto_quota::collect(&polling_app, "all", false);
                        }
                        next = Instant::now() + Duration::from_secs(60);
                    }
                    std::thread::sleep(Duration::from_millis(250));
                }
            });
            app.manage(settings::SettingsStore(std::sync::Mutex::new(
                app.path().app_config_dir()?.join("sources.json"),
            )));
            let show = MenuItem::with_id(app, "show", "Show AgentMeter", true, None::<&str>)?;
            let exit = MenuItem::with_id(app, "exit", "Exit", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &exit])?;
            let pixels = [0x58, 0xa6, 0xff, 0xff].repeat(32 * 32);

            TrayIconBuilder::with_id("main")
                .icon(Image::new_owned(pixels, 32, 32))
                .tooltip("AgentMeter P0")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => show_main(app),
                    "exit" => exit_app(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            if !launch_hidden {
                show_main(app.handle());
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!());
    if let Err(error) = result {
        diagnostics::report_failure(
            "desktop_startup_failed",
            "請重新啟動；若問題持續，請重新執行安裝程式以修復 WebView2。",
            &error.to_string(),
        );
        std::process::exit(1);
    }
}
