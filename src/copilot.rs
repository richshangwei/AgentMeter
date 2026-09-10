//! Live GitHub Copilot billing-usage collection through an existing `gh` login.
//!
//! The public interface deliberately returns billing usage, not remaining quota:
//! GitHub's documented Billing REST responses do not contain allowance or reset data.

use serde_json::{Value, json};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

pub const API_VERSION: &str = "2026-03-10";
const MAX_PROCESS_OUTPUT_BYTES: usize = 256 * 1024;

#[derive(Debug)]
pub struct CollectionFailure {
    pub code: &'static str,
    pub message: String,
}

impl CollectionFailure {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Copy)]
enum AccountContext {
    Personal,
    Business,
    Enterprise,
}

impl AccountContext {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "personal" => Some(Self::Personal),
            "business" | "organization" => Some(Self::Business),
            "enterprise" => Some(Self::Enterprise),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Personal => "Personal",
            Self::Business => "Business",
            Self::Enterprise => "Enterprise",
        }
    }

    fn account_field(self) -> &'static str {
        match self {
            Self::Personal => "user",
            Self::Business => "organization",
            Self::Enterprise => "enterprise",
        }
    }

    fn permission(self) -> &'static str {
        match self {
            Self::Personal => "Plan:read",
            Self::Business => "Administration:read",
            Self::Enterprise => "Enterprise billing:read",
        }
    }

    fn endpoint_prefix(self) -> &'static str {
        match self {
            Self::Personal => "users",
            Self::Business => "organizations",
            Self::Enterprise => "enterprises",
        }
    }
}

#[derive(Clone, Copy)]
enum Meter {
    AiCredits,
    PremiumRequests,
}

impl Meter {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "ai-credits" => Some(Self::AiCredits),
            "premium-requests" => Some(Self::PremiumRequests),
            _ => None,
        }
    }

    fn endpoint_segment(self) -> &'static str {
        match self {
            Self::AiCredits => "ai_credit",
            Self::PremiumRequests => "premium_request",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::AiCredits => "ai_credits",
            Self::PremiumRequests => "premium_requests",
        }
    }

    fn valid_unit(self, unit: &str) -> bool {
        match self {
            Self::AiCredits => matches!(unit, "credits" | "ai-credits"),
            Self::PremiumRequests => unit == "requests",
        }
    }
}

pub struct UsageRequest {
    gh_bin: PathBuf,
    context: AccountContext,
    account: String,
    meter: Meter,
    year: Option<u16>,
    month: Option<u8>,
    timeout: Duration,
}

impl UsageRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        gh_bin: impl AsRef<Path>,
        context: &str,
        account: &str,
        meter: &str,
        year: Option<u16>,
        month: Option<u8>,
        timeout: Duration,
    ) -> Result<Self, CollectionFailure> {
        let gh_bin = gh_bin.as_ref().to_path_buf();
        if !gh_bin.is_absolute() {
            return Err(CollectionFailure::new(
                "invalid_arguments",
                "GitHub CLI path must be absolute",
            ));
        }
        let context = AccountContext::parse(context).ok_or_else(|| {
            CollectionFailure::new(
                "invalid_arguments",
                "context must be personal, business, or enterprise",
            )
        })?;
        validate_account_slug(account)?;
        let meter = Meter::parse(meter).ok_or_else(|| {
            CollectionFailure::new(
                "invalid_arguments",
                "meter must be ai-credits or premium-requests",
            )
        })?;
        if let Some(year) = year
            && !(2000..=9999).contains(&year)
        {
            return Err(CollectionFailure::new(
                "invalid_arguments",
                "year must be between 2000 and 9999",
            ));
        }
        if let Some(month) = month
            && !(1..=12).contains(&month)
        {
            return Err(CollectionFailure::new(
                "invalid_arguments",
                "month must be between 1 and 12",
            ));
        }
        if timeout.is_zero() || timeout > Duration::from_secs(60) {
            return Err(CollectionFailure::new(
                "invalid_arguments",
                "timeout must be between 1 and 60000 milliseconds",
            ));
        }
        Ok(Self {
            gh_bin,
            context,
            account: account.to_owned(),
            meter,
            year,
            month,
            timeout,
        })
    }
}

struct CapturedProcess {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    output_exceeded: bool,
}

fn validate_account_slug(value: &str) -> Result<(), CollectionFailure> {
    let valid = !value.is_empty()
        && value.len() <= 100
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if valid {
        Ok(())
    } else {
        Err(CollectionFailure::new(
            "invalid_arguments",
            "account must be a 1-100 character GitHub slug containing only ASCII letters, digits, or interior hyphens",
        ))
    }
}

fn read_bounded<R: Read>(mut reader: R) -> (Vec<u8>, bool) {
    let mut stored = Vec::new();
    let mut exceeded = false;
    let mut chunk = [0_u8; 8192];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                let remaining = MAX_PROCESS_OUTPUT_BYTES.saturating_sub(stored.len());
                stored.extend_from_slice(&chunk[..count.min(remaining)]);
                exceeded |= count > remaining;
            }
        }
    }
    (stored, exceeded)
}

