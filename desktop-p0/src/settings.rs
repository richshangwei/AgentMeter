use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

pub struct SettingsStore(pub Mutex<PathBuf>);

pub const DEFAULT_SYNC_INTERVAL_SECONDS: u64 = 120;
const SYNC_INTERVAL_SECONDS: [u64; 6] = [120, 300, 600, 900, 1800, 3600];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SyncPreferences {
    pub enabled: bool,
    pub interval_seconds: u64,
}

impl Default for SyncPreferences {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: DEFAULT_SYNC_INTERVAL_SECONDS,
        }
    }
}

fn validate_sync_interval(interval_seconds: u64) -> Result<u64, String> {
    SYNC_INTERVAL_SECONDS
        .contains(&interval_seconds)
        .then_some(interval_seconds)
        .ok_or_else(|| "不支援的額度更新頻率".into())
}

pub fn sync_preferences(file: &Path) -> Result<SyncPreferences, String> {
    let handle = match fs::File::open(file) {
        Ok(handle) => handle,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SyncPreferences::default());
        }
        Err(_) => return Err("無法讀取額度同步設定".into()),
    };
    let mut bytes = Vec::new();
    handle
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "無法讀取額度同步設定")?;
    if bytes.len() > 4096 {
        return Err("額度同步設定超過大小限制".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "額度同步設定格式損毀")?;
    if value["version"] != 1 {
        return Err("不支援此額度同步設定版本".into());
    }
    Ok(SyncPreferences {
        enabled: value["enabled"]
            .as_bool()
            .ok_or("額度同步設定缺少啟用狀態")?,
        interval_seconds: validate_sync_interval(
            value["interval_seconds"]
                .as_u64()
                .ok_or("額度同步設定缺少更新頻率")?,
        )?,
    })
}

pub fn save_sync_preferences(
    file: &Path,
    enabled: bool,
    interval_seconds: u64,
) -> Result<SyncPreferences, String> {
    let preferences = SyncPreferences {
        enabled,
        interval_seconds: validate_sync_interval(interval_seconds)?,
    };
    let bytes = serde_json::to_vec(&json!({
        "version": 1,
        "enabled": preferences.enabled,
        "interval_seconds": preferences.interval_seconds
    }))
    .map_err(|_| "無法編碼額度同步設定")?;
    let parent = file.parent().ok_or("無效設定位置")?;
    fs::create_dir_all(parent).map_err(|_| "無法建立設定目錄")?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!("quota-sync-{}-{nonce}.tmp", std::process::id()));
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "無法建立暫存設定")?;
    let result = (|| {
        handle
            .write_all(&bytes)
            .map_err(|_| "無法寫入額度同步設定")?;
        handle.sync_all().map_err(|_| "無法同步額度同步設定")?;
        drop(handle);
        fs::rename(&temporary, file).map_err(|_| "無法替換額度同步設定")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map(|()| preferences)
}

fn validate_path(path: &str) -> Result<(), String> {
    if path.is_empty()
        || (path.len() <= 8192 && !path.contains('\0') && Path::new(path).is_absolute())
    {
        Ok(())
    } else {
        Err("請輸入有效的報告完整路徑".into())
    }
}

fn defaults() -> Value {
    json!({"version":2,"claude_path":"","copilot":Value::Null})
}

fn validate_copilot(value: &Value) -> Result<Value, String> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let gh_path = value["gh_path"]
        .as_str()
        .ok_or("Copilot 設定缺少 GitHub CLI 路徑")?;
    let context = value["context"]
        .as_str()
        .ok_or("Copilot 設定缺少帳務範圍")?;
    let account = value["account"].as_str().ok_or("Copilot 設定缺少帳號")?;
    let meter = value["meter"].as_str().ok_or("Copilot 設定缺少計量類型")?;
    // Empty values persist automatic discovery, not a guessed installation or identity.
    let validation_path = if gh_path.is_empty() {
        std::env::temp_dir().join("gh.exe")
    } else {
        PathBuf::from(gh_path)
    };
    let validation_account = if account.is_empty() && context == "personal" {
        "automatic"
    } else {
        account
    };
    agentmeter_p0::copilot::UsageRequest::new(
        validation_path,
        context,
        validation_account,
        meter,
        None,
        None,
        std::time::Duration::from_secs(10),
    )
    .map_err(|_| "Copilot 來源設定格式不符")?;
    Ok(json!({
        "gh_path":gh_path,
        "context":context,
        "account":account,
        "meter":meter
    }))
}

fn normalize(value: &Value) -> Result<Value, String> {
    let version = value["version"].as_u64().ok_or("來源設定缺少版本")?;
    let claude_path = value["claude_path"]
        .as_str()
        .ok_or("來源設定缺少 Claude 路徑")?;
    validate_path(claude_path)?;
    let copilot = match version {
        1 => Value::Null,
        2 => validate_copilot(value.get("copilot").ok_or("來源設定缺少 Copilot 欄位")?)?,
        _ => return Err("不支援此來源設定版本".into()),
    };
    Ok(json!({"version":2,"claude_path":claude_path,"copilot":copilot}))
}

