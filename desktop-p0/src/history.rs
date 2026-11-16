use std::{
    ffi::OsString,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};

use agentmeter_p0::provider_history::{
    ProviderHistory, ProviderHistoryReport, ProviderSessionContent,
};
use agentmeter_p0::token_analysis::{TokenAnalysis, TokenAnalysisReport};
use serde::Serialize;

static HISTORY_SCAN_ACTIVE: AtomicBool = AtomicBool::new(false);

struct HistoryScanGuard;

impl HistoryScanGuard {
    fn acquire() -> Result<Self, &'static str> {
        HISTORY_SCAN_ACTIVE
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| Self)
            .map_err(|_| "history_busy")
    }
}

impl Drop for HistoryScanGuard {
    fn drop(&mut self) {
        HISTORY_SCAN_ACTIVE.store(false, Ordering::Release);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Provider {
    Codex,
    Claude,
    Copilot,
    Antigravity,
}

impl Provider {
    fn parse(value: &str) -> Result<Self, &'static str> {
        match value {
            "codex" => Ok(Self::Codex),
            "claude" => Ok(Self::Claude),
            "copilot" => Ok(Self::Copilot),
            "antigravity" => Ok(Self::Antigravity),
            _ => Err("history_unsupported_provider"),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::Copilot => "copilot",
            Self::Antigravity => "antigravity",
        }
    }
}

fn history_window_allowed(label: &str) -> bool {
    label == "main"
}

fn provider_history_root_from(
    provider: Provider,
    codex_home: Option<OsString>,
    claude_config_dir: Option<OsString>,
    user_profile: Option<OsString>,
) -> Result<PathBuf, &'static str> {
    let profile = user_profile.map(PathBuf::from);
    let root = match provider {
        Provider::Codex => codex_home
            .map(PathBuf::from)
            .or_else(|| profile.as_ref().map(|path| path.join(".codex"))),
        Provider::Claude => claude_config_dir
            .map(PathBuf::from)
            .or_else(|| profile.as_ref().map(|path| path.join(".claude"))),
        Provider::Copilot => profile.as_ref().map(|path| path.join(".copilot")),
        Provider::Antigravity => profile
            .as_ref()
            .map(|path| path.join(".gemini").join("antigravity")),
    }
    .ok_or("history_source_unavailable")?;
    if !root.is_absolute() {
        return Err("history_source_unavailable");
    }
    Ok(root)
}

fn provider_history_root(provider: Provider) -> Result<PathBuf, &'static str> {
    provider_history_root_from(
        provider,
        std::env::var_os("CODEX_HOME"),
        std::env::var_os("CLAUDE_CONFIG_DIR"),
        std::env::var_os("USERPROFILE"),
    )
}

#[tauri::command]
pub async fn history_sessions(
    provider: String,
    window: tauri::WebviewWindow,
) -> Result<HistorySessionsView, String> {
    if !history_window_allowed(window.label()) {
        return Err("history_unauthorized".to_owned());
    }
    let provider = Provider::parse(&provider).map_err(str::to_owned)?;
    let _scan_guard = HistoryScanGuard::acquire().map_err(str::to_owned)?;
    let root = provider_history_root(provider).map_err(str::to_owned)?;
    let history =
        tauri::async_runtime::spawn_blocking(move || ProviderHistory::scan(provider.key(), &root))
            .await
            .map_err(|_| "history_scan_failed".to_owned())?
            .map_err(|error| error.to_string())?;
    let analysis = TokenAnalysis::analyze(&history);
    Ok(HistorySessionsView { history, analysis })
}

#[derive(Debug, Serialize)]
pub struct HistorySessionsView {
    #[serde(flatten)]
    pub history: ProviderHistoryReport,
    pub analysis: TokenAnalysisReport,
}

#[tauri::command]
pub async fn history_session_content(
    provider: String,
    session_ref: String,
    expected_started_at: Option<String>,
    expected_ended_at: Option<String>,
    consent: bool,
    window: tauri::WebviewWindow,
) -> Result<ProviderSessionContent, String> {
    if !history_window_allowed(window.label()) {
        return Err("history_content_unauthorized".to_owned());
    }
    if !consent {
        return Err("history_content_consent_required".to_owned());
    }
    let provider = Provider::parse(&provider).map_err(str::to_owned)?;
    let _scan_guard = HistoryScanGuard::acquire().map_err(str::to_owned)?;
    let root = provider_history_root(provider).map_err(str::to_owned)?;
    tauri::async_runtime::spawn_blocking(move || {
        ProviderHistory::read_session_content(
            provider.key(),
            &root,
            &session_ref,
            expected_started_at.as_deref(),
            expected_ended_at.as_deref(),
        )
    })
    .await
    .map_err(|_| "history_content_scan_failed".to_owned())?
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{HistoryScanGuard, Provider, history_window_allowed, provider_history_root_from};
    use std::{ffi::OsString, path::PathBuf};

    #[test]
    fn provider_allowlist_is_exact() {
        for provider in ["codex", "claude", "copilot", "antigravity"] {
            assert_eq!(Provider::parse(provider).unwrap().key(), provider);
        }
        assert!(Provider::parse("Codex").is_err());
        assert!(Provider::parse("../codex").is_err());
    }

    #[test]
    fn roots_are_derived_backend_side_and_overrides_must_be_absolute() {
        let profile = Some(OsString::from(r"C:\Users\fixture"));
        assert_eq!(
            provider_history_root_from(Provider::Codex, None, None, profile.clone()).unwrap(),
            PathBuf::from(r"C:\Users\fixture\.codex")
        );
        assert_eq!(
            provider_history_root_from(Provider::Claude, None, None, profile.clone()).unwrap(),
            PathBuf::from(r"C:\Users\fixture\.claude")
        );
        assert_eq!(
            provider_history_root_from(Provider::Copilot, None, None, profile.clone()).unwrap(),
            PathBuf::from(r"C:\Users\fixture\.copilot")
        );
        assert_eq!(
            provider_history_root_from(Provider::Antigravity, None, None, profile).unwrap(),
            PathBuf::from(r"C:\Users\fixture\.gemini\antigravity")
        );
        assert!(
            provider_history_root_from(Provider::Codex, Some("relative".into()), None, None)
                .is_err()
        );
        assert!(
            provider_history_root_from(Provider::Claude, None, Some("relative".into()), None)
                .is_err()
        );
    }

    #[test]
    fn history_is_main_window_only_and_scan_gate_is_single_flight() {
        assert!(history_window_allowed("main"));
        assert!(!history_window_allowed("hud"));
        let guard = HistoryScanGuard::acquire().unwrap();
        assert!(HistoryScanGuard::acquire().is_err());
        drop(guard);
        assert!(HistoryScanGuard::acquire().is_ok());
    }
}
