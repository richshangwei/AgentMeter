use std::{fs, path::PathBuf, time::SystemTime};

use agentmeter_p0::codex_history::CodexHistory;

fn temp_root() -> PathBuf {
    let mut root = std::env::temp_dir();
    root.push(format!(
        "agentmeter-codex-history-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("sessions/2026/09/12")).unwrap();
    root
}

#[test]
fn scan_reports_session_and_turn_metrics_without_content_or_source_identity() {
    let root = temp_root();
    fs::write(
        root.join("sessions/2026/09/12/rollout-fixture.jsonl"),
        include_str!("fixtures/codex-history/modern-metrics.jsonl"),
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan sanitized Codex rollout");

    assert_eq!(report.schema_version, "agentmeter.codex-history.v1");
    assert_eq!(report.provider, "codex");
    assert_eq!(report.files_scanned, 1);
    assert!(!report.limited);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].session_ref, "session-1");
    assert_eq!(report.sessions[0].turns.len(), 2);
    assert_eq!(report.sessions[0].turns[0].turn_ref, "turn-1");
    assert_eq!(
        report.sessions[0].turns[0].model.as_deref(),
        Some("gpt-fixture-a")
    );
    assert_eq!(report.sessions[0].turns[0].tokens.input_tokens, Some(150));
    assert_eq!(
        report.sessions[0].turns[0].tokens.cached_input_tokens,
        Some(40)
    );
    assert_eq!(report.sessions[0].turns[0].tokens.output_tokens, Some(30));
    assert_eq!(
        report.sessions[0].turns[0].tokens.reasoning_output_tokens,
        Some(6)
    );
    assert_eq!(report.sessions[0].turns[0].tokens.total_tokens, Some(180));
    assert_eq!(report.sessions[0].turns[0].duration_ms, Some(4_000));
    assert_eq!(report.totals.input_tokens, Some(200));
    assert_eq!(report.totals.cached_input_tokens, Some(50));
    assert_eq!(report.totals.cache_write_input_tokens, Some(2));
    assert_eq!(report.totals.output_tokens, Some(50));
    assert_eq!(report.totals.reasoning_output_tokens, Some(11));
    assert_eq!(report.totals.total_tokens, Some(250));

    let serialized = serde_json::to_string(&report).unwrap();
    assert!(serialized.contains("\"input_tokens\":\"200\""));
    assert!(!serialized.contains("\"input_tokens\":200"));
    for forbidden in [
        "SECRET_PROMPT_SHOULD_NOT_LEAK",
        "SECRET_RESPONSE_SHOULD_NOT_LEAK",
        "sensitive-session-id",
        "sensitive-turn-a",
        "private\\\\workspace",
        "repository_url",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }
    assert!(!serialized.contains(&root.display().to_string()));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scan_uses_legacy_cumulative_deltas_without_replaying_last_usage() {
    let root = temp_root();
    fs::write(
        root.join("sessions/2026/09/12/rollout-legacy.jsonl"),
        include_str!("fixtures/codex-history/legacy-metrics.jsonl"),
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan legacy Codex rollout");

    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].turns.len(), 1);
    let tokens = &report.sessions[0].turns[0].tokens;
    assert_eq!(tokens.input_tokens, Some(150));
    assert_eq!(tokens.cached_input_tokens, Some(30));
    assert_eq!(tokens.cache_write_input_tokens, None);
    assert_eq!(tokens.output_tokens, Some(30));
    assert_eq!(tokens.reasoning_output_tokens, Some(3));
    assert_eq!(tokens.total_tokens, Some(180));
    assert_eq!(report.sessions[0].totals.total_tokens, Some(180));
    assert_eq!(report.totals.total_tokens, Some(180));
    assert!(!report.limited);
    assert_eq!(report.sessions[0].duration_ms, Some(3_000));

    let serialized = serde_json::to_string(&report).unwrap();
    for forbidden in [
        "PRIVATE_PROMPT_SENTINEL",
        "PRIVATE_RESPONSE_SENTINEL",
        "PRIVATE_CWD_SENTINEL",
        "legacy-sensitive-session",
        "legacy-sensitive-turn",
    ] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn compressed_archives_are_reported_as_incomplete_and_temporary_files_are_ignored() {
    let root = temp_root();
    fs::create_dir_all(root.join("archived_sessions")).unwrap();
    fs::write(root.join("archived_sessions/rollout.jsonl.zst"), b"fixture").unwrap();
    fs::write(
        root.join("archived_sessions/rollout.jsonl.zst.tmp"),
        b"temporary fixture",
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan unsupported compressed archive");

    assert!(report.limited);
    assert_eq!(report.files_scanned, 0);
    assert_eq!(report.files_skipped, 0);
    assert!(report.sessions.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scan_limits_the_number_of_rollout_files() {
    let root = temp_root();
    for index in 0..51 {
        fs::write(
            root.join(format!("sessions/2026/09/12/rollout-{index:02}.jsonl")),
            format!(
                "{{\"timestamp\":\"2026-09-12T06:00:00Z\",\"type\":\"session_meta\",\"payload\":{{\"id\":\"private-session-{index}\"}}}}\n{{\"timestamp\":\"2026-09-12T06:00:01Z\",\"type\":\"token_usage_record\",\"payload\":{{\"session_id\":\"private-session-{index}\",\"turn_id\":\"private-turn-{index}\",\"turn_token_usage\":{{\"input_tokens\":1,\"cached_input_tokens\":0,\"cache_write_input_tokens\":0,\"output_tokens\":1,\"reasoning_output_tokens\":0,\"total_tokens\":2}},\"thread_token_usage\":{{\"input_tokens\":1,\"cached_input_tokens\":0,\"cache_write_input_tokens\":0,\"output_tokens\":1,\"reasoning_output_tokens\":0,\"total_tokens\":2}}}}}}\n"
            ),
        )
        .unwrap();
    }

    let report = CodexHistory::scan(&root).expect("scan bounded rollout set");

    assert!(report.limited);
    assert_eq!(report.files_scanned, 50);
    assert_eq!(report.sessions.len(), 50);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn untrusted_timestamp_and_model_strings_cannot_escape_the_metrics_projection() {
    let root = temp_root();
    fs::write(
        root.join("sessions/2026/09/12/rollout-untrusted.jsonl"),
        concat!(
            "{\"timestamp\":\"2026-09-12T07:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"private-untrusted-session\"}}\n",
            "{\"timestamp\":\"TOP_SECRET_TIME\",\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"private-untrusted-turn\",\"model\":\"C:/PRIVATE_MODEL_PATH\"}}\n",
            "{\"timestamp\":\"TOP_SECRET_TIME_2\",\"type\":\"token_usage_record\",\"payload\":{\"session_id\":\"private-untrusted-session\",\"turn_id\":\"private-untrusted-turn\",\"turn_token_usage\":{\"input_tokens\":1,\"cached_input_tokens\":0,\"cache_write_input_tokens\":0,\"output_tokens\":1,\"reasoning_output_tokens\":0,\"total_tokens\":2}}}\n"
        ),
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan untrusted metadata");
    let serialized = serde_json::to_string(&report).unwrap();

    assert_eq!(report.sessions[0].turns[0].model, None);
    for forbidden in ["TOP_SECRET_TIME", "PRIVATE_MODEL_PATH"] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_thread_snapshot_is_rejected_instead_of_looking_complete() {
    let root = temp_root();
    fs::write(
        root.join("sessions/2026/09/12/rollout-invalid-thread.jsonl"),
        concat!(
            "{\"timestamp\":\"2026-09-12T08:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"private-invalid-thread-session\"}}\n",
            "{\"timestamp\":\"2026-09-12T08:00:01Z\",\"type\":\"token_usage_record\",\"payload\":{\"session_id\":\"private-invalid-thread-session\",\"turn_id\":\"private-turn\",\"turn_token_usage\":{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2},\"thread_token_usage\":{\"input_tokens\":9,\"output_tokens\":1,\"total_tokens\":99}}}\n"
        ),
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan invalid thread snapshot");

    assert_eq!(report.records_rejected, 1);
    assert!(report.limited);
    assert_eq!(report.sessions[0].totals.total_tokens, Some(2));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn cumulative_snapshot_cannot_erase_a_previously_known_metric() {
    let root = temp_root();
    fs::write(
        root.join("sessions/2026/09/12/rollout-missing-field.jsonl"),
        concat!(
            "{\"timestamp\":\"2026-09-12T09:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"private-missing-field-session\"}}\n",
            "{\"timestamp\":\"2026-09-12T09:00:01Z\",\"type\":\"token_usage_record\",\"payload\":{\"session_id\":\"private-missing-field-session\",\"turn_id\":\"private-turn\",\"turn_token_usage\":{\"input_tokens\":10,\"cached_input_tokens\":5,\"output_tokens\":2,\"reasoning_output_tokens\":1,\"total_tokens\":12}}}\n",
            "{\"timestamp\":\"2026-09-12T09:00:02Z\",\"type\":\"token_usage_record\",\"payload\":{\"session_id\":\"private-missing-field-session\",\"turn_id\":\"private-turn\",\"turn_token_usage\":{\"input_tokens\":20,\"output_tokens\":4,\"total_tokens\":24}}}\n"
        ),
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan missing cumulative field");

    assert_eq!(report.records_rejected, 1);
    assert_eq!(report.sessions[0].turns[0].tokens.total_tokens, Some(12));
    assert_eq!(
        report.sessions[0].turns[0].tokens.cached_input_tokens,
        Some(5)
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scan_prefers_the_more_complete_archive_copy_and_ignores_partial_or_malformed_records() {
    let root = temp_root();
    fs::create_dir_all(root.join("archived_sessions")).unwrap();
    let complete = include_str!("fixtures/codex-history/modern-metrics.jsonl");
    fs::write(
        root.join("archived_sessions/rollout-archived.jsonl"),
        complete,
    )
    .unwrap();
    let partial_active = format!(
        "{}\n{{\"timestamp\":\"2026-09-12T03:00:00Z\",\"type\":\"response_item\",\"payload\":{{\"content\":\"PARTIAL_SECRET",
        complete.lines().take(6).collect::<Vec<_>>().join("\n")
    );
    let active_path = root.join("sessions/2026/09/12/rollout-active.jsonl");
    fs::write(&active_path, &partial_active).unwrap();
    let malformed_path = root.join("sessions/2026/09/12/rollout-malformed.jsonl");
    fs::write(
        &malformed_path,
        concat!(
            "{\"timestamp\":\"2026-09-10T00:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"malformed-private-id\"}}\n",
            "{\"timestamp\":\"2026-09-10T00:00:01Z\",\"type\":\"token_usage_record\",\"payload\":{\"session_id\":\"malformed-private-id\",\"turn_id\":\"bad-turn\",\"turn_token_usage\":{\"input_tokens\":-1}}}\n"
        ),
    )
    .unwrap();
    let active_before = fs::read(&active_path).unwrap();
    let malformed_before = fs::read(&malformed_path).unwrap();

    let report = CodexHistory::scan(&root).expect("scan duplicate and partial rollouts");

    assert_eq!(report.files_scanned, 3);
    assert_eq!(report.sessions.len(), 1);
    assert_eq!(report.sessions[0].turns.len(), 2);
    assert_eq!(report.duplicate_sessions, 1);
    assert_eq!(report.partial_records, 1);
    assert_eq!(report.records_rejected, 1);
    assert_eq!(fs::read(&active_path).unwrap(), active_before);
    assert_eq!(fs::read(&malformed_path).unwrap(), malformed_before);
    let serialized = serde_json::to_string(&report).unwrap();
    for forbidden in ["PARTIAL_SECRET", "malformed-private-id", "bad-turn"] {
        assert!(!serialized.contains(forbidden), "leaked {forbidden}");
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn session_totals_use_the_authoritative_thread_snapshot_not_only_returned_turns() {
    let root = temp_root();
    fs::write(
        root.join("sessions/2026/09/12/rollout-resumed.jsonl"),
        concat!(
            "{\"timestamp\":\"2026-09-12T04:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"resumed-private-session\"}}\n",
            "{\"timestamp\":\"2026-09-12T04:00:01Z\",\"type\":\"turn_context\",\"payload\":{\"turn_id\":\"resumed-private-turn\",\"model\":\"gpt-fixture\"}}\n",
            "{\"timestamp\":\"2026-09-12T04:00:02Z\",\"type\":\"token_usage_record\",\"payload\":{\"session_id\":\"resumed-private-session\",\"turn_id\":\"resumed-private-turn\",\"turn_token_usage\":{\"input_tokens\":50,\"cached_input_tokens\":10,\"cache_write_input_tokens\":0,\"output_tokens\":20,\"reasoning_output_tokens\":5,\"total_tokens\":70},\"thread_token_usage\":{\"input_tokens\":200,\"cached_input_tokens\":50,\"cache_write_input_tokens\":2,\"output_tokens\":50,\"reasoning_output_tokens\":11,\"total_tokens\":250}}}\n"
        ),
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan resumed Codex rollout");

    assert_eq!(report.sessions[0].turns[0].tokens.total_tokens, Some(70));
    assert_eq!(report.sessions[0].totals.total_tokens, Some(250));
    assert_eq!(report.totals.total_tokens, Some(250));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn session_summary_keeps_all_turns_when_returned_details_are_bounded() {
    let root = temp_root();
    let mut rollout = String::from(
        "{\"timestamp\":\"2026-09-12T05:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"bounded-private-session\"}}\n",
    );
    for index in 0..201 {
        rollout.push_str(&format!(
            "{{\"timestamp\":\"2026-09-12T05:00:01Z\",\"type\":\"turn_context\",\"payload\":{{\"turn_id\":\"private-turn-{index}\",\"model\":\"gpt-fixture\"}}}}\n"
        ));
        rollout.push_str(&format!(
            "{{\"timestamp\":\"2026-09-12T05:00:02Z\",\"type\":\"token_usage_record\",\"payload\":{{\"session_id\":\"bounded-private-session\",\"turn_id\":\"private-turn-{index}\",\"turn_token_usage\":{{\"input_tokens\":1,\"cached_input_tokens\":0,\"cache_write_input_tokens\":0,\"output_tokens\":1,\"reasoning_output_tokens\":0,\"total_tokens\":2}}}}}}\n"
        ));
    }
    fs::write(
        root.join("sessions/2026/09/12/rollout-bounded.jsonl"),
        rollout,
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan bounded Codex rollout");

    assert!(report.limited);
    assert_eq!(report.sessions[0].turn_count, 201);
    assert!(report.sessions[0].turn_count_exact);
    assert!(report.sessions[0].turns_truncated);
    assert_eq!(report.sessions[0].turns.len(), 200);
    assert_eq!(report.sessions[0].totals.total_tokens, Some(402));
    assert_eq!(report.totals.total_tokens, Some(402));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn turn_limit_is_reported_as_a_lower_bound_not_an_exact_count() {
    let root = temp_root();
    let mut rollout = String::from(
        "{\"timestamp\":\"2026-09-12T10:00:00Z\",\"type\":\"session_meta\",\"payload\":{\"id\":\"private-overflow-session\"}}\n",
    );
    for index in 0..1001 {
        rollout.push_str(&format!(
            "{{\"timestamp\":\"2026-09-12T10:00:01Z\",\"type\":\"token_usage_record\",\"payload\":{{\"session_id\":\"private-overflow-session\",\"turn_id\":\"private-turn-{index}\",\"turn_token_usage\":{{\"input_tokens\":1,\"output_tokens\":1,\"total_tokens\":2}}}}}}\n"
        ));
    }
    fs::write(
        root.join("sessions/2026/09/12/rollout-turn-overflow.jsonl"),
        rollout,
    )
    .unwrap();

    let report = CodexHistory::scan(&root).expect("scan turn overflow");

    assert!(report.limited);
    assert_eq!(report.sessions[0].turn_count, 1000);
    assert!(!report.sessions[0].turn_count_exact);
    assert!(report.sessions[0].turns_truncated);
    assert_eq!(report.sessions[0].turns.len(), 200);

    fs::remove_dir_all(root).unwrap();
}
