use std::{fs, path::PathBuf, time::SystemTime};

use agentmeter_p0::provider_history::{ProviderHistory, ProviderHistoryError};

fn temp_root(provider: &str) -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "agentmeter-provider-history-{provider}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn consented_content_counts_only_exact_nontrivial_duplicate_prompts() {
    let root = temp_root("claude-duplicates");
    let project = root.join("projects/fixture");
    fs::create_dir_all(&project).unwrap();
    let repeated = "Please review this exact module and list concrete regressions.";
    let body = format!(
        "{{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:00:00Z\",\"message\":{{\"role\":\"user\",\"content\":\"{repeated}\"}}}}\n\
         {{\"type\":\"assistant\",\"timestamp\":\"2026-09-12T01:00:01Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-sonnet-4-6\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":2}}}}}}\n\
         {{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:01:00Z\",\"message\":{{\"role\":\"user\",\"content\":\"  Please   review this exact module and list concrete regressions.  \"}}}}\n\
         {{\"type\":\"assistant\",\"timestamp\":\"2026-09-12T01:01:01Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-sonnet-4-6\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":2}}}}}}\n\
         {{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:02:00Z\",\"message\":{{\"role\":\"user\",\"content\":\"ok\"}}}}\n\
         {{\"type\":\"assistant\",\"timestamp\":\"2026-09-12T01:02:01Z\",\"message\":{{\"role\":\"assistant\",\"model\":\"claude-sonnet-4-6\",\"usage\":{{\"input_tokens\":10,\"output_tokens\":2}}}}}}\n"
    );
    fs::write(project.join("session.jsonl"), body).unwrap();

    let report = ProviderHistory::scan("claude", &root).unwrap();
    let session = &report.sessions[0];
    let content = ProviderHistory::read_session_content(
        "claude",
        &root,
        &session.session_ref,
        session.started_at.as_deref(),
        session.ended_at.as_deref(),
    )
    .unwrap();
    assert_eq!(content.duplicate_prompt_count, 1);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn all_four_providers_use_one_sanitized_report_interface() {
    let missing = temp_root("missing").join("absent");
    for provider in ["codex", "claude", "copilot"] {
        let report = ProviderHistory::scan(provider, &missing).unwrap();
        assert_eq!(report.schema_version, "agentmeter.provider-history.v1");
        assert_eq!(report.provider, provider);
        assert_eq!(report.status, "source_unavailable");
        assert!(report.sessions.is_empty());
    }

    let root = temp_root("antigravity");
    let report = ProviderHistory::scan("antigravity", &root).unwrap();
    assert_eq!(report.provider, "antigravity");
    assert_eq!(report.status, "schema_unverified");
    assert!(report.sessions.is_empty());
    fs::remove_dir_all(root).unwrap();

    assert_eq!(
        ProviderHistory::scan("not-a-provider", &missing).unwrap_err(),
        ProviderHistoryError::UnsupportedProvider
    );
}

#[test]
fn claude_projects_are_projected_to_sessions_and_tokens_without_content() {
    let root = temp_root("claude");
    let project = root.join("projects/private-project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("private-session.jsonl"),
        include_str!("fixtures/provider-history/claude-session.jsonl"),
    )
    .unwrap();

    let report = ProviderHistory::scan("claude", &root).unwrap();
    assert_eq!(report.status, "supported");
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].session_ref, "session-1");
    assert_eq!(report.sessions[0].turn_count, 2);
    assert_eq!(report.sessions[0].turns[0].tokens.input_tokens, Some(12));
    assert_eq!(
        report.sessions[0].turns[0].tokens.cached_input_tokens,
        Some(3)
    );
    assert_eq!(report.sessions[0].turns[0].tokens.output_tokens, Some(5));
    assert_eq!(report.sessions[0].turns[0].tokens.total_tokens, Some(22));
    assert_eq!(report.totals.total_tokens, Some(32));

    let serialized = serde_json::to_string(&report).unwrap();
    for forbidden in [
        "private-project",
        "private-session",
        "Review C:",
        "claude-secret-value",
        "PRIVATE_ASSISTANT_RESPONSE",
        "INTERNAL_MESSAGE_MUST_NOT_APPEAR",
        "PRIVATE_TOOL_RESULT",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn copilot_sessions_keep_token_metrics_unknown_instead_of_using_quota() {
    let root = temp_root("copilot");
    let session = root.join("session-state/private-session");
    fs::create_dir_all(&session).unwrap();
    fs::write(
        session.join("events.jsonl"),
        include_str!("fixtures/provider-history/copilot-events.jsonl"),
    )
    .unwrap();

    let report = ProviderHistory::scan("copilot", &root).unwrap();
    assert_eq!(report.status, "supported");
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].turn_count, 2);
    assert_eq!(report.sessions[0].totals.total_tokens, None);
    assert_eq!(report.totals.total_tokens, None);
    assert!(
        report.sessions[0]
            .turns
            .iter()
            .all(|turn| turn.tokens.total_tokens.is_none())
    );

    let serialized = serde_json::to_string(&report).unwrap();
    assert!(!serialized.contains("Copilot prompt"));
    assert!(!serialized.contains("PRIVATE_COPILOT_RESPONSE"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn content_requires_matching_public_session_projection_and_redacts_user_prompts() {
    let root = temp_root("claude-content");
    let project = root.join("projects/private-project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("private-session.jsonl"),
        include_str!("fixtures/provider-history/claude-session.jsonl"),
    )
    .unwrap();
    let report = ProviderHistory::scan("claude", &root).unwrap();
    let session = &report.sessions[0];

    assert_eq!(
        ProviderHistory::read_session_content(
            "claude",
            &root,
            &session.session_ref,
            session.started_at.as_deref(),
            Some("2026-09-12T09:09:09Z")
        )
        .unwrap_err(),
        ProviderHistoryError::SessionStale
    );

    let content = ProviderHistory::read_session_content(
        "claude",
        &root,
        &session.session_ref,
        session.started_at.as_deref(),
        session.ended_at.as_deref(),
    )
    .unwrap();
    assert_eq!(content.provider, "claude");
    assert_eq!(content.session_ref, "session-1");
    assert_eq!(content.prompts.len(), 2);
    assert_eq!(content.prompts[0].turn_ref, "turn-1");
    assert!(content.prompts[0].text.contains("[REDACTED_PATH]"));
    assert!(content.prompts[0].text.contains("api_key=[REDACTED]"));
    assert!(content.prompts[0].redacted);
    assert_eq!(content.prompts[1].text, "Second public prompt");
    assert!(!content.prompts[1].redacted);
    let serialized = serde_json::to_string(&content).unwrap();
    for forbidden in [
        "claude-secret-value",
        "PRIVATE_ASSISTANT_RESPONSE",
        "INTERNAL_MESSAGE_MUST_NOT_APPEAR",
        "PRIVATE_TOOL_RESULT",
        "private-session",
        "private-project",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }

    assert_eq!(
        ProviderHistory::read_session_content(
            "claude",
            &root,
            "session-99",
            session.started_at.as_deref(),
            session.ended_at.as_deref(),
        )
        .unwrap_err(),
        ProviderHistoryError::SessionNotFound
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn copilot_user_content_is_available_but_tokens_remain_unknown() {
    let root = temp_root("copilot-content");
    let session_dir = root.join("session-state/private-session");
    fs::create_dir_all(&session_dir).unwrap();
    fs::write(
        session_dir.join("events.jsonl"),
        include_str!("fixtures/provider-history/copilot-events.jsonl"),
    )
    .unwrap();
    let report = ProviderHistory::scan("copilot", &root).unwrap();
    let session = &report.sessions[0];
    let content = ProviderHistory::read_session_content(
        "copilot",
        &root,
        &session.session_ref,
        session.started_at.as_deref(),
        session.ended_at.as_deref(),
    )
    .unwrap();

    assert_eq!(content.prompts.len(), 2);
    assert!(content.prompts[0].text.contains("token=[REDACTED]"));
    assert!(content.prompts[0].text.contains("[REDACTED_PATH]"));
    assert_eq!(content.prompts[1].text, "Second Copilot prompt");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn codex_content_uses_the_same_public_session_guard_and_never_returns_responses() {
    let root = temp_root("codex-content");
    let sessions = root.join("sessions/2026/09/12");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("private-rollout.jsonl"),
        include_str!("fixtures/codex-history/modern-metrics.jsonl"),
    )
    .unwrap();
    let report = ProviderHistory::scan("codex", &root).unwrap();
    let session = &report.sessions[0];
    let content = ProviderHistory::read_session_content(
        "codex",
        &root,
        &session.session_ref,
        session.started_at.as_deref(),
        session.ended_at.as_deref(),
    )
    .unwrap();

    assert_eq!(content.prompts.len(), 1);
    assert_eq!(content.prompts[0].text, "SECRET_PROMPT_SHOULD_NOT_LEAK");
    let serialized = serde_json::to_string(&content).unwrap();
    assert!(!serialized.contains("SECRET_RESPONSE_SHOULD_NOT_LEAK"));
    assert!(!serialized.contains("sensitive-session-id"));
    assert!(!serialized.contains("private-rollout"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn antigravity_content_fails_with_a_fixed_schema_error() {
    let root = temp_root("antigravity-content");
    assert_eq!(
        ProviderHistory::read_session_content("antigravity", &root, "session-1", None, None,)
            .unwrap_err(),
        ProviderHistoryError::SchemaUnverified
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn codex_content_fails_closed_when_public_timestamps_are_ambiguous() {
    let root = temp_root("codex-ambiguous");
    let active = root.join("sessions/2026/09/12");
    let archived = root.join("archived_sessions");
    fs::create_dir_all(&active).unwrap();
    fs::create_dir_all(&archived).unwrap();
    let fixture = include_str!("fixtures/codex-history/modern-metrics.jsonl");
    fs::write(active.join("active.jsonl"), fixture).unwrap();
    fs::write(archived.join("archived.jsonl"), fixture).unwrap();
    let report = ProviderHistory::scan("codex", &root).unwrap();
    let session = &report.sessions[0];
    assert_eq!(
        ProviderHistory::read_session_content(
            "codex",
            &root,
            &session.session_ref,
            session.started_at.as_deref(),
            session.ended_at.as_deref(),
        )
        .unwrap_err(),
        ProviderHistoryError::ContentUnavailable
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn prompt_redaction_covers_headers_url_credentials_private_keys_and_common_tokens() {
    let root = temp_root("redaction");
    let project = root.join("projects/project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("session.jsonl"),
        concat!(
            "{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"Authorization: Bearer top-secret-token https://alice:password@example.invalid sk-1234567890abcdef\\n-----BEGIN PRIVATE KEY-----\\nPRIVATE_KEY_BODY\\n-----END PRIVATE KEY-----\"}}\n",
            "{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"run --token\\tcli-secret-value\"}}\n",
            "{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"{\\\"token\\\":\\\"json-secret-value\\\"}\"}}\n",
            "{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"{\\\"api_key\\\":\\\"json-api-secret-value\\\"}\"}}\n",
            "{\"type\":\"assistant\",\"timestamp\":\"2026-09-12T01:00:01Z\",\"message\":{\"role\":\"assistant\",\"usage\":{\"input_tokens\":1,\"output_tokens\":1}}}\n"
        ),
    )
    .unwrap();
    let report = ProviderHistory::scan("claude", &root).unwrap();
    let session = &report.sessions[0];
    let content = ProviderHistory::read_session_content(
        "claude",
        &root,
        &session.session_ref,
        session.started_at.as_deref(),
        session.ended_at.as_deref(),
    )
    .unwrap();
    let serialized = serde_json::to_string(&content).unwrap();
    for secret in [
        "top-secret-token",
        "alice:password",
        "sk-1234567890abcdef",
        "PRIVATE_KEY_BODY",
        "cli-secret-value",
        "json-secret-value",
        "json-api-secret-value",
    ] {
        assert!(!serialized.contains(secret), "leaked {secret}");
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn claude_and_copilot_content_fail_closed_when_timestamps_are_ambiguous() {
    for (provider, relative, fixture) in [
        (
            "claude",
            "projects/project",
            include_str!("fixtures/provider-history/claude-session.jsonl"),
        ),
        (
            "copilot",
            "session-state/session-a",
            include_str!("fixtures/provider-history/copilot-events.jsonl"),
        ),
    ] {
        let root = temp_root(&format!("{provider}-ambiguous"));
        let first = root.join(relative);
        fs::create_dir_all(&first).unwrap();
        fs::write(first.join("first.jsonl"), fixture).unwrap();
        let second = if provider == "claude" {
            root.join("projects/other-project")
        } else {
            root.join("session-state/session-b")
        };
        fs::create_dir_all(&second).unwrap();
        fs::write(second.join("second.jsonl"), fixture).unwrap();
        let report = ProviderHistory::scan(provider, &root).unwrap();
        let session = &report.sessions[0];
        assert_eq!(
            ProviderHistory::read_session_content(
                provider,
                &root,
                &session.session_ref,
                session.started_at.as_deref(),
                session.ended_at.as_deref(),
            )
            .unwrap_err(),
            ProviderHistoryError::ContentUnavailable
        );
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn malformed_and_partial_records_are_reported_without_leaking_lines() {
    let root = temp_root("malformed");
    let project = root.join("projects/project");
    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("session.jsonl"),
        concat!(
            "{not-json}\n",
            "{\"type\":\"user\",\"userType\":\"external\",\"timestamp\":\"2026-09-12T01:00:00Z\",\"message\":{\"role\":\"user\",\"content\":\"safe prompt\"}}\n",
            "{\"partial\":true}"
        ),
    )
    .unwrap();
    let report = ProviderHistory::scan("claude", &root).unwrap();
    assert_eq!(report.records_rejected, 1);
    assert_eq!(report.partial_records, 1);
    assert!(report.limited);
    assert!(!serde_json::to_string(&report).unwrap().contains("not-json"));
    fs::remove_dir_all(root).unwrap();
}
