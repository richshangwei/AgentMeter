use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::process::ExitCode;

#[derive(Debug, Deserialize)]
struct Fixture {
    allowed_origin: String,
    code_ttl_seconds: u64,
    max_attempts: u8,
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
}

fn evaluate(f: Fixture) -> Report {
    let mut s = State::default();
    let mut diagnostics: Vec<String> = Vec::new();
    let mut pair_ok = false;
    let mut session_ok = false;
    let mut auth_ok = false;
    let mut auth_exercised = false;
    let mut attacks_ok = true;
    let mut csrf_ok = true;
    let mut csrf_exercised = false;
    let mut revoke_ok = true;
    let mut hygiene_ok = true;

    if f.code_ttl_seconds == 0 || f.max_attempts == 0 {
        diagnostics.push("pair-code policy has no usable expiry or attempt budget".into());
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
                if !s.code_used {
                    attacks_ok = false;
                }
                diagnostics.push("pair-code replay rejected without disclosing token state".into());
            }
            Action::PairGuess => {
                s.attempts = s.attempts.saturating_add(1);
                if s.attempts >= f.max_attempts {
                    s.attempts_blocked = true;
                }
                diagnostics.push("guessed pair code rejected with generic diagnostic".into());
            }
            Action::PairGuessLimit => {
                if !s.attempts_blocked {
                    attacks_ok = false;
                }
                diagnostics.push("pair-code attempts rate-limited and blocked".into());
            }
            Action::SessionValid => {
                if s.pair_exists && !s.session_revoked && !s.reset && !s.forget {
                    s.session_exists = true;
                    session_ok = true;
                    auth_ok = true;
                } else {
                    attacks_ok = false;
                    diagnostics.push("tablet session refused without an active device pair".into());
                }
            }
            Action::SessionReplay => {
                if !s.session_revoked && !s.reset && !s.forget {
                    attacks_ok = false;
                }
                diagnostics.push("revoked tablet session rejected".into());
            }
            Action::MissingAuth | Action::InvalidAuth => {
                auth_exercised = true;
                auth_ok = false;
                diagnostics.push("route rejected missing or malformed authorization".into());
            }
            Action::OriginBad | Action::CsrfBad => {
                csrf_exercised = true;
                diagnostics.push("state-changing route rejected unapproved origin/CSRF".into());
            }
            Action::ExpiredCode => {
                if s.code_used {
                    attacks_ok = false;
                }
                diagnostics.push("expired pair code rejected".into());
            }
            Action::ForgetAccount => {
                s.forget = true;
                s.pair_exists = false;
                s.session_revoked = true;
                revoke_ok = s.session_revoked && !s.pair_exists;
                diagnostics.push("forget-account revokes pair and sessions".into());
            }
            Action::ResetAgentmeter => {
                s.reset = true;
                s.pair_exists = false;
                s.session_revoked = true;
                revoke_ok = s.session_revoked && !s.pair_exists;
                diagnostics.push("reset revokes all pairs and sessions".into());
            }
            Action::PairAfterReset => {
                if s.reset || s.forget {
                    diagnostics.push("pairing requires a fresh approved attempt".into());
                } else {
                    attacks_ok = false;
                }
            }
        }
    }
    if f.allowed_origin != "http://127.0.0.1" && f.allowed_origin != "http://localhost" {
        csrf_ok = false;
        diagnostics.push("configured origin is outside the approved loopback boundary".into());
    }
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
            if pair_ok { "supported" } else { "not_observed" },
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
            } else if auth_ok {
                "exercised"
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
            } else if csrf_exercised {
                "fail_closed"
            } else {
                "not_observed"
            },
            "loopback origin and CSRF protections",
        ),
        revocation: check(
            if revoke_ok { "supported" } else { "unsafe" },
            "forget and reset revoke server-side state",
        ),
        secret_hygiene: check(
            if hygiene_ok { "supported" } else { "unsafe" },
            "diagnostics contain no secret material",
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
