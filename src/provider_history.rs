//! Provider-neutral, read-only projection of local conversation history.
//!
//! Summary reports never contain prompt text, source paths, or provider IDs.
//! Prompt content is exposed only by the separate, bounded content interface and
//! is re-scanned against the public session ordinal and timestamps to prevent a
//! stale UI selection from revealing a different session.

use crate::codex_history::{CodexHistory, CodexHistoryReport, TokenMetrics};
use serde::Serialize;
use serde_json::Value;
use std::{
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_FILES: usize = 50;
const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 6;
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TURNS: usize = 1_000;
const MAX_RETURNED_TURNS: usize = 200;
const MAX_PROMPT_CHARS: usize = 4_000;

pub struct ProviderHistory;

impl ProviderHistory {
    pub fn scan(
        provider: &str,
        provider_home: &Path,
    ) -> Result<ProviderHistoryReport, ProviderHistoryError> {
        ensure_provider(provider)?;
        if !provider_home.is_dir() {
            return Ok(ProviderHistoryReport::unavailable(provider));
        }
        if provider == "antigravity" {
            return Ok(ProviderHistoryReport::schema_unverified(provider));
        }
        if provider == "codex" {
            return Ok(match CodexHistory::scan(provider_home) {
                Ok(report) => report.into(),
                Err(_) => ProviderHistoryReport::unavailable(provider),
            });
        }

        let (relative, recursive) = match provider {
            "claude" => ("projects", true),
            "copilot" => ("session-state", false),
            _ => unreachable!(),
        };
        let source = provider_home.join(relative);
        if !source.is_dir() {
            return Ok(ProviderHistoryReport::unavailable(provider));
        }
        let scan = scan_jsonl_sessions(provider, provider_home, &source, recursive)?;
        Ok(build_report(provider, scan))
    }

    pub fn read_session_content(
        provider: &str,
        provider_home: &Path,
        session_ref: &str,
        expected_started_at: Option<&str>,
        expected_ended_at: Option<&str>,
    ) -> Result<ProviderSessionContent, ProviderHistoryError> {
        ensure_provider(provider)?;
        if provider == "antigravity" {
            return Err(if provider_home.is_dir() {
                ProviderHistoryError::SchemaUnverified
            } else {
                ProviderHistoryError::SourceUnavailable
            });
        }
        let report = Self::scan(provider, provider_home)?;
        if report.status != "supported" {
            return Err(ProviderHistoryError::SourceUnavailable);
        }
        let session = report
            .sessions
            .iter()
            .find(|session| session.session_ref == session_ref)
            .ok_or(ProviderHistoryError::SessionNotFound)?;
        if session.started_at.as_deref() != expected_started_at
            || session.ended_at.as_deref() != expected_ended_at
        {
            return Err(ProviderHistoryError::SessionStale);
        }

        parse_session_ordinal(session_ref)?;
        let mut parsed = match provider {
            "claude" => {
                scan_jsonl_sessions(
                    provider,
                    provider_home,
                    &provider_home.join("projects"),
                    true,
                )?
                .sessions
            }
            "copilot" => {
                scan_jsonl_sessions(
                    provider,
                    provider_home,
                    &provider_home.join("session-state"),
                    false,
                )?
                .sessions
            }
            "codex" => scan_codex_content(provider_home)?,
            _ => unreachable!(),
        };
        sort_sessions(&mut parsed);

        let mut matches = parsed.into_iter().filter(|candidate| {
            candidate.started_at == session.started_at && candidate.ended_at == session.ended_at
        });
        let parsed = matches
            .next()
            .ok_or(ProviderHistoryError::ContentUnavailable)?;
        if matches.next().is_some() {
            return Err(ProviderHistoryError::ContentUnavailable);
        }

        if parsed.started_at != session.started_at || parsed.ended_at != session.ended_at {
            return Err(ProviderHistoryError::SessionStale);
        }
        let turns_truncated = parsed.turns.len() > MAX_RETURNED_TURNS || parsed.turns_limited;
        let start = parsed.turns.len().saturating_sub(MAX_RETURNED_TURNS);
        let prompts: Vec<ProviderPrompt> = parsed
            .turns
            .into_iter()
            .skip(start)
            .enumerate()
            .map(|(index, turn)| {
                let (text, redacted) = redact_prompt(&turn.prompt);
                ProviderPrompt {
                    turn_ref: format!("turn-{}", index + 1),
                    text,
                    redacted,
                    truncated: turn.prompt.chars().count() > MAX_PROMPT_CHARS,
                }
            })
            .collect();
        let duplicate_prompt_count = count_duplicate_prompts(&prompts);
        Ok(ProviderSessionContent {
            schema_version: "agentmeter.provider-session-content.v1",
            provider: provider.to_owned(),
            session_ref: session_ref.to_owned(),
            prompts,
            limited: turns_truncated,
            duplicate_prompt_count,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderHistoryReport {
    pub schema_version: &'static str,
    pub provider: String,
    pub status: &'static str,
    pub data_quality: &'static str,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub duplicate_sessions: usize,
    pub records_rejected: usize,
    pub partial_records: usize,
    pub limited: bool,
    pub totals: TokenMetrics,
    pub sessions: Vec<ProviderSessionMetrics>,
}

impl ProviderHistoryReport {
    fn unavailable(provider: &str) -> Self {
        Self::empty(provider, "source_unavailable")
    }

    fn schema_unverified(provider: &str) -> Self {
        Self::empty(provider, "schema_unverified")
    }

    fn empty(provider: &str, status: &'static str) -> Self {
        Self {
            schema_version: "agentmeter.provider-history.v1",
            provider: provider.to_owned(),
            status,
            data_quality: "local_observed",
            files_scanned: 0,
            files_skipped: 0,
            duplicate_sessions: 0,
            records_rejected: 0,
            partial_records: 0,
            limited: false,
            totals: TokenMetrics::default(),
            sessions: Vec::new(),
        }
    }
}

impl From<CodexHistoryReport> for ProviderHistoryReport {
    fn from(report: CodexHistoryReport) -> Self {
        Self {
            schema_version: "agentmeter.provider-history.v1",
            provider: "codex".to_owned(),
            status: "supported",
            data_quality: report.data_quality,
            files_scanned: report.files_scanned,
            files_skipped: report.files_skipped,
            duplicate_sessions: report.duplicate_sessions,
            records_rejected: report.records_rejected,
            partial_records: report.partial_records,
            limited: report.limited,
            totals: report.totals,
            sessions: report
                .sessions
                .into_iter()
                .map(|session| ProviderSessionMetrics {
                    session_ref: session.session_ref,
                    started_at: session.started_at,
                    ended_at: session.ended_at,
                    duration_ms: session.duration_ms,
                    turn_count: session.turn_count,
                    turn_count_exact: session.turn_count_exact,
                    turns_truncated: session.turns_truncated,
                    totals: session.totals,
                    turns: session
                        .turns
                        .into_iter()
                        .map(|turn| ProviderTurnMetrics {
                            turn_ref: turn.turn_ref,
                            started_at: turn.started_at,
                            ended_at: turn.ended_at,
                            duration_ms: turn.duration_ms,
                            model: turn.model,
                            tokens: turn.tokens,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderSessionMetrics {
    pub session_ref: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub turn_count: usize,
    pub turn_count_exact: bool,
    pub turns_truncated: bool,
    pub totals: TokenMetrics,
    pub turns: Vec<ProviderTurnMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderTurnMetrics {
    pub turn_ref: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub model: Option<String>,
    pub tokens: TokenMetrics,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderSessionContent {
    pub schema_version: &'static str,
    pub provider: String,
    pub session_ref: String,
    pub prompts: Vec<ProviderPrompt>,
    pub limited: bool,
    pub duplicate_prompt_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderPrompt {
    pub turn_ref: String,
    pub text: String,
    pub redacted: bool,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderHistoryError {
    UnsupportedProvider,
    SourceUnavailable,
    SchemaUnverified,
    SessionNotFound,
    SessionStale,
    ContentUnavailable,
}

impl fmt::Display for ProviderHistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedProvider => "history_unsupported_provider",
            Self::SourceUnavailable => "history_source_unavailable",
            Self::SchemaUnverified => "history_schema_unverified",
            Self::SessionNotFound => "history_session_not_found",
            Self::SessionStale => "history_session_stale",
            Self::ContentUnavailable => "history_content_unavailable",
        })
    }
}

impl std::error::Error for ProviderHistoryError {}

fn ensure_provider(provider: &str) -> Result<(), ProviderHistoryError> {
    match provider {
        "codex" | "claude" | "copilot" | "antigravity" => Ok(()),
        _ => Err(ProviderHistoryError::UnsupportedProvider),
    }
}

#[derive(Default)]
struct ParsedSession {
    path: PathBuf,
    modified: Option<SystemTime>,
    started_at: Option<String>,
    ended_at: Option<String>,
    turns: Vec<ParsedTurn>,
    turns_limited: bool,
    records_rejected: usize,
    partial_records: usize,
}

#[derive(Default)]
struct ParsedTurn {
    started_at: Option<String>,
    ended_at: Option<String>,
    model: Option<String>,
    prompt: String,
    tokens: TokenMetrics,
}

struct ParsedScan {
    sessions: Vec<ParsedSession>,
    files_scanned: usize,
    files_skipped: usize,
    records_rejected: usize,
    partial_records: usize,
    limited: bool,
}

fn scan_jsonl_sessions(
    provider: &str,
    home: &Path,
    source: &Path,
    recursive: bool,
) -> Result<ParsedScan, ProviderHistoryError> {
    let home = home
        .canonicalize()
        .map_err(|_| ProviderHistoryError::SourceUnavailable)?;
    let mut candidates = Vec::new();
    let mut entries = 0usize;
    let mut limited = false;
    collect_jsonl(
        &home,
        source,
        0,
        recursive,
        &mut entries,
        &mut candidates,
        &mut limited,
    );
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.0.cmp(&left.0)));
    if candidates.len() > MAX_FILES {
        candidates.truncate(MAX_FILES);
        limited = true;
    }
    let mut total_bytes = 0u64;
    let mut sessions = Vec::new();
    let mut files_scanned = 0usize;
    let mut files_skipped = 0usize;
    let mut records_rejected = 0usize;
    let mut partial_records = 0usize;
    for (path, modified, size) in candidates {
        if total_bytes >= MAX_TOTAL_BYTES {
            limited = true;
            break;
        }
        let byte_limit = size.min(MAX_FILE_BYTES).min(MAX_TOTAL_BYTES - total_bytes);
        total_bytes += byte_limit;
        if let Ok(mut session) = parse_provider_file(provider, &home, &path, byte_limit) {
            files_scanned += 1;
            session.modified = Some(modified);
            limited |= session.turns_limited || size > byte_limit;
            records_rejected += session.records_rejected;
            partial_records += session.partial_records;
            if !session.turns.is_empty() {
                sessions.push(session);
            }
        } else {
            files_skipped += 1;
            limited = true;
        }
    }
    sort_sessions(&mut sessions);
    Ok(ParsedScan {
        sessions,
        files_scanned,
        files_skipped,
        records_rejected,
        partial_records,
        limited,
    })
}

fn collect_jsonl(
    home: &Path,
    directory: &Path,
    depth: usize,
    recursive: bool,
    entries_seen: &mut usize,
    output: &mut Vec<(PathBuf, SystemTime, u64)>,
    limited: &mut bool,
) {
    if depth > MAX_DEPTH || *entries_seen >= MAX_ENTRIES {
        *limited = true;
        return;
    }
    let Ok(metadata) = fs::symlink_metadata(directory) else {
        return;
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        *limited = true;
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        *limited = true;
        return;
    };
    for entry in entries {
        let Ok(entry) = entry else {
            *limited = true;
            continue;
        };
        *entries_seen += 1;
        if *entries_seen > MAX_ENTRIES {
            *limited = true;
            break;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() && (recursive || depth == 0) {
            collect_jsonl(
                home,
                &path,
                depth + 1,
                recursive,
                entries_seen,
                output,
                limited,
            );
        } else if file_type.is_file()
            && path.extension().and_then(|value| value.to_str()) == Some("jsonl")
            && path
                .canonicalize()
                .is_ok_and(|value| value.starts_with(home))
            && let Ok(metadata) = entry.metadata()
        {
            output.push((
                path,
                metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                metadata.len(),
            ));
        }
    }
}

fn parse_provider_file(
    provider: &str,
    home: &Path,
    path: &Path,
    byte_limit: u64,
) -> Result<ParsedSession, ()> {
    let canonical = checked_file(home, path)?;
    let file = File::open(canonical).map_err(|_| ())?;
    let mut reader = BufReader::new(file).take(byte_limit.saturating_add(1));
    let mut session = ParsedSession {
        path: path.to_owned(),
        ..Default::default()
    };
    let mut bytes_read = 0u64;
    let mut line = String::new();
    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|_| ())?;
        if bytes == 0 {
            break;
        }
        bytes_read += bytes as u64;
        if bytes_read > byte_limit {
            session.turns_limited = true;
            break;
        }
        if !line.ends_with('\n') {
            session.partial_records += 1;
            session.turns_limited = true;
            break;
        }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            session.records_rejected += 1;
            session.turns_limited = true;
            continue;
        };
        match provider {
            "claude" => consume_claude(&mut session, &value),
            "copilot" => consume_copilot(&mut session, &value),
            _ => unreachable!(),
        }
    }
    Ok(session)
}

fn consume_claude(session: &mut ParsedSession, value: &Value) {
    let timestamp = safe_timestamp(value.get("timestamp").and_then(Value::as_str));
    match value.get("type").and_then(Value::as_str) {
        Some("user")
            if value.get("userType").and_then(Value::as_str) == Some("external")
                && value.pointer("/message/role").and_then(Value::as_str) == Some("user") =>
        {
            if session.turns.len() >= MAX_TURNS {
                session.turns_limited = true;
                return;
            }
            let Some(prompt) = extract_message_text(value.pointer("/message/content")) else {
                return;
            };
            session.started_at = session.started_at.take().or_else(|| timestamp.clone());
            session.ended_at = timestamp.clone().or_else(|| session.ended_at.take());
            session.turns.push(ParsedTurn {
                started_at: timestamp,
                prompt,
                ..Default::default()
            });
        }
        Some("assistant")
            if value.pointer("/message/role").and_then(Value::as_str) == Some("assistant") =>
        {
            let Some(turn) = session.turns.last_mut() else {
                return;
            };
            turn.ended_at = timestamp.clone().or_else(|| turn.ended_at.take());
            turn.model = safe_model(value.pointer("/message/model").and_then(Value::as_str));
            if let Some(usage) = value.pointer("/message/usage") {
                let input = usage.get("input_tokens").and_then(Value::as_u64);
                let output = usage.get("output_tokens").and_then(Value::as_u64);
                turn.tokens = TokenMetrics {
                    input_tokens: input,
                    cached_input_tokens: usage
                        .get("cache_read_input_tokens")
                        .and_then(Value::as_u64),
                    cache_write_input_tokens: usage
                        .get("cache_creation_input_tokens")
                        .and_then(Value::as_u64),
                    output_tokens: output,
                    reasoning_output_tokens: None,
                    total_tokens: input.zip(output).and_then(|(input, output)| {
                        input
                            .checked_add(
                                usage
                                    .get("cache_read_input_tokens")
                                    .and_then(Value::as_u64)
                                    .unwrap_or(0),
                            )?
                            .checked_add(
                                usage
                                    .get("cache_creation_input_tokens")
                                    .and_then(Value::as_u64)
                                    .unwrap_or(0),
                            )?
                            .checked_add(output)
                    }),
                };
            }
            session.ended_at = timestamp.or_else(|| session.ended_at.take());
        }
        _ => {}
    }
}

fn consume_copilot(session: &mut ParsedSession, value: &Value) {
    let timestamp = safe_timestamp(value.get("timestamp").and_then(Value::as_str));
    if value.get("type").and_then(Value::as_str) != Some("user.message") {
        return;
    }
    if session.turns.len() >= MAX_TURNS {
        session.turns_limited = true;
        return;
    }
    let Some(prompt) = value.pointer("/data/content").and_then(Value::as_str) else {
        return;
    };
    session.started_at = session.started_at.take().or_else(|| timestamp.clone());
    session.ended_at = timestamp.clone().or_else(|| session.ended_at.take());
    session.turns.push(ParsedTurn {
        started_at: timestamp.clone(),
        ended_at: timestamp,
        prompt: prompt.to_owned(),
        ..Default::default()
    });
}

fn extract_message_text(content: Option<&Value>) -> Option<String> {
    match content? {
        Value::String(text) if !text.is_empty() => Some(text.clone()),
        Value::Array(items) => {
            let text = items
                .iter()
                .filter(|item| {
                    matches!(
                        item.get("type").and_then(Value::as_str),
                        Some("text" | "input_text")
                    )
                })
                .filter_map(|item| item.get("text").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("\n");
            (!text.is_empty()).then_some(text)
        }
        _ => None,
    }
}

fn build_report(provider: &str, mut scan: ParsedScan) -> ProviderHistoryReport {
    let parsed_sessions = std::mem::take(&mut scan.sessions);
    let sessions = parsed_sessions
        .into_iter()
        .enumerate()
        .map(|(session_index, mut session)| {
            let turn_count = session.turns.len();
            let turns_truncated = turn_count > MAX_RETURNED_TURNS || session.turns_limited;
            scan.limited |= turns_truncated;
            let totals = sum_metrics(session.turns.iter().map(|turn| &turn.tokens));
            let start = turn_count.saturating_sub(MAX_RETURNED_TURNS);
            let turns = session
                .turns
                .drain(start..)
                .enumerate()
                .map(|(turn_index, turn)| ProviderTurnMetrics {
                    turn_ref: format!("turn-{}", turn_index + 1),
                    started_at: turn.started_at,
                    ended_at: turn.ended_at,
                    duration_ms: None,
                    model: turn.model,
                    tokens: turn.tokens,
                })
                .collect();
            ProviderSessionMetrics {
                session_ref: format!("session-{}", session_index + 1),
                started_at: session.started_at,
                ended_at: session.ended_at,
                duration_ms: None,
                turn_count,
                turn_count_exact: !session.turns_limited,
                turns_truncated,
                totals,
                turns,
            }
        })
        .collect::<Vec<_>>();
    let totals = sum_metrics(sessions.iter().map(|session| &session.totals));
    ProviderHistoryReport {
        schema_version: "agentmeter.provider-history.v1",
        provider: provider.to_owned(),
        status: if scan.files_scanned == 0 && (scan.files_skipped > 0 || scan.limited) {
            "schema_unverified"
        } else {
            "supported"
        },
        data_quality: "local_observed",
        files_scanned: scan.files_scanned,
        files_skipped: scan.files_skipped,
        duplicate_sessions: 0,
        records_rejected: scan.records_rejected,
        partial_records: scan.partial_records,
        limited: scan.limited,
        totals,
        sessions,
    }
}

fn sum_metrics<'a>(items: impl Iterator<Item = &'a TokenMetrics>) -> TokenMetrics {
    items.cloned().reduce(add_metrics).unwrap_or_default()
}

fn add_metrics(left: TokenMetrics, right: TokenMetrics) -> TokenMetrics {
    TokenMetrics {
        input_tokens: add_optional(left.input_tokens, right.input_tokens),
        cached_input_tokens: add_optional(left.cached_input_tokens, right.cached_input_tokens),
        cache_write_input_tokens: add_optional(
            left.cache_write_input_tokens,
            right.cache_write_input_tokens,
        ),
        output_tokens: add_optional(left.output_tokens, right.output_tokens),
        reasoning_output_tokens: add_optional(
            left.reasoning_output_tokens,
            right.reasoning_output_tokens,
        ),
        total_tokens: add_optional(left.total_tokens, right.total_tokens),
    }
}

fn add_optional(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    left?.checked_add(right?)
}

fn sort_sessions(sessions: &mut [ParsedSession]) {
    sessions.sort_by(|left, right| {
        right
            .ended_at
            .cmp(&left.ended_at)
            .then_with(|| right.started_at.cmp(&left.started_at))
            .then_with(|| right.modified.cmp(&left.modified))
            .then_with(|| right.path.cmp(&left.path))
    });
}

fn scan_codex_content(home: &Path) -> Result<Vec<ParsedSession>, ProviderHistoryError> {
    let canonical = home
        .canonicalize()
        .map_err(|_| ProviderHistoryError::SourceUnavailable)?;
    let mut candidates = Vec::new();
    let mut entries = 0;
    let mut limited = false;
    for relative in ["sessions", "archived_sessions"] {
        collect_jsonl(
            &canonical,
            &canonical.join(relative),
            0,
            true,
            &mut entries,
            &mut candidates,
            &mut limited,
        );
    }
    candidates.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.0.cmp(&left.0)));
    candidates.truncate(MAX_FILES);
    let mut sessions = Vec::new();
    let mut total_bytes = 0u64;
    for (path, modified, size) in candidates {
        if total_bytes >= MAX_TOTAL_BYTES {
            break;
        }
        let byte_limit = size.min(MAX_FILE_BYTES).min(MAX_TOTAL_BYTES - total_bytes);
        total_bytes += byte_limit;
        let canonical_path = checked_file(&canonical, &path)
            .map_err(|_| ProviderHistoryError::ContentUnavailable)?;
        let file =
            File::open(canonical_path).map_err(|_| ProviderHistoryError::ContentUnavailable)?;
        let mut reader = BufReader::new(file).take(byte_limit.saturating_add(1));
        let mut session = ParsedSession {
            path,
            modified: Some(modified),
            ..Default::default()
        };
        let mut line = String::new();
        let mut bytes_read = 0u64;
        loop {
            line.clear();
            let bytes = reader
                .read_line(&mut line)
                .map_err(|_| ProviderHistoryError::ContentUnavailable)?;
            if bytes == 0 {
                break;
            }
            bytes_read += bytes as u64;
            if bytes_read > byte_limit || !line.ends_with('\n') {
                break;
            }
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let timestamp = safe_timestamp(value.get("timestamp").and_then(Value::as_str));
            if session.started_at.is_none() {
                session.started_at = timestamp.clone();
            }
            if timestamp.is_some() {
                session.ended_at = timestamp.clone();
            }
            if value.get("type").and_then(Value::as_str) == Some("response_item")
                && value.pointer("/payload/type").and_then(Value::as_str) == Some("message")
                && value.pointer("/payload/role").and_then(Value::as_str) == Some("user")
                && session.turns.len() < MAX_TURNS
                && let Some(prompt) = extract_message_text(value.pointer("/payload/content"))
            {
                session.turns.push(ParsedTurn {
                    started_at: timestamp,
                    prompt,
                    ..Default::default()
                });
            }
        }
        if !session.turns.is_empty() {
            sessions.push(session);
        }
    }
    Ok(sessions)
}

fn checked_file(home: &Path, path: &Path) -> Result<PathBuf, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    let canonical = path.canonicalize().map_err(|_| ())?;
    canonical.starts_with(home).then_some(canonical).ok_or(())
}

fn parse_session_ordinal(session_ref: &str) -> Result<usize, ProviderHistoryError> {
    session_ref
        .strip_prefix("session-")
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or(ProviderHistoryError::SessionNotFound)
}

fn safe_timestamp(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| is_rfc3339_shape(value))
        .map(str::to_owned)
}

fn is_rfc3339_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    (20..=40).contains(&bytes.len())
        && bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && bytes.get(10) == Some(&b'T')
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':')
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && bytes[11..13].iter().all(u8::is_ascii_digit)
        && bytes[14..16].iter().all(u8::is_ascii_digit)
        && bytes[17..19].iter().all(u8::is_ascii_digit)
        && (bytes.ends_with(b"Z") || value[19..].contains('+') || value[19..].contains('-'))
}

fn safe_model(value: Option<&str>) -> Option<String> {
    value
        .filter(|value| {
            !value.is_empty()
                && value.len() <= 120
                && value
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
        })
        .map(str::to_owned)
}

fn count_duplicate_prompts(prompts: &[ProviderPrompt]) -> usize {
    let mut seen = std::collections::BTreeSet::new();
    prompts
        .iter()
        .filter(|prompt| !prompt.redacted && !prompt.truncated)
        .filter_map(|prompt| {
            let normalized = prompt.text.split_whitespace().collect::<Vec<_>>().join(" ");
            (normalized.chars().count() >= 20).then_some(normalized)
        })
        .filter(|normalized| !seen.insert(normalized.clone()))
        .count()
}

fn redact_prompt(value: &str) -> (String, bool) {
    let clean = value
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\t'))
        .take(MAX_PROMPT_CHARS)
        .collect::<String>();
    let mut without_private_keys = Vec::new();
    let mut inside_private_key = false;
    for line in clean.lines() {
        if line.contains("-----BEGIN ") && line.contains("PRIVATE KEY-----") {
            inside_private_key = true;
            without_private_keys.push("[REDACTED_PRIVATE_KEY]");
            continue;
        }
        if inside_private_key {
            if line.contains("-----END ") && line.contains("PRIVATE KEY-----") {
                inside_private_key = false;
            }
            continue;
        }
        if line_contains_unstructured_secret(line) {
            without_private_keys.push("[REDACTED_SENSITIVE_LINE]");
            continue;
        }
        without_private_keys.push(line);
    }
    let flattened = without_private_keys.join("\n");
    let mut output = Vec::new();
    let mut redact_next = false;
    for word in flattened.split_whitespace() {
        let lower = word.to_ascii_lowercase();
        let replacement = if redact_next {
            redact_next = false;
            "[REDACTED]".to_owned()
        } else if lower == "bearer" || lower == "bearer:" {
            redact_next = true;
            "Bearer".to_owned()
        } else if looks_like_absolute_path(word) {
            "[REDACTED_PATH]".to_owned()
        } else if let Some((key, _)) = word.split_once('=') {
            if is_secret_key(key) {
                format!("{key}=[REDACTED]")
            } else {
                redact_url_credentials(word).unwrap_or_else(|| word.to_owned())
            }
        } else if looks_like_common_token(word) {
            "[REDACTED_TOKEN]".to_owned()
        } else {
            redact_url_credentials(word).unwrap_or_else(|| word.to_owned())
        };
        output.push(replacement);
    }
    let redacted = output.join(" ");
    let normalized_clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
    let changed = redacted != normalized_clean;
    (redacted, changed)
}

fn line_contains_unstructured_secret(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let words = lower.split_whitespace().collect::<Vec<_>>();
    if lower.contains("authorization:")
        || words
            .iter()
            .any(|word| matches!(*word, "--token" | "--api-key" | "--secret" | "--password"))
    {
        return true;
    }
    let compact = lower.split_whitespace().collect::<String>();
    [
        "api_key:",
        "api-key:",
        "access_token:",
        "password:",
        "passwd:",
        "\"token\":",
        "\"api_key\":",
        "\"access_token\":",
        "\"secret\":",
        "\"password\":",
        "'token':",
        "'api_key':",
        "'access_token':",
        "'secret':",
        "'password':",
    ]
    .iter()
    .any(|needle| compact.contains(needle))
}

fn redact_url_credentials(value: &str) -> Option<String> {
    let scheme = value.find("://")?;
    let at = value[scheme + 3..].find('@')? + scheme + 3;
    if at == scheme + 3 {
        return None;
    }
    Some(format!(
        "{}[REDACTED_CREDENTIALS]{}",
        &value[..scheme + 3],
        &value[at..]
    ))
}

fn looks_like_common_token(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.len() >= 12
        && ["sk-", "ghp_", "github_pat_", "xoxb-", "xoxp-", "AIza"]
            .iter()
            .any(|prefix| lower.starts_with(&prefix.to_ascii_lowercase()))
}

fn looks_like_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/'))
        || value.starts_with('/')
        || value.starts_with("file://")
}

fn is_secret_key(value: &str) -> bool {
    let value = value.to_ascii_lowercase();
    [
        "key",
        "token",
        "secret",
        "password",
        "passwd",
        "authorization",
    ]
    .iter()
    .any(|needle| value.contains(needle))
}
