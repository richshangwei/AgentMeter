use serde::Serialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsbFailure {
    pub code: &'static str,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelectedDevice {
    pub serial: String,
    pub transport: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReverseMappingState {
    Owned,
    Missing,
    Changed,
}

#[derive(Debug)]
pub struct OwnedReverse {
    adb: PathBuf,
    timeout: Duration,
    pub serial: String,
    pub device_port: u16,
    pub host_port: u16,
    active: bool,
}

#[derive(Debug)]
struct ProcessOutput {
    success: bool,
    stdout: String,
    stderr: String,
}

impl UsbFailure {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
}

pub fn setup_reverse(
    adb: &Path,
    serial: &str,
    device_port: u16,
    host_port: u16,
    timeout: Duration,
) -> Result<(SelectedDevice, OwnedReverse), UsbFailure> {
    validate(adb, serial, device_port, host_port, timeout)?;
    let devices = run_adb(adb, &["devices", "-l"], timeout)?;
    if !devices.success {
        return Err(classify_adb_failure(&devices));
    }
    let device = select_device(&devices.stdout, serial)?;
    let remote = format!("tcp:{device_port}");
    let local = format!("tcp:{host_port}");
    let result = run_adb(
        adb,
        &["-s", serial, "reverse", "--no-rebind", &remote, &local],
        timeout,
    )?;
    if !result.success {
        return Err(UsbFailure::new(
            "reverse_conflict",
            "ADB refused the no-rebind mapping; inspect the selected device's existing reverse entries",
        ));
    }
    let list = run_adb(adb, &["-s", serial, "reverse", "--list"], timeout)?;
    if !list.success || !mapping_present(&list.stdout, serial, &remote, &local) {
        let _ = run_adb(
            adb,
            &["-s", serial, "reverse", "--remove", &remote],
            timeout,
        );
        return Err(UsbFailure::new(
            "reverse_verification_failed",
            "ADB did not report the exact selected-device mapping after setup",
        ));
    }
    Ok((
        device,
        OwnedReverse {
            adb: adb.to_owned(),
            timeout,
            serial: serial.to_owned(),
            device_port,
            host_port,
            active: true,
        },
    ))
}

/// Requests Android to open the loopback tablet URL. Success means only that
/// ADB accepted the launch command, never that the endpoint is reachable or an
/// authenticated Tablet Session exists.
pub fn launch_device_browser(
    adb: &Path,
    serial: &str,
    device_port: u16,
    timeout: Duration,
) -> Result<(), UsbFailure> {
    validate(adb, serial, device_port, device_port, timeout)?;
    let url = format!("http://127.0.0.1:{device_port}/");
    let result = run_adb(
        adb,
        &[
            "-s",
            serial,
            "shell",
            "am",
            "start",
            "-W",
            "-a",
            "android.intent.action.VIEW",
            "-d",
            &url,
        ],
        timeout,
    )?;
    if !result.success {
        return Err(UsbFailure::new(
            "browser_launch_failed",
            "ADB did not accept the browser launch request for the selected device",
        ));
    }
    Ok(())
}

impl OwnedReverse {
    pub fn inspect(&self) -> Result<ReverseMappingState, UsbFailure> {
        let remote = format!("tcp:{}", self.device_port);
        let local = format!("tcp:{}", self.host_port);
        let list = run_adb(
            &self.adb,
            &["-s", &self.serial, "reverse", "--list"],
            self.timeout,
        )?;
        if !list.success {
            return Err(classify_adb_failure(&list));
        }
        Ok(mapping_state(&list.stdout, &self.serial, &remote, &local))
    }

    /// Releases local ownership without issuing a remove command. Call only
    /// after `inspect` has proved that the mapping is already missing.
    pub fn abandon(mut self) {
        self.active = false;
    }

    pub fn teardown(mut self) -> Result<(), UsbFailure> {
        let result = self.remove_if_owned();
        self.active = false;
        result
    }

    fn remove_if_owned(&mut self) -> Result<(), UsbFailure> {
        if !self.active {
            return Ok(());
        }
        let remote = format!("tcp:{}", self.device_port);
        let local = format!("tcp:{}", self.host_port);
        let list = run_adb(
            &self.adb,
            &["-s", &self.serial, "reverse", "--list"],
            self.timeout,
        )?;
        if !list.success || !mapping_present(&list.stdout, &self.serial, &remote, &local) {
            return Err(UsbFailure::new(
                "mapping_ownership_lost",
                "the selected mapping changed; refusing to remove an unverified entry",
            ));
        }
        let removed = run_adb(
            &self.adb,
            &["-s", &self.serial, "reverse", "--remove", &remote],
            self.timeout,
        )?;
        if !removed.success {
            return Err(UsbFailure::new(
                "teardown_failed",
                "ADB could not remove the owned selected-device mapping",
            ));
        }
        self.active = false;
        Ok(())
    }
}

impl Drop for OwnedReverse {
    fn drop(&mut self) {
        let _ = self.remove_if_owned();
    }
}

fn validate(
    adb: &Path,
    serial: &str,
    device_port: u16,
    host_port: u16,
    timeout: Duration,
) -> Result<(), UsbFailure> {
    if !adb.is_absolute() {
        return Err(UsbFailure::new(
            "invalid_adb_path",
            "ADB executable must use an absolute path",
        ));
    }
    if serial.is_empty() || serial.len() > 256 || serial.chars().any(char::is_whitespace) {
        return Err(UsbFailure::new(
            "invalid_serial",
            "device serial is missing or malformed",
        ));
    }
    if device_port == 0 || host_port == 0 {
        return Err(UsbFailure::new(
            "invalid_port",
            "host and device ports must be represented separately and be non-zero",
        ));
    }
    if timeout.is_zero() || timeout > Duration::from_secs(60) {
        return Err(UsbFailure::new(
            "invalid_timeout",
            "ADB timeout must be between zero and sixty seconds",
        ));
    }
    Ok(())
}

fn select_device(output: &str, selected: &str) -> Result<SelectedDevice, UsbFailure> {
    let matching: Vec<_> = output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let serial = fields.next()?;
            let state = fields.next()?;
            (serial == selected).then(|| (state, fields.collect::<Vec<_>>()))
        })
        .collect();
    if matching.len() != 1 {
        return Err(UsbFailure::new(
            "selected_device_missing",
            "the explicitly selected serial is missing or duplicated",
        ));
    }
    let (state, details) = &matching[0];
    match *state {
        "unauthorized" => {
            return Err(UsbFailure::new(
                "device_unauthorized",
                "unlock the selected device and approve this computer's USB debugging key",
            ));
        }
        "offline" => {
            return Err(UsbFailure::new(
                "device_offline",
                "reconnect the selected device or restart ADB, then retry",
            ));
        }
        "device" => {}
        _ => {
            return Err(UsbFailure::new(
                "device_unavailable",
                "the selected serial is not in the usable device state",
            ));
        }
    }
    if selected.starts_with("emulator-")
        || details.iter().any(|item| item.starts_with("model:sdk_"))
    {
        return Err(UsbFailure::new(
            "emulator_rejected",
            "select an authorized physical USB tablet rather than an emulator",
        ));
    }
    // ADB's libusb backend intentionally omits the `usb:` location field from
    // `devices -l`. A non-network serial with a transport id is still a valid
    // physical candidate; reject explicit IP transports only.
    let explicit_network = selected.contains(':')
        || details
            .iter()
            .any(|item| item.starts_with("ip:") || item.starts_with("tcp:"));
    let has_transport_id = details.iter().any(|item| {
        item.strip_prefix("transport_id:")
            .and_then(|id| id.parse::<u64>().ok())
            .is_some_and(|id| id > 0)
    });
    if !details.iter().any(|item| item.starts_with("usb:"))
        && (explicit_network || !has_transport_id)
    {
        return Err(UsbFailure::new(
            "non_usb_transport",
            "the selected device is not reported on a physical USB transport",
        ));
    }
    Ok(SelectedDevice {
        serial: selected.to_owned(),
        transport: "usb",
    })
}

