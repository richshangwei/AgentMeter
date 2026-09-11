use agentmeter_p0::tablet::{MockCollectorBehavior, ServerConfig, TabletServer};
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
#[cfg(windows)]
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::time::Instant;

#[cfg(windows)]
static DURABLE_TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn request(addr: SocketAddr, request: &str) -> String {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn request_bytes(addr: SocketAddr, request: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(1)).unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    response
}

#[test]
fn verified_usb_origin_is_allowed_and_revoked_with_transport() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let port = if server.addr().port() == 8317 {
        8318
    } else {
        8317
    };
    let forwarded = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n");
    assert_eq!(status(&request(server.addr(), &forwarded)), 403);
    server.set_usb_device_port(port);
    assert_eq!(status(&request(server.addr(), &forwarded)), 200);
    let wrong_origin = format!(
        "POST /api/v1/pair HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://evil.test\r\nContent-Length: 2\r\n\r\n{{}}"
    );
    assert_eq!(status(&request(server.addr(), &wrong_origin)), 403);
    let payload = format!(r#"{{"code":"{}"}}"#, server.pair_code_for_setup());
    let pair_request = format!(
        "POST /api/v1/pair HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: http://127.0.0.1:{port}\r\nContent-Length: {}\r\n\r\n{payload}",
        payload.len()
    );
    assert_eq!(status(&request(server.addr(), &pair_request)), 200);
    server.revoke_sessions_for_transport_change();
    assert_eq!(status(&request(server.addr(), &forwarded)), 403);
}

#[test]
fn delayed_fragmented_browser_request_is_not_rejected() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let mut stream = TcpStream::connect(server.addr()).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .unwrap();
    std::thread::sleep(Duration::from_millis(100));
    stream.write_all(b"GET / HTTP/1.1\r\n").unwrap();
    std::thread::sleep(Duration::from_millis(100));
    let tail = format!("Host: {}\r\nConnection: close\r\n\r\n", server.addr());
    let _ = stream.write_all(tail.as_bytes());
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200"), "{response}");
}

#[test]
fn live_store_flows_through_authenticated_http_sse_and_coalesced_refresh() {
    use agentmeter_p0::live_dashboard::LiveDashboard;
    use serde_json::json;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    };
    let live = LiveDashboard::default();
    let target = live.clone();
    let calls = Arc::new(AtomicUsize::new(0));
    let worker_calls = calls.clone();
    let (release, receiver) = mpsc::channel();
    let receiver = Mutex::new(receiver);
    let mut config = ServerConfig::for_tests();
    config.pair_code_ttl = Duration::from_secs(5);
    config.session_ttl = Duration::from_secs(10);
    config.sse_heartbeat = Duration::from_secs(30);
    config.refresh_throttle = Duration::from_secs(5);
    let server = TabletServer::start_live(config, live.clone(), Arc::new(move |source| {
        worker_calls.fetch_add(1, Ordering::SeqCst);
        if source == "claude" { return Err("PRIVATE ERROR".into()); }
        if source == "codex" { receiver.lock().unwrap().recv_timeout(Duration::from_secs(3)).unwrap(); }
        target.publish(source, &json!({"availability":"available","collection_state":"ready","freshness":"fresh","quota_windows":[{"remaining_percent":37,"unit":"percent"}],"secret":"PRIVATE"})).unwrap();
        Ok(())
    })).unwrap();
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &[],
            r#"{"source":"codex"}"#
        )),
        401
    );
    let paired = pair(&server);
    let authorization = format!("Bearer {}", paired.session);
    let auth = [
        ("Authorization", authorization.as_str()),
        ("X-CSRF-Token", paired.csrf.as_str()),
    ];
    let initial = body(&http(&server, "GET", "/api/v1/dashboard", &auth, ""));
    assert_eq!(initial["provider_data"], "live");
    assert_eq!(initial["providers"].as_array().unwrap().len(), 4);
    assert!(
        initial["providers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|p| p["quota_windows"] == json!([]))
    );
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &[("Authorization", authorization.as_str())],
            r#"{"source":"codex"}"#
        )),
        403
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let mut events = open_event_stream(&server, &paired.session);
    read_initial_stream_event(&mut events);
    assert_eq!(
        body(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &auth,
            r#"{"source":"codex"}"#
        ))["result"],
        "accepted"
    );
    assert_eq!(
        body(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &auth,
            r#"{"source":"codex"}"#
        ))["result"],
        "coalesced"
    );
    release.send(()).unwrap();
    events
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut received = String::new();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !received.contains("\"remaining_percent\":37") && Instant::now() < deadline {
        let mut buffer = [0u8; 4096];
        let size = events.read(&mut buffer).unwrap();
        assert!(size > 0);
        received.push_str(&String::from_utf8_lossy(&buffer[..size]));
    }
    assert!(received.contains("\"remaining_percent\":37"), "{received}");
    assert!(!received.contains("PRIVATE"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        body(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &auth,
            r#"{"source":"codex"}"#
        ))["result"],
        "throttled"
    );
    assert_eq!(
        body(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &auth,
            r#"{"source":"antigravity"}"#
        ))["result"],
        "accepted"
    );
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &auth,
            r#"{"source":"arbitrary-command"}"#
        )),
        400
    );
    assert_eq!(
        body(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &auth,
            r#"{"source":"claude"}"#
        ))["result"],
        "accepted"
    );
    let deadline = Instant::now() + Duration::from_secs(1);
    while live.snapshot().1[1]["failure_code"] != "refresh_failed" && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(live.snapshot().1[1]["failure_code"], "refresh_failed");
    assert!(
        !serde_json::to_string(&live.snapshot().1)
            .unwrap()
            .contains("PRIVATE")
    );
    server.forget_account();
    assert_eq!(
        status(&http(&server, "GET", "/api/v1/dashboard", &auth, "")),
        401
    );
}

