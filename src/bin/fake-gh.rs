//! Test-only GitHub CLI process double. It never performs a network request.
use std::{fs::OpenOptions, io::Write, process::ExitCode, time::Duration};

fn main() -> ExitCode {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    if let Some(path) = std::env::var_os("AGENTMETER_FAKE_GH_LOG") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        writeln!(file, "{}", arguments.join(" ")).unwrap();
    }
    match std::env::var("AGENTMETER_FAKE_GH_MODE")
        .unwrap_or_else(|_| "success".into())
        .as_str()
    {
        "success" => {
            print!(
                "{}",
                std::env::var("AGENTMETER_FAKE_GH_RESPONSE").unwrap_or_else(|_| "{}".into())
            );
            ExitCode::SUCCESS
        }
        "matrix" => {
            let endpoint = arguments.last().map(String::as_str).unwrap_or_default();
            let (prefix, field) = if endpoint.starts_with("/users/") {
                ("/users/", "user")
            } else if endpoint.starts_with("/organizations/") {
                ("/organizations/", "organization")
            } else if endpoint.starts_with("/enterprises/") {
                ("/enterprises/", "enterprise")
            } else {
                eprintln!("HTTP 400: unexpected endpoint");
                return ExitCode::from(1);
            };
            let account = endpoint
                .strip_prefix(prefix)
                .and_then(|rest| rest.split('/').next())
                .unwrap_or_default();
            let (product, sku, unit) = if endpoint.contains("/ai_credit/usage") {
                ("Copilot AI Credits", "Copilot AI Credits", "credits")
            } else {
                ("Copilot", "Copilot Premium Request", "requests")
            };
            let mut response = serde_json::json!({
                "timePeriod":{"year":2026,"month":9},
                "usageItems":[{
                    "product":product,
                    "sku":sku,
                    "model":"GPT-5",
                    "unitType":unit,
                    "grossQuantity":12.0,
                    "discountQuantity":2.0,
                    "netQuantity":10.0
                }]
            });
            response
                .as_object_mut()
                .unwrap()
                .insert(field.into(), serde_json::json!(account));
            print!("{response}");
            ExitCode::SUCCESS
        }
        "timeout" => {
            std::thread::sleep(Duration::from_secs(30));
            ExitCode::SUCCESS
        }
        "oversize" => {
            print!("{}", "x".repeat(300 * 1024));
            ExitCode::SUCCESS
        }
        mode => {
            let status = match mode {
                "unauthenticated" => "HTTP 401: Requires authentication",
                "forbidden" => "HTTP 403: Resource not accessible by integration",
                "rate-limited" => "HTTP 403: API rate limit exceeded",
                "not-found" => "HTTP 404: Not Found",
                "bad-request" => "HTTP 400: Bad Request",
                "unavailable" => "HTTP 503: Service unavailable",
                _ => "request failed",
            };
            eprintln!("{status}");
            ExitCode::from(1)
        }
    }
}