fn mapping_present(output: &str, serial: &str, remote: &str, local: &str) -> bool {
    mapping_state(output, serial, remote, local) == ReverseMappingState::Owned
}

fn mapping_state(output: &str, serial: &str, remote: &str, local: &str) -> ReverseMappingState {
    let mut selected_remote_seen = false;
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        // `adb reverse --list` is documented as serial local(host) remote(device).
        // Some older/device-scoped ADB builds omit the serial; accept that form too.
        let (first, second, scoped) = match fields.as_slice() {
            [first, second] => (*first, *second, false),
            [seen_serial, first, second] if *seen_serial == serial => (*first, *second, true),
            ["UsbFfs", first, second] => (*first, *second, false),
            _ => continue,
        };
        if scoped || !serial.contains(':') {
            if (first == local && second == remote) || (first == remote && second == local) {
                return ReverseMappingState::Owned;
            }
            if first == remote || second == remote {
                selected_remote_seen = true;
            }
        }
    }
    if selected_remote_seen {
        ReverseMappingState::Changed
    } else {
        ReverseMappingState::Missing
    }
}

fn classify_adb_failure(output: &ProcessOutput) -> UsbFailure {
    let text = format!("{} {}", output.stdout, output.stderr).to_ascii_lowercase();
    if text.contains("unauthorized") {
        UsbFailure::new(
            "device_unauthorized",
            "unlock the selected device and approve USB debugging",
        )
    } else if text.contains("offline") {
        UsbFailure::new(
            "device_offline",
            "reconnect the selected device or restart ADB",
        )
    } else {
        UsbFailure::new(
            "adb_failed",
            "ADB command failed; inspect the selected device and ADB service",
        )
    }
}