fn status(response: &str) -> u16 {
    response
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap()
}

fn body(response: &str) -> serde_json::Value {
    serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap()
}

fn header(response: &str, name: &str) -> Option<String> {
    response
        .split_once("\r\n\r\n")
        .unwrap()
        .0
        .lines()
        .find_map(|line| {
            let (found, value) = line.split_once(':')?;
            found
                .eq_ignore_ascii_case(name)
                .then(|| value.trim().to_owned())
        })
}

fn http(
    server: &TabletServer,
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: &str,
) -> String {
    let mut wire_request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {}\r\nOrigin: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        server.addr(),
        server.origin(),
        body.len()
    );
    for (name, value) in headers {
        wire_request.push_str(&format!("{name}: {value}\r\n"));
    }
    wire_request.push_str("\r\n");
    wire_request.push_str(body);
    request(server.addr(), &wire_request)
}

struct PairResult {
    device_pair: String,
    session: String,
    csrf: String,
    pair_csrf: String,
}

#[test]
fn explicit_port_is_reused_after_restart_and_conflicts_never_fall_back() {
    let first = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let origin = first.origin().to_owned();
    let mut config = ServerConfig::for_tests();
    config.listen_port = first.addr().port();
    assert!(
        TabletServer::start(config.clone()).is_err(),
        "occupied explicit port must not silently change"
    );
    drop(first);
    let restarted = TabletServer::start(config).unwrap();
    assert_eq!(restarted.origin(), origin);
    assert!(restarted.addr().ip().is_loopback());
}

#[test]
fn public_client_has_no_secrets_and_browser_post_keeps_authorization_boundary() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    for route in ["/", "/client.js", "/protocol.js", "/recovery.js"] {
        let page = http(&server, "GET", route, &[], "");
        assert_eq!(status(&page), 200);
        assert_eq!(header(&page, "Cache-Control").as_deref(), Some("no-store"));
        assert!(
            header(&page, "Content-Security-Policy")
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
        assert!(!page.contains(&server.pair_code_for_setup()));
    }
    for route in ["/api/v1/dashboard", "/api/v1/events"] {
        let response = http(&server, "POST", route, &[], "{}");
        assert_eq!(status(&response), 401);
        assert_eq!(
            header(&response, "Cache-Control").as_deref(),
            Some("no-store")
        );
    }
    let paired = pair(&server);
    let auth = format!("Bearer {}", paired.session);
    let snapshot = http(
        &server,
        "POST",
        "/api/v1/dashboard",
        &[("Authorization", &auth)],
        "{}",
    );
    assert_eq!(status(&snapshot), 200);
    assert_eq!(body(&snapshot)["providers"].as_array().unwrap().len(), 4);
    let wrong_origin = request(
        server.addr(),
        &format!(
            "POST /api/v1/dashboard HTTP/1.1\r\nHost: {}\r\nOrigin: http://untrusted.invalid\r\nAuthorization: {auth}\r\nContent-Length: 2\r\n\r\n{{}}",
            server.addr()
        ),
    );
    assert_eq!(status(&wrong_origin), 403);
}