fn run_gh(
    request: &UsageRequest,
    endpoint: &str,
    cancelled: &AtomicBool,
) -> Result<CapturedProcess, CollectionFailure> {
    if cancelled.load(Ordering::Acquire) {
        return Err(CollectionFailure::new(
            "cancelled",
            "GitHub request cancelled",
        ));
    }
    let mut command = Command::new(&request.gh_bin);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .args([
            "api",
            "--hostname",
            "github.com",
            "--method",
            "GET",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            &format!("X-GitHub-Api-Version: {API_VERSION}"),
            endpoint,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|_| {
        CollectionFailure::new(
            "command_failed",
            "could not start the configured GitHub CLI",
        )
    })?;
    let stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let (sender, receiver) = mpsc::channel();
    let out_sender = sender.clone();
    thread::spawn(move || {
        let _ = out_sender.send((true, read_bounded(stdout)));
    });
    thread::spawn(move || {
        let _ = sender.send((false, read_bounded(stderr)));
    });
    let deadline = Instant::now() + request.timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancelled.load(Ordering::Acquire) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CollectionFailure::new(
                    "cancelled",
                    "GitHub CLI request was cancelled",
                ));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CollectionFailure::new(
                    "timeout",
                    "GitHub CLI request exceeded its bounded timeout",
                ));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(CollectionFailure::new(
                    "command_failed",
                    "could not wait for the GitHub CLI",
                ));
            }
        }
    };
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exceeded = false;
    for _ in 0..2 {
        let (is_stdout, (bytes, stream_exceeded)) =
            receiver.recv_timeout(Duration::from_secs(2)).map_err(|_| {
                CollectionFailure::new(
                    "command_failed",
                    "could not finish reading GitHub CLI output",
                )
            })?;
        if is_stdout {
            stdout = bytes;
        } else {
            stderr = bytes;
        }
        exceeded |= stream_exceeded;
    }
    Ok(CapturedProcess {
        success: status.success(),
        stdout,
        stderr,
        output_exceeded: exceeded,
    })
}

fn endpoint(request: &UsageRequest) -> String {
    let mut endpoint = format!(
        "/{}/{}/settings/billing/{}/usage?product=Copilot",
        request.context.endpoint_prefix(),
        request.account,
        request.meter.endpoint_segment()
    );
    if let Some(year) = request.year {
        endpoint.push_str(&format!("&year={year}"));
    }
    if let Some(month) = request.month {
        endpoint.push_str(&format!("&month={month}"));
    }
    endpoint
}

fn classify_gh_failure(stderr: &[u8]) -> CollectionFailure {
    let text = String::from_utf8_lossy(stderr).to_ascii_lowercase();
    if text.contains("rate limit") || text.contains("http 429") {
        CollectionFailure::new("rate_limited", "GitHub rate-limited the request")
    } else if text.contains("http 401")
        || text.contains("authentication")
        || text.contains("not logged")
    {
        CollectionFailure::new(
            "authentication_failed",
            "GitHub CLI is not authenticated for this request",
        )
    } else if text.contains("http 403") || text.contains("forbidden") {
        CollectionFailure::new(
            "permission_denied",
            "GitHub rejected the request permission or account role",
        )
    } else if text.contains("http 404") || text.contains("not found") {
        CollectionFailure::new(
            "endpoint_unavailable",
            "GitHub did not expose this endpoint for the requested account",
        )
    } else if text.contains("http 400") {
        CollectionFailure::new("invalid_request", "GitHub rejected the request parameters")
    } else if text.contains("http 500") || text.contains("http 503") {
        CollectionFailure::new(
            "provider_unavailable",
            "GitHub billing service was unavailable",
        )
    } else {
        CollectionFailure::new("command_failed", "GitHub CLI request failed")
    }
}

/// Resolve only the authenticated identity; never infer an organization or plan.
pub fn authenticated_account(
    gh_bin: &Path,
    cancelled: &AtomicBool,
) -> Result<String, CollectionFailure> {
    let request = UsageRequest::new(
        gh_bin,
        "personal",
        "placeholder",
        "ai-credits",
        None,
        None,
        Duration::from_secs(10),
    )?;
    let captured = run_gh(&request, "/user", cancelled)?;
    if captured.output_exceeded {
        return Err(CollectionFailure::new(
            "response_too_large",
            "GitHub identity response too large",
        ));
    }
    if !captured.success {
        return Err(classify_gh_failure(&captured.stderr));
    }
    let response: Value = serde_json::from_slice(&captured.stdout).map_err(|_| {
        CollectionFailure::new("malformed_response", "Invalid GitHub identity response")
    })?;
    identity_login(&response)
}

fn identity_login(response: &Value) -> Result<String, CollectionFailure> {
    let login = response["login"]
        .as_str()
        .ok_or_else(|| CollectionFailure::new("schema_changed", "Missing GitHub identity"))?;
    validate_account_slug(login)?;
    Ok(login.to_owned())
}

