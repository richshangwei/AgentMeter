use serde_json::{Value, json};
use std::{sync::Mutex, time::Duration};
use tauri::Manager;
use tauri_plugin_updater::{Update, UpdaterExt};

const UPDATE_ENDPOINT: Option<&str> = option_env!("AGENTMETER_UPDATE_ENDPOINT");
const UPDATE_PUBLIC_KEY: Option<&str> = option_env!("AGENTMETER_UPDATE_PUBLIC_KEY");

pub struct PendingUpdate(pub Mutex<Option<Update>>);

fn validate_release_config<'a>(
    endpoint: &'a str,
    public_key: &'a str,
) -> Result<(&'a str, &'a str), String> {
    if !endpoint.starts_with("https://github.com/")
        || !endpoint.ends_with("/releases/latest/download/latest.json")
        || endpoint.contains(['?', '#'])
    {
        return Err("update_endpoint_invalid".into());
    }
    if public_key.trim().len() < 32 || public_key.len() > 4096 {
        return Err("update_public_key_invalid".into());
    }
    Ok((endpoint, public_key))
}

fn release_config() -> Result<(&'static str, &'static str), String> {
    let endpoint = UPDATE_ENDPOINT.ok_or("update_not_configured")?;
    let public_key = UPDATE_PUBLIC_KEY.ok_or("update_not_configured")?;
    validate_release_config(endpoint, public_key)
}

#[tauri::command]
pub fn update_status(app: tauri::AppHandle) -> Value {
    json!({
        "configured": release_config().is_ok(),
        "current_version": app.package_info().version.to_string(),
        "channel": "stable"
    })
}

#[tauri::command]
pub async fn check_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, PendingUpdate>,
) -> Result<Value, String> {
    let (endpoint, public_key) = match release_config() {
        Ok(value) => value,
        Err(_) => {
            return Ok(
                json!({"configured":false,"available":false,"status":"not_configured",
                "current_version":app.package_info().version.to_string()}),
            );
        }
    };
    let endpoint = endpoint.parse().map_err(|_| "update_endpoint_invalid")?;
    let cleanup_app = app.clone();
    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|_| "update_endpoint_invalid")?
        .pubkey(public_key)
        .timeout(Duration::from_secs(15))
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
        .map_err(|_| "update_check_unavailable")?;
    let update = updater
        .check()
        .await
        .map_err(|_| "update_check_unavailable")?;
    let mut pending = state.0.lock().map_err(|_| "update_state_busy")?;
    if let Some(update) = update {
        let result = json!({"configured":true,"available":true,"status":"available",
            "current_version":update.current_version,"version":update.version,"date":update.date.map(|value|value.to_string())});
        *pending = Some(update);
        Ok(result)
    } else {
        *pending = None;
        Ok(
            json!({"configured":true,"available":false,"status":"current",
            "current_version":app.package_info().version.to_string()}),
        )
    }
}

#[tauri::command]
pub async fn install_update(state: tauri::State<'_, PendingUpdate>) -> Result<(), String> {
    let update = state
        .0
        .lock()
        .map_err(|_| "update_state_busy")?
        .take()
        .ok_or("update_not_checked")?;
    let bytes = match update.download(|_, _| {}, || {}).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let mut pending = state.0.lock().map_err(|_| "update_state_busy")?;
            if pending.is_none() {
                *pending = Some(update);
            }
            return Err("update_download_or_signature_failed".into());
        }
    };
    if update.install(bytes).is_err() {
        let mut pending = state.0.lock().map_err(|_| "update_state_busy")?;
        if pending.is_none() {
            *pending = Some(update);
        }
        return Err("update_install_failed".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_pinned_github_latest_release_configuration_is_accepted() {
        let key = "RWT0123456789012345678901234567890123456789";
        assert!(
            validate_release_config(
                "https://github.com/example/agentmeter/releases/latest/download/latest.json",
                key
            )
            .is_ok()
        );
        for endpoint in [
            "http://github.com/example/agentmeter/releases/latest/download/latest.json",
            "https://example.com/releases/latest/download/latest.json",
            "https://github.com/example/agentmeter/releases/latest/download/other.json",
            "https://github.com/example/agentmeter/releases/latest/download/latest.json?x=1",
        ] {
            assert!(validate_release_config(endpoint, key).is_err());
        }
        assert!(
            validate_release_config(
                "https://github.com/example/agentmeter/releases/latest/download/latest.json",
                "short"
            )
            .is_err()
        );
    }
}
