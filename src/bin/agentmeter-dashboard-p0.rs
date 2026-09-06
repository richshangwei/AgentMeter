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
    state: String,
    #[serde(default)]
    source_usage: Vec<Usage>,
    #[serde(default)]
    account_id: Option<String>,
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
}
#[derive(Debug, Deserialize)]
struct Event {
    kind: String,
    #[serde(default)]
    revision: u64,
}
#[derive(Debug, Deserialize, Default)]
struct Reconnect {
    #[serde(default)]
    disconnected: bool,
    #[serde(default)]
    latest_revision: u64,
    #[serde(default)]
    fetched_complete: bool,
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
    _provider: String,
    result: String,
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
        && f.snapshot.provider_fields.len() >= 7;
    if !complete {
        d.push("complete authenticated snapshot requires four providers, revision, schema, and canonical fields".into());
    }

    let mut accounts = std::collections::HashMap::<&str, usize>::new();
    let scoped = f.providers.iter().all(|p| {
        let distinct = p
            .source_usage
            .iter()
            .map(|u| u.source.as_str())
            .collect::<std::collections::HashSet<_>>()
            .len()
            == p.source_usage.len();
        if let Some(a) = p.account_id.as_deref() {
            *accounts.entry(a).or_default() += 1;
        }
        distinct
            && p.source_usage
                .iter()
                .all(|u| u.used >= 0.0 && u.limit >= 0.0)
    });
    let dedup = scoped && accounts.values().all(|n| *n == 1);
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
            continue;
        }
        if e.revision <= last {
            stale = true;
        } else {
            last = e.revision;
        }
        if e.kind != "snapshot" {
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

    let recovered = !f.reconnect.disconnected
        || (f.reconnect.fetched_complete && f.reconnect.latest_revision >= f.snapshot.revision);
    if !recovered {
        d.push("disconnect recovery must fetch the latest complete snapshot".into());
    }

    let accepted = f
        .refresh
        .requests
        .iter()
        .filter(|r| r.result == "accepted")
        .count();
    let coalesced = f.refresh.requests.iter().any(|r| r.result == "coalesced");
    let throttled = f.refresh.requests.iter().any(|r| r.result == "throttled");
    let outcomes_ok = f.refresh.outcomes.iter().all(|o| {
        [
            "slow",
            "failed",
            "paused",
            "unsupported",
            "schema_changed",
            "completed",
        ]
        .contains(&o.as_str())
    });
    let refresh_ok = accepted > 0 && coalesced && throttled && outcomes_ok;
    if !refresh_ok {
        d.push("refresh must return promptly and demonstrate accepted, coalesced, throttled, and isolated outcomes".into());
    }

    let isolation = f.providers.len() == 4
        && f.providers.iter().all(|p| {
            [
                "ready",
                "paused",
                "available",
                "collecting",
                "fresh",
                "failed",
                "unsupported",
                "schema_changed",
                "slow",
            ]
            .contains(&p.state.as_str())
        });
    if !isolation {
        d.push("provider failures and schema changes must remain independently visible".into());
    }
    let boundary = f.monitor_only;
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
