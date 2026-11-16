//! Read-only, metrics-only projection of Codex rollout files.
//!
//! The public report deliberately contains no provider session IDs, source paths,
//! prompts, responses, tool payloads, repository names, or working directories.

use serde::{Deserialize, Serialize, Serializer};
use std::{
    collections::HashMap,
    fmt,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    time::SystemTime,
};

const MAX_FILES: usize = 50;
const MAX_DISCOVERED_ENTRIES: usize = 10_000;
const MAX_DIRECTORY_DEPTH: usize = 5;
const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 128 * 1024 * 1024;
const MAX_TURNS_PER_SESSION: usize = 1_000;
const MAX_RETURNED_TURNS_PER_SESSION: usize = 200;

pub struct CodexHistory;

impl CodexHistory {
    pub fn scan(codex_home: &Path) -> Result<CodexHistoryReport, CodexHistoryError> {
        let home = codex_home
            .canonicalize()
            .map_err(|_| CodexHistoryError::SourceUnavailable)?;
        if !home.is_dir() {
            return Err(CodexHistoryError::SourceUnavailable);
        }

        let mut discovered = Vec::new();
        let mut entries_seen = 0usize;
        let mut limited = false;
        for directory in ["sessions", "archived_sessions"] {
            let candidate = home.join(directory);
            match fs::symlink_metadata(&candidate) {
                Ok(metadata) if metadata.file_type().is_symlink() => limited = true,
                Ok(metadata) if metadata.is_dir() => collect_rollouts(
                    &home,
                    &candidate,
                    0,
                    &mut entries_seen,
                    &mut discovered,
                    &mut limited,
                ),
                Ok(_) => limited = true,
                Err(error) if error.kind() != std::io::ErrorKind::NotFound => limited = true,
                Err(_) => {}
            }
        }
        discovered.sort_by(|left, right| {
            right
                .modified
                .cmp(&left.modified)
                .then_with(|| right.path.cmp(&left.path))
        });
        if discovered.len() > MAX_FILES {
            discovered.truncate(MAX_FILES);
            limited = true;
        }

        let mut report = CodexHistoryReport::empty();
        report.limited = limited;
        let mut parsed = Vec::new();
        let mut identities = HashMap::new();
        let mut total_bytes = 0u64;

        for candidate in discovered {
            if total_bytes >= MAX_TOTAL_BYTES {
                report.limited = true;
                break;
            }
            let remaining = MAX_TOTAL_BYTES - total_bytes;
            let byte_limit = MAX_FILE_BYTES.min(remaining).min(candidate.size);
            total_bytes = total_bytes.saturating_add(byte_limit);
            match parse_rollout(&home, &candidate.path, byte_limit) {
                Ok(mut session) => {
                    report.files_scanned += 1;
                    report.records_rejected += session.records_rejected;
                    report.partial_records += session.partial_records;
                    report.limited |= session.limited;
                    if session.turns.is_empty() {
                        continue;
                    }
                    let identity = session
                        .identity
                        .take()
                        .unwrap_or_else(|| candidate.path.to_string_lossy().into_owned());
                    if let Some(existing) = identities.get(&identity).copied() {
                        report.duplicate_sessions += 1;
                        if session.is_better_than(&parsed[existing]) {
                            parsed[existing] = session;
                        }
                        continue;
                    }
                    identities.insert(identity, parsed.len());
                    parsed.push(session);
                }
                Err(()) => report.files_skipped += 1,
            }
        }

        parsed.sort_by(|left, right| {
            right
                .ended_at
                .cmp(&left.ended_at)
                .then_with(|| right.started_at.cmp(&left.started_at))
        });
        report.sessions = parsed
            .into_iter()
            .enumerate()
            .map(|(index, session)| session.into_public(index + 1, &mut report.limited))
            .collect();
        report.totals = sum_tokens(report.sessions.iter().map(|session| &session.totals));
        Ok(report)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexHistoryError {
    SourceUnavailable,
}

impl fmt::Display for CodexHistoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceUnavailable => formatter.write_str("codex_history_source_unavailable"),
        }
    }
}

