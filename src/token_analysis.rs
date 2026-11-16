use std::collections::BTreeSet;

use serde::{Serialize, Serializer};

use crate::{
    codex_history::TokenMetrics,
    provider_history::{ProviderHistoryReport, ProviderSessionMetrics},
};

const PRICE_AS_OF: &str = "2026-09-12";

pub struct TokenAnalysis;

impl TokenAnalysis {
    pub fn analyze(history: &ProviderHistoryReport) -> TokenAnalysisReport {
        let flow = normalize_flow(&history.provider, &history.totals);
        let observed = flow.billable_total().map(u128::from);
        let mut priced_tokens = 0u128;
        let mut minimum_nanos = 0u128;
        let mut maximum_nanos = 0u128;
        let mut sources = BTreeSet::new();
        let sessions = history
            .sessions
            .iter()
            .map(|session| {
                let result = price_session(&history.provider, session);
                let session_observed = normalize_flow(&history.provider, &session.totals)
                    .billable_total()
                    .map(u128::from);
                priced_tokens = priced_tokens.saturating_add(result.priced_tokens);
                minimum_nanos = minimum_nanos.saturating_add(result.minimum_nanos);
                maximum_nanos = maximum_nanos.saturating_add(result.maximum_nanos);
                sources.extend(result.sources.iter().copied());
                SessionTokenAnalysis {
                    session_ref: session.session_ref.clone(),
                    total_tokens: session.totals.total_tokens,
                    share_percent: percent(
                        session.totals.total_tokens.map(u128::from),
                        history.totals.total_tokens.map(u128::from),
                    ),
                    cost: cost_estimate(
                        result.priced_tokens,
                        session_observed,
                        result.minimum_nanos,
                        result.maximum_nanos,
                    ),
                }
            })
            .collect();

        let cost = cost_estimate(priced_tokens, observed, minimum_nanos, maximum_nanos);
        let cache_reuse_percent = percent(
            flow.cached_input_tokens.map(u128::from),
            flow.input_total().map(u128::from),
        );
        let output_input_percent = percent(
            flow.output_tokens.map(u128::from),
            flow.input_total().map(u128::from),
        );
        let signals = efficiency_signals(history, &flow, &cost);

        TokenAnalysisReport {
            schema_version: "agentmeter.token-analysis.v1",
            provider: history.provider.clone(),
            pricing_basis: "current_api_equivalent",
            price_as_of: PRICE_AS_OF,
            price_sources: sources.into_iter().collect(),
            flow,
            cache_reuse_percent,
            output_input_percent,
            cost,
            sessions,
            signals,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TokenAnalysisReport {
    pub schema_version: &'static str,
    pub provider: String,
    pub pricing_basis: &'static str,
    pub price_as_of: &'static str,
    pub price_sources: Vec<&'static str>,
    pub flow: TokenFlow,
    pub cache_reuse_percent: Option<String>,
    pub output_input_percent: Option<String>,
    pub cost: CostEstimate,
    pub sessions: Vec<SessionTokenAnalysis>,
    pub signals: Vec<EfficiencySignal>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct TokenFlow {
    #[serde(serialize_with = "serialize_optional_u64")]
    pub uncached_input_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub cached_input_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub cache_write_input_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub output_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub unclassified_tokens: Option<u64>,
}

impl TokenFlow {
    fn input_total(&self) -> Option<u64> {
        checked_sum([
            self.uncached_input_tokens,
            self.cached_input_tokens,
            self.cache_write_input_tokens,
        ])
    }

    fn billable_total(&self) -> Option<u64> {
        checked_sum([
            self.uncached_input_tokens,
            self.cached_input_tokens,
            self.cache_write_input_tokens,
            self.output_tokens,
            self.unclassified_tokens,
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CostEstimate {
    pub state: &'static str,
    pub estimated_usd_min: Option<String>,
    pub estimated_usd_max: Option<String>,
    pub priced_tokens: String,
    pub observed_tokens: Option<String>,
    pub coverage_percent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionTokenAnalysis {
    pub session_ref: String,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub total_tokens: Option<u64>,
    pub share_percent: Option<String>,
    pub cost: CostEstimate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EfficiencySignal {
    pub kind: &'static str,
    pub severity: &'static str,
    pub title: &'static str,
    pub evidence: String,
    pub action: &'static str,
}

#[derive(Clone, Copy)]
struct PriceRate {
    input: u128,
    cached: u128,
    write_min: Option<u128>,
    write_max: Option<u128>,
    output: u128,
    source: &'static str,
    long_context_threshold: Option<u64>,
}

#[derive(Default)]
struct PriceResult {
    priced_tokens: u128,
    minimum_nanos: u128,
    maximum_nanos: u128,
    sources: Vec<&'static str>,
}

fn normalize_flow(provider: &str, tokens: &TokenMetrics) -> TokenFlow {
    let Some(input) = tokens.input_tokens else {
        return TokenFlow::default();
    };
    let output = tokens.output_tokens;
    let (uncached, cached, write, classified) = match provider {
        "codex" => {
            let Some(cached) = tokens.cached_input_tokens else {
                return TokenFlow::default();
            };
            let write = tokens.cache_write_input_tokens.unwrap_or(0);
            let Some(uncached) = input.checked_sub(cached).and_then(|v| v.checked_sub(write))
            else {
                return TokenFlow::default();
            };
            (
                uncached,
                cached,
                write,
                input.checked_add(output.unwrap_or(0)),
            )
        }
        "claude" => {
            let (Some(cached), Some(write)) =
                (tokens.cached_input_tokens, tokens.cache_write_input_tokens)
            else {
                return TokenFlow::default();
            };
            (
                input,
                cached,
                write,
                input
                    .checked_add(cached)
                    .and_then(|value| value.checked_add(write))
                    .and_then(|value| value.checked_add(output.unwrap_or(0))),
            )
        }
        _ => return TokenFlow::default(),
    };
    let unclassified = match (tokens.total_tokens, classified) {
        (Some(total), Some(classified)) => total.checked_sub(classified),
        (None, _) => Some(0),
        _ => None,
    };
    if unclassified.is_none() {
        return TokenFlow::default();
    }
    TokenFlow {
        uncached_input_tokens: Some(uncached),
        cached_input_tokens: Some(cached),
        cache_write_input_tokens: Some(write),
        output_tokens: output,
        unclassified_tokens: unclassified,
    }
}

fn price_session(provider: &str, session: &ProviderSessionMetrics) -> PriceResult {
    let mut result = PriceResult::default();
    for turn in &session.turns {
        let Some(model) = turn.model.as_deref() else {
            continue;
        };
        let Some(rate) = price_rate(provider, model) else {
            continue;
        };
        let flow = normalize_flow(provider, &turn.tokens);
        let input_total = flow.input_total();
        if rate
            .long_context_threshold
            .is_some_and(|threshold| input_total.is_some_and(|value| value > threshold))
        {
            continue;
        }
        let mut priced_any = false;
        for (tokens, min_rate, max_rate) in [
            (
                flow.uncached_input_tokens,
                Some(rate.input),
                Some(rate.input),
            ),
            (
                flow.cached_input_tokens,
                Some(rate.cached),
                Some(rate.cached),
            ),
            (
                flow.cache_write_input_tokens,
                rate.write_min,
                rate.write_max,
            ),
            (flow.output_tokens, Some(rate.output), Some(rate.output)),
        ] {
            let (Some(tokens), Some(min_rate), Some(max_rate)) = (tokens, min_rate, max_rate)
            else {
                continue;
            };
            let tokens = u128::from(tokens);
            result.priced_tokens = result.priced_tokens.saturating_add(tokens);
            result.minimum_nanos = result
                .minimum_nanos
                .saturating_add(tokens.saturating_mul(min_rate));
            result.maximum_nanos = result
                .maximum_nanos
                .saturating_add(tokens.saturating_mul(max_rate));
            priced_any = true;
        }
        if priced_any && !result.sources.contains(&rate.source) {
            result.sources.push(rate.source);
        }
    }
    result
}

fn price_rate(provider: &str, model: &str) -> Option<PriceRate> {
    const OPENAI_53: &str = "https://developers.openai.com/api/docs/models/gpt-5.3-codex";
    const OPENAI_52: &str = "https://developers.openai.com/api/docs/models/gpt-5.2-codex";
    const OPENAI_5: &str = "https://developers.openai.com/api/docs/models/gpt-5-codex";
    const OPENAI_56_SOL: &str = "https://developers.openai.com/api/docs/models/gpt-5.6-sol";
    const OPENAI_56_TERRA: &str = "https://developers.openai.com/api/docs/models/gpt-5.6-terra";
    const OPENAI_56_LUNA: &str = "https://developers.openai.com/api/docs/models/gpt-5.6-luna";
    const OPENAI_6: &str = "https://developers.openai.com/api/docs/models/gpt-6-astra";
    const CLAUDE: &str = "https://platform.claude.com/docs/en/about-claude/pricing";
    let exact = |input, cached, write_min, write_max, output, source, threshold| PriceRate {
        input,
        cached,
        write_min,
        write_max,
        output,
        source,
        long_context_threshold: threshold,
    };
    match (provider, model) {
        ("codex", "gpt-5.3-codex") => Some(exact(1_750, 175, None, None, 14_000, OPENAI_53, None)),
        ("codex", "gpt-5.2-codex") => Some(exact(1_750, 175, None, None, 14_000, OPENAI_52, None)),
        ("codex", "gpt-5-codex") => Some(exact(1_250, 125, None, None, 10_000, OPENAI_5, None)),
        ("codex", "gpt-5.6-sol" | "gpt-5.6") => Some(exact(
            4_000,
            400,
            Some(5_000),
            Some(5_000),
            20_000,
            OPENAI_56_SOL,
            Some(272_000),
        )),
        ("codex", "gpt-5.6-terra") => Some(exact(
            2_000,
            200,
            Some(2_500),
            Some(2_500),
            12_000,
            OPENAI_56_TERRA,
            Some(272_000),
        )),
        ("codex", "gpt-5.6-luna") => Some(exact(
            200,
            20,
            Some(250),
            Some(250),
            1_200,
            OPENAI_56_LUNA,
            Some(272_000),
        )),
        ("codex", "gpt-6-astra") => Some(exact(
            10_000,
            1_000,
            Some(12_500),
            Some(12_500),
            50_000,
            OPENAI_6,
            Some(272_000),
        )),
        ("claude", "claude-sonnet-4-6" | "claude-sonnet-4-5" | "claude-sonnet-4-5-20250929") => {
            Some(exact(
                3_000,
                300,
                Some(3_750),
                Some(6_000),
                15_000,
                CLAUDE,
                None,
            ))
        }
        ("claude", "claude-sonnet-5") => Some(exact(
            2_000,
            200,
            Some(2_500),
            Some(4_000),
            10_000,
            CLAUDE,
            None,
        )),
        ("claude", "claude-haiku-4-5" | "claude-haiku-4-5-20251001") => Some(exact(
            1_000,
            100,
            Some(1_250),
            Some(2_000),
            5_000,
            CLAUDE,
            None,
        )),
        (
            "claude",
            "claude-opus-5"
            | "claude-opus-4-8"
            | "claude-opus-4-7"
            | "claude-opus-4-6"
            | "claude-opus-4-5"
            | "claude-opus-4-5-20251101",
        ) => Some(exact(
            5_000,
            500,
            Some(6_250),
            Some(10_000),
            25_000,
            CLAUDE,
            None,
        )),
        ("claude", "claude-fable-5-1") => Some(exact(
            10_000,
            250,
            Some(12_500),
            Some(20_000),
            50_000,
            CLAUDE,
            None,
        )),
        _ => None,
    }
}

fn efficiency_signals(
    history: &ProviderHistoryReport,
    flow: &TokenFlow,
    cost: &CostEstimate,
) -> Vec<EfficiencySignal> {
    let mut signals = Vec::new();
    let turns = history
        .sessions
        .iter()
        .map(|session| session.turn_count)
        .sum::<usize>();
    let input = flow.input_total();
    let cache = flow.cached_input_tokens;
    if !history.limited
        && turns >= 3
        && input.is_some_and(|value| value >= 10_000)
        && ratio_less_than(cache, input, 20)
    {
        signals.push(EfficiencySignal {
            kind: "low_cache_reuse",
            severity: "review",
            title: "快取重用偏低",
            evidence: format!(
                "目前約 {}% 的輸入來自快取；這是效率線索，不等於已證明浪費。",
                percent(cache.map(u128::from), input.map(u128::from))
                    .unwrap_or_else(|| "—".to_owned())
            ),
            action: "可檢查是否每回合重送相同背景、完整規格或大型檔案。",
        });
    }
    if !history.limited
        && turns >= 3
        && input.is_some_and(|value| value >= 20_000)
        && ratio_at_most(flow.output_tokens, input, 5)
    {
        signals.push(EfficiencySignal {
            kind: "input_heavy",
            severity: "review",
            title: "輸入很多、輸出較少",
            evidence: format!(
                "輸出約為輸入的 {}%；程式碼分析本來也可能是輸入密集。",
                percent(flow.output_tokens.map(u128::from), input.map(u128::from))
                    .unwrap_or_else(|| "—".to_owned())
            ),
            action: "可檢查是否能縮小檔案範圍、先摘要背景，或移除已不相關的 context。",
        });
    }
    if history.limited
        || history.files_skipped > 0
        || history.records_rejected > 0
        || history.partial_records > 0
    {
        signals.push(EfficiencySignal {
            kind: "incomplete_data",
            severity: "info",
            title: "資料不完整",
            evidence: "部分 Session、回合或紀錄未完整納入，圖表與判斷可能偏低。".to_owned(),
            action: "先重新讀取；若仍存在，查看來源格式或掃描上限狀態。",
        });
    }
    if cost.state == "partial" || cost.state == "unavailable" {
        signals.push(EfficiencySignal {
            kind: "pricing_incomplete",
            severity: "info",
            title: "成本只能部分估算",
            evidence: format!(
                "目前 API 費率可覆蓋 {}% 的已觀察 Token。",
                cost.coverage_percent.as_deref().unwrap_or("—")
            ),
            action: "未辨識模型、長 context 或缺少 Token 分類不會套用猜測價格。",
        });
    }
    if signals.is_empty() && flow.billable_total().is_some_and(|value| value > 0) {
        signals.push(EfficiencySignal {
            kind: "no_obvious_signal",
            severity: "ok",
            title: "目前沒有明顯效率警訊",
            evidence: "現有 Token counter 未觸發保守門檻；這不代表所有 context 都必要。".to_owned(),
            action: "可從 Token 最高的工作階段開始檢查 Prompt 與 context。",
        });
    }
    signals
}

fn cost_estimate(
    priced_tokens: u128,
    observed_tokens: Option<u128>,
    minimum_nanos: u128,
    maximum_nanos: u128,
) -> CostEstimate {
    let anomalous = observed_tokens.is_some_and(|observed| priced_tokens > observed);
    let state = match (priced_tokens, observed_tokens) {
        (_, Some(0)) => "no_usage",
        (0, _) => "unavailable",
        (priced, Some(observed)) if priced == observed => "complete",
        _ => "partial",
    };
    CostEstimate {
        state: if anomalous { "unavailable" } else { state },
        estimated_usd_min: (priced_tokens > 0 && !anomalous).then(|| format_usd(minimum_nanos)),
        estimated_usd_max: (priced_tokens > 0 && !anomalous).then(|| format_usd(maximum_nanos)),
        priced_tokens: priced_tokens.to_string(),
        observed_tokens: observed_tokens.map(|value| value.to_string()),
        coverage_percent: (!anomalous)
            .then(|| percent(Some(priced_tokens), observed_tokens))
            .flatten(),
    }
}

fn format_usd(nanodollars: u128) -> String {
    let microdollars = nanodollars.saturating_add(500) / 1_000;
    format!(
        "{}.{:06}",
        microdollars / 1_000_000,
        microdollars % 1_000_000
    )
}

fn percent(numerator: Option<u128>, denominator: Option<u128>) -> Option<String> {
    let (Some(numerator), Some(denominator)) = (numerator, denominator) else {
        return None;
    };
    if denominator == 0 {
        return None;
    }
    let tenths = numerator
        .saturating_mul(1_000)
        .saturating_add(denominator / 2)
        / denominator;
    Some(format!("{}.{:01}", tenths / 10, tenths % 10))
}

fn ratio_less_than(numerator: Option<u64>, denominator: Option<u64>, percent: u64) -> bool {
    matches!((numerator, denominator), (Some(n), Some(d)) if d > 0 && u128::from(n) * 100 < u128::from(d) * u128::from(percent))
}

fn ratio_at_most(numerator: Option<u64>, denominator: Option<u64>, percent: u64) -> bool {
    matches!((numerator, denominator), (Some(n), Some(d)) if d > 0 && u128::from(n) * 100 <= u128::from(d) * u128::from(percent))
}

fn checked_sum<const N: usize>(values: [Option<u64>; N]) -> Option<u64> {
    values
        .into_iter()
        .try_fold(0u64, |sum, value| sum.checked_add(value?))
}

fn serialize_optional_u64<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_some(&value.to_string()),
        None => serializer.serialize_none(),
    }
}
