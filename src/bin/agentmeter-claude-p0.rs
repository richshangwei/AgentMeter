use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

fn main() {
    let a: Vec<String> = env::args().collect();
    if a.len() < 3 {
        fail("usage");
    }
    let result = match a[1].as_str() {
        "install" => install(&a[2]),
        "enable" => lifecycle(&a[2], true, true),
        "disable" => lifecycle(&a[2], false, false),
        "remove" => remove(&a[2]),
        "normalize" => normalize(&a[2]),
        "fallback" => fallback(&a[2]),
        "status" => Ok(
            json!({"collection_state":"unknown","diagnostic":"status_event_required","observation":null}),
        ),
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
                &json!({"config":v,"enabled":false,"manifest":"AgentMeter Claude statusLine"}),
            )
            .unwrap(),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(json!({"enabled":false,"preserved":read(&m)?["config"]["statusLine"],"manifest":m}))
}
fn lifecycle(s: &str, on: bool, existing: bool) -> Result<Value, String> {
    let c = path(s);
    let mut v = read(&c)?;
    let m = manifest(&c);
    if !existing {
        let old = read(&m)?;
        v = old["config"].clone();
        fs::write(&c, serde_json::to_vec_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
        return Ok(json!({"restored":true,"enabled":false}));
    }
    if !m.exists() {
        fs::write(
            &m,
            serde_json::to_vec_pretty(
                &json!({"config":v,"enabled":false,"manifest":"AgentMeter Claude statusLine"}),
            )
            .unwrap(),
        )
        .map_err(|e| e.to_string())?;
    }
    if on {
        v["statusLine"] = json!({"type":"command","command":"agentmeter-claude-p0 status"});
        fs::write(&c, serde_json::to_vec_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
    }
    Ok(json!({"enabled":on,"preserved":read(&m)?["config"]["statusLine"],"manifest":m}))
}
fn remove(s: &str) -> Result<Value, String> {
    let c = path(s);
    let m = manifest(&c);
    let old = read(&m)?;
    fs::write(&c, serde_json::to_vec_pretty(&old["config"]).unwrap()).map_err(|e| e.to_string())?;
    fs::remove_file(m).map_err(|e| e.to_string())?;
    Ok(json!({"restored":true}))
}
fn normalize(s: &str) -> Result<Value, String> {
    let v = match read(&path(s)) {
        Ok(v) => v,
        Err(e) if e == "malformed_payload" => {
            return Ok(
                json!({"collection_state":"unknown","diagnostic":"malformed_payload","observation":null}),
            );
        }
        Err(e) => return Err(e),
    };
    if v.get("type").and_then(Value::as_str) != Some("status") {
        return Ok(
            json!({"collection_state":"unknown","diagnostic":"schema_changed","observation":null}),
        );
    }
    let u = v.get("usage").and_then(Value::as_object);
    let Some(u) = u else {
        return Ok(
            json!({"collection_state":"unknown","diagnostic":"missing_quota","observation":null}),
        );
    };
    for k in ["window", "used", "limit"] {
        if !u.contains_key(k) {
            return Ok(
                json!({"collection_state":"unknown","diagnostic":"schema_changed","observation":null}),
            );
        }
    }
    Ok(
        json!({"source":"claude.statusLine","maturity":"structured","quality":"measured","collection_state":"available","observation":{"source_time":v["timestamp"],"quota_window":u["window"],"used":u["used"],"limit":u["limit"],"unit":u.get("unit").cloned().unwrap_or(json!("percent")),"reset_at":u.get("reset_at").cloned()}}),
    )
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
        json!({"source":"claude.log","maturity":"log_fallback","quality":"degraded","collection_state":"available","diagnostic":"structured_source_unavailable","observation":{"remaining":rem,"unit":"percent"}}),
    )
}
