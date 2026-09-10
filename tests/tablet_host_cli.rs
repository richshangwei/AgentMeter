use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};

struct Temp(std::path::PathBuf);

impl Temp {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "agentmeter-tablet-host-usb-{}-{nonce}",
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

#[test]
fn host_rejects_invalid_and_occupied_explicit_ports() {
    for port in ["0", "65536", "invalid", "-1"] {
        let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
            .args(["--mock-providers", "--port", port])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
    }
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .args([
            "--mock-providers",
            "--port",
            &occupied.local_addr().unwrap().port().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("occupied")
    );
}

#[test]
fn host_announces_mock_boundary_serves_and_stops_on_quit() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .arg("--mock-providers")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    output.read_line(&mut ready).unwrap();
    let ready: serde_json::Value = serde_json::from_str(&ready).unwrap();
    assert_eq!(ready["provider_data"], "mock");
    assert_eq!(ready["durable_pairing"], false);
    assert!(ready.get("code").is_none());
    let origin = ready["origin"].as_str().unwrap();
    let addr = origin.strip_prefix("http://").unwrap();
    let mut socket = TcpStream::connect(addr).unwrap();
    socket
        .set_read_timeout(Some(std::time::Duration::from_secs(3)))
        .unwrap();
    write!(socket, "GET /api/v1/dashboard HTTP/1.1\r\nHost: {addr}\r\nOrigin: {origin}\r\nContent-Length: 0\r\n\r\n").unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 401"),
        "private data requires pairing"
    );
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"pair\nquit\n")
        .unwrap();
    let mut rest = String::new();
    output.read_to_string(&mut rest).unwrap();
    assert!(
        rest.is_empty(),
        "redirected output must not disclose a code"
    );
    assert!(child.wait().unwrap().success());
    assert!(
        TcpStream::connect(addr).is_err(),
        "listener remains after quit"
    );
}

#[test]
fn host_requires_mock_opt_in_and_exits_on_eof() {
    let missing = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .output()
        .unwrap();
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    let eof = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .arg("--mock-providers")
        .output()
        .unwrap();
    assert!(eof.status.success());
    let ready: serde_json::Value = serde_json::from_slice(&eof.stdout).unwrap();
    assert_eq!(ready["event"], "ready");
}

#[test]
fn usb_host_owns_mapping_and_keeps_browser_launch_separate_from_health() {
    let temp = Temp::new();
    let log = temp.0.join("commands.log");
    let mapping = temp.0.join("mapping");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .args([
            "--mock-providers",
            "--adb",
            env!("CARGO_BIN_EXE_fake-adb"),
            "--serial",
            "USB-1",
            "--device-port",
            "43127",
        ])
        .env(
            "AGENTMETER_FAKE_ADB_DEVICES",
            "USB-1 device usb:1-2 model:Tablet\n",
        )
        .env("AGENTMETER_FAKE_ADB_LOG", &log)
        .env("AGENTMETER_FAKE_ADB_MAPPING", &mapping)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ready_line = String::new();
    stdout.read_line(&mut ready_line).unwrap();
    let ready: serde_json::Value = serde_json::from_str(&ready_line).unwrap();
    assert_eq!(ready["usb"]["selected_serial"], "USB-1");
    assert_eq!(ready["usb"]["transport"], "usb");
    assert_eq!(ready["usb"]["device_port"], 43127);
    assert_ne!(ready["usb"]["host_port"], 0);
    assert_eq!(ready["usb"]["mapping"], "verified");
    assert_eq!(ready["usb"]["browser_launch"], "not_requested");
    assert_eq!(ready["usb"]["authenticated_health"], "not_observed");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"open\nstatus\nquit\n")
        .unwrap();
    assert!(child.wait().unwrap().success());
    let mut rest = String::new();
    stdout.read_to_string(&mut rest).unwrap();
    let status_event: serde_json::Value = serde_json::from_str(rest.trim()).unwrap();
    assert_eq!(status_event["event"], "transport_status");
    assert_eq!(status_event["usb_mapping"]["state"], "owned");
    assert_eq!(status_event["browser_launch_requested"], true);
    assert_eq!(
        status_event["authenticated_activity"]["authenticated_session_established"],
        false
    );
    assert_eq!(
        status_event["authenticated_activity"]["authenticated_request_count"],
        0
    );
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.contains("authenticated tablet health is still unobserved"));
    let commands = std::fs::read_to_string(log).unwrap();
    assert!(commands.contains("reverse --no-rebind tcp:43127 tcp:"));
    assert!(
        commands
            .contains("shell am start -W -a android.intent.action.VIEW -d http://127.0.0.1:43127/")
    );
    assert!(commands.contains("reverse --remove tcp:43127"));
    assert!(!mapping.exists(), "owned mapping was not removed on exit");
}