#[test]
fn public_brand_and_provider_icons_are_served_as_png_without_relaxing_navigation() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    for route in [
        "/assets/agentmeter-icon.png",
        "/assets/codex-icon.png",
        "/assets/claude-icon.png",
        "/assets/copilot-icon.png",
        "/assets/antigravity-icon.png",
        "/assets/empty-cloud.png",
    ] {
        let raw = request_bytes(
            server.addr(),
            &format!("GET {route} HTTP/1.1\r\nHost: {}\r\n\r\n", server.addr()),
        );
        let split = raw
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
            .unwrap();
        let headers = String::from_utf8_lossy(&raw[..split]);
        assert!(headers.starts_with("HTTP/1.1 200 OK"), "{headers}");
        assert!(headers.contains("Content-Type: image/png"), "{headers}");
        assert_eq!(&raw[split + 4..split + 12], b"\x89PNG\r\n\x1a\n");
    }
    let query = http(
        &server,
        "GET",
        "/assets/agentmeter-icon.png?token=secret",
        &[],
        "",
    );
    assert_eq!(status(&query), 400);
}

#[test]
fn transport_status_advances_only_after_authenticated_activity() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let initial = server.transport_status();
    assert!(!initial.authenticated_session_established);
    assert_eq!(initial.authenticated_request_count, 0);

    let rejected = http(&server, "GET", "/health", &[], "");
    assert_eq!(status(&rejected), 401);
    assert_eq!(server.transport_status(), initial);

    let paired = pair(&server);
    let after_pair = server.transport_status();
    assert!(after_pair.authenticated_session_established);
    assert_eq!(after_pair.authenticated_request_count, 1);
    assert_eq!(
        after_pair.last_authenticated_route.as_deref(),
        Some("/api/v1/pair")
    );
    assert!(after_pair.last_authenticated_at_unix_ms.is_some());

    let auth = format!("Bearer {}", paired.session);
    let health = http(&server, "GET", "/health", &[("Authorization", &auth)], "");
    assert_eq!(status(&health), 200);
    let online = server.transport_status();
    assert_eq!(online.authenticated_request_count, 2);
    assert_eq!(online.last_authenticated_route.as_deref(), Some("/health"));
}

#[test]
fn transport_change_revokes_sessions_but_preserves_the_device_pair() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let paired = pair(&server);
    assert!(server.transport_status().authenticated_session_established);

    server.revoke_sessions_for_transport_change();
    assert!(!server.transport_status().authenticated_session_established);
    assert_eq!(
        status(&http(
            &server,
            "GET",
            "/health",
            &[("Authorization", &format!("Bearer {}", paired.session))],
            "",
        )),
        401
    );

    let renewed = exchange_pair(&server, &paired);
    assert_eq!(status(&renewed), 200);
    assert_ne!(body(&renewed)["tablet_session"], paired.session);
    assert!(server.transport_status().authenticated_session_established);
}

fn pair(server: &TabletServer) -> PairResult {
    let code = server.pair_code_for_setup();
    let response = http(
        server,
        "POST",
        "/api/v1/pair",
        &[],
        &format!(r#"{{"code":"{code}"}}"#),
    );
    assert_eq!(status(&response), 200, "{response}");
    let cookie = header(&response, "Set-Cookie").unwrap();
    assert_eq!(
        header(&response, "Cache-Control").as_deref(),
        Some("no-store")
    );
    assert_eq!(
        header(&response, "Referrer-Policy").as_deref(),
        Some("no-referrer")
    );
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Strict"));
    assert!(cookie.contains("Path=/api/v1/session"));
    assert!(cookie.contains("Max-Age=31536000"));
    let device_pair = cookie
        .split(';')
        .next()
        .unwrap()
        .strip_prefix("device_pair=")
        .unwrap()
        .to_owned();
    let value = body(&response);
    PairResult {
        device_pair,
        session: value["tablet_session"].as_str().unwrap().to_owned(),
        csrf: value["csrf_token"].as_str().unwrap().to_owned(),
        pair_csrf: value["pair_csrf_token"].as_str().unwrap().to_owned(),
    }
}

#[test]
fn secret_bearing_query_targets_are_rejected_without_echo_or_diagnostic_leakage() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let pair_code = server.pair_code_for_setup();
    let pair_query = format!("/api/v1/pair?code={pair_code}");
    let rejected_pair = http(&server, "POST", &pair_query, &[], "{}");
    assert_eq!(status(&rejected_pair), 400);
    assert_eq!(body(&rejected_pair)["error"], "query_not_allowed");
    assert_eq!(
        header(&rejected_pair, "Cache-Control").as_deref(),
        Some("no-store")
    );
    assert!(!rejected_pair.contains(&pair_code));

    let paired = pair(&server);
    let dashboard_query = format!("/api/v1/dashboard?session={}", paired.session);
    let authorization = format!("Bearer {}", paired.session);
    let rejected_dashboard = http(
        &server,
        "GET",
        &dashboard_query,
        &[("Authorization", &authorization)],
        "",
    );
    assert_eq!(status(&rejected_dashboard), 400);
    assert!(!rejected_dashboard.contains(&paired.session));

    let diagnostics = serde_json::to_string(&server.diagnostics()).unwrap();
    for secret in [
        &pair_code,
        &paired.device_pair,
        &paired.session,
        &paired.csrf,
        &paired.pair_csrf,
    ] {
        assert!(!diagnostics.contains(secret));
    }
    assert!(diagnostics.contains("query_rejected"));
    assert!(!diagnostics.contains('?'));
}