impl std::error::Error for CodexHistoryError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexHistoryReport {
    pub schema_version: &'static str,
    pub provider: &'static str,
    pub scope: &'static str,
    pub data_quality: &'static str,
    pub files_scanned: usize,
    pub files_skipped: usize,
    pub duplicate_sessions: usize,
    pub records_rejected: usize,
    pub partial_records: usize,
    pub limited: bool,
    pub totals: TokenMetrics,
    pub sessions: Vec<CodexSessionMetrics>,
}

impl CodexHistoryReport {
    fn empty() -> Self {
        Self {
            schema_version: "agentmeter.codex-history.v1",
            provider: "codex",
            scope: "local_sessions",
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexSessionMetrics {
    pub session_ref: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub turn_count: usize,
    pub turn_count_exact: bool,
    pub turns_truncated: bool,
    pub totals: TokenMetrics,
    pub turns: Vec<CodexTurnMetrics>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CodexTurnMetrics {
    pub turn_ref: String,
    pub started_at: Option<String>,
    pub ended_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub model: Option<String>,
    pub tokens: TokenMetrics,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMetrics {
    #[serde(serialize_with = "serialize_optional_u64")]
    pub input_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub cached_input_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub cache_write_input_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub output_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub reasoning_output_tokens: Option<u64>,
    #[serde(serialize_with = "serialize_optional_u64")]
    pub total_tokens: Option<u64>,
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

impl TokenMetrics {
    fn is_valid_turn_total(&self) -> bool {
        let (Some(input), Some(output), Some(total)) =
            (self.input_tokens, self.output_tokens, self.total_tokens)
        else {
            return false;
        };
        if input.checked_add(output) != Some(total) {
            return false;
        }
        if self
            .cached_input_tokens
            .is_some_and(|cached| cached > input)
        {
            return false;
        }
        if self
            .reasoning_output_tokens
            .is_some_and(|reasoning| reasoning > output)
        {
            return false;
        }
        true
    }

    fn is_monotonic_after(&self, previous: &Self) -> bool {
        [
            (self.input_tokens, previous.input_tokens),
            (self.cached_input_tokens, previous.cached_input_tokens),
            (
                self.cache_write_input_tokens,
                previous.cache_write_input_tokens,
            ),
            (self.output_tokens, previous.output_tokens),
            (
                self.reasoning_output_tokens,
                previous.reasoning_output_tokens,
            ),
            (self.total_tokens, previous.total_tokens),
        ]
        .into_iter()
        .all(|(current, old)| match (current, old) {
            (Some(current), Some(old)) => current >= old,
            (None, Some(_)) => false,
            _ => true,
        })
    }
}

struct Candidate {
    path: PathBuf,
    modified: SystemTime,
    size: u64,
}

fn collect_rollouts(
    home: &Path,
    directory: &Path,
    depth: usize,
    entries_seen: &mut usize,
    output: &mut Vec<Candidate>,
    limited: &mut bool,
) {
    if depth > MAX_DIRECTORY_DEPTH || *entries_seen >= MAX_DISCOVERED_ENTRIES {
        *limited = true;
        return;
    }
    let Ok(entries) = fs::read_dir(directory) else {
        *limited = true;
        return;
    };
    for entry in entries.flatten() {
        *entries_seen += 1;
        if *entries_seen > MAX_DISCOVERED_ENTRIES {
            *limited = true;
            return;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_rollouts(home, &path, depth + 1, entries_seen, output, limited);
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let file_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if file_name.ends_with(".tmp") {
            continue;
        }
        if file_name.ends_with(".jsonl.zst") {
            // Compressed archives are not parsed in this no-new-dependency slice.
            // Surface incompleteness instead of silently claiming a complete scan.
            *limited = true;
            continue;
        }
        if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(canonical) = path.canonicalize() else {
            *limited = true;
            continue;
        };
        if !canonical.starts_with(home) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            *limited = true;
            continue;
        };
        let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        output.push(Candidate {
            path: canonical,
            modified,
            size: metadata.len(),
        });
    }
}

#[derive(Default)]
struct ParsedSession {
    identity: Option<String>,
    started_at: Option<String>,
    ended_at: Option<String>,
    turns: HashMap<String, ParsedTurn>,
    turn_order: Vec<String>,
    current_turn: Option<String>,
    thread_tokens: Option<TokenMetrics>,
    legacy_total: Option<TokenMetrics>,
    has_modern_thread_usage: bool,
    bytes_read: u64,
    records_rejected: usize,
    partial_records: usize,
    limited: bool,
    turn_limit_reached: bool,
}

impl ParsedSession {
    fn is_better_than(&self, other: &Self) -> bool {
        let self_turns = self
            .turns
            .values()
            .filter(|turn| turn.tokens.is_some())
            .count();
        let other_turns = other
            .turns
            .values()
            .filter(|turn| turn.tokens.is_some())
            .count();
        if self_turns != other_turns {
            return self_turns > other_turns;
        }
        if self.thread_tokens.is_some() != other.thread_tokens.is_some() {
            return self.thread_tokens.is_some();
        }
        let self_issues = self.records_rejected.saturating_add(self.partial_records);
        let other_issues = other.records_rejected.saturating_add(other.partial_records);
        self_issues < other_issues
            || (self_issues == other_issues && self.ended_at > other.ended_at)
    }

    fn turn_mut(&mut self, identity: String) -> Option<&mut ParsedTurn> {
        if !self.turns.contains_key(&identity) {
            if self.turns.len() >= MAX_TURNS_PER_SESSION {
                self.limited = true;
                self.turn_limit_reached = true;
                return None;
            }
            self.turn_order.push(identity.clone());
            self.turns.insert(identity.clone(), ParsedTurn::default());
        }
        self.turns.get_mut(&identity)
    }

    fn into_public(self, session_number: usize, report_limited: &mut bool) -> CodexSessionMetrics {
        let mut turns: Vec<_> = self
            .turn_order
            .into_iter()
            .filter_map(|identity| self.turns.get(&identity).cloned())
            .filter(|turn| turn.tokens.is_some())
            .collect();
        turns.sort_by(|left, right| {
            left.started_at
                .cmp(&right.started_at)
                .then_with(|| left.ended_at.cmp(&right.ended_at))
        });
        let turn_count = turns.len();
        let turn_count_exact = !self.turn_limit_reached;
        let turns_truncated = turn_count > MAX_RETURNED_TURNS_PER_SESSION || !turn_count_exact;
        let started_at = self
            .started_at
            .or_else(|| turns.first().and_then(|turn| turn.started_at.clone()));
        let ended_at = self
            .ended_at
            .or_else(|| turns.last().and_then(|turn| turn.ended_at.clone()));
        let duration_ms = turns
            .iter()
            .try_fold(0u64, |total, turn| total.checked_add(turn.duration_ms?));
        let totals = self
            .thread_tokens
            .unwrap_or_else(|| sum_tokens(turns.iter().filter_map(|turn| turn.tokens.as_ref())));
        if turns_truncated {
            let keep_from = turns.len() - MAX_RETURNED_TURNS_PER_SESSION;
            turns.drain(0..keep_from);
            *report_limited = true;
        }
        let public_turns: Vec<_> = turns
            .into_iter()
            .enumerate()
            .map(|(index, turn)| CodexTurnMetrics {
                turn_ref: format!("turn-{}", index + 1),
                started_at: turn.started_at,
                ended_at: turn.ended_at,
                duration_ms: turn.duration_ms,
                model: turn.model,
                tokens: turn.tokens.unwrap_or_default(),
            })
            .collect();
        CodexSessionMetrics {
            session_ref: format!("session-{session_number}"),
            started_at,
            ended_at,
            duration_ms,
            turn_count,
            turn_count_exact,
            turns_truncated,
            totals,
            turns: public_turns,
        }
    }
}

#[derive(Clone, Default)]
struct ParsedTurn {
    started_at: Option<String>,
    ended_at: Option<String>,
    duration_ms: Option<u64>,
    model: Option<String>,
    tokens: Option<TokenMetrics>,
    has_modern_usage: bool,
}

#[derive(Deserialize)]
struct RolloutRecord {
    #[serde(rename = "type")]
    record_type: String,
    timestamp: Option<String>,
    payload: RolloutPayload,
}

#[derive(Default, Deserialize)]
struct RolloutPayload {
    #[serde(rename = "type")]
    payload_type: Option<String>,
    id: Option<String>,
    session_id: Option<String>,
    thread_id: Option<String>,
    turn_id: Option<String>,
    root_turn_id: Option<String>,
    model: Option<String>,
    turn_token_usage: Option<TokenMetrics>,
    thread_token_usage: Option<TokenMetrics>,
    usage: Option<TokenMetrics>,
    info: Option<TokenUsageInfo>,
    duration_ms: Option<u64>,
}

#[derive(Default, Deserialize)]
struct TokenUsageInfo {
    last_token_usage: Option<TokenMetrics>,
    total_token_usage: Option<TokenMetrics>,
}

fn parse_rollout(home: &Path, path: &Path, byte_limit: u64) -> Result<ParsedSession, ()> {
    let link_metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if link_metadata.file_type().is_symlink() {
        return Err(());
    }
    let canonical = path.canonicalize().map_err(|_| ())?;
    if !canonical.starts_with(home) {
        return Err(());
    }
    let file = File::open(canonical).map_err(|_| ())?;
    let file_size = file.metadata().map_err(|_| ())?.len();
    let mut reader = BufReader::new(file).take(byte_limit.saturating_add(1));
    let mut session = ParsedSession::default();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|_| ())?;
        if bytes == 0 {
            break;
        }
        session.bytes_read = session.bytes_read.saturating_add(bytes as u64);
        if session.bytes_read > byte_limit {
            session.limited = true;
            break;
        }
        if !line.ends_with('\n') {
            session.partial_records += 1;
            break;
        }
        let Ok(record) = serde_json::from_str::<RolloutRecord>(&line) else {
            session.records_rejected += 1;
            continue;
        };
        consume_record(&mut session, record);
    }
    if file_size > byte_limit {
        session.limited = true;
    }
    Ok(session)
}

fn consume_record(session: &mut ParsedSession, record: RolloutRecord) {
    let timestamp = safe_timestamp(record.timestamp);
    match record.record_type.as_str() {
        "session_meta" => {
            if session.identity.is_none() {
                session.identity = record.payload.id;
            }
            if session.started_at.is_none() {
                session.started_at = timestamp;
            }
        }
        "turn_context" => {
            let Some(identity) = record.payload.turn_id else {
                session.records_rejected += 1;
                return;
            };
            session.current_turn = Some(identity.clone());
            let model = safe_model(record.payload.model);
            if let Some(turn) = session.turn_mut(identity) {
                turn.started_at = turn.started_at.take().or(timestamp);
                turn.model = model.or_else(|| turn.model.take());
            }
        }
        "token_usage_record" => {
            if session.identity.is_none() {
                session.identity = record.payload.thread_id.or(record.payload.session_id);
            }
            let Some(identity) = record.payload.turn_id.or(record.payload.root_turn_id) else {
                session.records_rejected += 1;
                return;
            };
            let thread_tokens = match record.payload.thread_token_usage {
                Some(tokens) if tokens.is_valid_turn_total() => Some(tokens),
                Some(_) => {
                    session.records_rejected += 1;
                    session.limited = true;
                    None
                }
                None => None,
            };
            let Some(tokens) = record
                .payload
                .turn_token_usage
                .or(record.payload.usage)
                .filter(TokenMetrics::is_valid_turn_total)
            else {
                session.records_rejected += 1;
                return;
            };
            let mut rejected = false;
            if let Some(turn) = session.turn_mut(identity) {
                if turn
                    .tokens
                    .as_ref()
                    .is_some_and(|previous| !tokens.is_monotonic_after(previous))
                {
                    rejected = true;
                } else {
                    if turn.started_at.is_none() {
                        turn.started_at = timestamp.clone();
                    }
                    turn.ended_at = timestamp.clone().or_else(|| turn.ended_at.take());
                    turn.tokens = Some(tokens);
                    turn.has_modern_usage = true;
                }
            }
            if rejected {
                session.records_rejected += 1;
            }
            if let Some(thread_tokens) = thread_tokens {
                if session
                    .thread_tokens
                    .as_ref()
                    .is_some_and(|previous| !thread_tokens.is_monotonic_after(previous))
                {
                    session.records_rejected += 1;
                } else {
                    session.thread_tokens = Some(thread_tokens);
                    session.has_modern_thread_usage = true;
                }
            }
            session.ended_at = timestamp.or_else(|| session.ended_at.take());
        }
        "event_msg" if record.payload.payload_type.as_deref() == Some("task_started") => {
            let Some(identity) = record.payload.turn_id else {
                session.records_rejected += 1;
                return;
            };
            session.current_turn = Some(identity.clone());
            if let Some(turn) = session.turn_mut(identity) {
                turn.started_at = turn.started_at.take().or(timestamp);
            }
        }
        "event_msg" if record.payload.payload_type.as_deref() == Some("token_count") => {
            let Some(info) = record.payload.info else {
                return;
            };
            let last = info
                .last_token_usage
                .filter(TokenMetrics::is_valid_turn_total);
            let Some(total) = info
                .total_token_usage
                .filter(TokenMetrics::is_valid_turn_total)
            else {
                session.records_rejected += 1;
                session.limited = true;
                return;
            };
            if session
                .legacy_total
                .as_ref()
                .is_some_and(|previous| !total.is_monotonic_after(previous))
            {
                session.records_rejected += 1;
                session.limited = true;
                return;
            }
            if !session.has_modern_thread_usage {
                session.thread_tokens = Some(total.clone());
            }
            let tokens = if let Some(previous) = session.legacy_total.replace(total.clone()) {
                let Some(delta) = subtract_tokens(&total, &previous) else {
                    session.records_rejected += 1;
                    session.limited = true;
                    return;
                };
                delta
            } else {
                let Some(last) = last else {
                    session.limited = true;
                    return;
                };
                last
            };
            let Some(identity) = session.current_turn.clone() else {
                session.limited = true;
                return;
            };
            let mut rejected = false;
            if let Some(turn) = session.turn_mut(identity)
                && !turn.has_modern_usage
            {
                let combined = turn.tokens.take().map_or(tokens.clone(), |previous| {
                    add_tokens(previous, tokens.clone())
                });
                if combined.is_valid_turn_total() {
                    if turn.started_at.is_none() {
                        turn.started_at = timestamp.clone();
                    }
                    turn.ended_at = timestamp.clone().or_else(|| turn.ended_at.take());
                    turn.tokens = Some(combined);
                } else {
                    rejected = true;
                }
            }
            if rejected {
                session.records_rejected += 1;
            }
            session.ended_at = timestamp.or_else(|| session.ended_at.take());
        }
        "event_msg" if record.payload.payload_type.as_deref() == Some("task_complete") => {
            let Some(identity) = record.payload.turn_id else {
                session.records_rejected += 1;
                return;
            };
            if let Some(turn) = session.turn_mut(identity) {
                turn.ended_at = timestamp.clone().or_else(|| turn.ended_at.take());
                turn.duration_ms = record.payload.duration_ms;
            }
            session.current_turn = None;
            session.ended_at = timestamp.or_else(|| session.ended_at.take());
        }
        _ => {}
    }
}

fn safe_timestamp(value: Option<String>) -> Option<String> {
    value.filter(|value| is_rfc3339_shape(value))
}

fn safe_model(value: Option<String>) -> Option<String> {
    value.filter(|value| {
        !value.is_empty()
            && value.len() <= 120
            && value
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || ".-_".contains(character))
    })
}

fn is_rfc3339_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    if !(20..=40).contains(&bytes.len())
        || bytes.get(4) != Some(&b'-')
        || bytes.get(7) != Some(&b'-')
        || bytes.get(10) != Some(&b'T')
        || bytes.get(13) != Some(&b':')
        || bytes.get(16) != Some(&b':')
        || !bytes[..4].iter().all(u8::is_ascii_digit)
        || !bytes[5..7].iter().all(u8::is_ascii_digit)
        || !bytes[8..10].iter().all(u8::is_ascii_digit)
        || !bytes[11..13].iter().all(u8::is_ascii_digit)
        || !bytes[14..16].iter().all(u8::is_ascii_digit)
        || !bytes[17..19].iter().all(u8::is_ascii_digit)
    {
        return false;
    }
    let suffix = &bytes[19..];
    if suffix == b"Z" {
        return true;
    }
    let timezone_start = suffix
        .iter()
        .position(|value| matches!(value, b'Z' | b'+' | b'-'));
    let Some(timezone_start) = timezone_start else {
        return false;
    };
    let fraction = &suffix[..timezone_start];
    if !fraction.is_empty()
        && (fraction.first() != Some(&b'.')
            || fraction.len() < 2
            || !fraction[1..].iter().all(u8::is_ascii_digit))
    {
        return false;
    }
    let timezone = &suffix[timezone_start..];
    timezone == b"Z"
        || (timezone.len() == 6
            && matches!(timezone[0], b'+' | b'-')
            && timezone[1..3].iter().all(u8::is_ascii_digit)
            && timezone[3] == b':'
            && timezone[4..6].iter().all(u8::is_ascii_digit))
}

fn sum_tokens<'a>(values: impl Iterator<Item = &'a TokenMetrics>) -> TokenMetrics {
    values.cloned().reduce(add_tokens).unwrap_or_default()
}

fn add_tokens(left: TokenMetrics, right: TokenMetrics) -> TokenMetrics {
    TokenMetrics {
        input_tokens: add_known(left.input_tokens, right.input_tokens),
        cached_input_tokens: add_known(left.cached_input_tokens, right.cached_input_tokens),
        cache_write_input_tokens: add_known(
            left.cache_write_input_tokens,
            right.cache_write_input_tokens,
        ),
        output_tokens: add_known(left.output_tokens, right.output_tokens),
        reasoning_output_tokens: add_known(
            left.reasoning_output_tokens,
            right.reasoning_output_tokens,
        ),
        total_tokens: add_known(left.total_tokens, right.total_tokens),
    }
}

fn subtract_tokens(current: &TokenMetrics, previous: &TokenMetrics) -> Option<TokenMetrics> {
    let tokens = TokenMetrics {
        input_tokens: subtract_known(current.input_tokens, previous.input_tokens),
        cached_input_tokens: subtract_known(
            current.cached_input_tokens,
            previous.cached_input_tokens,
        ),
        cache_write_input_tokens: subtract_known(
            current.cache_write_input_tokens,
            previous.cache_write_input_tokens,
        ),
        output_tokens: subtract_known(current.output_tokens, previous.output_tokens),
        reasoning_output_tokens: subtract_known(
            current.reasoning_output_tokens,
            previous.reasoning_output_tokens,
        ),
        total_tokens: subtract_known(current.total_tokens, previous.total_tokens),
    };
    tokens.is_valid_turn_total().then_some(tokens)
}

fn add_known(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    left?.checked_add(right?)
}

fn subtract_known(current: Option<u64>, previous: Option<u64>) -> Option<u64> {
    current?.checked_sub(previous?)
}
