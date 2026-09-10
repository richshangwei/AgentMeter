use serde_json::Value;
use std::{path::PathBuf, process::Command};
fn run_fixture(fixture: &str) -> Value {
    let o = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .args(["--fixture", fixture])
        .output()
        .unwrap();
    assert!(o.status.success());
    serde_json::from_slice(&o.stdout).unwrap()
}
fn run() -> Value {
    run_fixture("tests/fixtures/copilot/contexts.json")
}
fn run_boundaries() -> Value {
    run_fixture("tests/fixtures/copilot/contexts-boundaries.json")
}
fn run_input(input: Value) -> Value {
    let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    serde_json::to_writer(child.stdin.as_mut().unwrap(), &input).unwrap();
    child.stdin.take();
    let output = child.wait_with_output().unwrap();
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn matrix_preserves_contexts() {
    let v = run();
    assert_eq!(v["schema_version"], "copilot-p0/v1");
    assert_eq!(v["contexts"].as_array().unwrap().len(), 3);
    assert_eq!(v["contexts"][0]["context"], "Personal");
    assert_eq!(v["contexts"][1]["context"], "Business");
    assert_eq!(v["contexts"][2]["context"], "Enterprise");
}
#[test]
fn permission_failure_is_distinct() {
    let v = run();
    assert_eq!(
        v["contexts"][1]["billing_api"]["status"],
        "permission_denied"
    );
    assert_eq!(
        v["contexts"][1]["quota"]["status"],
        "unknown_permission_denied"
    );
    assert_eq!(v["contexts"][1]["failure_code"], "permission_denied");
    assert!(v["contexts"][1]["observation"].is_null());
}
#[test]
fn metrics_separate_preview_experimental() {
    let v = run();
    let p = &v["contexts"][0];
    assert_eq!(p["ai_credits"]["remaining"], 120);
    assert_eq!(p["legacy_premium_requests"]["remaining"], 30);
    assert_eq!(p["preview_sdk"]["experimental"], true);
    assert_eq!(p["preview_sdk"]["stability"], "public_preview");
    assert_eq!(
        p["preview_sdk"]["permission"],
        "Provider Account quota read"
    );
    assert_eq!(p["preview_sdk"]["permission_status"], "granted_fixture");
    assert_eq!(
        p["preview_sdk"]["billing_api_comparison"]["status"],
        "diverged"
    );
    assert_eq!(
        p["preview_sdk"]["billing_api_comparison"]["billing_remaining"].as_f64(),
        Some(120.0)
    );
    assert_eq!(
        p["preview_sdk"]["billing_api_comparison"]["preview_remaining"].as_f64(),
        Some(119.0)
    );
    assert_eq!(p["observation"]["data_quality"], "local_observed");
    assert_eq!(p["observation"]["source"]["replay"], true);
    assert_eq!(p["observation"]["quota_windows"][0]["unit"], "AI Credits");
    assert_eq!(p["plan"], "Copilot Pro fixture");
}

#[test]
fn preview_without_comparable_billing_data_is_explicit() {
    let v = run();
    let enterprise = &v["contexts"][2];
    assert_eq!(
        enterprise["preview_sdk"]["billing_api_comparison"]["status"],
        "not_comparable"
    );
    assert_eq!(enterprise["preview_sdk"]["stability"], "public_preview");
}

#[test]
fn malformed_and_schema_changed_inputs_are_diagnosable() {
    let d = std::env::temp_dir().join(format!("agentmeter-copilot-{}", std::process::id()));
    std::fs::write(&d, b"not-json").unwrap();
    let malformed = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .args(["--fixture", d.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!malformed.status.success());
    let report: Value = serde_json::from_slice(&malformed.stdout).unwrap();
    assert_eq!(report["failure_code"], "malformed_response");
    std::fs::write(&d, b"{}").unwrap();
    let changed = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .args(["--fixture", d.to_str().unwrap()])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&changed.stdout).unwrap();
    assert_eq!(report["failure_code"], "schema_changed");
    std::fs::remove_file(d).unwrap();
}

#[test]
fn provider_failure_classes_remain_distinct_without_observations() {
    for (permission, expected) in [
        ("unauthenticated", "authentication_failed"),
        ("rate_limited", "rate_limited"),
        ("endpoint_unavailable", "endpoint_unavailable"),
    ] {
        let mut child = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        serde_json::to_writer(
            child.stdin.as_mut().unwrap(),
            &serde_json::json!({"contexts":[{"context":"Personal","billing_api":{"permission":permission},"quota":{"status":"unknown"}}]}),
        )
        .unwrap();
        child.stdin.take();
        let output = child.wait_with_output().unwrap();
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["contexts"][0]["failure_code"], expected);
        assert!(report["contexts"][0]["observation"].is_null());
        match permission {
            "unauthenticated" => {
                assert_eq!(report["contexts"][0]["availability"], "needs_login")
            }
            "rate_limited" => {
                assert_eq!(report["contexts"][0]["availability"], "available");
                assert_eq!(report["contexts"][0]["collection_state"], "backing_off");
            }
            "endpoint_unavailable" => {
                assert_eq!(report["contexts"][0]["availability"], "unsupported")
            }
            _ => unreachable!(),
        }
    }
}
#[test]
fn unknowns_preserved() {
    let v = run();
    assert_eq!(v["unknowns"][0], "real preview endpoint stability");
    assert_eq!(v["contexts"][2]["unknowns"][0], "enterprise contract limit");
}

