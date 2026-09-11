use serde_json::{Value, json};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::Manager;
use tauri_plugin_updater::{Update, UpdaterExt};

const UPDATE_ENDPOINT: Option<&str> = option_env!("AGENTMETER_UPDATE_ENDPOINT");
const UPDATE_PUBLIC_KEY: Option<&str> = option_env!("AGENTMETER_UPDATE_PUBLIC_KEY");

#[derive(Default)]
struct UpdateData {
    update: Option<Update>,
    bytes: Option<Arc<Vec<u8>>>,
    status: Option<&'static str>,
    downloaded: u64,
    total: Option<u64>,
    error: Option<&'static str>,
}

#[derive(Default)]
pub struct PendingUpdate {
    busy: AtomicBool,
    data: Mutex<UpdateData>,
}

struct Operation<'a>(&'a PendingUpdate);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        if let Ok(mut data) = self.0.data.lock()
            && matches!(data.status, Some("checking" | "downloading" | "installing"))
        {
            data.status = Some("error");
            data.error = Some("update_operation_interrupted");
        }
        self.0.busy.store(false, Ordering::Release);
    }
}

impl PendingUpdate {
    fn begin(&self) -> Result<Operation<'_>, String> {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "update_state_busy")?;
        Ok(Operation(self))
    }
    fn change(&self, f: impl FnOnce(&mut UpdateData)) -> Result<(), String> {
        let mut data = self.data.lock().map_err(|_| "update_state_busy")?;
        f(&mut data);
        Ok(())
    }
    fn fail(&self, error: &'static str) -> String {
        let _ = self.change(|data| {
            data.status = Some("error");
            data.error = Some(error);
        });
        error.into()
    }
    fn snapshot(&self, current_version: &str, configured: bool) -> Value {
        match self.data.lock() {
            Ok(data) => json!({"configured":configured,"current_version":current_version,
                "channel":"stable","status":if configured {data.status.unwrap_or("idle")} else {"not_configured"},
                "available": data.update.is_some(), "version":data.update.as_ref().map(|u| &u.version),
                "downloaded_bytes":data.downloaded,"total_bytes":data.total,"error":data.error,
                "can_download":configured && data.update.is_some() && data.bytes.is_none(),
                "can_install":configured && data.bytes.is_some()}),
            Err(_) => {
                json!({"configured":configured,"current_version":current_version,"status":"error","error":"update_state_busy"})
            }
        }
    }
}

fn release_repo(endpoint: &str) -> Option<&str> {
    let repo = endpoint
        .strip_prefix("https://github.com/")?
        .strip_suffix("/releases/latest/download/latest.json")?;
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|part| {
            part.is_empty()
                || *part == "."
                || *part == ".."
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        })
    {
        return None;
    }
    Some(repo)
}

fn validate_release_config<'a>(
    endpoint: &'a str,
    public_key: &'a str,
) -> Result<(&'a str, &'a str), String> {
    if release_repo(endpoint).is_none() {
        return Err("update_endpoint_invalid".into());
    }
    if public_key.trim().len() < 32 || public_key.len() > 4096 {
        return Err("update_public_key_invalid".into());
    }
    Ok((endpoint, public_key))
}

fn validate_artifact(endpoint: &str, version: &str, url: &tauri::Url) -> bool {
    let Some(repo) = release_repo(endpoint) else {
        return false;
    };
    // The manifest is not itself signed. Bind its version to the release tag and
    // exact installer name to reject references to another version's artifact.
    // Release publishing must still preserve the immutable tag/asset invariant.
    let components: Vec<_> = version.split('.').collect();
    if components.len() != 3
        || components.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return false;
    }
    let expected_path =
        format!("/{repo}/releases/download/v{version}/AgentMeter-P0_{version}_x64-setup.exe");
    url.scheme() == "https"
        && url.host_str() == Some("github.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path() == expected_path
}

fn release_config() -> Result<(&'static str, &'static str), String> {
    validate_release_config(
        UPDATE_ENDPOINT.ok_or("update_not_configured")?,
        UPDATE_PUBLIC_KEY.ok_or("update_not_configured")?,
    )
}

