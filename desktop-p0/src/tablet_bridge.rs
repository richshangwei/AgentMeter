use agentmeter_p0::live_dashboard::LiveDashboard;
use agentmeter_p0::tablet::{LiveRefresh, ServerConfig, TabletServer};
use agentmeter_p0::usb::{OwnedReverse, ReverseMappingState, launch_device_browser, setup_reverse};
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
    time::Duration,
};
use tauri::Manager;

#[derive(Default)]
pub struct TabletBridge {
    runtime: Mutex<Option<TabletRuntime>>,
}

struct TabletRuntime {
    server: Option<TabletServer>,
    usb: Option<UsbRuntime>,
}

struct UsbRuntime {
    adb: PathBuf,
    serial: String,
    device_port: u16,
    mapping: Option<OwnedReverse>,
    browser_launch_requested: bool,
}

impl Drop for TabletRuntime {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

impl TabletRuntime {
    fn shutdown(&mut self) -> Result<(), String> {
        // Stop authorization, SSE and the listener before removing the selected
        // device mapping. OwnedReverse refuses to remove a changed entry.
        drop(self.server.take());
        let teardown = self
            .usb
            .as_mut()
            .and_then(|usb| usb.mapping.take())
            .map(OwnedReverse::teardown);
        self.usb.take();
        if let Some(Err(failure)) = teardown {
            return Err(format!("{}: {}", failure.code, failure.message));
        }
        Ok(())
    }
}

impl TabletBridge {
    fn view(runtime: Option<&TabletRuntime>) -> Value {
        let Some(runtime) = runtime else {
            return json!({
                "running": false,
                "origin": null,
                "provider_data": "live",
                "transport": null,
                "usb": null,
            });
        };
        let server = runtime.server.as_ref().expect("running server");
        let usb = runtime.usb.as_ref().map(|usb| {
            let (mapping, trustworthy) = match usb.mapping.as_ref().map(OwnedReverse::inspect) {
                Some(Ok(ReverseMappingState::Owned)) => {
                    (json!({"state": ReverseMappingState::Owned}), true)
                }
                Some(Ok(state)) => (json!({"state": state}), false),
                Some(Err(failure)) => (
                    json!({
                        "state": "unavailable",
                        "failure_code": failure.code,
                        "message": failure.message,
                    }),
                    false,
                ),
                None => (json!({"state": ReverseMappingState::Missing}), false),
            };
            if !trustworthy {
                server.revoke_sessions_for_transport_change();
            } else {
                server.set_usb_device_port(usb.device_port);
            }
            json!({
                "selected_serial": usb.serial,
                "transport": "usb",
                "device_port": usb.device_port,
                "host_port": server.addr().port(),
                "mapping": mapping,
                "browser_launch_requested": usb.browser_launch_requested,
            })
        });
        json!({
            "running": true,
            "origin": server.origin(),
            "provider_data": "live",
            "transport": server.transport_status(),
            "usb": usb,
        })
    }

    fn start_at(
        &self,
        data_directory: &Path,
        dashboard: LiveDashboard,
        refresh: LiveRefresh,
    ) -> Result<Value, String> {
        let mut slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        if slot.is_some() {
            return Ok(Self::view(slot.as_ref()));
        }
        std::fs::create_dir_all(data_directory).map_err(|_| "無法建立平板配對資料目錄")?;
        let config = ServerConfig {
            pair_store_path: Some(data_directory.join("tablet-pairs.dpapi")),
            ..ServerConfig::default()
        };
        let server = TabletServer::start_live(config, dashboard, refresh)
            .map_err(|_| "無法啟動本機平板服務")?;
        let runtime = TabletRuntime {
            server: Some(server),
            usb: None,
        };
        let view = Self::view(Some(&runtime));
        *slot = Some(runtime);
        Ok(view)
    }

