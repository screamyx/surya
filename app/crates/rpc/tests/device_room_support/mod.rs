//! Shared scaffolding for the device-room tests: an in-memory fake device
//! room, a test RPC service, and the link/relay constructors.
//!
//! The fake implements the `DeviceRoom` DO's relay semantics
//! (edge/src/device-room.ts): route client frames to the single host socket
//! with `from` stamped; route host frames by `to` (bounce `client_gone` when
//! the target left); host supersede (a new host join closes the predecessor);
//! `client_closed` on client disconnect; `host_closed` broadcast on host
//! disconnect; `host_offline` bounce when a client sends with no host; nudge
//! frames delivered to the host.
//!
//! Split out of `device_room.rs` when that file crossed the 500-line cap
//! (decision 13). A pure move: nothing here changed on the way out.

// tungstenite's `accept_hdr_async` callback signature fixes the Err type as a full
// `Response` - its size is not ours to shrink.
#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::handshake::server::{
    Request as WsRequest, Response as WsResponse,
};

use surya_rpc::device_room::{
    CLIENT_CLOSED, CLIENT_GONE, HOST_CLOSED, HOST_OFFLINE, NUDGE_KIND, RELAY_KIND,
};
use surya_rpc::{
    DeviceFrameHeader, HostRelayConfig, LinkCache, LinkCacheConfig, RpcError, RpcReply,
    RpcService, StaticToken, TokenSource, decode_device_frame, encode_device_frame, methods,
};

// ---------------------------------------------------------------------------
// Fake relay (the DO semantics, in-memory)
// ---------------------------------------------------------------------------

enum Out {
    Frame(Vec<u8>),
    Close,
}

#[derive(Default)]
struct RelayState {
    host: Option<mpsc::UnboundedSender<Out>>,
    clients: HashMap<String, mpsc::UnboundedSender<Out>>,
    /// Zombie-path simulation: the host stays "connected" (no bounce) but
    /// client-to-host frames vanish - the 2026-08-19 dead edge-host leg.
    blackhole_host_bound: bool,
}

pub struct FakeRelay {
    port: u16,
    state: Arc<Mutex<RelayState>>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for FakeRelay {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl FakeRelay {
    pub async fn start() -> FakeRelay {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let state = Arc::new(Mutex::new(RelayState::default()));
        let accept_state = state.clone();
        let task = tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else {
                    break;
                };
                tokio::spawn(handle_socket(stream, accept_state.clone()));
            }
        });
        FakeRelay { port, state, task }
    }

    pub fn edge_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn host_connected(&self) -> bool {
        self.state.lock().expect("lock").host.is_some()
    }

    pub async fn wait_host_connected(&self) {
        wait_until(|| self.host_connected()).await;
    }

    /// Swallow client-to-host frames while keeping the host registered - the
    /// zombie relay path (client-to-edge healthy, edge-to-host dead).
    pub fn set_blackhole_host_bound(&self, on: bool) {
        self.state.lock().expect("lock").blackhole_host_bound = on;
    }

    /// Deliver a nudge frame to the connected host (the DO's /nudge live path).
    pub fn nudge(&self, chat_id: &str) {
        let header = DeviceFrameHeader::new(chat_id, NUDGE_KIND);
        let payload = serde_json::json!({ "chatId": chat_id }).to_string();
        let frame = encode_device_frame(&header, payload.as_bytes()).expect("encode nudge");
        let state = self.state.lock().expect("lock");
        state
            .host
            .as_ref()
            .expect("host connected")
            .send(Out::Frame(frame))
            .expect("send");
    }
}

fn relay_error(code: &str) -> Vec<u8> {
    serde_json::json!({ "error": code })
        .to_string()
        .into_bytes()
}

