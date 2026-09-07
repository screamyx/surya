//! Shared scaffolding for the auth tests: fake JWTs, and a stub edge HTTP
//! server on a plain tokio TcpListener that stands in for the real edge's
//! `/auth/exchange`, `/auth/refresh` and dev-mode probe.
//!
//! Split out of `auth.rs` when that file crossed the 500-line cap
//! (decision 13). A pure move: nothing here changed on the way out.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use surya_engine::AuthConfig;

// ---------------------------------------------------------------------------
// Fake JWTs
// ---------------------------------------------------------------------------

fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[(n >> 6) as usize & 63] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[n as usize & 63] as char);
        }
    }
    out
}

/// An unsigned JWT with the claims the engine reads (`exp`/`iat` for TTL, `org_id`).
pub fn fake_jwt(ttl_secs: i64, org_id: Option<&str>) -> String {
    let mut claims = serde_json::json!({ "sub": "user_1", "iat": 1_000, "exp": 1_000 + ttl_secs });
    if let Some(org) = org_id {
        claims["org_id"] = serde_json::json!(org);
    }
    format!("e30.{}.sig", base64url(claims.to_string().as_bytes()))
}

// ---------------------------------------------------------------------------
// Stub edge server
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct StubState {
    pub exchanges: AtomicUsize,
    pub block_exchange: AtomicBool,
    pub exchange_started: tokio::sync::Notify,
    pub release_exchange: tokio::sync::Notify,
    pub refreshes: AtomicUsize,
    pub drop_refresh: AtomicBool,
    /// Refresh tokens seen by /auth/refresh, in order.
    pub refresh_tokens: Mutex<Vec<String>>,
    /// TTL (seconds) for minted access tokens.
    pub token_ttl: AtomicUsize,
    /// org_id claim for exchange-minted tokens ("" = none).
    pub exchange_org: Mutex<String>,
}

pub struct StubEdge {
    port: u16,
    pub state: Arc<StubState>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for StubEdge {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl StubEdge {
    pub async fn start() -> StubEdge {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind stub");
        let port = listener.local_addr().expect("addr").port();
        let state = Arc::new(StubState::default());
        state.token_ttl.store(3600, Ordering::SeqCst);
        let handler_state = state.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(handle(stream, handler_state.clone()));
            }
        });
        StubEdge { port, state, task }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

pub async fn read_request(stream: &mut tokio::net::TcpStream) -> Option<(String, String, String)> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 1024];
    let header_end = loop {
        if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break pos + 4;
        }
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
    };
    let head = String::from_utf8_lossy(&buf[..header_end]).into_owned();
    let mut lines = head.lines();
    let request_line = lines.next()?.to_string();
    let mut content_length = 0usize;
    for line in lines {
        if let Some((k, v)) = line.split_once(':')
            && k.eq_ignore_ascii_case("content-length")
        {
            content_length = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = buf[header_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut chunk).await.ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
    }
    let mut parts = request_line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    Some((method, target, String::from_utf8_lossy(&body).into_owned()))
}

pub async fn respond(stream: &mut tokio::net::TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.shutdown().await;
}

pub async fn handle(mut stream: tokio::net::TcpStream, state: Arc<StubState>) {
    let Some((method, target, body)) = read_request(&mut stream).await else {
        return;
    };
    let path = target.split('?').next().unwrap_or("");
    let ttl = state.token_ttl.load(Ordering::SeqCst) as i64;
    match (method.as_str(), path) {
        ("GET", "/health") => {
            respond(&mut stream, "200 OK", r#"{"ok":true,"auth":"workos"}"#).await;
        }
        ("POST", "/auth/exchange") => {
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            if parsed.get("code").and_then(|v| v.as_str()).is_none() {
                respond(
                    &mut stream,
                    "400 Bad Request",
                    r#"{"error":"missing code"}"#,
                )
                .await;
                return;
            }
            let n = state.exchanges.fetch_add(1, Ordering::SeqCst) + 1;
            if state.block_exchange.load(Ordering::SeqCst) {
                state.exchange_started.notify_one();
                state.release_exchange.notified().await;
            }
            let org = state.exchange_org.lock().expect("lock").clone();
            let token = fake_jwt(ttl, (!org.is_empty()).then_some(org.as_str()));
            let response = serde_json::json!({
                "user": { "id": "user_1", "email": "w@example.com",
                          "firstName": "Wing", "lastName": "Test" },
                "accessToken": token,
                "refreshToken": format!("refresh-{n}"),
            });
            respond(&mut stream, "200 OK", &response.to_string()).await;
        }
        ("POST", "/auth/refresh") => {
            let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let refresh_token = parsed
                .get("refreshToken")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            state
                .refresh_tokens
                .lock()
                .expect("lock")
                .push(refresh_token.to_string());
            if state.drop_refresh.load(Ordering::SeqCst) {
                state.refreshes.fetch_add(1, Ordering::SeqCst);
                return;
            }
            if refresh_token == "dead" {
                respond(&mut stream, "401 Unauthorized", r#"{"error":"revoked"}"#).await;
                return;
            }
            let n = state.refreshes.fetch_add(1, Ordering::SeqCst) + 1;
            let org = parsed.get("organizationId").and_then(|v| v.as_str());
            let response = serde_json::json!({
                "accessToken": fake_jwt(ttl, org),
                "refreshToken": format!("rotated-{n}"),
            });
            respond(&mut stream, "200 OK", &response.to_string()).await;
        }
        ("GET", "/auth/orgs") => {
            respond(
                &mut stream,
                "200 OK",
                r#"{"orgs":[{"id":"om_1","organizationId":"org_1","name":"Acme"}]}"#,
            )
            .await;
        }
        ("POST", "/auth/orgs") => {
            respond(&mut stream, "200 OK", r#"{"organizationId":"org_new"}"#).await;
        }
        _ => respond(&mut stream, "404 Not Found", r#"{"error":"not_found"}"#).await,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wait until the stub edge has served at least `n` refreshes.
///
/// Polls the counter instead of sleeping a fixed span: the deadline only stops
/// a hung test, it is not an assertion about how fast the loop runs.
pub async fn wait_for_refreshes(state: &Arc<StubState>, n: usize) {
    let deadline = tokio::time::Instant::now() + surya_test_deadlines::WAIT;
    while state.refreshes.load(Ordering::SeqCst) < n {
        assert!(
            tokio::time::Instant::now() < deadline,
            "stub edge served {} refreshes, wanted {n}",
            state.refreshes.load(Ordering::SeqCst)
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub fn workos_config(edge_url: &str, data_dir: &std::path::Path) -> AuthConfig {
    let mut config = AuthConfig::new(edge_url, data_dir);
    config.workos_client_id = Some("client_test".into());
    config.workos_api_base = "https://authkit.example".into();
    config
}

pub fn query_param(url: &str, key: &str) -> Option<String> {
    url.split_once('?')?
        .1
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v.to_string())
}

pub async fn wait_for<T: Clone + PartialEq>(
    rx: &mut tokio::sync::watch::Receiver<T>,
    check: impl Fn(&T) -> bool,
) {
    tokio::time::timeout(surya_test_deadlines::WAIT, async {
        loop {
            if check(&rx.borrow()) {
                return;
            }
            rx.changed().await.expect("state channel open");
        }
    })
    .await
    .expect("state reached in time");
}
