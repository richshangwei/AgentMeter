//! Deterministic lifecycle probe for the Windows desktop shell.
//!
//! The P0 experiment keeps the shell state machine independent from a GUI
//! toolkit. A real Tauri harness can feed the same events and compare its
//! observations with this reference model.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

#[derive(Debug, Clone, Deserialize)]
struct Scenario {
    events: Vec<Event>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum Event {
    Name(String),
    Object { event: String },
}

impl Event {
    fn name(&self) -> &str {
        match self {
            Self::Name(s) => s,
            Self::Object { event } => event,
        }
    }
}

#[derive(Debug, Default, Serialize)]
struct State {
    instance_count: u8,
    window_visible: bool,
    tray_icon_count: u8,
    collector_count: u8,
    server_count: u8,
    database_writer_count: u8,
    startup_registered: bool,
    running: bool,
    anomalies: Vec<String>,
    trace: Vec<String>,
}

fn apply(state: &mut State, event: &str) {
    state.trace.push(event.to_owned());
    match event {
        "launch" | "startup-launch-hidden" => {
            if state.running {
                state.window_visible = true;
                state.trace.push("activate-existing-instance".into());
                return;
            }
            state.running = true;
            state.instance_count = 1;
            state.window_visible = event == "launch";
            state.tray_icon_count = 1;
            state.collector_count = 1;
            state.server_count = 1;
            state.database_writer_count = 1;
        }
        "close-window" => {
            if state.running {
                state.window_visible = false;
            }
        }
        "tray-show" => {
            if state.running {
                state.window_visible = true;
            }
        }
        "tray-exit" | "exit" => {
            state.running = false;
            state.window_visible = false;
            state.instance_count = 0;
            state.tray_icon_count = 0;
            state.collector_count = 0;
            state.server_count = 0;
            state.database_writer_count = 0;
        }
        "startup-enable" => state.startup_registered = true,
        "startup-disable" => state.startup_registered = false,
        "restart" => {
            apply(state, "exit");
            apply(state, "launch");
        }
        "abnormal-termination" => {
            state.running = false;
            state.window_visible = false;
            state.instance_count = 0;
            state.tray_icon_count = 0;
            state.collector_count = 0;
            state.server_count = 0;
            state.database_writer_count = 0;
        }
        "recreate-tray" => {
            if state.running {
                state.tray_icon_count = 1;
            }
        }
        unknown => state.anomalies.push(format!("unknown event: {unknown}")),
    }
    if state.instance_count > 1
        || state.tray_icon_count > 1
        || state.collector_count > 1
        || state.server_count > 1
        || state.database_writer_count > 1
    {
        state.anomalies.push("duplicate-owned-resource".into());
    }
}

fn collect(input: Value) -> Value {
    let scenario: Scenario = match serde_json::from_value(input) {
        Ok(s) => s,
        Err(_) => {
            return serde_json::json!({"schema_version":"windows-lifecycle-p0/v1","status":"invalid_input"});
        }
    };
    let mut state = State::default();
    for event in &scenario.events {
        apply(&mut state, event.name());
    }
    let healthy = state.anomalies.is_empty()
        && state.instance_count <= 1
        && state.tray_icon_count <= 1
        && state.collector_count <= 1
        && state.server_count <= 1
        && state.database_writer_count <= 1;
    serde_json::json!({
        "schema_version": "windows-lifecycle-p0/v1",
        "status": if healthy { "pass" } else { "needs-review" },
        "supported_environment": {"os": "Windows 10/11 (VM gate required)", "runtime": "Rust core; Tauri shell pending"},
        "state": state,
        "release_gate": if healthy { "reference-state-machine-viable" } else { "lifecycle-risk" },
    })
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let input = if args.next().as_deref() == Some("--fixture") {
        match args.next().and_then(|path| fs::read_to_string(path).ok()) {
            Some(s) => s,
            None => return ExitCode::from(2),
        }
    } else {
        let mut s = String::new();
        if io::stdin().read_to_string(&mut s).is_err() {
            return ExitCode::from(2);
        }
        s
    };
    let input = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => return ExitCode::from(2),
    };
    println!("{}", serde_json::to_string_pretty(&collect(input)).unwrap());
    ExitCode::SUCCESS
}