async fn handle_socket(stream: tokio::net::TcpStream, state: Arc<Mutex<RelayState>>) {
    let mut uri = String::new();
    let ws =
        match tokio_tungstenite::accept_hdr_async(stream, |req: &WsRequest, res: WsResponse| {
            uri = req.uri().to_string();
            Ok(res)
        })
        .await
        {
            Ok(ws) => ws,
            Err(_) => return,
        };
    let query: HashMap<String, String> = uri
        .split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .filter_map(|kv| kv.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let is_host = query.get("role").map(String::as_str) == Some("host");
    let conn_id = query
        .get("connId")
        .cloned()
        .unwrap_or_else(|| "anon".into());

    let (mut sink, mut ws_stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Out>();
    {
        let mut st = state.lock().expect("lock");
        if is_host {
            // One live host socket: close any predecessor (backend restart / supersede).
            if let Some(old) = st.host.take() {
                let _ = old.send(Out::Close);
            }
            st.host = Some(tx.clone());
        } else {
            st.clients.insert(conn_id.clone(), tx.clone());
        }
    }

    let writer = tokio::spawn(async move {
        while let Some(out) = rx.recv().await {
            match out {
                Out::Frame(bytes) => {
                    if sink.send(WsMessage::Binary(bytes)).await.is_err() {
                        break;
                    }
                }
                Out::Close => {
                    let _ = sink.send(WsMessage::Close(None)).await;
                    break;
                }
            }
        }
    });

    while let Some(message) = ws_stream.next().await {
        let bytes = match message {
            Ok(WsMessage::Binary(bytes)) => bytes,
            Ok(WsMessage::Close(_)) | Err(_) => break,
            Ok(_) => continue,
        };
        let Ok((header, payload)) = decode_device_frame(&bytes) else {
            break;
        };
        let st = state.lock().expect("lock");
        if !is_host {
            if st.blackhole_host_bound {
                continue; // zombie path: frame vanishes, no bounce
            }
            match &st.host {
                Some(host) => {
                    let mut routed = DeviceFrameHeader::new(header.s, header.k);
                    routed.from = Some(conn_id.clone());
                    let _ = host.send(Out::Frame(
                        encode_device_frame(&routed, &payload).expect("encode"),
                    ));
                }
                None => {
                    let bounce = DeviceFrameHeader::new(header.s, RELAY_KIND);
                    let _ = tx.send(Out::Frame(
                        encode_device_frame(&bounce, &relay_error(HOST_OFFLINE)).expect("encode"),
                    ));
                }
            }
            continue;
        }
        // Host frame: route by `to`.
        let Some(to) = header.to else { continue };
        match st.clients.get(&to) {
            Some(client) => {
                let stripped = DeviceFrameHeader::new(header.s, header.k);
                let _ = client.send(Out::Frame(
                    encode_device_frame(&stripped, &payload).expect("encode"),
                ));
            }
            None => {
                let bounce = DeviceFrameHeader::new(header.s, RELAY_KIND).with_to(to);
                let _ = tx.send(Out::Frame(
                    encode_device_frame(&bounce, &relay_error(CLIENT_GONE)).expect("encode"),
                ));
            }
        }
    }

    // Disconnect bookkeeping (mirrors DeviceRoom.webSocketClose).
    {
        let mut st = state.lock().expect("lock");
        if is_host {
            if st.host.as_ref().is_some_and(|h| h.same_channel(&tx)) {
                st.host = None;
            }
            for client in st.clients.values() {
                let header = DeviceFrameHeader::new("", RELAY_KIND);
                let _ = client.send(Out::Frame(
                    encode_device_frame(&header, &relay_error(HOST_CLOSED)).expect("encode"),
                ));
            }
        } else {
            st.clients.remove(&conn_id);
            if let Some(host) = &st.host {
                let mut header = DeviceFrameHeader::new("", RELAY_KIND);
                header.from = Some(conn_id.clone());
                let _ = host.send(Out::Frame(
                    encode_device_frame(&header, &relay_error(CLIENT_CLOSED)).expect("encode"),
                ));
            }
        }
    }
    writer.abort();
}

// ---------------------------------------------------------------------------
// Test service + helpers
// ---------------------------------------------------------------------------

pub struct TestService {
    label: String,
    pub active_streams: Arc<AtomicUsize>,
}

impl TestService {
    pub fn new(label: &str) -> Arc<Self> {
        Arc::new(Self {
            label: label.into(),
            active_streams: Arc::new(AtomicUsize::new(0)),
        })
    }
}

struct StreamGuard(Arc<AtomicUsize>);

impl Drop for StreamGuard {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

#[async_trait]
impl RpcService for TestService {
    async fn handle(&self, method: &str, params: serde_json::Value) -> Result<RpcReply, RpcError> {
        match method {
            methods::LIST_HARNESSES => Ok(RpcReply::Value(serde_json::json!([]))),
            "Echo" => Ok(RpcReply::Value(
                serde_json::json!({ "host": self.label, "params": params }),
            )),
            "Count" => {
                let n = params.get("n").and_then(|v| v.as_u64()).unwrap_or(0);
                Ok(RpcReply::Stream(
                    futures::stream::iter((0..n).map(|i| serde_json::json!(i))).boxed(),
                ))
            }
            "Never" => {
                self.active_streams.fetch_add(1, Ordering::SeqCst);
                let guard = StreamGuard(self.active_streams.clone());
                Ok(RpcReply::Stream(
                    futures::stream::poll_fn(move |_| {
                        let _keep = &guard;
                        std::task::Poll::Pending
                    })
                    .boxed(),
                ))
            }
            other => Err(RpcError::UnknownMethod(other.into())),
        }
    }
}

/// Poll `check` until it holds. The deadline is `surya_test_deadlines::WAIT`,
/// not a hand-picked span: it exists to stop a hung test, not to assert speed.
pub async fn wait_until(mut check: impl FnMut() -> bool) {
    let deadline = tokio::time::Instant::now() + surya_test_deadlines::WAIT;
    loop {
        if check() {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "condition not reached within {:?}",
            surya_test_deadlines::WAIT
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

pub fn relay_config(edge_url: &str, retry_ms: u64) -> HostRelayConfig {
    let mut config =
        HostRelayConfig::new(edge_url, "dev-a", Arc::new(StaticToken("test-user".into())));
    config.retry = Duration::from_millis(retry_ms);
    config
}

pub fn cache(edge_url: &str) -> Arc<LinkCache> {
    let mut config = LinkCacheConfig::new(edge_url, Arc::new(StaticToken("test-user".into())));
    config.cooldown_base = Duration::from_millis(100);
    config.cooldown_max = Duration::from_millis(400);
    config.probe_timeout = Duration::from_millis(1_500);
    LinkCache::new(config)
}

pub fn noop_nudge() -> surya_rpc::NudgeHandler {
    Arc::new(|_| {})
}

pub struct RecoveringToken {
    value: Mutex<Option<String>>,
    changes: tokio::sync::watch::Sender<u64>,
}

impl RecoveringToken {
    pub fn new(value: Option<&str>) -> Self {
        let (changes, _) = tokio::sync::watch::channel(0);
        Self {
            value: Mutex::new(value.map(str::to_string)),
            changes,
        }
    }

    pub fn replace(&self, value: &str) {
        *self.value.lock().expect("lock") = Some(value.into());
        self.changes
            .send_modify(|epoch| *epoch = epoch.wrapping_add(1));
    }

    pub fn clear(&self) {
        *self.value.lock().expect("lock") = None;
        self.changes
            .send_modify(|epoch| *epoch = epoch.wrapping_add(1));
    }
}

#[async_trait]
impl TokenSource for RecoveringToken {
    async fn token(&self) -> Option<String> {
        self.value.lock().expect("lock").clone()
    }

    fn subscribe(&self) -> Option<tokio::sync::watch::Receiver<u64>> {
        Some(self.changes.subscribe())
    }
}
