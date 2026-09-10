//! Fixture-driven proof of the Dashboard Snapshot/SSE/refresh contract.
//! This deliberately models the transport boundary without requiring a tablet.
use serde::{Deserialize, Serialize};
use std::{env, fs, process::ExitCode};

#[derive(Debug, Deserialize)]
struct Fixture {
    #[serde(default)]
    authenticated: bool,
    #[serde(default = "four")]
    providers: Vec<Provider>,
    #[serde(default)]
    snapshot: Snapshot,
    #[serde(default)]
    events: Vec<Event>,
    #[serde(default)]
    reconnect: Reconnect,
    #[serde(default)]
    refresh: Refresh,
    #[serde(default = "yes")]
    monitor_only: bool,
    #[serde(default)]
    tablet_routes: Vec<String>,
    #[serde(default)]
    measurements: Option<Measurements>,
}
fn four() -> Vec<Provider> {
    vec![]
}
fn yes() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct Provider {
    id: String,
    signals: Signals,
    #[serde(default)]
    source_usage: Vec<Usage>,
    #[serde(default)]
    account_id: Option<String>,
    #[serde(default)]
    quota_windows: Vec<QuotaWindow>,
}
#[derive(Debug, Deserialize)]
struct Signals {
    setup: String,
    paused: bool,
    availability: String,
    collection: String,
    freshness: String,
    quality: String,
    maturity: String,
}
#[derive(Debug, Deserialize)]
struct QuotaWindow {
    bucket: String,
}
#[derive(Debug, Deserialize)]
struct Usage {
    source: String,
    used: f64,
    limit: f64,
}
#[derive(Debug, Deserialize, Default)]
struct Snapshot {
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    schema: String,
    #[serde(default)]
    canonical_setup: String,
    #[serde(default)]
    provider_fields: Vec<String>,
    #[serde(default)]
    stream_id: String,
    #[serde(default)]
    generated_at: String,
}
#[derive(Debug, Deserialize)]
struct Event {
    kind: String,
    #[serde(default)]
    revision: u64,
    #[serde(default)]
    stream_id: String,
    #[serde(default)]
    complete: bool,
}
#[derive(Debug, Deserialize, Default)]
struct Reconnect {
    #[serde(default)]
    disconnected: bool,
    #[serde(default)]
    latest_revision: u64,
    #[serde(default)]
    fetched_complete: bool,
    #[serde(default)]
    reasons: Vec<String>,
}
#[derive(Debug, Deserialize, Default)]
struct Refresh {
    #[serde(default)]
    requests: Vec<RefreshRequest>,
    #[serde(default)]
    outcomes: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct RefreshRequest {
    #[serde(rename = "provider")]
    provider: String,
    result: String,
    #[serde(default)]
    status_code: u16,
    #[serde(default)]
    latency_ms: u64,
}
#[derive(Debug, Deserialize)]
struct Measurements {
    delivery_latency_ms_p95: u64,
    reconnect_ms: u64,
    idle_memory_mb: f64,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: &'static str,
    capability: &'static str,
    complete_snapshot: Check,
    source_scoping: Check,
    sse_ordering: Check,
    reconnect: Check,
    async_refresh: Check,
    failure_isolation: Check,
    monitor_boundary: Check,
    measurements: Check,
    diagnostics: Vec<String>,
}
#[derive(Debug, Serialize)]
struct Check {
    status: &'static str,
    detail: String,
}
fn check(status: &'static str, detail: impl Into<String>) -> Check {
    Check {
        status,
        detail: detail.into(),
    }
}

fn evaluate(f: Fixture) -> Report {
    let mut d = Vec::new();
    let ids: Vec<&str> = f.providers.iter().map(|p| p.id.as_str()).collect();
    let expected = ["codex", "claude", "copilot", "antigravity"];
    let complete = f.authenticated
        && ids.len() == 4
        && expected.iter().all(|id| ids.contains(id))
        && f.snapshot.revision > 0
        && f.snapshot.schema == "dashboard-snapshot/v1"
        && f.snapshot.canonical_setup == "canonical"
        && !f.snapshot.stream_id.is_empty()
        && !f.snapshot.generated_at.is_empty()
        && [
            "setup",
            "paused",
            "availability",
            "collection",
            "freshness",
            "quality",
            "maturity",
        ]
        .iter()
        .all(|field| f.snapshot.provider_fields.iter().any(|seen| seen == field));
    if !complete {
        d.push("complete authenticated snapshot requires four providers, revision, schema, and canonical fields".into());
    }

    let mut multi_source_dedup_seen = false;
    let scoped = f.providers.iter().all(|p| {
        let distinct = p
            .source_usage
            .iter()
            .map(|u| u.source.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len()
            == p.source_usage.len();
        let quota_keys = p
            .quota_windows
            .iter()
            .map(|q| q.bucket.as_str())
            .collect::<std::collections::HashSet<_>>();
        if p.account_id.is_some() && p.source_usage.len() > 1 && p.quota_windows.len() == 1 {
            multi_source_dedup_seen = true;
        }
        distinct
            && quota_keys.len() == p.quota_windows.len()
            && p.source_usage
                .iter()
                .all(|u| u.used >= 0.0 && u.limit >= 0.0)
    });
    let dedup = scoped && multi_source_dedup_seen;
    if !dedup {
        d.push(
            "source-scoped usage or approved Provider Account deduplication is ambiguous".into(),
        );
    }

    let mut last = 0;
    let mut stale = false;
    let mut heartbeat = false;
    let mut complete_events = true;
    for e in &f.events {
        if e.kind == "heartbeat" {
            heartbeat = true;
            if e.revision != last || e.stream_id != f.snapshot.stream_id {
                stale = true;
            }
            continue;
        }
        if e.revision <= last {
            stale = true;
        } else {
            last = e.revision;
        }
        if e.kind != "snapshot" || !e.complete || e.stream_id != f.snapshot.stream_id {
            complete_events = false;
        }
    }
    let ordered = !f.events.is_empty()
        && heartbeat
        && !stale
        && complete_events
        && last >= f.snapshot.revision;
    if !ordered {
        d.push("SSE must identify complete snapshots by strictly increasing revision and heartbeat events".into());
    }

    let required_reconnects = [
        "disconnect",
        "tablet-sleep",
        "host-restart",
        "expired-session",
        "missed-events",
    ];
    let recovered = (!f.reconnect.disconnected
        || (f.reconnect.fetched_complete && f.reconnect.latest_revision >= f.snapshot.revision))
        && required_reconnects
            .iter()
            .all(|reason| f.reconnect.reasons.iter().any(|seen| seen == reason));
    if !recovered {
        d.push("disconnect recovery must fetch the latest complete snapshot".into());
    }

    let accepted_providers = f
        .refresh
        .requests
        .iter()
        .filter(|r| r.result == "accepted" && r.status_code == 202 && r.latency_ms <= 300)
        .map(|r| r.provider.as_str())
        .collect::<std::collections::HashSet<_>>();
    let accepted = accepted_providers.len();
    let coalesced = accepted_providers.iter().any(|provider| {
        f.refresh
            .requests
            .iter()
            .any(|request| request.provider.as_str() == *provider && request.result == "coalesced")
    });
    let throttled = accepted_providers.iter().any(|provider| {
        f.refresh
            .requests
            .iter()
            .any(|request| request.provider.as_str() == *provider && request.result == "throttled")
    });
    let required_outcomes = [
        "slow",
        "failed",
        "paused",
        "unsupported",
        "schema_changed",
        "completed",
    ];
    let outcomes_ok = required_outcomes
        .iter()
        .all(|outcome| f.refresh.outcomes.iter().any(|seen| seen == outcome));
    let refresh_ok = accepted > 0 && coalesced && throttled && outcomes_ok;
    if !refresh_ok {
        d.push("refresh must return promptly and demonstrate accepted, coalesced, throttled, and isolated outcomes".into());
    }

    let isolation = f.providers.len() == 4
        && f.providers.iter().all(|p| {
            !p.signals.setup.is_empty()
                && !p.signals.availability.is_empty()
                && !p.signals.collection.is_empty()
                && !p.signals.freshness.is_empty()
                && !p.signals.quality.is_empty()
                && !p.signals.maturity.is_empty()
                && (p.signals.paused || !p.signals.collection.is_empty())
        })
        && required_outcomes
            .iter()
            .all(|outcome| f.refresh.outcomes.iter().any(|seen| seen == outcome));
    if !isolation {
        d.push("provider failures and schema changes must remain independently visible".into());
    }
    let allowed_routes = ["dashboard", "history", "events", "refresh"];
    let boundary = f.monitor_only
        && f.tablet_routes
            .iter()
            .all(|route| allowed_routes.contains(&route.as_str()))
        && allowed_routes
            .iter()
            .all(|route| f.tablet_routes.iter().any(|seen| seen == route));
    if !boundary {
        d.push(
            "tablet surface must be monitor-only and expose no credential/config mutation".into(),
        );
    }
    Report {
        schema_version: "dashboard-p0/v1",
        capability: "dashboard_snapshot_streaming_and_async_refresh",
        complete_snapshot: check(
            if complete {
                "supported"
            } else {
                "not_observed"
            },
            "authenticated complete four-provider snapshot",
        ),
        source_scoping: check(
            if dedup { "supported" } else { "constrained" },
            "source-scoped usage with account deduplication",
        ),
        sse_ordering: check(
            if ordered { "supported" } else { "constrained" },
            "revisioned complete snapshots, heartbeat, stale rejection",
        ),
        reconnect: check(
            if recovered {
                "supported"
            } else {
                "constrained"
            },
            "reconnect obtains latest complete snapshot",
        ),
        async_refresh: check(
            if refresh_ok {
                "supported"
            } else {
                "constrained"
            },
            format!("accepted={accepted}, coalesced={coalesced}, throttled={throttled}"),
        ),
        failure_isolation: check(
            if isolation {
                "supported"
            } else {
                "constrained"
            },
            "slow/failed/paused/unsupported/schema-changed collectors isolate",
        ),
        monitor_boundary: check(
            if boundary { "supported" } else { "blocked" },
            "tablet can inspect and request refresh only",
        ),
        measurements: check(
            if f.measurements.as_ref().is_some_and(|m| {
                m.delivery_latency_ms_p95 > 0 && m.reconnect_ms > 0 && m.idle_memory_mb > 0.0
            }) {
                "recorded"
            } else {
                "not_observed"
            },
            "real-device latency, reconnect time, and idle resource use",
        ),
        diagnostics: d,
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    if args.next().as_deref() != Some("--fixture") {
        return ExitCode::from(2);
    }
    let Some(path) = args.next() else {
        return ExitCode::from(2);
    };
    let Ok(text) = fs::read_to_string(path) else {
        return ExitCode::from(2);
    };
    let Ok(fixture) = serde_json::from_str::<Fixture>(&text) else {
        return ExitCode::from(2);
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&evaluate(fixture)).expect("serializes")
    );
    ExitCode::SUCCESS
}
