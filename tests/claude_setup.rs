use agentmeter_p0::claude_setup::{DisplayShell, Plan, integration_state, report_path};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let mut random = [0u8; 8];
        getrandom::fill(&mut random).unwrap();
        let path = std::env::temp_dir().join(format!("agentmeter setup ' 中文 {:?}", random));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn settings(&self) -> PathBuf {
        self.0.join("settings.json")
    }
    fn write(&self, value: Value) {
        fs::write(self.settings(), serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    }
    fn read(&self) -> Value {
        serde_json::from_slice(&fs::read(self.settings()).unwrap()).unwrap()
    }
    fn plan(&self, enable: bool) -> Plan {
        let executable = std::env::var_os("AGENTMETER_TEST_DESKTOP_RECEIVER")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_agentmeter-claude-p0")));
        Plan::preview(self.settings(), &executable, Some(shell()), enable).unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn shell() -> DisplayShell {
    #[cfg(windows)]
    {
        DisplayShell {
            executable: PathBuf::from(std::env::var_os("SystemRoot").unwrap())
                .join("System32/WindowsPowerShell/v1.0/powershell.exe"),
            kind: "powershell",
        }
    }
    #[cfg(not(windows))]
    {
        DisplayShell {
            executable: PathBuf::from("/bin/sh"),
            kind: "bash",
        }
    }
}

#[test]
fn preview_is_read_only_enable_preserves_display_and_disable_preserves_other_edits() {
    let fixture = Fixture::new();
    let original = json!({"theme":"dark", "statusLine":{"type":"command","command":"echo DISPLAY","padding":3}});
    fixture.write(original.clone());
    let raw = fs::read(fixture.settings()).unwrap();
    let plan = fixture.plan(true);
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    assert_eq!(fs::read(fixture.settings()).unwrap(), raw);
    plan.apply().unwrap();
    assert_eq!(integration_state(&fixture.settings()).unwrap(), "enabled");
    let backup = fs::read_dir(&fixture.0)
        .unwrap()
        .map(Result::unwrap)
        .find(|e| e.file_name().to_string_lossy().contains("backup"))
        .unwrap();
    assert_eq!(fs::read(backup.path()).unwrap(), raw);
    assert_eq!(fixture.read()["statusLine"]["padding"], 3);
    let mut changed = fixture.read();
    changed["theme"] = json!("light");
    fixture.write(changed);
    fixture.plan(false).apply().unwrap();
    assert_eq!(fixture.read()["statusLine"], original["statusLine"]);
    assert_eq!(fixture.read()["theme"], "light");
    assert_eq!(integration_state(&fixture.settings()).unwrap(), "disabled");
    fixture.plan(true).apply().unwrap();
    assert_eq!(integration_state(&fixture.settings()).unwrap(), "enabled");
}

#[test]
fn concurrent_edits_and_user_modified_statusline_are_never_overwritten() {
    let fixture = Fixture::new();
    fixture.write(json!({"theme":"dark"}));
    let plan = fixture.plan(true);
    fixture.write(json!({"theme":"new"}));
    assert!(plan.apply().is_err());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    fixture.plan(true).apply().unwrap();
    fixture.write(json!({"statusLine":{"type":"command","command":"new user command"}}));
    assert_eq!(integration_state(&fixture.settings()).unwrap(), "conflict");
    assert!(
        Plan::preview(
            fixture.settings(),
            Path::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0")),
            Some(shell()),
            false
        )
        .is_err()
    );
    assert_eq!(fixture.read()["statusLine"]["command"], "new user command");
}

#[test]
fn missing_and_null_statusline_restore_distinctly_and_invalid_config_is_unchanged() {
    for original in [json!({}), json!({"statusLine":null})] {
        let fixture = Fixture::new();
        fixture.write(original.clone());
        fixture.plan(true).apply().unwrap();
        fixture.plan(false).apply().unwrap();
        assert_eq!(fixture.read(), original);
    }
    for bytes in [
        b"[]".as_slice(),
        b"broken",
        br#"{"disableAllHooks":true}"#,
        br#"{"statusLine":"unknown"}"#,
    ] {
        let fixture = Fixture::new();
        fs::write(fixture.settings(), bytes).unwrap();
        assert!(
            Plan::preview(
                fixture.settings(),
                Path::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0")),
                Some(shell()),
                true
            )
            .is_err()
        );
        assert_eq!(fs::read(fixture.settings()).unwrap(), bytes);
    }
    let fixture = Fixture::new();
    fixture.plan(true).apply().unwrap();
    fixture.plan(false).apply().unwrap();
    assert_eq!(fixture.read(), json!({}));
}

#[test]
#[cfg(windows)]
fn installed_command_receives_stdin_preserves_original_display_and_sanitizes_report() {
    let fixture = Fixture::new();
    fixture.write(
        json!({"statusLine":{"type":"command","command":"[Console]::Write('ORIGINAL DISPLAY')"}}),
    );
    fixture.plan(true).apply().unwrap();
    let settings = fixture.read();
    let mut child = Command::new(shell().executable)
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            settings["statusLine"]["command"].as_str().unwrap(),
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(br#"{"session_id":"PRIVATE","cwd":"PRIVATE","rate_limits":{"five_hour":{"used_percentage":12.5}}}"#).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ORIGINAL DISPLAY");
    let text = fs::read_to_string(report_path(&fixture.settings())).unwrap();
    assert!(!text.contains("PRIVATE"));
    let report: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(report["observation"]["quota_windows"][0]["used"], 12.5);
}

#[test]
fn broken_manifest_is_not_used_to_replace_settings() {
    let fixture = Fixture::new();
    fixture.write(json!({"theme":"keep"}));
    fixture.plan(true).apply().unwrap();
    let before = fs::read(fixture.settings()).unwrap();
    fs::write(
        fixture.settings().with_extension("agentmeter-desktop.json"),
        br#"{"schema":"agentmeter.claude-setup/v1"}"#,
    )
    .unwrap();
    assert!(
        Plan::preview(
            fixture.settings(),
            Path::new(env!("CARGO_BIN_EXE_agentmeter-claude-p0")),
            Some(shell()),
            false
        )
        .is_err()
    );
    assert_eq!(fs::read(fixture.settings()).unwrap(), before);
}
