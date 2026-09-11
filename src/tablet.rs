//! Minimal production-shaped tablet HTTP slice for P0 validation.
//!
//! The transport adapter deliberately uses `std::net` because Axum is not in
//! the approved offline crate cache. Public behavior is kept at the HTTP seam
//! so the adapter can be replaced without changing the security/state model.

use crate::live_dashboard::{LiveDashboard, PROVIDERS};
use crate::pair_store::{PairStore, StoredPair};

pub type LiveRefresh = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PRIVATE_ROUTES: &[&str] = &[
    "/health",
    "/api/v1/dashboard",
    "/api/v1/history",
    "/api/v1/events",
    "/api/v1/refresh",
    "/api/v1/pairs",
    "/api/v1/monitor/ping",
];

#[derive(Clone, Debug)]
pub struct ServerConfig {
    /// Explicit host loopback port; zero requests an ephemeral probe port.
    pub listen_port: u16,
    /// Opt-in durable Windows DPAPI store; None is the isolated ephemeral probe.
    pub pair_store_path: Option<std::path::PathBuf>,
    pub pair_code_ttl: Duration,
    pub session_ttl: Duration,
    pub refresh_throttle: Duration,
    pub refresh_work: Duration,
    pub sse_heartbeat: Duration,
    pub collector_behaviors: HashMap<String, MockCollectorBehavior>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MockCollectorBehavior {
    Complete,
    Slow,
    Failed,
    Paused,
    Unsupported,
    SchemaChanged,
}

impl ServerConfig {
    pub fn for_tests() -> Self {
        Self {
            listen_port: 0,
            pair_store_path: None,
            pair_code_ttl: Duration::from_millis(80),
            session_ttl: Duration::from_secs(2),
            refresh_throttle: Duration::from_millis(150),
            refresh_work: Duration::from_millis(35),
            sse_heartbeat: Duration::from_millis(15),
            collector_behaviors: default_collector_behaviors(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_port: 0,
            pair_store_path: None,
            pair_code_ttl: Duration::from_secs(120),
            session_ttl: Duration::from_secs(15 * 60),
            refresh_throttle: Duration::from_secs(10),
            refresh_work: Duration::from_millis(50),
            sse_heartbeat: Duration::from_secs(15),
            collector_behaviors: default_collector_behaviors(),
        }
    }
}

pub struct TabletServer {
    addr: SocketAddr,
    origin: String,
    state: Arc<Mutex<State>>,
    shutdown: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SafeDiagnostic {
    pub failure_code: &'static str,
    pub route: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransportStatus {
    pub authenticated_session_established: bool,
    pub authenticated_request_count: u64,
    pub last_authenticated_route: Option<String>,
    pub last_authenticated_at_unix_ms: Option<u128>,
}

#[derive(Clone)]
struct PairCode {
    value: String,
    expires_at: Instant,
    attempts: u8,
    used: bool,
}

#[derive(Clone)]
struct DevicePair {
    token: String,
    exchange_csrf: String,
    revoked: bool,
}

#[derive(Clone)]
struct TabletSession {
    pair_token: String,
    csrf: String,
    expires_at: Instant,
    revoked: bool,
}

struct State {
    usb_origin: Option<String>,
    transport_generation: u64,
    live: Option<(LiveDashboard, LiveRefresh)>,
    config: ServerConfig,
    pair_store: Option<PairStore>,
    stopped: bool,
    pair_code: PairCode,
    pairs: HashMap<String, DevicePair>,
    sessions: HashMap<String, TabletSession>,
    active_streams: HashMap<String, (String, TcpStream)>,
    stream_id: String,
    revision: u64,
    refresh_in_flight: HashSet<String>,
    last_refresh: HashMap<String, Instant>,
    diagnostics: Vec<SafeDiagnostic>,
    provider_runtime: HashMap<String, ProviderRuntime>,
    transport_status: TransportStatus,
}

#[derive(Clone)]
struct ProviderRuntime {
    availability: &'static str,
    collection_state: &'static str,
    freshness: &'static str,
    failure_code: Option<&'static str>,
    paused: bool,
}

#[derive(Serialize)]
struct DashboardSnapshot {
    provider_data: &'static str,
    schema_version: u8,
    stream_id: String,
    revision: u64,
    generated_at_unix_ms: u128,
    paused: bool,
    providers: Vec<serde_json::Value>,
}

impl TabletServer {
    pub fn start(config: ServerConfig) -> std::io::Result<Self> {
        Self::start_inner(config, None)
    }

    pub fn start_live(
        config: ServerConfig,
        dashboard: LiveDashboard,
        refresh: LiveRefresh,
    ) -> std::io::Result<Self> {
        Self::start_inner(config, Some((dashboard, refresh)))
    }

    fn start_inner(
        config: ServerConfig,
        live: Option<(LiveDashboard, LiveRefresh)>,
    ) -> std::io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, config.listen_port))?;
        listener.set_nonblocking(true)?;
        let addr = listener.local_addr()?;
        if !addr.ip().is_loopback() {
            return Err(std::io::Error::other("tablet listener is not loopback"));
        }
        let origin = format!("http://{addr}");
        let mut initial = State::new(config)?;
        initial.live = live;
        let state = Arc::new(Mutex::new(initial));
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_state = Arc::clone(&state);
        let thread_shutdown = Arc::clone(&shutdown);
        let thread_origin = origin.clone();
        let accept_thread = thread::spawn(move || {
            while !thread_shutdown.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let state = Arc::clone(&thread_state);
                        let origin = thread_origin.clone();
                        thread::spawn(move || handle_connection(stream, state, &origin));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            addr,
            origin,
            state,
            shutdown,
            accept_thread: Some(accept_thread),
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    /// Allow only the loopback origin of a verified, selected USB mapping.
    pub fn set_usb_device_port(&self, port: u16) {
        assert_ne!(port, 0);
        self.state.lock().unwrap().usb_origin = Some(format!("http://127.0.0.1:{port}"));
    }

    pub fn transport_status(&self) -> TransportStatus {
        let mut state = self.state.lock().unwrap();
        state.close_unauthorized_streams();
        state.transport_status.authenticated_session_established = state
            .sessions
            .iter()
            .any(|(token, _)| state.session_authorized(token));
        state.transport_status.clone()
    }

    /// A selected-device mapping generation changed or became untrustworthy.
    /// Durable Device Pairs remain, but every transport-bound session and SSE
    /// connection is revoked before a new mapping can be trusted.
    pub fn revoke_sessions_for_transport_change(&self) {
        let mut state = self.state.lock().unwrap();
        state.usb_origin = None;
        state.transport_generation = state.transport_generation.wrapping_add(1);
        for session in state.sessions.values_mut() {
            session.revoked = true;
        }
        state.transport_status.authenticated_session_established = false;
        state.close_unauthorized_streams();
    }

    /// Desktop-only setup seam. The code is never exposed by an HTTP URL.
    pub fn pair_code_for_setup(&self) -> String {
        self.state.lock().unwrap().pair_code.value.clone()
    }

    pub fn rotate_pair_code(&self) -> String {
        let mut state = self.state.lock().unwrap();
        state.pair_code = PairCode::new(state.config.pair_code_ttl);
        state.pair_code.value.clone()
    }

    /// Desktop Forget Account invalidates sessions but preserves Device Pairs.
    pub fn forget_account(&self) {
        let mut state = self.state.lock().unwrap();
        for session in state.sessions.values_mut() {
            session.revoked = true;
        }
        state.close_unauthorized_streams();
    }

    /// Desktop Clear Pairing invalidates Device Pairs and Tablet Sessions.
    pub fn clear_pairing(&self) -> std::io::Result<()> {
        let mut state = self.state.lock().unwrap();
        for pair in state.pairs.values_mut() {
            pair.revoked = true;
        }
        for session in state.sessions.values_mut() {
            session.revoked = true;
        }
        state.close_unauthorized_streams();
        if let Some(store) = &state.pair_store
            && let Err(error) = store.save(&[])
        {
            state.stopped = true;
            state.diagnostics.push(SafeDiagnostic {
                failure_code: "pair_store_write_failed",
                route: "desktop/clear_pairing".into(),
            });
            return Err(error);
        }
        state.pairs.clear();
        Ok(())
    }

    pub fn reset_agentmeter(&self) -> std::io::Result<()> {
        self.clear_pairing()
    }

    pub fn diagnostics(&self) -> Vec<SafeDiagnostic> {
        self.state.lock().unwrap().diagnostics.clone()
    }
}

impl Drop for TabletServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        {
            let mut state = self.state.lock().unwrap();
            state.stopped = true;
            state.close_unauthorized_streams();
            // No further pair mutation is authorized. Release ownership even
            // if a stopped HTTP or mock-refresh worker still holds the Arc.
            state.pair_store.take();
        }
        let _ = TcpStream::connect(self.addr);
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

impl State {
    fn session_authorized(&self, token: &str) -> bool {
        !self.stopped
            && self.sessions.get(token).is_some_and(|session| {
                !session.revoked
                    && Instant::now() < session.expires_at
                    && self
                        .pairs
                        .get(&session.pair_token)
                        .is_some_and(|pair| !pair.revoked)
            })
    }

    fn close_unauthorized_streams(&mut self) {
        let ended: Vec<String> = self
            .active_streams
            .iter()
            .filter(|(_, (token, _))| !self.session_authorized(token))
            .map(|(id, _)| id.clone())
            .collect();
        for id in ended {
            if let Some((_, socket)) = self.active_streams.remove(&id) {
                let _ = socket.shutdown(std::net::Shutdown::Both);
                self.diagnostics.push(SafeDiagnostic {
                    failure_code: "stream_authorization_ended",
                    route: "/api/v1/events".into(),
                });
            }
        }
    }

    fn new(config: ServerConfig) -> std::io::Result<Self> {
        let pair_store = config
            .pair_store_path
            .as_deref()
            .map(PairStore::open)
            .transpose()?;
        let mut pairs = HashMap::new();
        if let Some(store) = &pair_store {
            for stored in store.load()? {
                pairs.insert(
                    stored.token.clone(),
                    DevicePair {
                        token: stored.token,
                        exchange_csrf: stored.exchange_csrf,
                        revoked: stored.revoked,
                    },
                );
            }
        }
        let provider_runtime = ["codex", "claude", "copilot", "antigravity"]
            .into_iter()
            .map(|provider| {
                let behavior = config
                    .collector_behaviors
                    .get(provider)
                    .cloned()
                    .unwrap_or(MockCollectorBehavior::Complete);
                (provider.to_owned(), runtime_for(&behavior))
            })
            .collect();
        Ok(Self {
            pair_code: PairCode::new(config.pair_code_ttl),
            usb_origin: None,
            transport_generation: 0,
            live: None,
            config,
            pairs,
            pair_store,
            stopped: false,
            sessions: HashMap::new(),
            active_streams: HashMap::new(),
            stream_id: random_hex(16),
            revision: 1,
            refresh_in_flight: HashSet::new(),
            last_refresh: HashMap::new(),
            diagnostics: Vec::new(),
            provider_runtime,
            transport_status: TransportStatus {
                authenticated_session_established: false,
                authenticated_request_count: 0,
                last_authenticated_route: None,
                last_authenticated_at_unix_ms: None,
            },
        })
    }

    fn record_authenticated_activity(&mut self, route: &str, establishes_session: bool) {
        self.transport_status.authenticated_session_established |= establishes_session;
        self.transport_status.authenticated_request_count = self
            .transport_status
            .authenticated_request_count
            .saturating_add(1);
        self.transport_status.last_authenticated_route = Some(route.to_owned());
        self.transport_status.last_authenticated_at_unix_ms = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        );
    }

    fn snapshot(&self) -> DashboardSnapshot {
        if let Some((dashboard, _)) = &self.live {
            let (revision, providers) = dashboard.snapshot();
            return DashboardSnapshot {
                provider_data: "live",
                schema_version: 1,
                stream_id: self.stream_id.clone(),
                revision,
                generated_at_unix_ms: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis(),
                paused: false,
                providers,
            };
        }
        DashboardSnapshot {
            provider_data: "mock",
            schema_version: 1,
            stream_id: self.stream_id.clone(),
            revision: self.revision,
            generated_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
            paused: false,
            providers: ["codex", "claude", "copilot", "antigravity"]
                .into_iter()
                .map(|id| {
                    provider(
                        id,
                        self.provider_runtime
                            .get(id)
                            .expect("provider runtime exists"),
                        if matches!(id, "codex" | "copilot") {
                            "official"
                        } else {
                            "local_observed"
                        },
                        id == "codex",
                    )
                })
                .collect(),
        }
    }
}

fn default_collector_behaviors() -> HashMap<String, MockCollectorBehavior> {
    [
        ("codex", MockCollectorBehavior::Complete),
        ("claude", MockCollectorBehavior::Slow),
        ("copilot", MockCollectorBehavior::Paused),
        ("antigravity", MockCollectorBehavior::SchemaChanged),
    ]
    .into_iter()
    .map(|(provider, behavior)| (provider.to_owned(), behavior))
    .collect()
}

fn runtime_for(behavior: &MockCollectorBehavior) -> ProviderRuntime {
    match behavior {
        MockCollectorBehavior::Complete => ProviderRuntime {
            availability: "available",
            collection_state: "ready",
            freshness: "fresh",
            failure_code: None,
            paused: false,
        },
        MockCollectorBehavior::Slow => ProviderRuntime {
            availability: "available",
            collection_state: "collecting",
            freshness: "stale",
            failure_code: None,
            paused: false,
        },
        MockCollectorBehavior::Failed => ProviderRuntime {
            availability: "available",
            collection_state: "error",
            freshness: "stale",
            failure_code: Some("collector_failed"),
            paused: false,
        },
        MockCollectorBehavior::Paused => ProviderRuntime {
            availability: "available",
            collection_state: "idle",
            freshness: "stale",
            failure_code: None,
            paused: true,
        },
        MockCollectorBehavior::Unsupported => ProviderRuntime {
            availability: "unsupported",
            collection_state: "idle",
            freshness: "unknown",
            failure_code: Some("unsupported"),
            paused: false,
        },
        MockCollectorBehavior::SchemaChanged => ProviderRuntime {
            availability: "available",
            collection_state: "error",
            freshness: "stale",
            failure_code: Some("schema_changed"),
            paused: false,
        },
    }
}

impl PairCode {
    fn new(ttl: Duration) -> Self {
        let mut bytes = [0_u8; 4];
        getrandom::fill(&mut bytes).expect("OS random source is required for pairing");
        let number = u32::from_le_bytes(bytes) % 100_000_000;
        Self {
            value: format!("{number:08}"),
            expires_at: Instant::now() + ttl,
            attempts: 0,
            used: false,
        }
    }
}

fn provider(
    id: &str,
    runtime: &ProviderRuntime,
    data_quality: &str,
    multi_source: bool,
) -> serde_json::Value {
    let source_usage = if multi_source {
        json!([
            {"source":"native-windows","used":10,"unit":"requests"},
            {"source":"wsl-ubuntu","used":4,"unit":"requests"}
        ])
    } else {
        json!([{"source":"native-windows","used":1,"unit":"requests"}])
    };
    json!({
        "provider": id,
        "provider_account":"account-1",
        "setup":"ready",
        "paused": runtime.paused,
        "availability":runtime.availability,
        "collection_state":runtime.collection_state,
        "freshness":runtime.freshness,
        "failure_code":runtime.failure_code,
        "data_quality":data_quality,
        "collector_maturity":"experimental",
        "source_usage":source_usage,
        "quota_windows":[{"bucket_key":"account-window","remaining_percent":50,"unit":"percent"}]
    })
}

fn random_hex(bytes: usize) -> String {
    let mut random = vec![0_u8; bytes];
    getrandom::fill(&mut random).expect("OS random source is required for session credentials");
    let mut output = String::with_capacity(bytes * 2);
    for byte in random {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").unwrap();
    }
    output
}

fn handle_connection(mut stream: TcpStream, state: Arc<Mutex<State>>, origin: &str) {
    // Windows accepted sockets inherit the listener's nonblocking mode.
    // Each connection has its own worker and must wait for browser/ADB bytes.
    if stream.set_nonblocking(false).is_err()
        || stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .is_err()
    {
        return;
    }
    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(error) => {
            record_diagnostic(&state, "malformed_request", "unknown");
            let detail = if error.kind() == std::io::ErrorKind::InvalidData {
                "invalid_encoding"
            } else if error.kind() == std::io::ErrorKind::TimedOut {
                "request_timeout"
            } else if error.to_string().contains("incomplete request") {
                "incomplete_request"
            } else {
                "request_parse_failed"
            };
            write_response(
                &mut stream,
                400,
                "Bad Request",
                "application/json",
                &format!(r#"{{"error":"bad_request","detail":"{detail}"}}"#),
                &[],
            );
            return;
        }
    };
    let (usb_origin, transport_generation) = {
        let guard = state.lock().unwrap();
        (guard.usb_origin.clone(), guard.transport_generation)
    };
    let origin = usb_origin
        .as_deref()
        .filter(|candidate| {
            request
                .headers
                .get("host")
                .is_some_and(|host| host == &state_host(candidate))
        })
        .unwrap_or(origin);
    let host_ok = request.headers.get("host").is_some_and(|host| {
        host == &state_host(origin)
            || host == &state_host(&origin.replace("127.0.0.1", "localhost"))
    });
    if !host_ok {
        record_diagnostic(&state, "host_rejected", &request.path);
        write_json(
            &mut stream,
            403,
            "Forbidden",
            json!({"error":"forbidden"}),
            &[],
        );
        return;
    }
    if request.had_query {
        record_diagnostic(&state, "query_rejected", &request.path);
        write_json(
            &mut stream,
            400,
            "Bad Request",
            json!({"error":"query_not_allowed"}),
            &[],
        );
        return;
    }
    if request.method == "GET" {
        let image: Option<&'static [u8]> = match request.path.as_str() {
            "/assets/agentmeter-icon.png" => {
                Some(include_bytes!("../tablet-ui/assets/agentmeter-icon.png").as_slice())
            }
            "/assets/codex-icon.png" => {
                Some(include_bytes!("../tablet-ui/assets/codex-icon.png").as_slice())
            }
            "/assets/claude-icon.png" => {
                Some(include_bytes!("../tablet-ui/assets/claude-icon.png").as_slice())
            }
            "/assets/copilot-icon.png" => {
                Some(include_bytes!("../tablet-ui/assets/copilot-icon.png").as_slice())
            }
            "/assets/antigravity-icon.png" => {
                Some(include_bytes!("../tablet-ui/assets/antigravity-icon.png").as_slice())
            }
            "/assets/empty-cloud.png" => {
                Some(include_bytes!("../tablet-ui/assets/empty-cloud.png").as_slice())
            }
            _ => None,
        };
        if let Some(body) = image {
            write_binary_response(&mut stream, 200, "OK", "image/png", body, &[
                "Cache-Control: no-store".into(),
                "Referrer-Policy: no-referrer".into(),
                "X-Content-Type-Options: nosniff".into(),
                "Content-Security-Policy: default-src 'none'; frame-ancestors 'none'; form-action 'none'; base-uri 'none'".into(),
            ]);
            return;
        }
        let asset = match request.path.as_str() {
            "/recovery.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../tablet-ui/recovery.js"),
            )),
            "/protocol.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../tablet-ui/protocol.js"),
            )),
            "/view.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../tablet-ui/view.js"),
            )),
            "/" => Some((
                "text/html; charset=utf-8",
                include_str!("../tablet-ui/index.html"),
            )),
            "/client.js" => Some((
                "text/javascript; charset=utf-8",
                include_str!("../tablet-ui/client.js"),
            )),
            _ => None,
        };
        if let Some((kind, body)) = asset {
            write_response(&mut stream, 200, "OK", kind, body, &[
                "Cache-Control: no-store".into(),
                "Referrer-Policy: no-referrer".into(),
                "X-Content-Type-Options: nosniff".into(),
                "Content-Security-Policy: default-src 'none'; script-src 'self'; connect-src 'self'; style-src 'unsafe-inline'; img-src 'self'; frame-ancestors 'none'; form-action 'none'; base-uri 'none'".into(),
            ]);
            return;
        }
    }
    if request.path == "/api/v1/pair" && request.method == "POST" {
        handle_pair(&mut stream, &request, state, origin, transport_generation);
        return;
    }
    if request.path == "/api/v1/session" && request.method == "POST" {
        handle_session_exchange(&mut stream, &request, state, origin, transport_generation);
        return;
    }
    if !PRIVATE_ROUTES.contains(&request.path.as_str()) {
        write_json(
            &mut stream,
            404,
            "Not Found",
            json!({"error":"not_found"}),
            &[],
        );
        return;
    }
    if request.headers.get("origin").map(String::as_str) != Some(origin) {
        record_diagnostic(&state, "origin_rejected", &request.path);
        write_json(
            &mut stream,
            403,
            "Forbidden",
            json!({"error":"forbidden"}),
            &[],
        );
        return;
    }
    let Some((session_token, session)) = authorized_session(&request, &state) else {
        record_diagnostic(&state, "authorization_rejected", &request.path);
        write_json(
            &mut stream,
            401,
            "Unauthorized",
            json!({"error":"unauthorized"}),
            &[],
        );
        return;
    };
    state
        .lock()
        .unwrap()
        .record_authenticated_activity(&request.path, false);
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/health") => write_json(&mut stream, 200, "OK", json!({"status":"ok"}), &[]),
        ("GET" | "POST", "/api/v1/dashboard") => {
            let snapshot = state.lock().unwrap().snapshot();
            write_json(
                &mut stream,
                200,
                "OK",
                serde_json::to_value(snapshot).unwrap(),
                &[],
            );
        }
        ("GET", "/api/v1/history") => write_json(
            &mut stream,
            200,
            "OK",
            json!({"items":[],"bounded":true}),
            &[],
        ),
        ("GET", "/api/v1/pairs") => write_json(&mut stream, 200, "OK", json!({"paired":true}), &[]),
        ("GET", "/api/v1/monitor/ping") => {
            write_json(&mut stream, 200, "OK", json!({"monitor":"ok"}), &[])
        }
        ("GET" | "POST", "/api/v1/events") => {
            handle_events(&mut stream, &request, &state, &session_token)
        }
        ("POST", "/api/v1/refresh") => {
            if request.headers.get("x-csrf-token").map(String::as_str) != Some(&session.csrf) {
                record_diagnostic(&state, "csrf_rejected", &request.path);
                write_json(
                    &mut stream,
                    403,
                    "Forbidden",
                    json!({"error":"forbidden"}),
                    &[],
                );
                return;
            }
            handle_refresh(&mut stream, &request, state, session_token);
        }
        _ => write_json(
            &mut stream,
            405,
            "Method Not Allowed",
            json!({"error":"method_not_allowed"}),
            &[],
        ),
    }
}