#[tauri::command]
pub fn update_status(app: tauri::AppHandle, state: tauri::State<'_, PendingUpdate>) -> Value {
    state.snapshot(
        &app.package_info().version.to_string(),
        release_config().is_ok(),
    )
}

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Value, String> {
    let _operation = state.begin()?;
    let current = app.package_info().version.to_string();
    let Ok((endpoint, public_key)) = release_config() else {
        return Ok(state.snapshot(&current, false));
    };
    let ready = state
        .data
        .lock()
        .map_err(|_| "update_state_busy")?
        .bytes
        .is_some();
    if ready {
        return Ok(state.snapshot(&current, true));
    }
    state.change(|data| {
        data.status = Some("checking");
        data.error = None;
    })?;
    let cleanup_app = app.clone();
    let updater = app
        .updater_builder()
        .endpoints(vec![
            endpoint
                .parse()
                .map_err(|_| state.fail("update_endpoint_invalid"))?,
        ])
        .map_err(|_| state.fail("update_endpoint_invalid"))?
        .pubkey(public_key)
        .timeout(Duration::from_secs(120))
        .configure_client(|client| client.https_only(true))
        .on_before_exit(move || {
            let _ = cleanup_app
                .state::<crate::tablet_bridge::TabletBridge>()
                .stop();
            cleanup_app
                .state::<crate::dashboard::Dashboard>()
                .lifecycle
                .stop_and_wait();
        })
        .build()
        .map_err(|_| state.fail("update_check_unavailable"))?;
    let update = updater
        .check()
        .await
        .map_err(|_| state.fail("update_check_unavailable"))?;
    if update
        .as_ref()
        .is_some_and(|u| !validate_artifact(endpoint, &u.version, &u.download_url))
    {
        return Err(state.fail("update_artifact_url_invalid"));
    }
    state.change(|data| {
        data.status = Some(if update.is_some() {
            "available"
        } else {
            "current"
        });
        data.update = update;
        data.bytes = None;
        data.downloaded = 0;
        data.total = None;
    })?;
    Ok(state.snapshot(&current, true))
}

#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Value, String> {
    let _operation = state.begin()?;
    release_config()?;
    let current = app.package_info().version.to_string();
    let update = {
        let data = state.data.lock().map_err(|_| "update_state_busy")?;
        if data.bytes.is_some() {
            drop(data);
            return Ok(state.snapshot(&current, true));
        }
        data.update.clone().ok_or("update_not_checked")?
    };
    state.change(|data| {
        data.status = Some("downloading");
        data.error = None;
        data.downloaded = 0;
        data.total = None;
    })?;
    // The pinned SDK verifies the signature before returning these bytes.
    let bytes = update
        .download(
            |chunk, total| {
                let _ = state.change(|data| {
                    data.downloaded = data.downloaded.saturating_add(chunk as u64);
                    data.total = total;
                });
            },
            || {},
        )
        .await
        .map_err(|_| state.fail("update_download_or_signature_failed"))?;
    state.change(|data| {
        data.downloaded = bytes.len() as u64;
        data.bytes = Some(Arc::new(bytes));
        data.status = Some("ready");
    })?;
    Ok(state.snapshot(&current, true))
}

#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Value, String> {
    let _operation = state.begin()?;
    release_config()?;
    let (update, bytes) = {
        let data = state.data.lock().map_err(|_| "update_state_busy")?;
        (
            data.update.clone().ok_or("update_not_checked")?,
            data.bytes.clone().ok_or("update_not_downloaded")?,
        )
    };
    state.change(|data| {
        data.status = Some("installing");
        data.error = None;
    })?;
    update
        .install(bytes.as_slice())
        // SDK errors before on_before_exit (including extraction) are retryable.
        // On Windows the SDK then launches via ShellExecuteW and exits without
        // checking its result; launch failures after shutdown cannot be recovered here.
        .map_err(|_| state.fail("update_install_failed"))?;
    state.change(|data| data.status = Some("ready"))?;
    Ok(state.snapshot(&app.package_info().version.to_string(), true))
}

