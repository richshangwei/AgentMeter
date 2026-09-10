use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const DIAGNOSTIC_NAME: &str = "startup-error.log";
const MAX_DETAIL_CHARS: usize = 2_048;

fn clean_line(value: &str, limit: usize) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(limit)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn write_failure(
    directory: &Path,
    code: &str,
    guidance: &str,
    detail: &str,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(directory)?;
    let path = directory.join(DIAGNOSTIC_NAME);
    let observed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let body = format!(
        "schema_version=agentmeter.desktop-startup-diagnostic/v1\nobserved_at_unix_ms={observed_at}\nfailure_code={}\nguidance={}\ndetail={}\n",
        clean_line(code, 64),
        clean_line(guidance, 512),
        clean_line(detail, MAX_DETAIL_CHARS),
    );
    fs::write(&path, body)?;
    Ok(path)
}

fn diagnostic_directory() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("com.agentmeter.p0")
        .join("logs")
}

#[cfg(windows)]
fn show_error(message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MB_ICONERROR, MB_OK, MB_SETFOREGROUND, MessageBoxW,
    };

    let title: Vec<u16> = "AgentMeter 啟動失敗\0".encode_utf16().collect();
    let message: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

#[cfg(not(windows))]
fn show_error(message: &str) {
    eprintln!("{message}");
}

pub fn report_failure(code: &str, guidance: &str, detail: &str) {
    let directory = diagnostic_directory();
    let diagnostic = write_failure(&directory, code, guidance, detail);
    let location = diagnostic
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| "診斷檔也無法寫入".to_owned());
    let message = format!(
        "AgentMeter 無法繼續（{}）。\n\n{}\n\n診斷：{}",
        clean_line(code, 64),
        clean_line(guidance, 512),
        location
    );
    eprintln!("{message}");
    show_error(&message);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn diagnostic_is_bounded_single_line_and_actionable() {
        let directory = std::env::temp_dir().join(format!(
            "agentmeter-startup-diagnostic-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let detail = format!("first\r\nsecond{}", "x".repeat(10_000));
        let path = write_failure(
            &directory,
            "desktop_startup_failed",
            "repair WebView2",
            &detail,
        )
        .unwrap();
        let report = fs::read_to_string(&path).unwrap();
        assert!(report.starts_with("schema_version=agentmeter.desktop-startup-diagnostic/v1\n"));
        assert!(report.contains("failure_code=desktop_startup_failed\n"));
        assert!(report.contains("guidance=repair WebView2\n"));
        assert!(report.contains("detail=first  second"));
        assert!(report.len() < 3_000);
        assert_eq!(report.lines().count(), 5);
        fs::remove_dir_all(directory).unwrap();
    }
}
