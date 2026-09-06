//! Fixture-driven P0 probe for clean Windows installation and WebView2 gates.
//! A real VM run can feed the same scenario shape to preserve comparable evidence.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

#[derive(Debug, Deserialize)]
struct Scenario {
    environment: Environment,
    webview2: WebView2,
    operations: Vec<Operation>,
    measurements: Option<Measurements>,
}
#[derive(Debug, Deserialize)]
struct Environment {
    os: String,
    clean_vm: bool,
    package_identity: Option<String>,
}
#[derive(Debug, Deserialize)]
struct WebView2 {
    state: String,
    bootstrapper: String,
    download: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct Operation {
    name: String,
    result: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct Measurements {
    installed_size_mb: Option<f64>,
    download_size_mb: Option<f64>,
    first_launch_ms: Option<u64>,
    idle_memory_mb: Option<f64>,
}

fn collect(input: Value) -> Value {
    let scenario: Scenario = match serde_json::from_value(input) {
        Ok(value) => value,
        Err(_) => {
            return serde_json::json!({"schema_version":"windows-install-p0/v1","status":"invalid_input"});
        }
    };
    let mut diagnostics = Vec::new();
    let webview_ready = matches!(
        scenario.webview2.state.as_str(),
        "present-current" | "installed-by-bootstrapper"
    ) && scenario.webview2.download != "failed";
    if !webview_ready {
        diagnostics
            .push("WebView2 runtime is unavailable or bootstrapper download failed".to_string());
    }
    if scenario.webview2.bootstrapper != "approved-download-bootstrapper" {
        diagnostics.push("unsupported WebView2 deployment strategy".to_string());
    }
    for operation in &scenario.operations {
        if operation.result != "pass" {
            diagnostics.push(format!("{}: {}", operation.name, operation.result));
        }
    }
    if !scenario.environment.clean_vm {
        diagnostics.push("clean VM prerequisite not proven".to_string());
    }
    let supported = scenario.environment.os.starts_with("Windows 10")
        || scenario.environment.os.starts_with("Windows 11");
    if !supported {
        diagnostics.push("unsupported Windows version".to_string());
    }
    let status = if diagnostics.is_empty() {
        "pass"
    } else {
        "needs-review"
    };
    serde_json::json!({
        "schema_version":"windows-install-p0/v1", "status":status,
        "support": if diagnostics.is_empty() {"constrained"} else {"blocked"},
        "environment": {"os":scenario.environment.os, "clean_vm":scenario.environment.clean_vm, "package_identity":scenario.environment.package_identity},
        "webview2": {"state":scenario.webview2.state, "bootstrapper":scenario.webview2.bootstrapper, "download":scenario.webview2.download, "ready":webview_ready},
        "operations":scenario.operations, "measurements":scenario.measurements.unwrap_or(Measurements {installed_size_mb:None,download_size_mb:None,first_launch_ms:None,idle_memory_mb:None}),
        "diagnostics":diagnostics,
        "release_gate": if diagnostics.is_empty() {"VM-evidence-required"} else {"block-Windows-v1-distribution"}
    })
}
fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let source = if args.next().as_deref() == Some("--fixture") {
        match args.next().and_then(|p| fs::read_to_string(p).ok()) {
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
    let input: Value = match serde_json::from_str(&source) {
        Ok(v) => v,
        Err(_) => return ExitCode::from(2),
    };
    println!("{}", serde_json::to_string_pretty(&collect(input)).unwrap());
    ExitCode::SUCCESS
}
