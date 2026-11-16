use agentmeter_p0::{
    codex_history::TokenMetrics,
    provider_history::{ProviderHistoryReport, ProviderSessionMetrics, ProviderTurnMetrics},
    token_analysis::TokenAnalysis,
};

fn metrics(input: u64, cached: u64, write: u64, output: u64) -> TokenMetrics {
    TokenMetrics {
        input_tokens: Some(input),
        cached_input_tokens: Some(cached),
        cache_write_input_tokens: Some(write),
        output_tokens: Some(output),
        reasoning_output_tokens: Some(0),
        total_tokens: Some(input + cached + write + output),
    }
}

fn report(provider: &str, model: &str, tokens: TokenMetrics) -> ProviderHistoryReport {
    ProviderHistoryReport {
        schema_version: "agentmeter.provider-history.v1",
        provider: provider.to_owned(),
        status: "supported",
        data_quality: "local_observed",
        files_scanned: 1,
        files_skipped: 0,
        duplicate_sessions: 0,
        records_rejected: 0,
        partial_records: 0,
        limited: false,
        totals: tokens.clone(),
        sessions: vec![ProviderSessionMetrics {
            session_ref: "session-1".to_owned(),
            started_at: None,
            ended_at: None,
            duration_ms: None,
            turn_count: 1,
            turn_count_exact: true,
            turns_truncated: false,
            totals: tokens.clone(),
            turns: vec![ProviderTurnMetrics {
                turn_ref: "turn-1".to_owned(),
                started_at: None,
                ended_at: None,
                duration_ms: None,
                model: Some(model.to_owned()),
                tokens,
            }],
        }],
    }
}

#[test]
fn codex_flow_is_mutually_exclusive_and_cost_uses_cached_discount() {
    let mut tokens = metrics(1_000, 400, 0, 100);
    tokens.total_tokens = Some(1_100);
    let analysis = TokenAnalysis::analyze(&report("codex", "gpt-5.3-codex", tokens));

    assert_eq!(analysis.schema_version, "agentmeter.token-analysis.v1");
    assert_eq!(analysis.flow.uncached_input_tokens, Some(600));
    assert_eq!(analysis.flow.cached_input_tokens, Some(400));
    assert_eq!(analysis.flow.cache_write_input_tokens, Some(0));
    assert_eq!(analysis.flow.output_tokens, Some(100));
    assert_eq!(analysis.cache_reuse_percent.as_deref(), Some("40.0"));
    assert_eq!(analysis.cost.state, "complete");
    assert_eq!(analysis.cost.estimated_usd_min.as_deref(), Some("0.002520"));
    assert_eq!(analysis.cost.estimated_usd_max.as_deref(), Some("0.002520"));
    assert_eq!(analysis.cost.coverage_percent.as_deref(), Some("100.0"));
}

#[test]
fn claude_cache_creation_with_unknown_ttl_returns_a_cost_range() {
    let analysis = TokenAnalysis::analyze(&report(
        "claude",
        "claude-sonnet-4-6",
        metrics(1_000, 500, 200, 100),
    ));

    assert_eq!(analysis.flow.uncached_input_tokens, Some(1_000));
    assert_eq!(analysis.flow.cached_input_tokens, Some(500));
    assert_eq!(analysis.flow.cache_write_input_tokens, Some(200));
    assert_eq!(analysis.flow.output_tokens, Some(100));
    assert_eq!(analysis.cost.state, "complete");
    assert_eq!(analysis.cost.estimated_usd_min.as_deref(), Some("0.005400"));
    assert_eq!(analysis.cost.estimated_usd_max.as_deref(), Some("0.005850"));
}

#[test]
fn unknown_models_are_unpriced_and_risk_signals_are_not_a_waste_score() {
    let mut source = report(
        "codex",
        "private-future-model",
        metrics(50_000, 1_000, 0, 500),
    );
    source.sessions[0].turn_count = 3;
    let analysis = TokenAnalysis::analyze(&source);

    assert_eq!(analysis.cost.state, "unavailable");
    assert_eq!(analysis.cost.estimated_usd_min, None);
    assert_eq!(analysis.cost.coverage_percent.as_deref(), Some("0.0"));
    assert!(
        analysis
            .signals
            .iter()
            .any(|signal| signal.kind == "low_cache_reuse")
    );
    assert!(
        analysis
            .signals
            .iter()
            .any(|signal| signal.kind == "input_heavy")
    );
    let serialized = serde_json::to_string(&analysis).unwrap();
    assert!(!serialized.contains("waste_score"));
    assert!(!serialized.contains("private-future-model"));
}

#[test]
fn truncated_turn_details_produce_partial_cost_coverage() {
    let mut source = report("codex", "gpt-5.3-codex", metrics(1_000, 0, 0, 100));
    source.totals.input_tokens = Some(10_000);
    source.totals.output_tokens = Some(1_000);
    source.totals.total_tokens = Some(11_000);
    source.sessions[0].totals = source.totals.clone();
    source.sessions[0].turns_truncated = true;

    let analysis = TokenAnalysis::analyze(&source);
    assert_eq!(analysis.cost.state, "partial");
    assert_eq!(analysis.cost.coverage_percent.as_deref(), Some("10.0"));
    assert_eq!(analysis.sessions[0].cost.state, "partial");
}

#[test]
fn missing_cache_counters_remain_unknown_instead_of_becoming_zero() {
    let mut tokens = metrics(10_000, 0, 0, 1_000);
    tokens.cached_input_tokens = None;
    tokens.cache_write_input_tokens = None;
    tokens.total_tokens = Some(11_000);
    let mut source = report("codex", "gpt-5.3-codex", tokens);
    source.sessions[0].turn_count = 3;

    let analysis = TokenAnalysis::analyze(&source);
    assert_eq!(analysis.flow.uncached_input_tokens, None);
    assert_eq!(analysis.flow.cached_input_tokens, None);
    assert_eq!(analysis.cache_reuse_percent, None);
    assert_eq!(analysis.cost.state, "unavailable");
    assert_eq!(analysis.cost.estimated_usd_min, None);
    assert!(
        !analysis
            .signals
            .iter()
            .any(|signal| signal.kind == "low_cache_reuse")
    );
}

#[test]
fn priced_turns_above_observed_total_fail_closed() {
    let tokens = metrics(100, 0, 0, 0);
    let mut source = report("codex", "gpt-5.3-codex", tokens.clone());
    source.sessions[0].turns.push(ProviderTurnMetrics {
        turn_ref: "turn-2".to_owned(),
        started_at: None,
        ended_at: None,
        duration_ms: None,
        model: Some("gpt-5.3-codex".to_owned()),
        tokens,
    });

    let analysis = TokenAnalysis::analyze(&source);
    assert_eq!(analysis.cost.state, "unavailable");
    assert_eq!(analysis.cost.estimated_usd_min, None);
    assert_eq!(analysis.cost.coverage_percent, None);
    assert_eq!(analysis.sessions[0].cost.state, "unavailable");
}