#[test]
fn sanitized_fixture_evidence_is_complete_and_secret_free() {
    let report = run();
    let evidence = &report["evidence"];
    assert_eq!(evidence["fixture_schema_version"], "copilot-contexts-v1");
    assert_eq!(evidence["evidence_kind"], "sanitized_fixture_replay");
    assert_eq!(evidence["tested_at"], "2026-09-07T00:00:00+08:00");
    assert_eq!(evidence["plan_types"].as_array().unwrap().len(), 3);
    assert_eq!(
        evidence["api_contract_versions"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        evidence["sdk_contract_versions"].as_array().unwrap().len(),
        1
    );
    assert!(
        evidence["observed_fields"]["quota"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "billing_period")
    );
    assert_eq!(evidence["credentials_present"], false);
    assert_eq!(evidence["billing_secrets_present"], false);
    assert!(evidence["limitations"].as_array().unwrap().len() >= 3);
}

#[test]
fn permission_scope_and_role_contracts_are_explicit_and_missing_access_is_diagnosable() {
    let v = run_boundaries();
    let personal = &v["contexts"][0]["permission_requirements"];
    assert_eq!(personal["required_permissions"][0], "Plan:read");
    assert_eq!(personal["status"], "satisfied_fixture");
    assert!(
        personal["missing_permissions"]
            .as_array()
            .unwrap()
            .is_empty()
    );

    let business = &v["contexts"][1]["permission_requirements"];
    assert_eq!(business["required_permissions"][0], "Administration:read");
    assert_eq!(business["required_roles"][0], "organization_administrator");
    assert_eq!(business["missing_permissions"][0], "Administration:read");
    assert_eq!(business["missing_roles"][0], "organization_administrator");
    assert_eq!(business["status"], "missing_or_untested_fixture");
    assert!(
        v["contexts"][1]["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d == "missing required fine-grained permission(s)")
    );

    let enterprise = &v["contexts"][2]["permission_requirements"];
    assert_eq!(
        enterprise["required_permissions"][0],
        "Enterprise billing:read"
    );
    assert_eq!(
        enterprise["observed_roles"][0],
        "enterprise_billing_manager"
    );
    assert_eq!(enterprise["status"], "satisfied_fixture");
    assert!(enterprise["missing_roles"].as_array().unwrap().is_empty());
}

#[test]
fn credits_allowances_overage_and_scopes_are_separate_from_legacy_requests() {
    let v = run_boundaries();
    let personal = &v["contexts"][0]["quota_boundaries"];
    let credits = &personal["ai_credits"];
    let legacy = &personal["legacy_premium_requests"];
    assert_eq!(credits["unit"], "AI Credits");
    assert_eq!(credits["included_allowance"], 300);
    assert_eq!(credits["remaining"], 120);
    assert_eq!(credits["overage"]["extra_paid"], true);
    assert_eq!(credits["overage"]["used"], 12);
    assert_eq!(credits["scope"]["pool_kind"], "individual");
    assert_eq!(legacy["unit"], "premium requests");
    assert_eq!(legacy["included_allowance"], 300);
    assert_eq!(legacy["remaining"], 30);
    assert_eq!(legacy["overage"]["extra_paid"], true);
    assert_eq!(legacy["overage"]["used"], 2);
    assert_ne!(credits["unit"], legacy["unit"]);
    assert_eq!(personal["aggregation"], "never_sum_across_units");
    assert_eq!(
        credits["remaining"].as_i64().unwrap() + legacy["remaining"].as_i64().unwrap(),
        150
    );
}

#[test]
fn enterprise_shared_pool_and_individual_limit_are_not_conflated() {
    let v = run_boundaries();
    let enterprise = &v["contexts"][2]["quota_boundaries"]["ai_credits"];
    assert_eq!(enterprise["unit"], "AI Credits");
    assert_eq!(enterprise["scope"]["pool_kind"], "enterprise_shared");
    assert_eq!(enterprise["scope"]["shared_pool_remaining"], 180000);
    assert_eq!(enterprise["scope"]["individual_limit"], 5000);
    assert_eq!(enterprise["remaining"], 180000);
    assert_eq!(enterprise["overage"]["extra_paid"], false);
    assert_eq!(
        v["contexts"][2]["legacy_premium_requests"]["status"],
        "not_applicable"
    );
}

#[test]
fn granted_shape_without_required_permission_fails_closed() {
    let report = run_input(serde_json::json!({"contexts":[{
        "context":"Personal",
        "billing_api":{"permission":"granted"},
        "permission_contract":{"required_permissions":["Plan:read"],"observed_permissions":[]},
        "quota":{"status":"reported","remaining":1,"unit":"AI Credits","billing_period":"2026-09"}
    }]}));
    let context = &report["contexts"][0];
    assert_eq!(context["failure_code"], "permission_requirements_missing");
    assert_eq!(context["collection_state"], "error");
    assert!(context["observation"].is_null());
    assert_eq!(
        context["permission_requirements"]["missing_permissions"][0],
        "Plan:read"
    );
}

#[test]
fn identical_meter_units_fail_closed_instead_of_being_aggregated() {
    let report = run_input(serde_json::json!({"contexts":[{
        "context":"Personal",
        "billing_api":{"permission":"granted"},
        "permission_contract":{"required_permissions":["Plan:read"],"observed_permissions":["Plan:read"]},
        "quota":{"status":"reported","remaining":1,"unit":"AI Credits","billing_period":"2026-09"},
        "ai_credits":{"status":"reported","remaining":1,"unit":"AI Credits"},
        "legacy_premium_requests":{"status":"reported","remaining":2,"unit":"AI Credits"}
    }]}));
    let context = &report["contexts"][0];
    assert_eq!(context["failure_code"], "schema_changed");
    assert_eq!(context["collection_state"], "error");
    assert!(context["observation"].is_null());
    assert_eq!(
        context["quota_boundaries"]["aggregation"],
        "never_sum_across_units"
    );
    assert!(
        context["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d == "AI Credits and legacy premium requests must use distinct units")
    );
}

fn temp_path(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "agentmeter-copilot-{label}-{}-{nonce}",
        std::process::id()
    ))
}

fn run_live(
    context: &str,
    account: &str,
    meter: &str,
    mode: &str,
    response: Value,
    extra: &[&str],
) -> (std::process::Output, PathBuf) {
    let log = temp_path("gh-log");
    let mut command = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"));
    command
        .args([
            "collect",
            "--gh-bin",
            env!("CARGO_BIN_EXE_fake-gh"),
            "--context",
            context,
            "--account",
            account,
            "--meter",
            meter,
        ])
        .args(extra)
        .env("AGENTMETER_FAKE_GH_MODE", mode)
        .env("AGENTMETER_FAKE_GH_RESPONSE", response.to_string())
        .env("AGENTMETER_FAKE_GH_LOG", &log);
    (command.output().unwrap(), log)
}

fn live_response(context: &str, account: &str, unit: &str) -> Value {
    let mut response = serde_json::json!({
        "timePeriod":{"year":2026,"month":9},
        "usageItems":[{
            "product":"Copilot",
            "sku":"Copilot metered usage",
            "model":"GPT-5",
            "unitType":unit,
            "pricePerUnit":0.01,
            "grossQuantity":12.0,
            "grossAmount":0.12,
            "discountQuantity":2.0,
            "discountAmount":0.02,
            "netQuantity":10.0,
            "netAmount":0.10
        }]
    });
    response
        .as_object_mut()
        .unwrap()
        .insert(context.to_owned(), serde_json::json!(account));
    response
}

#[test]
fn live_gh_path_covers_all_documented_context_and_meter_endpoints_without_conflating_quota() {
    for (context, account_field, account, prefix) in [
        ("personal", "user", "octocat", "users"),
        ("business", "organization", "example-org", "organizations"),
        (
            "enterprise",
            "enterprise",
            "example-enterprise",
            "enterprises",
        ),
    ] {
        for (meter, segment, unit) in [
            ("ai-credits", "ai_credit", "credits"),
            ("premium-requests", "premium_request", "requests"),
        ] {
            let (output, log) = run_live(
                context,
                account,
                meter,
                "success",
                live_response(account_field, account, unit),
                &["--year", "2026", "--month", "9"],
            );
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stdout)
            );
            let report: Value = serde_json::from_slice(&output.stdout).unwrap();
            let result = &report["contexts"][0];
            assert_eq!(result["data_quality"], "official");
            assert_eq!(result["authoritative_usage"]["items"][0]["net_used"], 10.0);
            assert_eq!(
                result["quota"]["status"],
                "not_exposed_by_billing_usage_endpoint"
            );
            assert!(result["observation"].is_null());
            assert!(result["quota"]["remaining"].is_null());
            let arguments = std::fs::read_to_string(&log).unwrap();
            assert!(arguments.contains("Accept: application/vnd.github+json"));
            assert!(arguments.contains("X-GitHub-Api-Version: 2026-03-10"));
            assert!(arguments.contains(&format!(
                "/{prefix}/{account}/settings/billing/{segment}/usage?product=Copilot&year=2026&month=9"
            )));
            assert!(!arguments.to_ascii_lowercase().contains("token"));
            let _ = std::fs::remove_file(log);
        }
    }
}

