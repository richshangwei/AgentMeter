use std::{
    path::{Path, PathBuf},
    process::Command,
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "agentmeter-usb-command-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn run_child(case: &str, temp: &Path, environment: &[(&str, &str)]) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", "command_child", "--nocapture"])
        .env("AGENTMETER_USB_COMMAND_CHILD", case)
        .env("AGENTMETER_FAKE_ADB_MODE", case)
        .env("AGENTMETER_FAKE_ADB_LOG", temp.join("commands.log"))
        .env("AGENTMETER_FAKE_ADB_COUNT", temp.join("count"));
    for (name, value) in environment {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "child case {case} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn actual_command_process_uses_no_rebind_verifies_and_removes_only_owned_mapping() {
    let temp = Temp::new();
    run_child(
        "healthy",
        &temp.0,
        &[
            (
                "AGENTMETER_FAKE_ADB_DEVICES",
                "List of devices attached\nUSB-1 device usb:1-2 model:Tablet transport_id:1\n",
            ),
            ("AGENTMETER_FAKE_ADB_LIST", "USB-1 tcp:43127 tcp:43128\n"),
        ],
    );
    let log = std::fs::read_to_string(temp.0.join("commands.log")).unwrap();
    let lines: Vec<_> = log.lines().collect();
    assert_eq!(lines[0], "devices -l");
    assert_eq!(lines[1], "-s USB-1 reverse --no-rebind tcp:43127 tcp:43128");
    assert_eq!(lines[2], "-s USB-1 reverse --list");
    assert_eq!(
        lines[3],
        "-s USB-1 shell am start -W -a android.intent.action.VIEW -d http://127.0.0.1:43127/"
    );
    assert_eq!(lines[4], "-s USB-1 reverse --list");
    assert_eq!(lines[5], "-s USB-1 reverse --remove tcp:43127");
}

#[test]
fn conflict_timeout_and_changed_mapping_fail_closed() {
    for case in ["conflict", "timeout", "changed"] {
        let temp = Temp::new();
        run_child(
            case,
            &temp.0,
            &[
                (
                    "AGENTMETER_FAKE_ADB_DEVICES",
                    "USB-1 device usb:1-2 model:Tablet\n",
                ),
                ("AGENTMETER_FAKE_ADB_LIST", "USB-1 tcp:43127 tcp:43128\n"),
                (
                    "AGENTMETER_FAKE_ADB_LIST_AFTER",
                    "USB-1 tcp:43127 tcp:9999\n",
                ),
            ],
        );
        let log = std::fs::read_to_string(temp.0.join("commands.log")).unwrap();
        if case == "changed" {
            assert!(!log.contains("--remove"));
        }
    }
}

#[test]
fn command_child() {
    let Ok(case) = std::env::var("AGENTMETER_USB_COMMAND_CHILD") else {
        return;
    };
    use agentmeter_p0::usb::{launch_device_browser, setup_reverse};
    use std::time::{Duration, Instant};
    let adb = Path::new(env!("CARGO_BIN_EXE_fake-adb"));
    match case.as_str() {
        "healthy" => {
            let (device, mapping) =
                setup_reverse(adb, "USB-1", 43127, 43128, Duration::from_secs(5)).unwrap();
            assert_eq!(device.transport, "usb");
            launch_device_browser(adb, "USB-1", 43127, Duration::from_secs(5)).unwrap();
            mapping.teardown().unwrap();
        }
        "conflict" => {
            let result = setup_reverse(adb, "USB-1", 43127, 43128, Duration::from_secs(5));
            assert_eq!(result.unwrap_err().code, "reverse_conflict");
        }
        "timeout" => {
            let started = Instant::now();
            let result = setup_reverse(adb, "USB-1", 43127, 43128, Duration::from_millis(100));
            assert_eq!(result.unwrap_err().code, "adb_timeout");
            assert!(started.elapsed() < Duration::from_secs(3));
        }
        "changed" => {
            let (_, mapping) =
                setup_reverse(adb, "USB-1", 43127, 43128, Duration::from_secs(5)).unwrap();
            assert_eq!(
                mapping.teardown().unwrap_err().code,
                "mapping_ownership_lost"
            );
        }
        _ => panic!("unknown case"),
    }
}