#[cfg(windows)]
fn durable_test_config() -> (std::path::PathBuf, ServerConfig) {
    let directory = std::env::temp_dir().join(format!(
        "agentmeter-durable-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        DURABLE_TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir(&directory).unwrap();
    let mut config = ServerConfig::for_tests();
    config.pair_code_ttl = Duration::from_secs(10);
    config.pair_store_path = Some(directory.join("pairs.dpapi"));
    (directory, config)
}

fn exchange_pair(server: &TabletServer, paired: &PairResult) -> String {
    http(
        server,
        "POST",
        "/api/v1/session",
        &[
            ("Cookie", &format!("device_pair={}", paired.device_pair)),
            ("X-CSRF-Token", &paired.pair_csrf),
        ],
        "{}",
    )
}

#[test]
#[cfg(windows)]
fn durable_pair_survives_restart_but_session_does_not_and_clear_is_persistent() {
    let (directory, config) = durable_test_config();
    let server = TabletServer::start(config.clone()).unwrap();
    let paired = pair(&server);
    let ciphertext = std::fs::read(config.pair_store_path.as_ref().unwrap()).unwrap();
    for secret in [
        &paired.device_pair,
        &paired.session,
        &paired.pair_csrf,
        &paired.csrf,
    ] {
        assert!(
            !ciphertext
                .windows(secret.len())
                .any(|window| window == secret.as_bytes())
        );
    }
    server.forget_account();
    assert_eq!(status(&exchange_pair(&server, &paired)), 200);
    drop(server);

    let restarted = TabletServer::start(config.clone()).unwrap();
    assert_eq!(
        status(&http(
            &restarted,
            "GET",
            "/health",
            &[("Authorization", &format!("Bearer {}", paired.session)),],
            ""
        )),
        401
    );
    let renewed = exchange_pair(&restarted, &paired);
    assert_eq!(status(&renewed), 200);
    assert_ne!(body(&renewed)["tablet_session"], paired.session);
    restarted.clear_pairing().unwrap();
    drop(restarted);

    let cleared = TabletServer::start(config.clone()).unwrap();
    assert_eq!(status(&exchange_pair(&cleared, &paired)), 401);
    let new_pair = pair(&cleared);
    cleared.reset_agentmeter().unwrap();
    drop(cleared);
    let reset = TabletServer::start(config).unwrap();
    assert_eq!(status(&exchange_pair(&reset, &new_pair)), 401);
    drop(reset);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(windows)]
fn durable_store_lock_corruption_and_failed_pair_write_fail_closed() {
    let (directory, config) = durable_test_config();
    let path = config.pair_store_path.as_ref().unwrap();
    let server = TabletServer::start(config.clone()).unwrap();
    assert!(
        TabletServer::start(config.clone()).is_err(),
        "duplicate writer must fail"
    );
    // A directory at the destination makes the atomic replacement fail.
    std::fs::create_dir(path).unwrap();
    let code = server.pair_code_for_setup();
    let response = http(
        &server,
        "POST",
        "/api/v1/pair",
        &[],
        &format!(r#"{{"code":"{code}"}}"#),
    );
    assert_eq!(status(&response), 503);
    assert!(header(&response, "Set-Cookie").is_none());
    assert!(body(&response)["tablet_session"].is_null());
    drop(server);
    std::fs::remove_dir(path).unwrap();
    std::fs::write(path, b"corrupt-ciphertext").unwrap();
    assert!(TabletServer::start(config.clone()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"corrupt-ciphertext");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
#[cfg(windows)]
fn failed_persistent_revocation_is_reported_and_disables_current_authorization() {
    use std::os::windows::fs::OpenOptionsExt;
    let (directory, config) = durable_test_config();
    let server = TabletServer::start(config.clone()).unwrap();
    let paired = pair(&server);
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(config.pair_store_path.as_ref().unwrap())
        .unwrap();
    assert!(server.clear_pairing().is_err());
    assert_eq!(
        status(&http(
            &server,
            "GET",
            "/health",
            &[("Authorization", &format!("Bearer {}", paired.session)),],
            ""
        )),
        401
    );
    assert_eq!(status(&exchange_pair(&server, &paired)), 503);
    drop(locked);
    // Retry the failed durable operation explicitly before allowing restart.
    server.clear_pairing().unwrap();
    drop(server);
    let restarted = TabletServer::start(config).unwrap();
    assert_eq!(status(&exchange_pair(&restarted, &paired)), 401);
    drop(restarted);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn server_is_loopback_only_and_health_requires_authorization() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    assert_eq!(server.addr().ip(), IpAddr::from([127, 0, 0, 1]));

    let response = request(
        server.addr(),
        &format!(
            "GET /health HTTP/1.1\r\nHost: {}\r\nOrigin: {}\r\nConnection: close\r\n\r\n",
            server.addr(),
            server.origin()
        ),
    );
    assert!(response.starts_with("HTTP/1.1 401 Unauthorized"));
    assert!(!response.contains("providers"));
}

#[test]
fn pair_code_is_single_use_expires_and_locks_after_five_attempts() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let code = server.pair_code_for_setup();
    assert_eq!(code.len(), 8);
    assert!(code.bytes().all(|byte| byte.is_ascii_digit()));
    let paired = pair(&server);
    assert_ne!(paired.device_pair, paired.session);

    let replay = http(
        &server,
        "POST",
        "/api/v1/pair",
        &[],
        &format!(r#"{{"code":"{code}"}}"#),
    );
    assert_eq!(status(&replay), 401);
    assert!(!replay.contains(&code));
    assert!(!replay.contains(&paired.session));

    let code = server.rotate_pair_code();
    let wrong_code = if code == "00000000" {
        "00000001"
    } else {
        "00000000"
    };
    for _ in 0..5 {
        assert_eq!(
            status(&http(
                &server,
                "POST",
                "/api/v1/pair",
                &[],
                &format!(r#"{{"code":"{wrong_code}"}}"#),
            )),
            401
        );
    }
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/pair",
            &[],
            &format!(r#"{{"code":"{code}"}}"#),
        )),
        401
    );

    let mut config = ServerConfig::for_tests();
    config.pair_code_ttl = Duration::from_millis(10);
    let expiring = TabletServer::start(config).unwrap();
    let expired_code = expiring.pair_code_for_setup();
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(
        status(&http(
            &expiring,
            "POST",
            "/api/v1/pair",
            &[],
            &format!(r#"{{"code":"{expired_code}"}}"#),
        )),
        401
    );
}

#[test]
fn every_private_route_rejects_missing_and_malformed_sessions() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    for (method, route) in [
        ("GET", "/health"),
        ("GET", "/api/v1/dashboard"),
        ("GET", "/api/v1/history"),
        ("GET", "/api/v1/events"),
        ("POST", "/api/v1/refresh"),
        ("GET", "/api/v1/pairs"),
        ("GET", "/api/v1/monitor/ping"),
    ] {
        let missing = http(&server, method, route, &[], "{}");
        assert_eq!(status(&missing), 401, "missing auth on {route}");
        let malformed = http(
            &server,
            method,
            route,
            &[("Authorization", "Bearer malformed")],
            "{}",
        );
        assert_eq!(status(&malformed), 401, "malformed auth on {route}");
        assert!(!malformed.contains("malformed"));
    }
}

fn open_event_stream(server: &TabletServer, session: &str) -> TcpStream {
    let mut stream = TcpStream::connect_timeout(&server.addr(), Duration::from_secs(1)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    let request = format!(
        "GET /api/v1/events HTTP/1.1\r\nHost: {}\r\nOrigin: {}\r\nAuthorization: Bearer {session}\r\nConnection: close\r\n\r\n",
        server.addr(),
        server.origin()
    );
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    stream
}

fn read_initial_stream_event(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    while !bytes.ends_with(b": heartbeat\n\n") {
        let mut byte = [0_u8; 1];
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    let response = String::from_utf8(bytes).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK"));
    response
}

#[test]
fn revocation_closes_active_streams_without_waiting_for_heartbeat() {
    for action in ["forget", "clear", "reset", "rotate", "shutdown"] {
        let mut config = ServerConfig::for_tests();
        config.sse_heartbeat = Duration::from_secs(15);
        let server = TabletServer::start(config).unwrap();
        let paired = pair(&server);
        let mut stream = open_event_stream(&server, &paired.session);
        read_initial_stream_event(&mut stream);
        match action {
            "forget" => server.forget_account(),
            "clear" => server.clear_pairing().unwrap(),
            "reset" => server.reset_agentmeter().unwrap(),
            "rotate" => {
                let response = http(
                    &server,
                    "POST",
                    "/api/v1/session",
                    &[
                        ("Cookie", &format!("device_pair={}", paired.device_pair)),
                        ("X-CSRF-Token", &paired.pair_csrf),
                    ],
                    "{}",
                );
                assert_eq!(status(&response), 200);
            }
            "shutdown" => drop(server),
            _ => unreachable!(),
        }
        let mut remaining = String::new();
        // A one-second socket timeout makes a delayed 15-second heartbeat fail.
        stream.read_to_string(&mut remaining).unwrap();
        assert!(remaining.is_empty(), "data after {action}: {remaining}");
    }
}

#[test]
fn active_event_stream_closes_after_session_revocation_or_expiry() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let paired = pair(&server);
    let mut revoked_stream = open_event_stream(&server, &paired.session);
    let initial = read_initial_stream_event(&mut revoked_stream);
    assert!(initial.contains("text/event-stream"));
    server.forget_account();
    let mut after_revoke = String::new();
    revoked_stream.read_to_string(&mut after_revoke).unwrap();
    assert!(
        server
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.failure_code == "stream_authorization_ended")
    );

    let mut config = ServerConfig::for_tests();
    config.session_ttl = Duration::from_millis(35);
    config.sse_heartbeat = Duration::from_millis(15);
    let expiring_server = TabletServer::start(config).unwrap();
    let expiring_pair = pair(&expiring_server);
    let mut expiring_stream = open_event_stream(&expiring_server, &expiring_pair.session);
    let _ = read_initial_stream_event(&mut expiring_stream);
    std::thread::sleep(Duration::from_millis(45));
    let mut after_expiry = String::new();
    expiring_stream.read_to_string(&mut after_expiry).unwrap();
    assert!(
        expiring_server
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.failure_code == "stream_authorization_ended")
    );
}

#[test]
fn origin_csrf_session_rotation_and_desktop_revocation_fail_closed() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let paired = pair(&server);
    let authorization = format!("Bearer {}", paired.session);

    let bad_origin_request = format!(
        "GET /api/v1/dashboard HTTP/1.1\r\nHost: {}\r\nOrigin: https://evil.example\r\nAuthorization: {authorization}\r\nConnection: close\r\n\r\n",
        server.addr()
    );
    assert_eq!(status(&request(server.addr(), &bad_origin_request)), 403);

    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &[("Authorization", &authorization)],
            r#"{"source":"codex"}"#,
        )),
        403
    );
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &[("Authorization", &authorization), ("X-CSRF-Token", "wrong"),],
            r#"{"source":"codex"}"#,
        )),
        403
    );
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/refresh",
            &[
                ("Authorization", &authorization),
                ("X-CSRF-Token", &paired.csrf),
            ],
            r#"{"source":"codex"}"#,
        )),
        202
    );

    let cookie = format!("device_pair={}", paired.device_pair);
    let missing_session_csrf = http(
        &server,
        "POST",
        "/api/v1/session",
        &[("Cookie", &cookie)],
        "{}",
    );
    assert_eq!(status(&missing_session_csrf), 403);
    let rotated = http(
        &server,
        "POST",
        "/api/v1/session",
        &[("Cookie", &cookie), ("X-CSRF-Token", &paired.pair_csrf)],
        "{}",
    );
    assert_eq!(status(&rotated), 200);
    assert_eq!(
        header(&rotated, "Cache-Control").as_deref(),
        Some("no-store")
    );
    let rotated_value = body(&rotated);
    let rotated_session = rotated_value["tablet_session"].as_str().unwrap();
    assert_ne!(rotated_session, paired.session);
    assert_eq!(
        status(&http(
            &server,
            "GET",
            "/health",
            &[("Authorization", &authorization)],
            "",
        )),
        401,
        "rotated session must revoke the old session"
    );

    let rotated_auth = format!("Bearer {rotated_session}");
    server.forget_account();
    assert_eq!(
        status(&http(
            &server,
            "GET",
            "/health",
            &[("Authorization", &rotated_auth)],
            "",
        )),
        401
    );
    let after_forget = http(
        &server,
        "POST",
        "/api/v1/session",
        &[("Cookie", &cookie), ("X-CSRF-Token", &paired.pair_csrf)],
        "{}",
    );
    assert_eq!(
        status(&after_forget),
        200,
        "Forget Account preserves Device Pair"
    );

    server.clear_pairing().unwrap();
    assert_eq!(
        status(&http(
            &server,
            "POST",
            "/api/v1/session",
            &[("Cookie", &cookie), ("X-CSRF-Token", &paired.pair_csrf)],
            "{}",
        )),
        401
    );
    server.reset_agentmeter().unwrap();

    let diagnostics = serde_json::to_string(&server.diagnostics()).unwrap();
    for expected in [
        "origin_rejected",
        "csrf_rejected",
        "authorization_rejected",
        "device_pair_rejected",
    ] {
        assert!(diagnostics.contains(expected), "missing {expected}");
    }
    assert!(!diagnostics.contains(&paired.session));
    assert!(!diagnostics.contains(&paired.device_pair));
    assert!(!diagnostics.to_ascii_lowercase().contains("authorization:"));
}