fn state_host(origin: &str) -> String {
    origin.strip_prefix("http://").unwrap_or(origin).to_owned()
}

fn handle_pair(
    stream: &mut TcpStream,
    request: &HttpRequest,
    state: Arc<Mutex<State>>,
    origin: &str,
    transport_generation: u64,
) {
    if request.headers.get("origin").map(String::as_str) != Some(origin) {
        record_diagnostic(&state, "origin_rejected", &request.path);
        write_json(stream, 403, "Forbidden", json!({"error":"forbidden"}), &[]);
        return;
    }
    let submitted = serde_json::from_str::<serde_json::Value>(&request.body)
        .ok()
        .and_then(|value| {
            value
                .get("code")
                .and_then(|code| code.as_str())
                .map(str::to_owned)
        });
    let mut guard = state.lock().unwrap();
    if guard.transport_generation != transport_generation {
        write_json(
            stream,
            403,
            "Forbidden",
            json!({"error":"transport_changed"}),
            &[],
        );
        return;
    }
    let valid = !guard.stopped
        && !guard.pair_code.used
        && guard.pair_code.attempts < 5
        && Instant::now() < guard.pair_code.expires_at
        && submitted.as_deref() == Some(guard.pair_code.value.as_str());
    if !valid {
        guard.pair_code.attempts = guard.pair_code.attempts.saturating_add(1);
        guard.diagnostics.push(SafeDiagnostic {
            failure_code: "pairing_rejected",
            route: request.path.clone(),
        });
        write_json(
            stream,
            401,
            "Unauthorized",
            json!({"error":"pairing_failed"}),
            &[],
        );
        return;
    }
    guard.pair_code.used = true;
    let pair_token = random_hex(32);
    let session_token = random_hex(32);
    let pair_csrf = random_hex(16);
    let csrf = random_hex(16);
    let session_ttl = guard.config.session_ttl;
    if let Some(store) = &guard.pair_store {
        let mut records: Vec<StoredPair> = guard
            .pairs
            .values()
            .filter(|pair| !pair.revoked)
            .map(|pair| StoredPair {
                token: pair.token.clone(),
                exchange_csrf: pair.exchange_csrf.clone(),
                revoked: false,
            })
            .collect();
        records.push(StoredPair {
            token: pair_token.clone(),
            exchange_csrf: pair_csrf.clone(),
            revoked: false,
        });
        if store.save(&records).is_err() {
            guard.diagnostics.push(SafeDiagnostic {
                failure_code: "pair_store_write_failed",
                route: request.path.clone(),
            });
            write_json(
                stream,
                503,
                "Service Unavailable",
                json!({"error":"pair_store_unavailable"}),
                &[],
            );
            return;
        }
    }
    guard.pairs.insert(
        pair_token.clone(),
        DevicePair {
            token: pair_token.clone(),
            exchange_csrf: pair_csrf.clone(),
            revoked: false,
        },
    );
    guard.sessions.insert(
        session_token.clone(),
        TabletSession {
            pair_token: pair_token.clone(),
            csrf: csrf.clone(),
            expires_at: Instant::now() + session_ttl,
            revoked: false,
        },
    );
    guard.record_authenticated_activity(&request.path, true);
    let cookie = format!(
        "Set-Cookie: device_pair={pair_token}; HttpOnly; SameSite=Strict; Path=/api/v1/session; Max-Age=31536000"
    );
    write_json(
        stream,
        200,
        "OK",
        json!({"tablet_session":session_token,"csrf_token":csrf,"pair_csrf_token":pair_csrf}),
        &[cookie],
    );
}