fn run_adb(adb: &Path, arguments: &[&str], timeout: Duration) -> Result<ProcessOutput, UsbFailure> {
    let mut command = Command::new(adb);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let mut child = command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| UsbFailure::new("adb_not_found", "ADB executable could not be started"))?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let read = |pipe: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.take(OUTPUT_LIMIT + 1).read_to_end(&mut bytes);
            bytes
        })
    };
    let stdout_reader = read(Box::new(stdout));
    let stderr_reader = read(Box::new(stderr));
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(UsbFailure::new(
                    "adb_timeout",
                    "ADB command exceeded its bounded timeout",
                ));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(UsbFailure::new(
                    "adb_wait_failed",
                    "ADB process state could not be observed",
                ));
            }
        }
    };
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    if stdout.len() > OUTPUT_LIMIT as usize || stderr.len() > OUTPUT_LIMIT as usize {
        return Err(UsbFailure::new(
            "adb_output_too_large",
            "ADB output exceeded the safe limit",
        ));
    }
    Ok(ProcessOutput {
        success: status.success(),
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn device_selection_requires_exact_authorized_physical_usb_serial() {
        let good = "List of devices attached\nUSB-1 device usb:1-2 model:Tablet transport_id:1\nemulator-5554 device model:sdk_phone transport_id:2\n";
        assert_eq!(select_device(good, "USB-1").unwrap().transport, "usb");
        assert_eq!(
            select_device(good, "missing").unwrap_err().code,
            "selected_device_missing"
        );
        assert_eq!(
            select_device("USB-1 unauthorized usb:1-2\n", "USB-1")
                .unwrap_err()
                .code,
            "device_unauthorized"
        );
        assert_eq!(
            select_device("USB-1 offline usb:1-2\n", "USB-1")
                .unwrap_err()
                .code,
            "device_offline"
        );
        assert_eq!(
            select_device(good, "emulator-5554").unwrap_err().code,
            "emulator_rejected"
        );
        assert_eq!(
            select_device("WIFI device product:x model:Tablet\n", "WIFI")
                .unwrap_err()
                .code,
            "non_usb_transport"
        );
        assert_eq!(
            select_device(
                "R58SF device product:gta4xlxx model:SM_P615 device:gta4xl transport_id:1\n",
                "R58SF"
            )
            .unwrap()
            .transport,
            "usb"
        );
    }
    #[test]
    fn mapping_parser_accepts_only_exact_selected_port_pair() {
        assert!(mapping_present(
            "USB-1 tcp:43127 tcp:43128\n",
            "USB-1",
            "tcp:43127",
            "tcp:43128"
        ));
        assert!(mapping_present(
            "UsbFfs tcp:43127 tcp:43128\n",
            "USB-1",
            "tcp:43127",
            "tcp:43128"
        ));
        assert!(mapping_present(
            "tcp:43127 tcp:43128\n",
            "USB-1",
            "tcp:43127",
            "tcp:43128"
        ));
        assert!(!mapping_present(
            "OTHER tcp:43127 tcp:43128\n",
            "USB-1",
            "tcp:43127",
            "tcp:43128"
        ));
        assert!(!mapping_present(
            "USB-1 tcp:43127 tcp:9\n",
            "USB-1",
            "tcp:43127",
            "tcp:43128"
        ));
        assert_eq!(
            mapping_state(
                "USB-1 tcp:43127 tcp:9999\n",
                "USB-1",
                "tcp:43127",
                "tcp:43128"
            ),
            ReverseMappingState::Changed
        );
        assert_eq!(
            mapping_state("", "USB-1", "tcp:43127", "tcp:43128"),
            ReverseMappingState::Missing
        );
    }
}