pub fn collect_usage(
    request: UsageRequest,
    cancelled: &AtomicBool,
) -> Result<Value, CollectionFailure> {
    if !request.gh_bin.is_file() {
        return Err(CollectionFailure::new(
            "invalid_arguments",
            "GitHub CLI path must identify an existing file",
        ));
    }
    let endpoint = endpoint(&request);
    let captured = run_gh(&request, &endpoint, cancelled)?;
    if captured.output_exceeded {
        return Err(CollectionFailure::new(
            "response_too_large",
            format!("GitHub CLI output exceeded {MAX_PROCESS_OUTPUT_BYTES} bytes per stream"),
        ));
    }
    if !captured.success {
        return Err(classify_gh_failure(&captured.stderr));
    }
    let response: Value = serde_json::from_slice(&captured.stdout).map_err(|_| {
        CollectionFailure::new("malformed_response", "GitHub CLI returned invalid JSON")
    })?;
    let object = response.as_object().ok_or_else(|| {
        CollectionFailure::new("schema_changed", "GitHub billing response is not an object")
    })?;
    let reported_account = object
        .get(request.context.account_field())
        .and_then(Value::as_str)
        .ok_or_else(|| {
            CollectionFailure::new(
                "schema_changed",
                "GitHub billing response omitted its account field",
            )
        })?;
    let period = object
        .get("timePeriod")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            CollectionFailure::new(
                "schema_changed",
                "GitHub billing response omitted timePeriod",
            )
        })?;
    let items = object
        .get("usageItems")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            CollectionFailure::new(
                "schema_changed",
                "GitHub billing response omitted usageItems",
            )
        })?;
    let mut normalized_items = Vec::with_capacity(items.len());
    for item in items {
        let product = item.get("product").and_then(Value::as_str);
        let sku = item.get("sku").and_then(Value::as_str);
        let model = item.get("model").and_then(Value::as_str);
        let unit = item.get("unitType").and_then(Value::as_str);
        let gross = item.get("grossQuantity").and_then(Value::as_f64);
        let net = item.get("netQuantity").and_then(Value::as_f64);
        let (Some(product), Some(sku), Some(unit), Some(gross), Some(net)) =
            (product, sku, unit, gross, net)
        else {
            return Err(CollectionFailure::new(
                "schema_changed",
                "GitHub usage item omitted a required field",
            ));
        };
        if !request.meter.valid_unit(unit) {
            return Err(CollectionFailure::new(
                "schema_changed",
                format!("GitHub returned unitType {unit:?} for the selected meter"),
            ));
        }
        if !product.to_ascii_lowercase().contains("copilot") {
            return Err(CollectionFailure::new(
                "schema_changed",
                "GitHub returned a non-Copilot item from the filtered endpoint",
            ));
        }
        normalized_items.push(json!({
            "product": product,
            "sku": sku,
            "model": model,
            "unit": unit,
            "gross_used": gross,
            "discounted_used": item.get("discountQuantity").cloned().unwrap_or(Value::Null),
            "net_used": net
        }));
    }
    Ok(json!({
        "schema_version":"copilot-p0/v1",
        "outcome":"success",
        "capability":"copilot_authoritative_billing_usage",
        "contexts":[{
            "context":request.context.label(),
            "account_scope":{"kind":request.context.label().to_ascii_lowercase(),"requested":request.account,"reported":reported_account},
            "billing_api":{"permission":"granted","status":"available","api_version":API_VERSION,"endpoint":endpoint},
            "permission_requirements":{"required_permissions":[request.context.permission()],"status":"request_authorized"},
            "authoritative_usage":{"status":"reported","meter":request.meter.label(),"billing_period":period,"items":normalized_items},
            "quota":{"status":"not_exposed_by_billing_usage_endpoint","included_allowance":Value::Null,"remaining":Value::Null,"reset":Value::Null},
            "observation":Value::Null,
            "availability":"available",
            "collection_state":"ready",
            "freshness":"provider_reported_period",
            "data_quality":"official",
            "collector_maturity":"experimental",
            "failure_code":Value::Null,
            "diagnostics":["authoritative usage collected; this endpoint does not report included allowance or remaining quota"]
        }],
        "unknowns":["included allowance","remaining quota","quota reset"],
        "evidence":{"mode":"live_gh_api","replay":false,"api_version":API_VERSION,"credentials_present_in_output":false}
    }))
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    #[test]
    fn identity_uses_only_valid_login_and_never_guesses_from_profile() {
        assert_eq!(
            identity_login(&json!({"login":"octocat", "name":"Someone", "token":"private"}))
                .unwrap(),
            "octocat"
        );
        for value in [
            json!({"name":"octocat"}),
            json!({"login":""}),
            json!({"login":"../organization"}),
            json!({"login":"--token"}),
        ] {
            assert!(identity_login(&value).is_err());
        }
    }

    #[test]
    fn cancelled_identity_does_not_start_a_process() {
        let missing = std::env::temp_dir().join("missing-agentmeter-gh.exe");
        let error = authenticated_account(&missing, &AtomicBool::new(true)).unwrap_err();
        assert_eq!(error.code, "cancelled");
    }
}