fn handle_session_exchange(
    stream: &mut TcpStream,
    request: &HttpRequest,
    state: Arc<Mutex<State>>,
    origin: &str,
    transport_generation: u64,
) {
    if request.headers.get("origin").map(String::as_str) != Some(origin) {
        record_diagnostic(&state, "origin_rejected", &request.path);
        write_json(stream, 403, "Forbidden", json!({"error":"forbidden"}), &[]);
        return;
    }
    let pair_token = request
        .headers
        .get("cookie")
        .and_then(|cookie| {
            cookie
                .split(';')
                .find_map(|part| part.trim().strip_prefix("device_pair="))
        })
        .map(str::to_owned);
    let mut guard = state.lock().unwrap();
    if guard.transport_generation != transport_generation {
        write_json(
            stream,
            403,
            "Forbidden",
            json!({"error":"transport_changed"}),
            &[],
        );
        return;
    }
    if guard.stopped {
        write_json(
            stream,
            503,
            "Service Unavailable",
            json!({"error":"service_unavailable"}),
            &[],
        );
        return;
    }
    let Some((pair_token, pair)) = pair_token.and_then(|token| {
        guard
            .pairs
            .get(&token)
            .filter(|pair| !pair.revoked && pair.token == token)
            .cloned()
            .map(|pair| (token, pair))
    }) else {
        guard.diagnostics.push(SafeDiagnostic {
            failure_code: "device_pair_rejected",
            route: request.path.clone(),
        });
        write_json(
            stream,
            401,
            "Unauthorized",
            json!({"error":"unauthorized"}),
            &[],
        );
        return;
    };
    if request.headers.get("x-csrf-token").map(String::as_str) != Some(pair.exchange_csrf.as_str())
    {
        guard.diagnostics.push(SafeDiagnostic {
            failure_code: "csrf_rejected",
            route: request.path.clone(),
        });
        write_json(stream, 403, "Forbidden", json!({"error":"forbidden"}), &[]);
        return;
    }
    let session_token = random_hex(32);
    let csrf = random_hex(16);
    let expires_at = Instant::now() + guard.config.session_ttl;
    for session in guard.sessions.values_mut() {
        if session.pair_token == pair_token {
            session.revoked = true;
        }
    }
    guard.close_unauthorized_streams();
    guard.sessions.insert(
        session_token.clone(),
        TabletSession {
            pair_token,
            csrf: csrf.clone(),
            expires_at,
            revoked: false,
        },
    );
    guard.record_authenticated_activity(&request.path, true);
    write_json(
        stream,
        200,
        "OK",
        json!({"tablet_session":session_token,"csrf_token":csrf}),
        &[],
    );
}

