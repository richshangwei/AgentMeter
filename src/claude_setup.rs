//! Previewed, reversible user-level Claude integration. No provider credentials are read.
use serde_json::{Value, json};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const SCHEMA: &str = "agentmeter.claude-setup/v1";
const MAX_CONFIG: u64 = 1_048_576;

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("無法讀取 Claude 設定；請確認檔案權限。".into()),
    };
    let mut bytes = Vec::new();
    file.take(MAX_CONFIG + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "無法讀取 Claude 設定。")?;
    if bytes.len() as u64 > MAX_CONFIG {
        return Err("Claude 設定超過大小限制。".into());
    }
    Ok(Some(bytes))
}

fn object(bytes: Option<&[u8]>) -> Result<Value, String> {
    let value: Value = match bytes {
        Some(bytes) => serde_json::from_slice(bytes)
            .map_err(|_| "Claude 設定不是有效 JSON；尚未變更任何設定。")?,
        None => json!({}),
    };
    if !value.is_object() {
        return Err("Claude 設定必須是 JSON 物件。".into());
    }
    Ok(value)
}

fn nonce() -> String {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).expect("OS random source unavailable");
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "無法建立備份或交接檔；既有檔案未覆寫。")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "無法完成設定寫入。".into())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!("{}.tmp", nonce()));
    write_new(&temporary, bytes)?;
    let result = fs::rename(&temporary, path)
        .map_err(|_| "無法替換設定；請關閉正在編輯設定的工具後重試。".into());
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn manifest_path(settings: &Path) -> PathBuf {
    settings.with_extension("agentmeter-desktop.json")
}

fn validate_manifest(value: Value) -> Result<Value, String> {
    if value["schema"] != SCHEMA
        || !value["original_present"].is_boolean()
        || value["installed"]["type"] != "command"
        || value["installed"]["command"]
            .as_str()
            .is_none_or(str::is_empty)
        || value.get("original").is_none()
    {
        return Err("整合備份格式不完整；原 Claude 設定未變更。".into());
    }
    if !value["original"].is_null()
        && (value["original"]["type"] != "command"
            || value["original"]["command"]
                .as_str()
                .is_none_or(str::is_empty)
            || !matches!(value["shell"]["kind"].as_str(), Some("bash" | "powershell"))
            || !value["shell"]["executable"]
                .as_str()
                .is_some_and(|p| Path::new(p).is_absolute()))
    {
        return Err("無法驗證原狀態列備份，請先確認整合設定。".into());
    }
    Ok(value)
}

pub fn report_path(settings: &Path) -> PathBuf {
    manifest_path(settings).with_extension("observation.json")
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in bytes.chunks(3) {
        let bits = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        result.push(ALPHABET[((bits >> 18) & 63) as usize] as char);
        result.push(ALPHABET[((bits >> 12) & 63) as usize] as char);
        result.push(if chunk.len() > 1 {
            ALPHABET[((bits >> 6) & 63) as usize] as char
        } else {
            '='
        });
        result.push(if chunk.len() > 2 {
            ALPHABET[(bits & 63) as usize] as char
        } else {
            '='
        });
    }
    result
}

fn receiver_command(executable: &Path, manifest: &Path) -> Result<String, String> {
    if !executable.is_absolute() || !executable.is_file() || !manifest.is_absolute() {
        return Err("找不到內建 Claude 接收器，請重新安裝 AgentMeter。".into());
    }
    let quote = |path: &Path| {
        path.to_str()
            .map(|s| format!("'{}'", s.replace('\\', "/").replace('\'', "''")))
            .ok_or("路徑無法編碼。".to_string())
    };
    // EncodedCommand is UTF-16LE. No path text is interpreted by the outer shell.
    let script = format!(
        "& {} --claude-status {}",
        quote(executable)?,
        quote(manifest)?
    );
    let bytes: Vec<_> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    Ok(format!(
        "powershell.exe -NoProfile -NonInteractive -EncodedCommand {}",
        base64(&bytes)
    ))
}

#[derive(Clone)]
pub struct DisplayShell {
    pub executable: PathBuf,
    pub kind: &'static str,
}

pub struct Plan {
    settings: PathBuf,
    before: Option<Vec<u8>>,
    manifest_before: Option<Vec<u8>>,
    manifest: Value,
    after: Value,
    enable: bool,
}

impl Plan {
    pub fn preview(
        settings: PathBuf,
        executable: &Path,
        shell: Option<DisplayShell>,
        enable: bool,
    ) -> Result<Self, String> {
        if !settings.is_absolute() {
            return Err("找不到 Claude 使用者設定位置。".into());
        }
        let before = read_optional(&settings)?;
        let mut after = object(before.as_deref())?;
        if enable && after["disableAllHooks"] == true {
            return Err("Claude 設定已停用狀態列整合；請先在 Claude 中確認此設定。".into());
        }
        let manifest_file = manifest_path(&settings);
        let manifest_before = read_optional(&manifest_file)?;
        let manifest = if let Some(bytes) = &manifest_before {
            let saved = validate_manifest(object(Some(bytes))?)?;
            if saved["schema"] != SCHEMA {
                return Err("發現不同版本的整合設定，請先還原原整合。".into());
            }
            saved
        } else {
            if !enable {
                return Err("尚未啟用 AgentMeter 的 Claude 整合。".into());
            }
            let original = after.get("statusLine").cloned();
            if let Some(original) = original.as_ref().filter(|v| !v.is_null()) {
                if original["type"] != "command"
                    || original["command"]
                        .as_str()
                        .is_none_or(|s| s.trim().is_empty())
                {
                    return Err("既有狀態列格式不受支援，無法可靠保留；原設定未變更。".into());
                }
                if original["command"].as_str().is_some_and(|s| {
                    s.contains("agentmeter-claude-p0") || s.contains("--claude-status")
                }) {
                    return Err("已存在 AgentMeter 整合；請先停用舊版整合，避免重複串接。".into());
                }
                if shell.is_none() {
                    return Err("找不到既有狀態列使用的 Shell，無法保留原顯示。".into());
                }
            }
            let mut installed = original
                .clone()
                .filter(Value::is_object)
                .unwrap_or(json!({}));
            installed["type"] = json!("command");
            installed["command"] = json!(receiver_command(executable, &manifest_file)?);
            json!({"schema":SCHEMA,"original_present":original.is_some(),"original":original,"installed":installed,
                "shell":shell.map(|s| json!({"executable":s.executable,"kind":s.kind}))})
        };
        let current = after.get("statusLine");
        let original = if manifest["original_present"] == true {
            Some(&manifest["original"])
        } else {
            None
        };
        let installed = Some(&manifest["installed"]);
        if current != original && current != installed {
            return Err("Claude 狀態列已被修改；為保留你的變更，請先手動確認設定。".into());
        }
        if enable {
            after["statusLine"] = manifest["installed"].clone();
        } else if let Some(original) = original {
            after["statusLine"] = original.clone();
        } else {
            after.as_object_mut().unwrap().remove("statusLine");
        }
        Ok(Self {
            settings,
            before,
            manifest_before,
            manifest,
            after,
            enable,
        })
    }