fn read(file: &Path) -> Result<Value, String> {
    let handle = match fs::File::open(file) {
        Ok(handle) => handle,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(defaults()),
        Err(_) => return Err("無法讀取來源設定".into()),
    };
    let mut bytes = Vec::new();
    handle
        .take(16385)
        .read_to_end(&mut bytes)
        .map_err(|_| "無法讀取來源設定")?;
    if bytes.len() > 16384 {
        return Err("來源設定超過大小限制".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "來源設定格式損毀")?;
    normalize(&value)
}

fn write_document(file: &Path, value: &Value) -> Result<(), String> {
    let normalized = normalize(value)?;
    let bytes = serde_json::to_vec(&normalized).map_err(|_| "無法編碼設定")?;
    if bytes.len() > 16384 {
        return Err("來源設定超過大小限制".into());
    }
    let parent = file.parent().ok_or("無效設定位置")?;
    fs::create_dir_all(parent).map_err(|_| "無法建立設定目錄")?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!("sources-{}-{nonce}.tmp", std::process::id()));
    let mut handle = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "無法建立暫存設定")?;
    let result = (|| {
        handle.write_all(&bytes).map_err(|_| "無法寫入設定")?;
        handle.sync_all().map_err(|_| "無法同步設定")?;
        drop(handle);
        fs::rename(&temporary, file).map_err(|_| "無法替換設定")?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn save_claude(file: &Path, path: &str) -> Result<(), String> {
    validate_path(path)?;
    let mut settings = read(file)?;
    settings["claude_path"] = json!(path);
    write_document(file, &settings)
}

fn save_copilot(
    file: &Path,
    gh_path: &str,
    context: &str,
    account: &str,
    meter: &str,
) -> Result<(), String> {
    let copilot = validate_copilot(&json!({
        "gh_path":gh_path,
        "context":context,
        "account":account,
        "meter":meter
    }))?;
    let mut settings = read(file)?;
    settings["copilot"] = copilot;
    write_document(file, &settings)
}

fn clear_copilot(file: &Path) -> Result<(), String> {
    let mut settings = read(file)?;
    settings["copilot"] = Value::Null;
    write_document(file, &settings)
}

#[tauri::command]
pub fn source_settings(state: tauri::State<'_, SettingsStore>) -> Result<Value, String> {
    let file = state.0.lock().map_err(|_| "設定忙碌")?;
    read(&file)
}

#[tauri::command]
pub fn save_claude_path(
    path: String,
    state: tauri::State<'_, SettingsStore>,
) -> Result<(), String> {
    let file = state.0.lock().map_err(|_| "設定忙碌")?;
    save_claude(&file, path.trim())
}

#[tauri::command]
pub fn save_copilot_source(
    gh_path: String,
    context: String,
    account: String,
    meter: String,
    state: tauri::State<'_, SettingsStore>,
) -> Result<(), String> {
    let file = state.0.lock().map_err(|_| "設定忙碌")?;
    save_copilot(
        &file,
        gh_path.trim(),
        context.trim(),
        account.trim(),
        meter.trim(),
    )
}

#[tauri::command]
pub fn clear_copilot_source(state: tauri::State<'_, SettingsStore>) -> Result<(), String> {
    let file = state.0.lock().map_err(|_| "設定忙碌")?;
    clear_copilot(&file)
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!(
                "agentmeter-settings-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&dir).unwrap();
            Self(dir)
        }
        fn file(&self) -> PathBuf {
            self.0.join("sources.json")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn sources_round_trip_without_overwriting_each_other_and_without_token_fields() {
        let fixture = Fixture::new();
        let file = fixture.file();
        let empty = read(&file).unwrap();
        assert_eq!(empty["version"], 2);
        assert_eq!(empty["claude_path"], "");
        assert!(empty["copilot"].is_null());
        let selected = fixture.0.join("report.json").to_string_lossy().into_owned();
        let gh = fixture.0.join("gh.exe").to_string_lossy().into_owned();
        save_claude(&file, &selected).unwrap();
        save_copilot(&file, &gh, "business", "example-org", "ai-credits").unwrap();
        let saved = read(&file).unwrap();
        assert_eq!(saved["claude_path"], selected);
        assert_eq!(saved["copilot"]["gh_path"], gh);
        assert_eq!(saved["copilot"]["context"], "business");
        assert_eq!(saved["copilot"]["account"], "example-org");
        assert_eq!(saved["copilot"]["meter"], "ai-credits");
        let raw = fs::read_to_string(&file).unwrap();
        assert!(!raw.to_ascii_lowercase().contains("token"));
        clear_copilot(&file).unwrap();
        let cleared = read(&file).unwrap();
        assert_eq!(cleared["claude_path"], selected);
        assert!(cleared["copilot"].is_null());
        save_claude(&file, "").unwrap();
        assert_eq!(read(&file).unwrap()["claude_path"], "");
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    }

    #[test]
    fn version_one_is_read_compatibly_and_migrated_on_the_next_write() {
        let fixture = Fixture::new();
        let selected = fixture.0.join("report.json").to_string_lossy().into_owned();
        fs::write(
            fixture.file(),
            serde_json::to_vec(&json!({"version":1,"claude_path":selected})).unwrap(),
        )
        .unwrap();
        let migrated = read(&fixture.file()).unwrap();
        assert_eq!(migrated["version"], 2);
        assert_eq!(migrated["claude_path"], selected);
        assert!(migrated["copilot"].is_null());
        assert!(
            fs::read_to_string(fixture.file())
                .unwrap()
                .contains("\"version\":1")
        );

        let gh = fixture.0.join("gh.exe").to_string_lossy().into_owned();
        save_copilot(
            &fixture.file(),
            &gh,
            "personal",
            "octocat",
            "premium-requests",
        )
        .unwrap();
        let persisted = read(&fixture.file()).unwrap();
        assert_eq!(persisted["version"], 2);
        assert_eq!(persisted["claude_path"], selected);
        assert_eq!(persisted["copilot"]["account"], "octocat");
    }

    #[test]
    fn invalid_source_values_preserve_previous_settings() {
        let fixture = Fixture::new();
        save_claude(&fixture.file(), "").unwrap();
        let before = fs::read(fixture.file()).unwrap();
        assert!(save_claude(&fixture.file(), "relative.json").is_err());
        for invalid in [
            ("relative-gh.exe", "personal", "octocat", "ai-credits"),
            ("C:\\gh.exe", "team", "octocat", "ai-credits"),
            ("C:\\gh.exe", "personal", "../octocat", "ai-credits"),
            ("C:\\gh.exe", "personal", "octocat", "requests"),
        ] {
            assert!(
                save_copilot(&fixture.file(), invalid.0, invalid.1, invalid.2, invalid.3).is_err()
            );
        }
        assert_eq!(fs::read(fixture.file()).unwrap(), before);
    }

    #[test]
    fn automatic_source_round_trips_without_persisting_a_guessed_identity() {
        let fixture = Fixture::new();
        save_copilot(&fixture.file(), "", "personal", "", "ai-credits").unwrap();
        let saved = read(&fixture.file()).unwrap();
        assert_eq!(saved["copilot"]["gh_path"], "");
        assert_eq!(saved["copilot"]["account"], "");
        assert!(save_copilot(&fixture.file(), "", "business", "", "ai-credits").is_err());
    }
    #[test]
    fn malformed_unknown_version_and_oversized_are_not_silently_reset() {
        let fixture = Fixture::new();
        for bytes in [
            b"broken".to_vec(),
            br#"{"version":3,"claude_path":"","copilot":null}"#.to_vec(),
            br#"{"version":2,"claude_path":""}"#.to_vec(),
            vec![b' '; 16385],
        ] {
            fs::write(fixture.file(), &bytes).unwrap();
            assert!(read(&fixture.file()).is_err());
            assert_eq!(fs::read(fixture.file()).unwrap(), bytes);
        }
    }

    #[test]
    fn quota_sync_preferences_migrate_validate_and_persist() {
        let fixture = Fixture::new();
        let file = fixture.0.join("quota-sync.json");

        let defaults = sync_preferences(&file).unwrap();
        assert!(defaults.enabled);
        assert_eq!(defaults.interval_seconds, 120);

        save_sync_preferences(&file, false, 900).unwrap();
        let saved = sync_preferences(&file).unwrap();
        assert!(!saved.enabled);
        assert_eq!(saved.interval_seconds, 900);
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(&file).unwrap()).unwrap()["version"],
            1
        );

        save_sync_preferences(&file, true, 300).unwrap();
        assert_eq!(
            sync_preferences(&file).unwrap(),
            SyncPreferences {
                enabled: true,
                interval_seconds: 300
            }
        );

        let before = fs::read(&file).unwrap();
        for invalid in [0, 59, 60, 121, 86_400] {
            assert!(save_sync_preferences(&file, true, invalid).is_err());
            assert_eq!(fs::read(&file).unwrap(), before);
        }

        for bytes in [
            b"broken".to_vec(),
            br#"{"version":2,"enabled":true,"interval_seconds":300}"#.to_vec(),
            br#"{"version":1,"enabled":true,"interval_seconds":60}"#.to_vec(),
            vec![b' '; 4097],
        ] {
            fs::write(&file, &bytes).unwrap();
            assert!(sync_preferences(&file).is_err());
            assert_eq!(fs::read(&file).unwrap(), bytes);
        }
    }
}