#[cfg(test)]
mod tests {
    use super::*;
    const ENDPOINT: &str =
        "https://github.com/example/agentmeter/releases/latest/download/latest.json";
    #[test]
    fn only_exact_github_release_configuration_is_accepted() {
        let key = "RWT0123456789012345678901234567890123456789";
        assert!(validate_release_config(ENDPOINT, key).is_ok());
        for endpoint in [
            "http://github.com/a/b/releases/latest/download/latest.json",
            "https://github.com/a/b/c/releases/latest/download/latest.json",
            "https://github.com/a/../releases/latest/download/latest.json",
            "https://github.com/a/b/releases/latest/download/latest.json?x=1",
        ] {
            assert!(validate_release_config(endpoint, key).is_err());
        }
        assert!(validate_release_config(ENDPOINT, "short").is_err());
    }
    #[test]
    fn artifacts_must_be_https_releases_in_configured_repo() {
        assert!(validate_artifact(
            ENDPOINT,
            "0.2.0",
            &"https://github.com/example/agentmeter/releases/download/v0.2.0/AgentMeter-P0_0.2.0_x64-setup.exe"
                .parse()
                .unwrap()
        ));
        for url in [
            "http://github.com/example/agentmeter/releases/download/v0.2.0/AgentMeter-P0_0.2.0_x64-setup.exe",
            "https://github.com/attacker/agentmeter/releases/download/v0.2.0/AgentMeter-P0_0.2.0_x64-setup.exe",
            "https://github.com/example/agentmeter/releases/download/v0.2.0/AgentMeter-P0_0.2.0_x64-setup.exe?x=1",
            "https://example.com/app.exe",
            "https://github.com/example/agentmeter/releases/download/v1/app.json",
        ] {
            assert!(!validate_artifact(ENDPOINT, "0.2.0", &url.parse().unwrap()));
        }
    }
    #[test]
    fn artifact_tag_and_exact_installer_name_must_match_manifest_version() {
        for path in [
            "v0.1.0/AgentMeter-P0_0.2.0_x64-setup.exe",
            "v0.2.0/AgentMeter-P0_0.1.0_x64-setup.exe",
            "v0.2.0/OtherApp_0.2.0_x64-setup.exe",
            "v0.2.0/AgentMeter-P0_0.2.0_arm64-setup.exe",
            "v0.2.0/AgentMeter-P0_0.2.0_x64-setup.exe.nsis.zip",
            "v0.2.0/AgentMeter%20P0_0.2.0_x64-setup.exe",
        ] {
            let url = format!("https://github.com/example/agentmeter/releases/download/{path}")
                .parse()
                .unwrap();
            assert!(!validate_artifact(ENDPOINT, "0.2.0", &url));
        }
        let url = "https://github.com/example/agentmeter/releases/download/v0.2.0/AgentMeter-P0_0.2.0_x64-setup.exe".parse().unwrap();
        for version in ["", "../0.2.0", "00.2.0", "0.2", "0.2.0-beta.1"] {
            assert!(!validate_artifact(ENDPOINT, version, &url));
        }
    }
    #[test]
    fn operations_are_serialized_and_drop_unlocks_after_cancellation() {
        let state = PendingUpdate::default();
        let guard = state.begin().unwrap();
        state
            .change(|data| data.status = Some("downloading"))
            .unwrap();
        assert!(state.begin().is_err());
        drop(guard);
        assert_eq!(
            state.snapshot("1.0.0", true)["error"],
            "update_operation_interrupted"
        );
        assert!(state.begin().is_ok());
    }
    #[test]
    fn failure_retains_verified_bytes_and_finished_state_survives_guard_drop() {
        let state = PendingUpdate::default();
        let guard = state.begin().unwrap();
        state
            .change(|data| {
                data.bytes = Some(Arc::new(vec![1, 2, 3]));
                data.status = Some("ready");
            })
            .unwrap();
        drop(guard);
        assert_eq!(state.snapshot("1", true)["status"], "ready");
        state.fail("update_install_failed");
        assert_eq!(
            state
                .data
                .lock()
                .unwrap()
                .bytes
                .as_deref()
                .unwrap()
                .as_slice(),
            &[1, 2, 3]
        );
        assert_eq!(state.snapshot("1", true)["can_install"], true);
        assert_eq!(state.snapshot("1", false)["status"], "not_configured");
    }
}