fn record_diagnostic(state: &Arc<Mutex<State>>, failure_code: &'static str, route: &str) {
    state.lock().unwrap().diagnostics.push(SafeDiagnostic {
        failure_code,
        route: route.to_owned(),
    });
}

fn authorized_session(
    request: &HttpRequest,
    state: &Arc<Mutex<State>>,
) -> Option<(String, TabletSession)> {
    let token = request
        .headers
        .get("authorization")?
        .strip_prefix("Bearer ")?
        .to_owned();
    let guard = state.lock().unwrap();
    let session = guard.sessions.get(&token)?.clone();
    guard.session_authorized(&token).then_some((token, session))
}

fn handle_events(
    stream: &mut TcpStream,
    request: &HttpRequest,
    state: &Arc<Mutex<State>>,
    session_token: &str,
) {
    let connection_id = random_hex(16);
    let (stream_id, heartbeat) = {
        let mut guard = state.lock().unwrap();
        if !guard.session_authorized(session_token) {
            return;
        }
        let Ok(socket) = stream.try_clone() else {
            return;
        };
        guard
            .active_streams
            .insert(connection_id.clone(), (session_token.to_owned(), socket));
        (guard.stream_id.clone(), guard.config.sse_heartbeat)
    };
    let mut sent_revision = request
        .headers
        .get("last-event-id")
        .and_then(|id| id.split_once(':'))
        .filter(|(seen_stream, _)| *seen_stream == stream_id)
        .and_then(|(_, revision)| revision.parse::<u64>().ok())
        .unwrap_or(0);
    let headers = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n";
    let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
    let send = || -> std::io::Result<()> {
        stream.write_all(headers.as_bytes())?;
        for _ in 0..8 {
            // Serialize authorization, payload delivery, and revocation so no
            // application writes can occur after a revocation call returns.
            let mut guard = state.lock().unwrap();
            if !guard.session_authorized(session_token) {
                guard.close_unauthorized_streams();
                return Ok(());
            }
            let snapshot = guard.snapshot();
            if snapshot.revision > sent_revision {
                sent_revision = snapshot.revision;
                let payload = serde_json::to_string(&snapshot).unwrap();
                let event = format!(
                    "id: {stream_id}:{sent_revision}\nevent: dashboard\ndata: {payload}\n\n"
                );
                stream.write_all(event.as_bytes())?;
            }
            stream.write_all(b": heartbeat\n\n")?;
            stream.flush()?;
            drop(guard);
            let deadline = Instant::now() + heartbeat;
            while Instant::now() < deadline {
                thread::sleep(
                    deadline
                        .saturating_duration_since(Instant::now())
                        .min(Duration::from_millis(10)),
                );
                let mut guard = state.lock().unwrap();
                if !guard.session_authorized(session_token) {
                    guard.close_unauthorized_streams();
                    return Ok(());
                }
                if guard.live.is_some() && guard.snapshot().revision > sent_revision {
                    break;
                }
            }
        }
        Ok(())
    };
    let mut send = send;
    let _ = send();
    state.lock().unwrap().active_streams.remove(&connection_id);
}

