use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum DataQuality {
    Official,
    LocalObserved,
    Estimated,
    Manual,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum CollectorMaturity {
    Stable,
    Experimental,
    Unsupported,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Availability {
    Available,
    NotInstalled,
    SetupRequired,
    NeedsLogin,
    Unsupported,
    PermissionDenied,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum CollectionState {
    Idle,
    Collecting,
    Ready,
    BackingOff,
    Error,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum Freshness {
    Unknown,
    Fresh,
    Stale,
}

#[derive(Debug, Serialize)]
pub struct CollectionReport {
    pub schema_version: &'static str,
    pub outcome: &'static str,
    pub app_server_version: Option<String>,
    pub capabilities: BTreeMap<String, &'static str>,
    pub observation: Observation,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Serialize)]
pub struct Observation {
    pub provider: &'static str,
    pub provider_account: ProviderAccount,
    pub source: Source,
    pub quota_windows: Vec<QuotaWindow>,
    pub credits: Vec<CreditsSnapshot>,
    pub source_usage: Option<Value>,
    pub data_quality: DataQuality,
    pub collector_maturity: CollectorMaturity,
    pub availability: Availability,
    pub collection_state: CollectionState,
    pub freshness: Freshness,
    pub source_timestamp: Option<i64>,
    pub collected_at_unix_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct ProviderAccount {
    pub kind: String,
    pub plan_type: Option<String>,
    pub display_hint: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Source {
    pub kind: &'static str,
    pub version: Option<String>,
    pub mode: &'static str,
    pub replay: bool,
}

#[derive(Debug, Serialize)]
pub struct QuotaWindow {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub window: &'static str,
    pub scope: &'static str,
    pub used_percent: i64,
    pub remaining_percent: i64,
    pub over_limit: bool,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
    pub unit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CreditsSnapshot {
    pub limit_id: Option<String>,
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
    pub unit: &'static str,
}

#[derive(Debug, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct FailureReport {
    pub schema_version: &'static str,
    pub outcome: &'static str,
    pub observation: (),
    pub app_server_version: Option<String>,
    pub provider_account_kind: Option<String>,
    pub capabilities: BTreeMap<String, String>,
    pub failure_code: &'static str,
    pub availability: &'static str,
    pub collection_state: &'static str,
    pub freshness: &'static str,
    pub message: String,
}

impl FailureReport {
    pub fn from_error(error: String) -> Self {
        let (failure_code, availability) = if error.starts_with("authentication_failed:") {
            ("authentication_failed", "needs_login")
        } else if error.starts_with("method_unsupported:") {
            ("method_unsupported", "unsupported")
        } else if error.starts_with("timeout:") {
            ("timeout", "available")
        } else if error.starts_with("malformed_response") {
            ("malformed_response", "available")
        } else if error.starts_with("schema_changed:") {
            ("schema_changed", "available")
        } else if error.starts_with("process_exited:") {
            ("process_exited", "available")
        } else {
            ("collector_error", "available")
        };

        Self {
            schema_version: "agentmeter.p0.collection-report.v1",
            outcome: "failure",
            observation: (),
            app_server_version: None,
            provider_account_kind: None,
            capabilities: BTreeMap::new(),
            failure_code,
            availability,
            collection_state: "error",
            freshness: "unknown",
            message: error,
        }
    }

    pub fn with_evidence(
        mut self,
        app_server_version: Option<String>,
        provider_account_kind: Option<String>,
        capabilities: BTreeMap<String, String>,
    ) -> Self {
        self.app_server_version = app_server_version;
        self.provider_account_kind = provider_account_kind;
        self.capabilities = capabilities;
        self
    }
}

impl From<String> for FailureReport {
    fn from(error: String) -> Self {
        Self::from_error(error)
    }
}

impl From<String> for Box<FailureReport> {
    fn from(error: String) -> Self {
        Box::new(FailureReport::from_error(error))
    }
}
