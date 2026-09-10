use std::io::{self, BufRead, Write};

use serde_json::{Value, json};

fn main() {
    let mode = std::env::var("AGENTMETER_FAKE_MODE").unwrap_or_default();
    match mode.as_str() {
        "cancellable_idle" => {
            let marker = std::env::var("AGENTMETER_FAKE_READY").expect("fake ready path");
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.share_mode(0);
            }
            let _held_file = options.open(marker).expect("create readiness lock");
            std::thread::sleep(std::time::Duration::from_secs(30));
            return;
        }
        "exit" => return,
        "malformed" => {
            println!("this is not JSON");
            return;
        }
        "timeout" => {
            std::thread::sleep(std::time::Duration::from_secs(1));
            return;
        }
        "exit_once" => {
            let marker = std::env::var("AGENTMETER_FAKE_MARKER").expect("fake marker path");
            if !std::path::Path::new(&marker).exists() {
                std::fs::write(marker, b"first attempt exited").expect("write fake marker");
                return;
            }
        }
        _ => {}
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    let mut initialized = false;

    for line in stdin.lock().lines() {
        let request: Value =
            serde_json::from_str(&line.expect("read request")).expect("JSON request");
        let method = request.get("method").and_then(Value::as_str).unwrap_or("");
        let id = request.get("id").and_then(Value::as_i64);

        if method == "initialized" {
            initialized = true;
            continue;
        }

        if method == "account/rateLimits/read"
            && request.get("params").is_some_and(|value| !value.is_null())
        {
            write_message(
                &mut stdout,
                &json!({
                    "id": id,
                    "error": {"code": -32602, "message": "rateLimits params must be null or absent"}
                }),
            );
            continue;
        }
        if mode == "auth_rate_limits" && method == "account/rateLimits/read" {
            write_message(
                &mut stdout,
                &json!({
                    "id": id,
                    "error": {"code": -32001, "message": "Codex account authentication required to read rate limits"}
                }),
            );
            continue;
        }

        let response = match (id, method, initialized) {
            (Some(1), "initialize", false) => {
                write_message(
                    &mut stdout,
                    &json!({"method":"account/rateLimits/updated","params":{"ignored":true}}),
                );
                json!({
                    "id": 1,
                    "result": {
                        "codexHome": "C:\\fixture",
                        "platformFamily": "windows",
                        "platformOs": "windows",
                        "userAgent": "codex_cli_rs/test-fixture"
                    }
                })
            }
            (Some(2), "account/read", true) => json!({
                "id": 2,
                "result": {
                    "account": {
                        "type": "chatgpt",
                        "email": "fixture@example.invalid",
                        "planType": "plus"
                    },
                    "requiresOpenaiAuth": true
                }
            }),
            (Some(3), "account/rateLimits/read", true) => json!({
                "id": 3,
                "result": {
                    "rateLimits": {
                        "limitId": "codex",
                        "limitName": "Codex",
                        "planType": "plus",
                        "primary": {
                            "usedPercent": 23,
                            "windowDurationMins": 300,
                            "resetsAt": 1788739200
                        },
                        "secondary": null,
                        "credits": null
                    },
                    "rateLimitsByLimitId": null
                }
            }),
            (Some(4), "account/usage/read", true) => json!({
                "id": 4,
                "error": {"code": -32601, "message": "Method not found: account/usage/read"}
            }),
            (Some(id), _, _) => json!({
                "id": id,
                "error": {"code": -32600, "message": "Unexpected request sequence"}
            }),
            (None, _, _) => continue,
        };
        write_message(&mut stdout, &response);
        if id == Some(4) {
            break;
        }
    }
}

fn write_message(stdout: &mut impl Write, message: &Value) {
    serde_json::to_writer(&mut *stdout, message).expect("write response");
    stdout.write_all(b"\n").expect("terminate response");
    stdout.flush().expect("flush response");
}