    pub fn stop(&self) -> Result<Value, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "平板服務狀態無法使用")?
            .take();
        if let Some(runtime) = runtime.as_mut() {
            runtime.shutdown()?;
        }
        drop(runtime);
        Ok(Self::view(None))
    }

    fn status(&self) -> Result<Value, String> {
        let slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        Ok(Self::view(slot.as_ref()))
    }

    fn activity(&self) -> Result<Value, String> {
        let slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let Some(server) = slot.as_ref().and_then(|runtime| runtime.server.as_ref()) else {
            return Ok(json!({"running": false, "transport": null}));
        };
        Ok(json!({
            "running": true,
            "transport": server.transport_status(),
        }))
    }

    fn rotate_pair_code(&self) -> Result<Value, String> {
        let slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let server = slot
            .as_ref()
            .and_then(|runtime| runtime.server.as_ref())
            .ok_or("請先啟動平板服務")?;
        Ok(json!({
            "code": server.rotate_pair_code(),
            "expires_in_seconds": 120,
        }))
    }

    fn clear_pairing(&self) -> Result<Value, String> {
        let slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let server = slot
            .as_ref()
            .and_then(|runtime| runtime.server.as_ref())
            .ok_or("請先啟動平板服務")?;
        server
            .clear_pairing()
            .map_err(|_| "配對撤銷無法安全寫入；服務已停止授權")?;
        Ok(json!({"cleared": true}))
    }

    fn connect_usb(&self, adb: PathBuf, serial: String, device_port: u16) -> Result<Value, String> {
        let mut slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let runtime = slot.as_mut().ok_or("請先啟動平板服務")?;
        if runtime.usb.is_some() {
            return Err("USB mapping 已設定；請先安全中斷再更換裝置".into());
        }
        let host_port = runtime
            .server
            .as_ref()
            .expect("running server")
            .addr()
            .port();
        let (_, mapping) = setup_reverse(
            &adb,
            &serial,
            device_port,
            host_port,
            Duration::from_secs(5),
        )
        .map_err(|failure| format!("{}: {}", failure.code, failure.message))?;
        runtime
            .server
            .as_ref()
            .expect("running server")
            .revoke_sessions_for_transport_change();
        runtime.usb = Some(UsbRuntime {
            adb,
            serial,
            device_port,
            mapping: Some(mapping),
            browser_launch_requested: false,
        });
        runtime
            .server
            .as_ref()
            .expect("running server")
            .set_usb_device_port(device_port);
        Ok(Self::view(Some(runtime)))
    }

    fn disconnect_usb(&self) -> Result<Value, String> {
        let mut slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let runtime = slot.as_mut().ok_or("請先啟動平板服務")?;
        let Some(mut usb) = runtime.usb.take() else {
            return Ok(Self::view(Some(runtime)));
        };
        runtime
            .server
            .as_ref()
            .expect("running server")
            .revoke_sessions_for_transport_change();
        if let Some(Err(failure)) = usb.mapping.take().map(OwnedReverse::teardown) {
            return Err(format!("{}: {}", failure.code, failure.message));
        }
        Ok(Self::view(Some(runtime)))
    }

    fn recover_usb(&self) -> Result<Value, String> {
        let mut slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let runtime = slot.as_mut().ok_or("請先啟動平板服務")?;
        let server = runtime.server.as_ref().expect("running server");
        let usb = runtime.usb.as_mut().ok_or("尚未設定 USB mapping")?;
        match usb.mapping.as_ref().map(OwnedReverse::inspect) {
            Some(Ok(ReverseMappingState::Owned)) => return Ok(Self::view(Some(runtime))),
            Some(Ok(ReverseMappingState::Changed)) => {
                server.revoke_sessions_for_transport_change();
                return Err("mapping_ownership_lost: mapping 已由外部變更，拒絕覆寫".into());
            }
            Some(Err(failure)) => {
                server.revoke_sessions_for_transport_change();
                return Err(format!("{}: {}", failure.code, failure.message));
            }
            Some(Ok(ReverseMappingState::Missing)) => {
                server.revoke_sessions_for_transport_change();
                usb.mapping.take().expect("inspected mapping").abandon();
            }
            None => server.revoke_sessions_for_transport_change(),
        }
        let (_, mapping) = setup_reverse(
            &usb.adb,
            &usb.serial,
            usb.device_port,
            server.addr().port(),
            Duration::from_secs(5),
        )
        .map_err(|failure| format!("{}: {}", failure.code, failure.message))?;
        usb.mapping = Some(mapping);
        server.set_usb_device_port(usb.device_port);
        usb.browser_launch_requested = false;
        Ok(Self::view(Some(runtime)))
    }

    fn open_usb_browser(&self) -> Result<Value, String> {
        let mut slot = self.runtime.lock().map_err(|_| "平板服務狀態無法使用")?;
        let runtime = slot.as_mut().ok_or("請先啟動平板服務")?;
        let usb = runtime.usb.as_mut().ok_or("尚未設定 USB mapping")?;
        match usb.mapping.as_ref().map(OwnedReverse::inspect) {
            Some(Ok(ReverseMappingState::Owned)) => {}
            Some(Ok(ReverseMappingState::Missing)) | None => {
                runtime
                    .server
                    .as_ref()
                    .expect("running server")
                    .revoke_sessions_for_transport_change();
                return Err("reverse_missing: 請先執行 USB 復原".into());
            }
            Some(Ok(ReverseMappingState::Changed)) => {
                runtime
                    .server
                    .as_ref()
                    .expect("running server")
                    .revoke_sessions_for_transport_change();
                return Err("mapping_ownership_lost: mapping 已由外部變更".into());
            }
            Some(Err(failure)) => {
                runtime
                    .server
                    .as_ref()
                    .expect("running server")
                    .revoke_sessions_for_transport_change();
                return Err(format!("{}: {}", failure.code, failure.message));
            }
        }
        launch_device_browser(
            &usb.adb,
            &usb.serial,
            usb.device_port,
            Duration::from_secs(5),
        )
        .map_err(|failure| format!("{}: {}", failure.code, failure.message))?;
        usb.browser_launch_requested = true;
        Ok(Self::view(Some(runtime)))
    }
}