fn handle_refresh(
    stream: &mut TcpStream,
    request: &HttpRequest,
    state: Arc<Mutex<State>>,
    _session_token: String,
) {
    let source = serde_json::from_str::<serde_json::Value>(&request.body)
        .ok()
        .and_then(|value| {
            value
                .get("source")
                .and_then(|source| source.as_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "all".to_owned());
    let now = Instant::now();
    let mut guard = state.lock().unwrap();
    if let Some((dashboard, refresh)) = guard.live.clone() {
        if !PROVIDERS.contains(&source.as_str()) {
            drop(guard);
            write_json(
                stream,
                400,
                "Bad Request",
                json!({"error":"unknown_source"}),
                &[],
            );
            return;
        }
        let result = if guard.refresh_in_flight.contains(&source) {
            "coalesced"
        } else if guard
            .last_refresh
            .get(&source)
            .is_some_and(|last| now.duration_since(*last) < guard.config.refresh_throttle)
        {
            "throttled"
        } else {
            guard.refresh_in_flight.insert(source.clone());
            let background_state = Arc::clone(&state);
            let provider = source.clone();
            thread::spawn(move || {
                // No HTTP-provided arguments or credentials are passed to the collector.
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| refresh(&provider)));
                if !matches!(result, Ok(Ok(()))) {
                    dashboard.failure(&provider);
                }
                let mut state = background_state.lock().unwrap();
                state.refresh_in_flight.remove(&provider);
                state.last_refresh.insert(provider, Instant::now());
            });
            "accepted"
        };
        drop(guard);
        write_json(
            stream,
            202,
            "Accepted",
            json!({"result":result,"source":source}),
            &[],
        );
        return;
    }
    let Some(behavior) = guard.config.collector_behaviors.get(&source).cloned() else {
        drop(guard);
        write_json(
            stream,
            400,
            "Bad Request",
            json!({"error":"unknown_source"}),
            &[],
        );
        return;
    };
    let result = if behavior == MockCollectorBehavior::Paused {
        "paused"
    } else if behavior == MockCollectorBehavior::Unsupported {
        "unsupported"
    } else if guard.refresh_in_flight.contains(&source) {
        "coalesced"
    } else if guard
        .last_refresh
        .get(&source)
        .is_some_and(|last| now.duration_since(*last) < guard.config.refresh_throttle)
    {
        "throttled"
    } else {
        guard.refresh_in_flight.insert(source.clone());
        let work = if behavior == MockCollectorBehavior::Slow {
            guard.config.refresh_work.saturating_mul(3)
        } else {
            guard.config.refresh_work
        };
        let background_state = Arc::clone(&state);
        let background_source = source.clone();
        let completed_runtime = match behavior {
            MockCollectorBehavior::Complete | MockCollectorBehavior::Slow => {
                runtime_for(&MockCollectorBehavior::Complete)
            }
            MockCollectorBehavior::Failed => runtime_for(&MockCollectorBehavior::Failed),
            MockCollectorBehavior::SchemaChanged => {
                runtime_for(&MockCollectorBehavior::SchemaChanged)
            }
            MockCollectorBehavior::Paused | MockCollectorBehavior::Unsupported => unreachable!(),
        };
        thread::spawn(move || {
            thread::sleep(work);
            let mut state = background_state.lock().unwrap();
            state.revision += 1;
            state.refresh_in_flight.remove(&background_source);
            state
                .provider_runtime
                .insert(background_source.clone(), completed_runtime);
            state.last_refresh.insert(background_source, Instant::now());
        });
        "accepted"
    };
    drop(guard);
    write_json(
        stream,
        202,
        "Accepted",
        json!({"result":result,"source":source}),
        &[],
    );
}