#[test]
fn partial_usb_configuration_fails_before_readiness() {
    let output = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .args(["--mock-providers", "--serial", "USB-1"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .contains("requires --adb, --serial and --device-port together")
    );
}

#[test]
fn missing_mapping_can_be_recovered_and_is_then_owned_until_exit() {
    let temp = Temp::new();
    let log = temp.0.join("commands.log");
    let mapping = temp.0.join("mapping");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .args([
            "--mock-providers",
            "--adb",
            env!("CARGO_BIN_EXE_fake-adb"),
            "--serial",
            "USB-1",
            "--device-port",
            "43127",
        ])
        .env(
            "AGENTMETER_FAKE_ADB_DEVICES",
            "USB-1 device usb:1-2 model:Tablet\n",
        )
        .env("AGENTMETER_FAKE_ADB_LOG", &log)
        .env("AGENTMETER_FAKE_ADB_MAPPING", &mapping)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    assert!(mapping.exists());
    std::fs::remove_file(&mapping).unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"status\nrecover-usb\nstatus\nquit\n")
        .unwrap();
    let mut events = String::new();
    stdout.read_to_string(&mut events).unwrap();
    assert!(child.wait().unwrap().success());
    let events: Vec<serde_json::Value> = events
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0]["usb_mapping"]["state"], "missing");
    assert_eq!(events[1]["usb_mapping"]["state"], "owned");
    let commands = std::fs::read_to_string(log).unwrap();
    assert_eq!(commands.matches("reverse --no-rebind").count(), 2);
    assert!(commands.contains("reverse --remove tcp:43127"));
    assert!(!mapping.exists());
}

#[test]
fn changed_mapping_is_never_replaced_or_removed_by_recovery() {
    let temp = Temp::new();
    let log = temp.0.join("commands.log");
    let mapping = temp.0.join("mapping");
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-tablet-host"))
        .args([
            "--mock-providers",
            "--adb",
            env!("CARGO_BIN_EXE_fake-adb"),
            "--serial",
            "USB-1",
            "--device-port",
            "43127",
        ])
        .env(
            "AGENTMETER_FAKE_ADB_DEVICES",
            "USB-1 device usb:1-2 model:Tablet\n",
        )
        .env("AGENTMETER_FAKE_ADB_LOG", &log)
        .env("AGENTMETER_FAKE_ADB_MAPPING", &mapping)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ready = String::new();
    stdout.read_line(&mut ready).unwrap();
    std::fs::write(&mapping, "USB-1 tcp:43127 tcp:9999\n").unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"recover-usb\nstatus\nquit\n")
        .unwrap();
    let mut event = String::new();
    stdout.read_to_string(&mut event).unwrap();
    assert!(child.wait().unwrap().success());
    let event: serde_json::Value = serde_json::from_str(event.trim()).unwrap();
    assert_eq!(event["usb_mapping"]["state"], "changed");
    let commands = std::fs::read_to_string(log).unwrap();
    assert_eq!(commands.matches("reverse --no-rebind").count(), 1);
    assert!(!commands.contains("reverse --remove"));
    assert!(mapping.exists());
    let mut stderr = String::new();
    child
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    assert!(stderr.contains("refusing to replace or remove"));
}