#[tauri::command]
pub fn tablet_status(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.status()
}

#[tauri::command]
pub fn tablet_activity(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.activity()
}

#[tauri::command]
pub fn tablet_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, TabletBridge>,
) -> Result<Value, String> {
    let data_directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "無法解析本機應用程式資料目錄")?;
    let live = app.state::<crate::dashboard::Dashboard>().live.clone();
    let refresh_app = app.clone();
    let refresh: LiveRefresh = std::sync::Arc::new(move |provider| {
        crate::auto_quota::collect(&refresh_app, provider, false)
    });
    state.start_at(&data_directory, live, refresh)
}

#[tauri::command]
pub fn tablet_stop(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.stop()
}

#[tauri::command]
pub fn tablet_pair_code(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.rotate_pair_code()
}

#[tauri::command]
pub fn tablet_clear_pairing(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.clear_pairing()
}

#[tauri::command]
pub fn tablet_usb_connect(
    adb_path: String,
    serial: String,
    device_port: u16,
    state: tauri::State<'_, TabletBridge>,
) -> Result<Value, String> {
    state.connect_usb(PathBuf::from(adb_path), serial, device_port)
}

#[tauri::command]
pub fn tablet_usb_disconnect(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.disconnect_usb()
}

#[tauri::command]
pub fn tablet_usb_recover(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.recover_usb()
}

#[tauri::command]
pub fn tablet_usb_open(state: tauri::State<'_, TabletBridge>) -> Result<Value, String> {
    state.open_usb_browser()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::TcpStream,
        sync::atomic::{AtomicU64, Ordering},
        time::{SystemTime, UNIX_EPOCH},
    };

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    fn test_directory() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "agentmeter-desktop-tablet-{}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn explicit_start_pair_clear_and_stop_own_the_server_lifecycle() {
        let directory = test_directory();
        let bridge = TabletBridge::default();
        assert_eq!(bridge.status().unwrap()["running"], false);
        let started = bridge
            .start_at(
                &directory,
                LiveDashboard::default(),
                std::sync::Arc::new(|_| Ok(())),
            )
            .unwrap();
        assert_eq!(started["running"], true);
        assert_eq!(started["provider_data"], "live");
        assert!(started.get("code").is_none());
        let origin = started["origin"].as_str().unwrap();
        let address = origin.strip_prefix("http://").unwrap();
        let mut connection = TcpStream::connect(address).unwrap();
        write!(
            connection,
            "GET / HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut response = String::new();
        connection.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 200"));
        let code = bridge.rotate_pair_code().unwrap();
        assert_eq!(code["code"].as_str().unwrap().len(), 8);
        assert_eq!(code["expires_in_seconds"], 120);
        assert!(bridge.status().unwrap().get("code").is_none());
        assert_eq!(
            bridge.activity().unwrap()["transport"]["authenticated_session_established"],
            false
        );
        assert!(!response.contains(code["code"].as_str().unwrap()));
        let invalid_usb = bridge
            .connect_usb(PathBuf::from("adb.exe"), "USB-1".into(), 8317)
            .unwrap_err();
        assert!(invalid_usb.starts_with("invalid_adb_path:"));
        assert!(bridge.status().unwrap()["usb"].is_null());
        assert_eq!(bridge.clear_pairing().unwrap()["cleared"], true);
        assert_eq!(bridge.stop().unwrap()["running"], false);
        assert!(bridge.rotate_pair_code().is_err());
        std::fs::remove_dir_all(directory).unwrap();
    }
}