struct HttpRequest {
    method: String,
    path: String,
    had_query: bool,
    headers: HashMap<String, String>,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> std::io::Result<HttpRequest> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 2048];
    loop {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        let separator = if bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            Some((b"\r\n\r\n".as_slice(), 4))
        } else if bytes.windows(2).any(|window| window == b"\n\n") {
            Some((b"\n\n".as_slice(), 2))
        } else {
            None
        };
        if let Some((separator, separator_len)) = separator {
            let text = String::from_utf8_lossy(&bytes);
            let header_end = text.find(std::str::from_utf8(separator).unwrap()).unwrap();
            let length = text[..header_end]
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(str::trim)
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap_or(0);
            if bytes.len() >= header_end + separator_len + length {
                break;
            }
        }
        if bytes.len() > 64 * 1024 {
            return Err(std::io::Error::other("request too large"));
        }
    }
    let text = String::from_utf8(bytes).map_err(|_| std::io::Error::other("invalid UTF-8"))?;
    let (head, body) = text
        .split_once("\r\n\r\n")
        .or_else(|| text.split_once("\n\n"))
        .ok_or_else(|| std::io::Error::other("incomplete request"))?;
    let mut lines = head.lines();
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default().to_owned();
    let request_target = request_line.next().unwrap_or_default();
    let (path, had_query) = request_target
        .split_once('?')
        .map_or((request_target, false), |(path, _)| (path, true));
    let headers = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    Ok(HttpRequest {
        method,
        path: path.to_owned(),
        had_query,
        headers,
        body: body.to_owned(),
    })
}

fn write_json(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    value: serde_json::Value,
    extra_headers: &[String],
) {
    let mut safe_headers = vec![
        "Cache-Control: no-store".to_owned(),
        "Pragma: no-cache".to_owned(),
        "Referrer-Policy: no-referrer".to_owned(),
        "X-Content-Type-Options: nosniff".to_owned(),
    ];
    safe_headers.extend_from_slice(extra_headers);
    write_response(
        stream,
        status,
        reason,
        "application/json",
        &serde_json::to_string(&value).unwrap(),
        &safe_headers,
    );
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &str,
    extra_headers: &[String],
) {
    let extra = if extra_headers.is_empty() {
        String::new()
    } else {
        format!("{}\r\n", extra_headers.join("\r\n"))
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn write_binary_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    extra_headers: &[String],
) {
    let extra = if extra_headers.is_empty() {
        String::new()
    } else {
        format!("{}\r\n", extra_headers.join("\r\n"))
    };
    let headers = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n",
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes());
    let _ = stream.write_all(body);
}
