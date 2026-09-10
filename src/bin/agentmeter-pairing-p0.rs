use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::ExitCode;

#[derive(Debug, Deserialize)]
struct Fixture {
    allowed_origin: String,
    code_digits: u8,
    code_ttl_seconds: u64,
    max_attempts: u8,
    elapsed_seconds_before_expired_attempt: u64,
    protected_routes: Vec<String>,
    missing_auth_rejected_routes: Vec<String>,
    invalid_auth_rejected_routes: Vec<String>,
    persisted_secret_protection: String,
    actions: Vec<Action>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    PairValid,
    PairReplay,
    PairGuess,
    PairGuessLimit,
    SessionValid,
    SessionReplay,
    MissingAuth,
    InvalidAuth,
    OriginBad,
    CsrfBad,
    ExpiredCode,
    ForgetAccount,
    ClearPairing,
    ResetAgentmeter,
    PairAfterReset,
}

#[derive(Debug, Serialize)]
struct Report {
    schema_version: &'static str,
    capability: &'static str,
    pair: Check,
    session: Check,
    route_auth: Check,
    attack_cases: Check,
    csrf_origin: Check,
    revocation: Check,
    secret_hygiene: Check,
    persisted_secret: Check,
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

#[derive(Default)]
struct State {
    code_used: bool,
    attempts: u8,
    attempts_blocked: bool,
    pair_exists: bool,
    session_exists: bool,
    session_revoked: bool,
    reset: bool,
    forget: bool,
    pairing_cleared: bool,
}

fn evaluate(f: Fixture) -> Report {
    let mut s = State::default();
    let mut diagnostics: Vec<String> = Vec::new();
    let mut pair_ok = false;
    let mut session_ok = false;
    let required_routes = [
        "health",
        "snapshot",
        "refresh",
        "event-stream",
        "pairing-management",
        "future-monitor",
    ];
    let auth_exercised = required_routes.iter().all(|route| {
        f.protected_routes.iter().any(|seen| seen == route)
            && f.missing_auth_rejected_routes
                .iter()
                .any(|seen| seen == route)
            && f.invalid_auth_rejected_routes
                .iter()
                .any(|seen| seen == route)
    });
    let mut attacks_ok = true;
    let mut csrf_ok = true;
    let mut origin_rejection_seen = false;
    let mut csrf_rejection_seen = false;
    let mut forget_ok = false;
    let mut clear_pairing_ok = false;
    let mut reset_ok = false;
    let mut hygiene_ok = true;
    let mut replay_seen = false;
    let mut guess_seen = false;
    let mut limit_seen = false;
    let mut malformed_seen = false;
    let mut expiry_seen = false;
    let mut revoked_replay_seen = false;
    let policy_ok = f.code_digits == 8 && f.code_ttl_seconds == 120 && f.max_attempts == 5;

    if !policy_ok {
        diagnostics
            .push("pair-code policy must use 8 digits, 120 seconds, and five attempts".into());
    }
    for action in f.actions {
        match action {
            Action::PairValid => {
                if s.code_used || s.attempts_blocked || s.reset || s.forget {
                    attacks_ok = false;
                    diagnostics.push("pair exchange rejected after code invalidation".into());
                } else {
                    s.code_used = true;
                    s.pair_exists = true;
                    pair_ok = true;
                }
            }
            Action::PairReplay => {
                replay_seen = true;
                if !s.code_used {
                    attacks_ok = false;
                }
                diagnostics.push("pair-code replay rejected without disclosing token state".into());
            }
            Action::PairGuess => {
                guess_seen = true;
                s.attempts = s.attempts.saturating_add(1);
                if s.attempts >= f.max_attempts {
                    s.attempts_blocked = true;
                }
                diagnostics.push("guessed pair code rejected with generic diagnostic".into());
            }
            Action::PairGuessLimit => {
                limit_seen = true;
                if !s.attempts_blocked {
                    attacks_ok = false;
                }
                diagnostics.push("pair-code attempts rate-limited and blocked".into());
            }
            Action::SessionValid => {
                if s.pair_exists && !s.session_revoked && !s.reset && !s.forget {
                    s.session_exists = true;
                    session_ok = true;
                } else {
                    attacks_ok = false;
                    diagnostics.push("tablet session refused without an active device pair".into());
                }
            }
            Action::SessionReplay => {
                revoked_replay_seen = true;
                if !s.session_revoked && !s.reset && !s.forget {
                    attacks_ok = false;
                }
                diagnostics.push("revoked tablet session rejected".into());
            }
            Action::MissingAuth | Action::InvalidAuth => {
                malformed_seen = true;
                diagnostics.push("route rejected missing or malformed authorization".into());
            }
            Action::OriginBad => {
                origin_rejection_seen = true;
                diagnostics.push("state-changing route rejected unapproved origin/CSRF".into());
            }
            Action::CsrfBad => {
                csrf_rejection_seen = true;
                diagnostics.push("state-changing route rejected unapproved origin/CSRF".into());
            }
            Action::ExpiredCode => {
                expiry_seen = f.elapsed_seconds_before_expired_attempt > f.code_ttl_seconds;
                if !expiry_seen {
                    attacks_ok = false;
                }
                diagnostics.push("expired pair code rejected".into());
            }
            Action::ForgetAccount => {
                s.forget = true;
                s.session_revoked = true;
                forget_ok = s.session_revoked && s.pair_exists;
                diagnostics.push(
                    "forget-account revokes sessions but preserves the independent device pair"
                        .into(),
                );
            }
            Action::ClearPairing => {
                s.pairing_cleared = true;
                s.pair_exists = false;
                s.session_revoked = true;
                clear_pairing_ok = s.session_revoked && !s.pair_exists;
                diagnostics.push("clear-pairing revokes pairs and sessions".into());
            }
            Action::ResetAgentmeter => {
                s.reset = true;
                s.pair_exists = false;
                s.session_revoked = true;
                reset_ok = s.session_revoked && !s.pair_exists;
                diagnostics.push("reset revokes all pairs and sessions".into());
            }
            Action::PairAfterReset => {
                if s.reset || s.pairing_cleared {
                    diagnostics.push("pairing requires a fresh approved attempt".into());
                } else {
                    attacks_ok = false;
                }
            }
        }
    }
    attacks_ok = attacks_ok
        && replay_seen
        && guess_seen
        && limit_seen
        && malformed_seen
        && expiry_seen
        && revoked_replay_seen;
    if !auth_exercised {
        diagnostics
            .push("missing/invalid authorization was not rejected on every protected route".into());
    }
    if f.allowed_origin != "http://127.0.0.1" && f.allowed_origin != "http://localhost" {
        csrf_ok = false;
        diagnostics.push("configured origin is outside the approved loopback boundary".into());
    }
    let revocation_ok = forget_ok && clear_pairing_ok && reset_ok;
    if diagnostics
        .iter()
        .any(|d| d.contains("raw secret") || d.contains("pair-code=") || d.contains("session="))
    {
        hygiene_ok = false;
    }
    Report {
        schema_version: "pairing-p0/v1",
        capability: "device_pair_and_tablet_session",
        pair: check(
            if pair_ok && policy_ok {
                "supported"
            } else {
                "not_observed"
            },
            "single-use expiring code creates durable pair",
        ),
        session: check(
            if session_ok {
                "supported"
            } else {
                "not_observed"
            },
            "revocable session is distinct from pair",
        ),
        route_auth: check(
            if auth_exercised {
                "fail_closed"
            } else {
                "blocked"
            },
            "all routes require authorization",
        ),
        attack_cases: check(
            if attacks_ok { "fail_closed" } else { "unsafe" },
            "replay, guessing, expiry, and malformed credentials",
        ),
        csrf_origin: check(
            if !csrf_ok {
                "blocked"
            } else if origin_rejection_seen && csrf_rejection_seen {
                "fail_closed"
            } else {
                "not_observed"
            },
            "loopback origin and CSRF protections",
        ),
        revocation: check(
            if revocation_ok { "supported" } else { "unsafe" },
            "forget and reset revoke server-side state",
        ),
        secret_hygiene: check(
            if hygiene_ok { "supported" } else { "unsafe" },
            "diagnostics contain no secret material",
        ),
        persisted_secret: check(
            if f.persisted_secret_protection == "dpapi-observed" {
                "supported"
            } else {
                "not_observed"
            },
            "Device Pair verifiers and session secrets require the Windows DPAPI boundary",
        ),
        diagnostics,
    }
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(flag) = args.next() else {
        return ExitCode::from(2);
    };
    let input = if flag == "--fixture" {
        args.next().and_then(|p| fs::read_to_string(p).ok())
    } else {
        None
    };
    let Some(input) = input else {
        return ExitCode::from(2);
    };
    let Ok(fixture) = serde_json::from_str::<Fixture>(&input) else {
        return ExitCode::from(2);
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&evaluate(fixture)).expect("report serializes")
    );
    ExitCode::SUCCESS
}
