use serde_json::{Value, json};
use std::{
    env, fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() < 3 {
        fail("usage");
    }
    if a[1] == "--claude-status" {
        if agentmeter_p0::claude_setup::receive(Path::new(&a[2])).is_err() {
            std::process::exit(2);
        }
        return;
    }
    if a[1] == "status" {
        if let Err(e) = status(&a[2]) {
            let report = diagnostic_report(
                "command_failed",
                "available",
                "error",
                "status_line_command",
                &e,
            );
            eprintln!("{report}");
            std::process::exit(2);
        }
        return;
    }
    let result = match a[1].as_str() {
        "install" => install(&a[2]),
        "enable" => lifecycle(&a[2], true, true),
        "disable" => lifecycle(&a[2], false, false),
        "remove" => remove(&a[2]),
        "normalize" => normalize(&a[2]),
        "fallback" => fallback(&a[2]),
        _ => Err("usage".into()),
    };
    match result {
        Ok(v) => println!("{}", v),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    }
}
fn fail(s: &str) -> ! {
    eprintln!("{s}");
    std::process::exit(2)
}
fn path(s: &str) -> PathBuf {
    PathBuf::from(s)
}
fn read(p: &Path) -> Result<Value, String> {
    let b = fs::read(p).map_err(|e| e.to_string())?;
    serde_json::from_slice(&b).map_err(|_| "malformed_payload".into())
}
fn manifest(c: &Path) -> PathBuf {
    c.with_extension("agentmeter-statusline.json")
}
fn install(s: &str) -> Result<Value, String> {
    let c = path(s);
    let v = read(&c)?;
    let m = manifest(&c);
    if !m.exists() {
        fs::write(
            &m,
            serde_json::to_vec_pretty(
                &json!({"original_status_line":v.get("statusLine").cloned().unwrap_or(Value::Null),"installed_status_line":Value::Null,"enabled":false,"manifest":"AgentMeter Claude statusLine"}),
            )
            .unwrap(),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(
        json!({"enabled":false,"preserved":read(&m)?["original_status_line"],"observation_sink":m.with_extension("observation.json"),"manifest":m,"change":"install AgentMeter wrapper only after enable"}),
    )
}
fn lifecycle(s: &str, on: bool, existing: bool) -> Result<Value, String> {
    let c = path(s);
    let mut v = read(&c)?;
    let m = manifest(&c);
    if !existing {
        let mut saved = read(&m)?;
        let current = v.get("statusLine").cloned().unwrap_or(Value::Null);
        let installed = saved["installed_status_line"].clone();
        let original = saved["original_status_line"].clone();
        if current != installed && current != original {
            let mut report = diagnostic_report(
                "configuration_conflict",
                "setup_required",
                "error",
                "status_line_config",
                "statusLine changed after AgentMeter was enabled; no user configuration was overwritten",
            );
            report["restored"] = json!(false);
            report["enabled"] = json!(true);
            return Ok(report);
        }
        if current == installed {
            if original.is_null() {
                v.as_object_mut().unwrap().remove("statusLine");
            } else {
                v["statusLine"] = original;
            }
            fs::write(&c, serde_json::to_vec_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
        }
        saved["enabled"] = json!(false);
        fs::write(&m, serde_json::to_vec_pretty(&saved).unwrap()).map_err(|e| e.to_string())?;
        return Ok(json!({"restored":true,"enabled":false}));
    }
    if !m.exists() {
        install(s)?;
    }
    if on {
        let mut saved = read(&m)?;
        let current = v.get("statusLine").cloned().unwrap_or(Value::Null);
        let original = saved["original_status_line"].clone();
        let previous_install = saved["installed_status_line"].clone();
        if current != original && current != previous_install {
            let mut report = diagnostic_report(
                "configuration_conflict",
                "setup_required",
                "error",
                "status_line_config",
                "statusLine differs from the configuration previewed during install",
            );
            report["enabled"] = json!(false);
            return Ok(report);
        }
        let wrapper = json!({"type":"command","command":format!("agentmeter-claude-p0 status \"{}\"", m.display())});
        v["statusLine"] = wrapper.clone();
        fs::write(&c, serde_json::to_vec_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
        saved["installed_status_line"] = wrapper.clone();
        saved["enabled"] = json!(true);
        fs::write(&m, serde_json::to_vec_pretty(&saved).unwrap()).map_err(|e| e.to_string())?;
        return Ok(json!({"enabled":true,"preserved":original,"installed":wrapper,"manifest":m}));
    }
    Ok(json!({"enabled":on,"preserved":read(&m)?["original_status_line"],"manifest":m}))
}
fn remove(s: &str) -> Result<Value, String> {
    let c = path(s);
    let m = manifest(&c);
    let result = lifecycle(s, false, false)?;
    if result["diagnostic"] == "configuration_conflict" {
        return Ok(result);
    }
    match fs::remove_file(m.with_extension("observation.json")) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("normalized observation cleanup failed".into()),
    }
    fs::remove_file(m).map_err(|e| e.to_string())?;
    Ok(json!({"restored":true}))
}

fn status(s: &str) -> Result<(), String> {
    let saved = read(&path(s))?;
    let original = saved["original_status_line"]["command"].as_str();
    let mut input = Vec::new();
    std::io::stdin()
        .read_to_end(&mut input)
        .map_err(|e| format!("command_failed: {e}"))?;
    // Only the allowlisted report reaches disk; raw stdin stays in memory and is
    // forwarded unchanged to the user's display command.
    let report = normalize_status_event(&input);
    let sink = path(s).with_extension("observation.json");
    let sink_result = fs::write(&sink, serde_json::to_vec(&report).unwrap());
    let Some(command) = original else {
        return sink_result.map_err(|_| "normalized observation sink unavailable".into());
    };
    #[cfg(windows)]
    let mut child = Command::new("cmd")
        .args(["/D", "/S", "/C", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("command_failed: {e}"))?;
    #[cfg(not(windows))]
    let mut child = Command::new("sh")
        .args(["-c", command])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| format!("command_failed: {e}"))?;
    child
        .stdin
        .as_mut()
        .ok_or("command_failed: wrapper stdin unavailable")?
        .write_all(&input)
        .map_err(|e| format!("command_failed: {e}"))?;
    let output = child
        .wait_with_output()
        .map_err(|e| format!("command_failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "command_failed: prior statusLine exited with {}",
            output.status
        ));
    }
    std::io::stdout()
        .write_all(&output.stdout)
        .map_err(|_| "prior statusLine output unavailable")?;
    sink_result.map_err(|_| "normalized observation sink unavailable".into())
}

use agentmeter_p0::claude::normalize_status_event;
fn normalize(s: &str) -> Result<Value, String> {
    let bytes = match fs::read(path(s)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(diagnostic_report(
                "missing_event",
                "available",
                "idle",
                "status_line",
                "no statusLine event is available",
            ));
        }
        Err(error) => {
            return Ok(diagnostic_report(
                "command_failed",
                "available",
                "error",
                "status_line",
                &error.to_string(),
            ));
        }
    };
    let v: Value = match serde_json::from_slice(&bytes) {
        Ok(value) => value,
        Err(error) => {
            return Ok(diagnostic_report(
                "malformed_payload",
                "available",
                "error",
                "status_line",
                &error.to_string(),
            ));
        }
    };
    if v.get("type").and_then(Value::as_str) != Some("status") {
        return Ok(diagnostic_report(
            "schema_changed",
            "available",
            "error",
            "status_line",
            "status event type is missing or unsupported",
        ));
    }
    let u = v.get("usage").and_then(Value::as_object);
    let Some(u) = u else {
        let mut report = diagnostic_report(
            "missing_quota",
            "available",
            "idle",
            "status_line",
            "valid status event contains no quota fields",
        );
        report["outcome"] = json!("success");
        return Ok(report);
    };
    for k in ["window", "used", "limit"] {
        if !u.contains_key(k) {
            return Ok(diagnostic_report(
                "schema_changed",
                "available",
                "error",
                "status_line",
                &format!("usage.{k} is missing"),
            ));
        }
    }
    if !u["window"].is_string() || !u["used"].is_number() || !u["limit"].is_number() {
        return Ok(diagnostic_report(
            "schema_changed",
            "available",
            "error",
            "status_line",
            "usage window, used, or limit has an unsupported type",
        ));
    }
    Ok(
        json!({"schema_version":"agentmeter.p0.collection-report.v1","outcome":"success","provider":"claude","provider_account":{"kind":"unknown"},"source":{"kind":"status_line","mode":"fixture_replay","replay":true},"data_quality":"local_observed","collector_maturity":"experimental","availability":"available","collection_state":"ready","freshness":"unknown","failure_code":Value::Null,"diagnostics":[],"observation":{"source_timestamp":v.get("timestamp").cloned().unwrap_or(Value::Null),"quota_windows":[{"scope":u["window"],"used":u["used"],"limit":u["limit"],"unit":u.get("unit").cloned().unwrap_or(Value::Null),"resets_at":u.get("reset_at").cloned().unwrap_or(Value::Null)}]}}),
    )
}

fn diagnostic_report(
    code: &str,
    availability: &str,
    collection_state: &str,
    source_kind: &str,
    message: &str,
) -> Value {
    json!({
        "schema_version":"agentmeter.p0.collection-report.v1",
        "outcome":"failure",
        "provider":"claude",
        "provider_account":{"kind":"unknown"},
        "source":{"kind":source_kind,"mode":"fixture_replay","replay":true},
        "data_quality":"local_observed",
        "collector_maturity":"experimental",
        "availability":availability,
        "collection_state":collection_state,
        "freshness":"unknown",
        "failure_code":code,
        "diagnostic":code,
        "diagnostics":[{"code":code,"message":message}],
        "observation":Value::Null
    })
}
fn fallback(s: &str) -> Result<Value, String> {
    let t = fs::read_to_string(path(s)).map_err(|e| e.to_string())?;
    let rem = t
        .split('%')
        .next()
        .and_then(|x| x.rsplit_once(' '))
        .and_then(|(_, n)| n.parse::<u64>().ok())
        .ok_or("schema_changed")?;
    Ok(
        json!({"provider":"claude","source":{"kind":"log_fallback","mode":"fixture_replay"},"data_quality":"estimated","collector_maturity":"experimental","availability":"available","collection_state":"ready","freshness":"unknown","failure_code":Value::Null,"diagnostic":"structured_source_unavailable","observation":{"remaining":rem,"unit":"percent"}}),
    )
}