    pub fn view(&self) -> Value {
        let before = object(self.before.as_deref()).unwrap();
        json!({"action":if self.enable {"enable"} else {"disable"},
            "existing_display": !self.manifest["original"].is_null(),
            "before":before.get("statusLine"), "after":self.after.get("statusLine"),
            "settings_path":self.settings,
            "message":if self.enable {"會先備份設定，只調整使用者層級狀態列並保留既有顯示。專案或管理員設定可能優先生效。"} else {"只還原 AgentMeter 接管的狀態列，其他設定保持目前內容。備份會保留。"}})
    }

    pub fn apply(self) -> Result<Value, String> {
        if read_optional(&self.settings)? != self.before
            || read_optional(&manifest_path(&self.settings))? != self.manifest_before
        {
            return Err("設定在預覽後已改變，請重新預覽再套用。".into());
        }
        fs::create_dir_all(self.settings.parent().ok_or("無效設定位置。")?)
            .map_err(|_| "無法建立 Claude 設定目錄。")?;
        if let Some(bytes) = &self.before {
            write_new(
                &self
                    .settings
                    .with_extension(format!("agentmeter-backup-{}.json", nonce())),
                bytes,
            )?;
        }
        if self.manifest_before.is_none() {
            write_new(
                &manifest_path(&self.settings),
                &serde_json::to_vec_pretty(&self.manifest).unwrap(),
            )?;
        }
        // Check again after backup, before replacing the live settings.
        if read_optional(&self.settings)? != self.before {
            return Err("設定已改變，請重新預覽。備份已保留。".into());
        }
        atomic_write(
            &self.settings,
            &serde_json::to_vec_pretty(&self.after).unwrap(),
        )?;
        Ok(
            json!({"enabled":self.enable,"report_path":report_path(&self.settings),"message":if self.enable {"已啟用。請照常使用 Claude，收到新的狀態事件後即可顯示資料。"} else {"已停用並還原原狀態列。備份與先前報告仍保留。"}}),
        )
    }
}

pub fn integration_state(settings: &Path) -> Result<&'static str, String> {
    let Some(bytes) = read_optional(&manifest_path(settings))? else {
        return Ok("not_enabled");
    };
    let saved = validate_manifest(object(Some(&bytes))?)?;
    if saved["schema"] != SCHEMA {
        return Ok("conflict");
    }
    let current = object(read_optional(settings)?.as_deref())?;
    if current.get("statusLine") == Some(&saved["installed"]) {
        Ok("enabled")
    } else if current.get("statusLine")
        == (if saved["original_present"] == true {
            Some(&saved["original"])
        } else {
            None
        })
    {
        Ok("disabled")
    } else {
        Ok("conflict")
    }
}

/// Invoked before Tauri initializes. Only the normalized report is persisted.
pub fn receive(manifest: &Path) -> Result<(), String> {
    let saved = validate_manifest(object(read_optional(manifest)?.as_deref())?)?;
    if saved["schema"] != SCHEMA {
        return Err("unsupported integration".into());
    }
    let mut input = Vec::new();
    std::io::stdin()
        .take(4_194_305)
        .read_to_end(&mut input)
        .map_err(|_| "event read failed")?;
    if input.len() > 4_194_304 {
        return Err("event too large".into());
    }
    let report = crate::claude::normalize_status_event(&input);
    let result = atomic_write(
        &manifest.with_extension("observation.json"),
        &serde_json::to_vec(&report).unwrap(),
    );
    if let Some(original) = saved["original"]["command"].as_str() {
        let executable = saved["shell"]["executable"]
            .as_str()
            .ok_or("missing display shell")?;
        if !Path::new(executable).is_absolute() {
            return Err("invalid display shell".into());
        }
        let mut command = Command::new(executable);
        match saved["shell"]["kind"].as_str() {
            Some("bash") => {
                command.args(["-c", original]);
            }
            Some("powershell") => {
                command.args(["-NoProfile", "-NonInteractive", "-Command", original]);
            }
            _ => return Err("unsupported display shell".into()),
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::inherit())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "display command unavailable")?;
        let mut stdin = child.stdin.take().ok_or("display input unavailable")?;
        std::thread::spawn(move || {
            let _ = stdin.write_all(&input);
        });
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => break,
                Ok(Some(_)) => return Err("display command failed".into()),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err("display command timed out".into());
                }
            }
        }
    }
    result
}
