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
    #[serde(default)]
    user_data: Option<UserDataPolicy>,
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

#[derive(Debug, Deserialize, Serialize)]
struct UserDataPolicy {
    uninstall_default: String,
    explicit_delete: String,
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
    match scenario.webview2.state.as_str() {
        "present-current" if scenario.webview2.download != "not-needed" => diagnostics
            .push("current WebView2 runtime must not trigger a bootstrapper download".to_string()),
        "missing"
            if scenario.webview2.download != "installed"
                && scenario.webview2.download != "failed" =>
        {
            diagnostics.push(
                "missing WebView2 runtime must be installed or report a download failure"
                    .to_string(),
            )
        }
        "outdated"
            if scenario.webview2.download != "updated-restart-required"
                && scenario.webview2.download != "failed" =>
        {
            diagnostics.push(
                "outdated WebView2 runtime must update and require restart or report failure"
                    .to_string(),
            )
        }
        _ => {}
    }
    for operation in &scenario.operations {
        if operation.result != "pass" {
            diagnostics.push(format!("{}: {}", operation.name, operation.result));
        }
    }
    for required in [
        "install",
        "launch",
        "restart",
        "repair",
        "reinstall-same-version",
        "upgrade",
        "uninstall",
    ] {
        if !scenario
            .operations
            .iter()
            .any(|operation| operation.name == required)
        {
            diagnostics.push(format!(
                "required lifecycle operation not recorded: {required}"
            ));
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
    let measurements_complete = scenario.measurements.as_ref().is_some_and(|measurements| {
        measurements.installed_size_mb.is_some()
            && measurements.download_size_mb.is_some()
            && measurements.first_launch_ms.is_some()
            && measurements.idle_memory_mb.is_some()
    });
    if !measurements_complete {
        diagnostics.push(
            "clean-VM size, launch-time, and idle-memory measurements are incomplete".to_string(),
        );
    }
    let user_data_ok = scenario.user_data.as_ref().is_some_and(|policy| {
        policy.uninstall_default == "preserve-local-app-data"
            && policy.explicit_delete == "remove-local-app-data"
    });
    if !user_data_ok {
        diagnostics
            .push("uninstall user-data preservation/deletion behavior is not recorded".to_string());
    }
    let blocking = diagnostics.iter().any(|diagnostic| {
        !diagnostic.starts_with("clean-VM")
            && !diagnostic.starts_with("uninstall user-data")
            && !diagnostic.starts_with("required lifecycle operation")
    });
    let status = if diagnostics.is_empty() {
        "pass"
    } else if blocking {
        "blocked"
    } else {
        "needs-info"
    };
    serde_json::json!({
        "schema_version":"windows-install-p0/v1", "status":status,
        "support": if diagnostics.is_empty() {"constrained"} else if blocking {"blocked"} else {"needs-info"},
        "environment": {"os":scenario.environment.os, "clean_vm":scenario.environment.clean_vm, "package_identity":scenario.environment.package_identity},
        "webview2": {"state":scenario.webview2.state, "bootstrapper":scenario.webview2.bootstrapper, "download":scenario.webview2.download, "ready":webview_ready},
        "operations":scenario.operations, "measurements":scenario.measurements.unwrap_or(Measurements {installed_size_mb:None,download_size_mb:None,first_launch_ms:None,idle_memory_mb:None}), "user_data":scenario.user_data,
        "diagnostics":diagnostics,
        "release_gate": if blocking {"block-Windows-v1-distribution"} else {"VM-evidence-required"}
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