#[test]
fn expired_sessions_and_reset_credentials_are_rejected() {
    let mut config = ServerConfig::for_tests();
    config.session_ttl = Duration::from_millis(10);
    let server = TabletServer::start(config).unwrap();
    let paired = pair(&server);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(
        status(&http(
            &server,
            "GET",
            "/health",
            &[("Authorization", &format!("Bearer {}", paired.session))],
            "",
        )),
        401
    );
    assert!(!server.transport_status().authenticated_session_established);

    let reset_server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let reset_pair = pair(&reset_server);
    reset_server.reset_agentmeter().unwrap();
    assert!(
        !reset_server
            .transport_status()
            .authenticated_session_established
    );
    assert_eq!(
        status(&http(
            &reset_server,
            "GET",
            "/health",
            &[("Authorization", &format!("Bearer {}", reset_pair.session))],
            "",
        )),
        401
    );
    assert_eq!(
        status(&http(
            &reset_server,
            "POST",
            "/api/v1/session",
            &[
                ("Cookie", &format!("device_pair={}", reset_pair.device_pair)),
                ("X-CSRF-Token", &reset_pair.pair_csrf),
            ],
            "{}",
        )),
        401
    );
}

#[test]
fn dashboard_preserves_source_usage_and_deduplicates_account_quota() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let paired = pair(&server);
    let authorization = format!("Bearer {}", paired.session);
    let response = http(
        &server,
        "GET",
        "/api/v1/dashboard",
        &[("Authorization", &authorization)],
        "",
    );
    assert_eq!(status(&response), 200);
    let snapshot = body(&response);
    assert_eq!(snapshot["schema_version"], 1);
    assert!(!snapshot["stream_id"].as_str().unwrap().is_empty());
    assert_eq!(snapshot["providers"].as_array().unwrap().len(), 4);
    let codex = snapshot["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|provider| provider["provider"] == "codex")
        .unwrap();
    assert_eq!(codex["source_usage"].as_array().unwrap().len(), 2);
    assert_eq!(codex["quota_windows"].as_array().unwrap().len(), 1);
    for field in [
        "setup",
        "paused",
        "availability",
        "collection_state",
        "freshness",
        "data_quality",
        "collector_maturity",
    ] {
        assert!(codex.get(field).is_some(), "missing {field}");
    }
}