#[test]
fn live_gh_failures_are_classified_without_echoing_provider_output() {
    for (mode, expected) in [
        ("unauthenticated", "authentication_failed"),
        ("forbidden", "permission_denied"),
        ("rate-limited", "rate_limited"),
        ("not-found", "endpoint_unavailable"),
        ("bad-request", "invalid_request"),
        ("unavailable", "provider_unavailable"),
        ("other", "command_failed"),
    ] {
        let (output, log) = run_live(
            "personal",
            "octocat",
            "ai-credits",
            mode,
            serde_json::json!({}),
            &[],
        );
        assert!(!output.status.success());
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["failure_code"], expected);
        assert!(!String::from_utf8_lossy(&output.stdout).contains("HTTP"));
        let _ = std::fs::remove_file(log);
    }
}

#[test]
fn live_gh_timeout_and_output_cap_fail_closed() {
    let started = std::time::Instant::now();
    let (timeout, timeout_log) = run_live(
        "personal",
        "octocat",
        "ai-credits",
        "timeout",
        serde_json::json!({}),
        &["--timeout-ms", "50"],
    );
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
    let timeout_report: Value = serde_json::from_slice(&timeout.stdout).unwrap();
    assert_eq!(timeout_report["failure_code"], "timeout");
    let _ = std::fs::remove_file(timeout_log);

    let (oversize, oversize_log) = run_live(
        "personal",
        "octocat",
        "ai-credits",
        "oversize",
        serde_json::json!({}),
        &[],
    );
    let oversize_report: Value = serde_json::from_slice(&oversize.stdout).unwrap();
    assert_eq!(oversize_report["failure_code"], "response_too_large");
    let _ = std::fs::remove_file(oversize_log);
}

