use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

#[derive(Debug, Deserialize)]
struct Fixture {
    host_bind: HostBind,
    selected_serial: Option<String>,
    devices: Vec<Device>,
    reverse_entries: Vec<ReverseEntry>,
    probe: Probe,
    events: Vec<String>,
    teardown: Teardown,
}
#[derive(Debug, Deserialize)]
struct HostBind {
    address: String,
    port: u16,
}
#[derive(Debug, Deserialize)]
struct Device {
    serial: String,
    state: String,
    transport: String,
    emulator: bool,
    authorized: bool,
}
#[derive(Debug, Deserialize)]
struct ReverseEntry {
    serial: String,
    device_port: u16,
    host_port: u16,
    owner: String,
}
#[derive(Debug, Deserialize)]
struct Probe {
    browser_launched: bool,
    authenticated_health: bool,
}
#[derive(Debug, Deserialize)]
struct Teardown {
    serial: String,
    device_port: u16,
    host_port: u16,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: &'static str,
    capability: &'static str,
    selected_serial: Option<String>,
    transport: String,
    host_port: u16,
    device_port: u16,
    bind: Check,
    selection: Check,
    reverse: Check,
    reachability: Check,
    browser: Check,
    reconnect: Check,
    teardown: Check,
    diagnostics: Vec<String>,
}
#[derive(Debug, Serialize)]
struct Check {
    status: &'static str,
    detail: String,
}
fn check(status: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status,
        detail: detail.into(),
    }
}

fn evaluate(f: Fixture) -> Report {
    let serial = f.selected_serial.clone();
    let selected = serial
        .as_ref()
        .and_then(|s| f.devices.iter().find(|d| &d.serial == s));
    let host_port = f.host_bind.port;
    let device_port = f
        .reverse_entries
        .iter()
        .find(|e| serial.as_ref() == Some(&e.serial))
        .map_or(0, |e| e.device_port);
    let mut diagnostics = Vec::new();
    let bind_ok = f.host_bind.address == "127.0.0.1" || f.host_bind.address == "::1";
    if !bind_ok {
        diagnostics.push(format!("host bind {} is not loopback", f.host_bind.address));
    }
    let selection_ok = match selected {
        Some(d) if d.authorized && d.state == "device" && !d.emulator && d.transport == "usb" => {
            true
        }
        Some(d) => {
            diagnostics.push(format!(
                "selected device {} is {} (authorized={}, emulator={})",
                d.serial, d.state, d.authorized, d.emulator
            ));
            false
        }
        None => {
            diagnostics.push("selected serial is missing or not present".into());
            false
        }
    };
    let mut reverse_ok = false;
    if let Some(s) = &serial {
        if let Some(entry) = f.reverse_entries.iter().find(|e| &e.serial == s) {
            reverse_ok = entry.host_port == host_port && entry.owner == "agentmeter";
            if !reverse_ok {
                diagnostics
                    .push("reverse mapping conflicts with the selected host port or owner".into());
            }
        } else {
            diagnostics.push("no reverse mapping exists for selected serial".into());
        }
        if f.reverse_entries
            .iter()
            .any(|e| &e.serial != s && e.device_port == device_port && e.host_port == host_port)
        {
            reverse_ok = false;
            diagnostics
                .push("reverse mapping is occupied by another device; no-rebind required".into());
        }
    }
    let reachable = selection_ok && reverse_ok && f.probe.authenticated_health;
    if !f.probe.authenticated_health {
        diagnostics.push("authenticated health request did not succeed".into());
    }
    let browser_status = if reachable {
        "observed"
    } else if f.probe.browser_launched {
        "not_online"
    } else {
        "not_attempted"
    };
    let browser_detail = if f.probe.browser_launched {
        "browser launch recorded separately from endpoint reachability"
    } else {
        "no browser launch recorded"
    };
    let recovery = [
        "unplug",
        "replug",
        "adb_restart",
        "host_restart",
        "tablet_sleep",
        "stale_mapping",
    ]
    .iter()
    .all(|event| f.events.iter().any(|seen| seen == event));
    if !recovery {
        diagnostics.push("reconnect fixture does not cover every expected recovery event".into());
    }
    let teardown_ok = f.teardown.serial == serial.clone().unwrap_or_default()
        && f.teardown.host_port == host_port
        && f.teardown.device_port == device_port;
    if !teardown_ok {
        diagnostics.push("teardown target is not the owned selected-device mapping".into());
    }
    Report {
        schema_version: "usb-p0/v1",
        capability: "selected_device_usb_loopback",
        selected_serial: serial,
        transport: selected.map_or("unknown", |d| d.transport.as_str()).into(),
        host_port,
        device_port,
        bind: check(
            if bind_ok { "supported" } else { "blocked" },
            format!("{}:{}", f.host_bind.address, host_port),
        ),
        selection: check(
            if selection_ok { "supported" } else { "blocked" },
            "explicit authorized physical device selection",
        ),
        reverse: check(
            if reverse_ok { "supported" } else { "blocked" },
            "ADB reverse with no-rebind semantics",
        ),
        reachability: check(
            if reachable { "supported" } else { "blocked" },
            if reachable {
                "authenticated health response observed"
            } else {
                "endpoint reachability not proven"
            },
        ),
        browser: check(browser_status, browser_detail),
        reconnect: check(
            if recovery { "exercised" } else { "incomplete" },
            "unplug/replug and restart recovery matrix",
        ),
        teardown: check(
            if teardown_ok { "supported" } else { "blocked" },
            "remove only the selected owned mapping",
        ),
        diagnostics,
    }
}
fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let input = if args.next().as_deref() == Some("--fixture") {
        args.next().and_then(|p| fs::read_to_string(p).ok())
    } else {
        let mut s = String::new();
        io::stdin().read_to_string(&mut s).ok().map(|_| s)
    };
    let Some(input) = input else {
        return ExitCode::from(2);
    };
    let Ok(fixture) = serde_json::from_str::<Fixture>(&input) else {
        return ExitCode::from(2);
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&evaluate(fixture)).expect("report serializes")
    );
    ExitCode::SUCCESS
}