#[test]
fn sse_orders_complete_revisions_and_refresh_is_async_coalesced_then_throttled() {
    let server = TabletServer::start(ServerConfig::for_tests()).unwrap();
    let paired = pair(&server);
    let authorization = format!("Bearer {}", paired.session);
    let initial = body(&http(
        &server,
        "GET",
        "/api/v1/dashboard",
        &[("Authorization", &authorization)],
        "",
    ));
    let stream_id = initial["stream_id"].as_str().unwrap().to_owned();
    let initial_revision = initial["revision"].as_u64().unwrap();

    let mut events = TcpStream::connect(server.addr()).unwrap();
    let event_request = format!(
        "GET /api/v1/events HTTP/1.1\r\nHost: {}\r\nOrigin: {}\r\nAuthorization: {authorization}\r\nLast-Event-ID: {stream_id}:0\r\nConnection: close\r\n\r\n",
        server.addr(),
        server.origin()
    );
    events.write_all(event_request.as_bytes()).unwrap();

    let started = Instant::now();
    let accepted = http(
        &server,
        "POST",
        "/api/v1/refresh",
        &[
            ("Authorization", &authorization),
            ("X-CSRF-Token", &paired.csrf),
        ],
        r#"{"source":"codex"}"#,
    );
    assert_eq!(status(&accepted), 202);
    assert_eq!(body(&accepted)["result"], "accepted");
    assert!(started.elapsed() < Duration::from_millis(100));

    let coalesced = http(
        &server,
        "POST",
        "/api/v1/refresh",
        &[
            ("Authorization", &authorization),
            ("X-CSRF-Token", &paired.csrf),
        ],
        r#"{"source":"codex"}"#,
    );
    assert_eq!(status(&coalesced), 202);
    assert_eq!(body(&coalesced)["result"], "coalesced");

    let mut event_response = String::new();
    events.read_to_string(&mut event_response).unwrap();
    assert!(event_response.starts_with("HTTP/1.1 200 OK"));
    assert!(event_response.contains("Content-Type: text/event-stream"));
    assert!(event_response.matches("event: dashboard").count() >= 2);
    assert!(event_response.contains(": heartbeat"));
    assert!(event_response.contains(&format!("id: {stream_id}:{initial_revision}")));
    assert!(event_response.contains(&format!("id: {stream_id}:{}", initial_revision + 1)));

    let latest = body(&http(
        &server,
        "GET",
        "/api/v1/dashboard",
        &[("Authorization", &authorization)],
        "",
    ));
    assert_eq!(latest["revision"], initial_revision + 1);
    let throttled = http(
        &server,
        "POST",
        "/api/v1/refresh",
        &[
            ("Authorization", &authorization),
            ("X-CSRF-Token", &paired.csrf),
        ],
        r#"{"source":"codex"}"#,
    );
    assert_eq!(status(&throttled), 202);
    assert_eq!(body(&throttled)["result"], "throttled");

    let resumed = http(
        &server,
        "GET",
        "/api/v1/events",
        &[
            ("Authorization", &authorization),
            ("Last-Event-ID", &format!("{stream_id}:{initial_revision}")),
        ],
        "",
    );
    assert!(resumed.contains(&format!("id: {stream_id}:{}", initial_revision + 1)));
    assert!(!resumed.contains(&format!("id: {stream_id}:{initial_revision}\n")));
}