#[test]
fn live_gh_rejects_unsafe_paths_slugs_and_schema_changes() {
    let relative = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .args([
            "collect",
            "--gh-bin",
            "gh",
            "--context",
            "personal",
            "--account",
            "octocat",
            "--meter",
            "ai-credits",
        ])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&relative.stdout).unwrap();
    assert_eq!(report["failure_code"], "invalid_arguments");

    let missing = temp_path("missing-gh");
    let missing_output = Command::new(env!("CARGO_BIN_EXE_agentmeter-copilot-p0"))
        .args([
            "collect",
            "--gh-bin",
            missing.to_str().unwrap(),
            "--context",
            "personal",
            "--account",
            "octocat",
            "--meter",
            "ai-credits",
        ])
        .output()
        .unwrap();
    let report: Value = serde_json::from_slice(&missing_output.stdout).unwrap();
    assert_eq!(report["failure_code"], "invalid_arguments");

    let (bad_slug, bad_slug_log) = run_live(
        "personal",
        "../octocat",
        "ai-credits",
        "success",
        serde_json::json!({}),
        &[],
    );
    let report: Value = serde_json::from_slice(&bad_slug.stdout).unwrap();
    assert_eq!(report["failure_code"], "invalid_arguments");
    assert!(!bad_slug_log.exists());

    let (schema, schema_log) = run_live(
        "personal",
        "octocat",
        "premium-requests",
        "success",
        live_response("user", "octocat", "credits"),
        &[],
    );
    let report: Value = serde_json::from_slice(&schema.stdout).unwrap();
    assert_eq!(report["failure_code"], "schema_changed");
    let _ = std::fs::remove_file(schema_log);
}