#[test]
fn collector_outcomes_remain_visible_while_another_provider_updates() {
    for (behavior, availability, collection, failure) in [
        (MockCollectorBehavior::Slow, "available", "collecting", None),
        (
            MockCollectorBehavior::Failed,
            "available",
            "error",
            Some("collector_failed"),
        ),
        (MockCollectorBehavior::Paused, "available", "idle", None),
        (
            MockCollectorBehavior::Unsupported,
            "unsupported",
            "idle",
            Some("unsupported"),
        ),
        (
            MockCollectorBehavior::SchemaChanged,
            "available",
            "error",
            Some("schema_changed"),
        ),
    ] {
        let mut config = ServerConfig::for_tests();
        config
            .collector_behaviors
            .insert("antigravity".to_owned(), behavior);
        let server = TabletServer::start(config).unwrap();
        let paired = pair(&server);
        let authorization = format!("Bearer {}", paired.session);
        let initial = body(&http(
            &server,
            "GET",
            "/api/v1/dashboard",
            &[("Authorization", &authorization)],
            "",
        ));
        let initial_revision = initial["revision"].as_u64().unwrap();
        let antigravity = initial["providers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|provider| provider["provider"] == "antigravity")
            .unwrap()
            .clone();
        assert_eq!(antigravity["availability"], availability);
        assert_eq!(antigravity["collection_state"], collection);
        match failure {
            Some(code) => assert_eq!(antigravity["failure_code"], code),
            None => assert!(antigravity["failure_code"].is_null()),
        }

        let accepted = http(
            &server,
            "POST",
            "/api/v1/refresh",
            &[
                ("Authorization", &authorization),
                ("X-CSRF-Token", &paired.csrf),
            ],
            r#"{"source":"codex"}"#,
        );
        assert_eq!(body(&accepted)["result"], "accepted");
        std::thread::sleep(Duration::from_millis(50));
        let updated = body(&http(
            &server,
            "GET",
            "/api/v1/dashboard",
            &[("Authorization", &authorization)],
            "",
        ));
        assert_eq!(updated["revision"], initial_revision + 1);
        let unchanged = updated["providers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|provider| provider["provider"] == "antigravity")
            .unwrap();
        assert_eq!(unchanged["availability"], availability);
        assert_eq!(unchanged["collection_state"], collection);
        assert_eq!(unchanged["failure_code"], antigravity["failure_code"]);
    }
}
